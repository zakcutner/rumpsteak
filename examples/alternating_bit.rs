use futures::{executor, try_join};
use session::{
    choice::Choice,
    role::{Role, Roles, ToFrom},
    try_session, Branch, End, Label, Result, Send,
};

#[derive(Roles)]
struct Roles(S, R);

#[derive(Role)]
#[message(Message)]
struct S(#[route(R)] ToFrom<R>);

#[derive(Role)]
#[message(Message)]
struct R(#[route(S)] ToFrom<S>);

#[derive(Label)]
enum Message {
    A0(A0),
    A1(A1),
    D0(D0),
    D1(D1),
}

struct A0;
struct A1;
struct D0(i32);
struct D1(i32);

type Sender<'s> = Send<'s, S, R, D0, Branch<'s, S, R, SenderChoice0<'s>>>;

#[derive(Choice)]
#[role('s, S)]
enum SenderChoice0<'s> {
    A0(A0, Send<'s, S, R, D1, Branch<'s, S, R, SenderChoice1<'s>>>),
    A1(A1, Send<'s, S, R, D0, Branch<'s, S, R, SenderChoice0<'s>>>),
}

#[derive(Choice)]
#[role('s, S)]
enum SenderChoice1<'s> {
    A0(A0, Send<'s, S, R, D1, Branch<'s, S, R, SenderChoice1<'s>>>),
    A1(A1, End<'s>),
}

type Receiver<'r> = Branch<'r, R, S, ReceiverChoice0<'r>>;

#[derive(Choice)]
#[role('r, R)]
enum ReceiverChoice0<'r> {
    #[rustfmt::skip]
    D0(D0, Send<'r, R, S, A0, Branch<'r, R, S, ReceiverChoice1<'r>>>),
    #[rustfmt::skip]
    D1(D1, Send<'r, R, S, A1, Branch<'r, R, S, ReceiverChoice0<'r>>>),
}

#[derive(Choice)]
#[role('r, R)]
enum ReceiverChoice1<'r> {
    #[rustfmt::skip]
    D0(D0, Send<'r, R, S, A0, Branch<'r, R, S, ReceiverChoice1<'r>>>),
    D1(D1, Send<'r, R, S, A1, End<'r>>),
}

async fn sender(role: &mut S, input: (i32, i32)) -> Result<()> {
    try_session(role, |mut s: Sender<'_>| async {
        let mut s = loop {
            s = {
                let s = s.send(D0(input.0))?;
                match s.branch().await? {
                    SenderChoice0::A0(A0, s) => break s,
                    SenderChoice0::A1(A1, s) => s,
                }
            };
        };

        let s = loop {
            s = {
                let s = s.send(D1(input.1))?;
                match s.branch().await? {
                    SenderChoice1::A0(A0, s) => s,
                    SenderChoice1::A1(A1, s) => break s,
                }
            };
        };

        Ok(((), s))
    })
    .await
}

async fn receiver(role: &mut R) -> Result<(i32, i32)> {
    try_session(role, |mut s: Receiver<'_>| async {
        let (x, mut s) = loop {
            s = match s.branch().await? {
                ReceiverChoice0::D0(D0(x), s) => break (x, s.send(A0)?),
                ReceiverChoice0::D1(D1(_), s) => s.send(A1)?,
            }
        };

        let (y, s) = loop {
            s = match s.branch().await? {
                ReceiverChoice1::D0(D0(_), s) => s.send(A0)?,
                ReceiverChoice1::D1(D1(y), s) => break (y, s.send(A1)?),
            }
        };

        Ok(((x, y), s))
    })
    .await
}

fn main() {
    let Roles(mut s, mut r) = Roles::default();

    let input = (1, 2);
    println!("input = {:?}", input);

    let (_, output) =
        executor::block_on(async { try_join!(sender(&mut s, input), receiver(&mut r)).unwrap() });
    println!("output = {:?}", output);
}
