use futures::{
    channel::mpsc::{UnboundedReceiver, UnboundedSender},
    future::LocalBoxFuture,
    FutureExt,
};
use rand::Rng;
use rumpsteak::{
    channel::Bidirectional, try_session, Branch, Choice, End, Message, Receive, Role, Roles,
    Select, Send,
};
use std::{error::Error, result, time::Duration};
use tokio::{time, try_join};

type Result<T> = result::Result<T, Box<dyn Error>>;

type Channel = Bidirectional<UnboundedSender<Label>, UnboundedReceiver<Label>>;

#[derive(Roles)]
struct Roles(U, D, E);

#[derive(Role)]
#[message(Label)]
struct U(#[route(D)] Channel, #[route(E)] Channel);

#[derive(Role)]
#[message(Label)]
struct D(#[route(U)] Channel, #[route(E)] Channel);

#[derive(Role)]
#[message(Label)]
struct E(#[route(U)] Channel, #[route(D)] Channel);

#[derive(Message)]
enum Label {
    Close(Close),
    CloseDoor(CloseDoor),
    DoorClosed(DoorClosed),
    DoorOpened(DoorOpened),
    DoorStopped(DoorStopped),
    Open(Open),
    OpenDoor(OpenDoor),
    Reset(Reset),
    Stop(Stop),
}

struct Close;
struct CloseDoor;
struct DoorClosed;
struct DoorOpened;
struct DoorStopped;
struct Open;
struct OpenDoor;
struct Reset;
struct Stop;

type User = Select<E, UserChoice>;

#[derive(Choice)]
#[message(Label)]
enum UserChoice {
    CloseDoor(CloseDoor, User),
    OpenDoor(OpenDoor, User),
}

type Door = Branch<E, DoorInit>;

#[derive(Choice)]
#[message(Label)]
enum DoorInit {
    Close(Close, Send<E, DoorClosed, Branch<E, DoorReset>>),
    Open(Open, Send<E, DoorOpened, Branch<E, DoorReset>>),
    Reset(Reset, Door),
    Stop(Stop, Door),
}

#[derive(Choice)]
#[message(Label)]
enum DoorReset {
    Close(Close, Branch<E, DoorReset>),
    Open(Open, Branch<E, DoorReset>),
    Reset(Reset, Door),
    Stop(Stop, Branch<E, DoorReset>),
}

type Elevator = Send<D, Reset, Branch<U, ElevatorClosed>>;

#[derive(Choice)]
#[message(Label)]
enum ElevatorClosed {
    CloseDoor(CloseDoor, Branch<U, ElevatorClosed>),
    OpenDoor(OpenDoor, ElevatorOpening),
}

type ElevatorOpening = Send<D, Open, Receive<D, DoorOpened, ElevatorOpened>>;

type ElevatorOpened = Send<D, Reset, Send<D, Close, Send<D, Stop, Branch<D, ElevatorStopping>>>>;

#[derive(Choice)]
#[message(Label)]
#[allow(clippy::enum_variant_names)]
enum ElevatorStopping {
    DoorStopped(DoorStopped, ElevatorOpening),
    DoorOpened(DoorOpened, ElevatorOpened),
    DoorClosed(DoorClosed, Elevator),
}

enum Never {}

async fn sleep(mut rng: impl Rng) {
    time::sleep(Duration::from_micros(rng.gen_range(0..500))).await;
}

async fn user(role: &mut U) -> Result<Never> {
    let mut rng = rand::thread_rng();
    try_session(|mut s: User| async {
        loop {
            s = if rng.gen() {
                println!("user: close");
                s.select(role, CloseDoor).await?
            } else {
                println!("user: open");
                s.select(role, OpenDoor).await?
            };

            sleep(&mut rng).await;
        }
    })
    .await
}

async fn door(role: &mut D) -> Result<Never> {
    try_session(|mut s: Door| async {
        async fn reset(mut s: Branch<E, DoorReset>, role: &mut D) -> Result<Door> {
            loop {
                s = match s.branch(role).await? {
                    DoorReset::Close(Close, s)
                    | DoorReset::Open(Open, s)
                    | DoorReset::Stop(Stop, s) => s,
                    DoorReset::Reset(Reset, s) => break Ok(s),
                }
            }
        }

        loop {
            s = match s.branch(role).await? {
                DoorInit::Close(Close, s) => {
                    println!("door: close");
                    reset(s.send(role, DoorClosed).await?, role).await?
                }
                DoorInit::Open(Open, s) => {
                    println!("door: open");
                    reset(s.send(role, DoorOpened).await?, role).await?
                }
                DoorInit::Reset(Reset, s) | DoorInit::Stop(Stop, s) => s,
            };
        }
    })
    .await
}

async fn elevator(role: &mut E) -> Result<Never> {
    fn opened<'a>(
        s: ElevatorOpened,
        role: &'a mut E,
        mut rng: impl Rng + 'a,
    ) -> LocalBoxFuture<'a, Result<(Never, End)>> {
        async move {
            let s = s.send(role, Reset).await?;
            sleep(&mut rng).await;

            let s = s.send(role, Close).await?.send(role, Stop).await?;
            match s.branch(role).await? {
                ElevatorStopping::DoorStopped(DoorStopped, s) => opening(s, role, rng).await,
                ElevatorStopping::DoorOpened(DoorOpened, s) => opened(s, role, rng).await,
                ElevatorStopping::DoorClosed(DoorClosed, s) => elevator(s, role, rng).await,
            }
        }
        .boxed_local()
    }

    fn opening<'a>(
        s: ElevatorOpening,
        role: &'a mut E,
        rng: impl Rng + 'a,
    ) -> LocalBoxFuture<'a, Result<(Never, End)>> {
        async move {
            let (DoorOpened, s) = s.send(role, Open).await?.receive(role).await?;
            opened(s, role, rng).await
        }
        .boxed_local()
    }

    fn closed<'a>(
        s: Branch<U, ElevatorClosed>,
        role: &'a mut E,
        rng: impl Rng + 'a,
    ) -> LocalBoxFuture<'a, Result<(Never, End)>> {
        async move {
            match s.branch(role).await? {
                ElevatorClosed::CloseDoor(CloseDoor, s) => closed(s, role, rng).await,
                ElevatorClosed::OpenDoor(OpenDoor, s) => opening(s, role, rng).await,
            }
        }
        .boxed_local()
    }

    fn elevator<'a>(
        s: Elevator,
        role: &'a mut E,
        rng: impl Rng + 'a,
    ) -> LocalBoxFuture<'a, Result<(Never, End)>> {
        async move {
            let s = s.send(role, Reset).await?;
            closed(s, role, rng).await
        }
        .boxed_local()
    }

    let rng = rand::thread_rng();
    try_session(|s| async { elevator(s, role, rng).await }).await
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let Roles(mut u, mut d, mut e) = Roles::default();
    try_join!(user(&mut u), door(&mut d), elevator(&mut e)).unwrap();
}
