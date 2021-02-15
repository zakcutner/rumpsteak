use futures::{future::LocalBoxFuture, FutureExt};
use rand::Rng;
use rumpsteak::{
    choice::Choice,
    role::{Role, Roles, ToFrom},
    try_session, Branch, End, Label, Receive, Result, Select, Send,
};
use std::time::Duration;
use tokio::{time, try_join};

#[derive(Roles)]
struct Roles(U, D, E);

#[derive(Role)]
#[message(Message)]
struct U(#[route(D)] ToFrom<D>, #[route(E)] ToFrom<E>);

#[derive(Role)]
#[message(Message)]
struct D(#[route(U)] ToFrom<U>, #[route(E)] ToFrom<E>);

#[derive(Role)]
#[message(Message)]
struct E(#[route(U)] ToFrom<U>, #[route(D)] ToFrom<D>);

#[derive(Label)]
enum Message {
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

type User<'u> = Select<'u, U, E, UserChoice<'u>>;

#[derive(Choice)]
#[role('u, U)]
enum UserChoice<'u> {
    CloseDoor(CloseDoor, User<'u>),
    OpenDoor(OpenDoor, User<'u>),
}

type Door<'d> = Branch<'d, D, E, DoorInit<'d>>;

#[derive(Choice)]
#[role('d, D)]
enum DoorInit<'d> {
    #[rustfmt::skip]
    Close(Close, Send<'d, D, E, DoorClosed, Branch<'d, D, E, DoorReset<'d>>>),
    #[rustfmt::skip]
    Open(Open, Send<'d, D, E, DoorOpened, Branch<'d, D, E, DoorReset<'d>>>),
    Reset(Reset, Door<'d>),
    Stop(Stop, Door<'d>),
}

#[derive(Choice)]
#[role('d, D)]
enum DoorReset<'d> {
    Close(Close, Branch<'d, D, E, DoorReset<'d>>),
    Open(Open, Branch<'d, D, E, DoorReset<'d>>),
    Reset(Reset, Door<'d>),
    Stop(Stop, Branch<'d, D, E, DoorReset<'d>>),
}

type Elevator<'e> = Send<'e, E, D, Reset, Branch<'e, E, U, ElevatorClosed<'e>>>;

#[derive(Choice)]
#[role('e, E)]
enum ElevatorClosed<'e> {
    CloseDoor(CloseDoor, Branch<'e, E, U, ElevatorClosed<'e>>),
    OpenDoor(OpenDoor, ElevatorOpening<'e>),
}

type ElevatorOpening<'e> = Send<'e, E, D, Open, Receive<'e, E, D, DoorOpened, ElevatorOpened<'e>>>;

#[rustfmt::skip]
type ElevatorOpened<'e> = Send<'e, E, D, Reset, Send<'e, E, D, Close, Send<'e, E, D, Stop, Branch<'e, E, D, ElevatorStopping<'e>>>>>;

#[derive(Choice)]
#[role('e, E)]
#[allow(clippy::enum_variant_names)]
enum ElevatorStopping<'e> {
    DoorStopped(DoorStopped, ElevatorOpening<'e>),
    DoorOpened(DoorOpened, ElevatorOpened<'e>),
    DoorClosed(DoorClosed, Elevator<'e>),
}

enum Never {}

async fn sleep(mut rng: impl Rng) {
    time::sleep(Duration::from_micros(rng.gen_range(0..500))).await;
}

async fn user(role: &mut U) -> Result<Never> {
    let mut rng = rand::thread_rng();
    try_session(role, |mut s: User<'_>| async {
        loop {
            s = if rng.gen() {
                println!("user: close");
                s.select(CloseDoor)?
            } else {
                println!("user: open");
                s.select(OpenDoor)?
            };

            sleep(&mut rng).await;
        }
    })
    .await
}

async fn door(role: &mut D) -> Result<Never> {
    try_session(role, |mut s: Door<'_>| async {
        async fn reset<'d>(mut s: Branch<'d, D, E, DoorReset<'d>>) -> Result<Door<'d>> {
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
                    reset(s.send(DoorClosed)?).await?
                }
                DoorInit::Open(Open, s) => {
                    println!("door: open");
                    reset(s.send(DoorOpened)?).await?
                }
                DoorInit::Reset(Reset, s) | DoorInit::Stop(Stop, s) => s,
            };
        }
    })
    .await
}

async fn elevator(role: &mut E) -> Result<Never> {
    fn opened<'e>(
        s: ElevatorOpened<'e>,
        mut rng: impl Rng + 'e,
    ) -> LocalBoxFuture<'e, Result<(Never, End<'_>)>> {
        async move {
            let s = s.send(Reset)?;
            sleep(&mut rng).await;

            let s = s.send(Close)?.send(Stop)?;
            match s.branch().await? {
                ElevatorStopping::DoorStopped(DoorStopped, s) => opening(s, rng).await,
                ElevatorStopping::DoorOpened(DoorOpened, s) => opened(s, rng).await,
                ElevatorStopping::DoorClosed(DoorClosed, s) => elevator(s, rng).await,
            }
        }
        .boxed_local()
    }

    fn opening<'e>(
        s: ElevatorOpening<'e>,
        rng: impl Rng + 'e,
    ) -> LocalBoxFuture<'e, Result<(Never, End<'_>)>> {
        async move {
            let (DoorOpened, s) = s.send(Open)?.receive().await?;
            opened(s, rng).await
        }
        .boxed_local()
    }

    fn closed<'e>(
        s: Branch<'e, E, U, ElevatorClosed<'e>>,
        rng: impl Rng + 'e,
    ) -> LocalBoxFuture<'e, Result<(Never, End<'_>)>> {
        async move {
            match s.branch().await? {
                ElevatorClosed::CloseDoor(CloseDoor, s) => closed(s, rng).await,
                ElevatorClosed::OpenDoor(OpenDoor, s) => opening(s, rng).await,
            }
        }
        .boxed_local()
    }

    fn elevator<'e>(
        s: Elevator<'e>,
        rng: impl Rng + 'e,
    ) -> LocalBoxFuture<'e, Result<(Never, End<'_>)>> {
        async move {
            let s = s.send(Reset)?;
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
