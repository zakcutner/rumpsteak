use futures::{executor, try_join};
use rumpsteak::{
    role::{From, Role, Roles, To},
    try_session, End, Label, Receive, Result, Send,
};

#[derive(Roles)]
struct Roles(A, B, C);

#[derive(Role)]
#[message(Message)]
struct A(#[route(B)] To<B>, #[route(C)] From<C>);

#[derive(Role)]
#[message(Message)]
struct B(#[route(A)] From<A>, #[route(C)] To<C>);

#[derive(Role)]
#[message(Message)]
struct C(#[route(A)] To<A>, #[route(B)] From<B>);

#[derive(Label)]
enum Message {
    Value(Value),
}

struct Value(i32);

#[rustfmt::skip]
type RingA<'a> = Send<'a, A, B, Value, Receive<'a, A, C, Value, End<'a>>>;

#[rustfmt::skip]
type RingB<'b> = Receive<'b, B, A, Value, Send<'b, B, C, Value, End<'b>>>;

#[rustfmt::skip]
type RingC<'c> = Receive<'c, C, B, Value, Send<'c, C, A, Value, End<'c>>>;

async fn ring_a(role: &mut A, input: i32) -> Result<i32> {
    let x = input;
    try_session(role, |s: RingA<'_>| async {
        let s = s.send(Value(x))?;
        let (Value(y), s) = s.receive().await?;
        Ok((x + y, s))
    })
    .await
}

async fn ring_b(role: &mut B, input: i32) -> Result<i32> {
    let x = input;
    try_session(role, |s: RingB<'_>| async {
        let (Value(y), s) = s.receive().await?;
        let s = s.send(Value(x))?;
        Ok((x + y, s))
    })
    .await
}

async fn ring_c(role: &mut C, input: i32) -> Result<i32> {
    let x = input;
    try_session(role, |s: RingC<'_>| async {
        let (Value(y), s) = s.receive().await?;
        let s = s.send(Value(x))?;
        Ok((x + y, s))
    })
    .await
}

fn main() {
    let Roles(mut a, mut b, mut c) = Roles::default();

    let input = (1, 2, 3);
    println!("input = {:?}", input);

    let output = executor::block_on(async {
        try_join!(
            ring_a(&mut a, input.0),
            ring_b(&mut b, input.1),
            ring_c(&mut c, input.2),
        )
        .unwrap()
    });
    println!("output = {:?}", output);
}
