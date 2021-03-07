use futures::{
    channel::mpsc::{UnboundedReceiver, UnboundedSender},
    future::LocalBoxFuture,
    FutureExt,
};
use rand::Rng;
use rumpsteak::{
    channel::Bidirectional, session, try_session, Branch, End, Message, Receive, Role, Roles,
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

#[session]
type User = Select<E, UserChoice>;

#[session]
enum UserChoice {
    CloseDoor(CloseDoor, User),
    OpenDoor(OpenDoor, User),
}

#[session]
type Door = Branch<E, DoorInit>;

#[session]
enum DoorInit {
    Close(Close, Send<E, DoorClosed, Branch<E, DoorReset>>),
    Open(Open, Send<E, DoorOpened, Branch<E, DoorReset>>),
    Reset(Reset, Door),
    Stop(Stop, Door),
}

#[session]
enum DoorReset {
    Close(Close, Branch<E, DoorReset>),
    Open(Open, Branch<E, DoorReset>),
    Reset(Reset, Door),
    Stop(Stop, Branch<E, DoorReset>),
}

#[session]
type Elevator = Send<D, Reset, Branch<U, ElevatorClosed>>;

#[session]
enum ElevatorClosed {
    CloseDoor(CloseDoor, Branch<U, ElevatorClosed>),
    OpenDoor(OpenDoor, ElevatorOpening),
}

#[session]
type ElevatorOpening = Send<D, Open, Receive<D, DoorOpened, ElevatorOpened>>;

#[session]
type ElevatorOpened = Send<D, Reset, Send<D, Close, Send<D, Stop, Branch<D, ElevatorStopping>>>>;

#[session]
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
    try_session(role, |mut s: User<'_, _>| async {
        loop {
            s = if rng.gen() {
                println!("user: close");
                s.select(CloseDoor).await?
            } else {
                println!("user: open");
                s.select(OpenDoor).await?
            };

            sleep(&mut rng).await;
        }
    })
    .await
}

async fn door(role: &mut D) -> Result<Never> {
    try_session(role, |mut s: Door<'_, _>| async {
        async fn reset<'d>(mut s: Branch<'d, D, E, DoorReset<'d, D>>) -> Result<Door<'d, D>> {
            loop {
                s = match s.branch().await? {
                    DoorReset::Close(Close, s)
                    | DoorReset::Open(Open, s)
                    | DoorReset::Stop(Stop, s) => s,
                    DoorReset::Reset(Reset, s) => break Ok(s),
                }
            }
        }

        loop {
            s = match s.branch().await? {
                DoorInit::Close(Close, s) => {
                    println!("door: close");
                    reset(s.send(DoorClosed).await?).await?
                }
                DoorInit::Open(Open, s) => {
                    println!("door: open");
                    reset(s.send(DoorOpened).await?).await?
                }
                DoorInit::Reset(Reset, s) | DoorInit::Stop(Stop, s) => s,
            };
        }
    })
    .await
}

async fn elevator(role: &mut E) -> Result<Never> {
    fn opened<'e>(
        s: ElevatorOpened<'e, E>,
        mut rng: impl Rng + 'e,
    ) -> LocalBoxFuture<'e, Result<(Never, End<'e, E>)>> {
        async move {
            let s = s.send(Reset).await?;
            sleep(&mut rng).await;

            let s = s.send(Close).await?.send(Stop).await?;
            match s.branch().await? {
                ElevatorStopping::DoorStopped(DoorStopped, s) => opening(s, rng).await,
                ElevatorStopping::DoorOpened(DoorOpened, s) => opened(s, rng).await,
                ElevatorStopping::DoorClosed(DoorClosed, s) => elevator(s, rng).await,
            }
        }
        .boxed_local()
    }

    fn opening<'e>(
        s: ElevatorOpening<'e, E>,
        rng: impl Rng + 'e,
    ) -> LocalBoxFuture<'e, Result<(Never, End<'e, E>)>> {
        async move {
            let (DoorOpened, s) = s.send(Open).await?.receive().await?;
            opened(s, rng).await
        }
        .boxed_local()
    }

    fn closed<'e>(
        s: Branch<'e, E, U, ElevatorClosed<'e, E>>,
        rng: impl Rng + 'e,
    ) -> LocalBoxFuture<'e, Result<(Never, End<'e, E>)>> {
        async move {
            match s.branch().await? {
                ElevatorClosed::CloseDoor(CloseDoor, s) => closed(s, rng).await,
                ElevatorClosed::OpenDoor(OpenDoor, s) => opening(s, rng).await,
            }
        }
        .boxed_local()
    }

    fn elevator<'e>(
        s: Elevator<'e, E>,
        rng: impl Rng + 'e,
    ) -> LocalBoxFuture<'e, Result<(Never, End<'e, E>)>> {
        async move {
            let s = s.send(Reset).await?;
            closed(s, rng).await
        }
        .boxed_local()
    }

    let rng = rand::thread_rng();
    try_session(role, |s| async { elevator(s, rng).await }).await
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let Roles(mut u, mut d, mut e) = Roles::default();
    try_join!(user(&mut u), door(&mut d), elevator(&mut e)).unwrap();
}
