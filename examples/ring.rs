use futures::{executor, try_join};
use session::{
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
    Value(i32),
}

type RingA<'a> = Send<'a, A, B, i32, Receive<'a, A, C, i32, End<'a>>>;

type RingB<'b> = Receive<'b, B, A, i32, Send<'b, B, C, i32, End<'b>>>;

type RingC<'c> = Receive<'c, C, B, i32, Send<'c, C, A, i32, End<'c>>>;

async fn ring_a(s: RingA<'_>) -> Result<((), End<'_>)> {
    let input = 10;
    println!("input = {}", input);

    let s = s.send(input)?;
    let (output, s) = s.receive().await?;

    println!("output = {}", output);
    assert_eq!(input.pow(2) * 2, output);

    Ok(((), s))
}

async fn ring_b(s: RingB<'_>) -> Result<((), End<'_>)> {
    let (input, s) = s.receive().await?;
    Ok(((), s.send(input.pow(2))?))
}

async fn ring_c(s: RingC<'_>) -> Result<((), End<'_>)> {
    let (input, s) = s.receive().await?;
    Ok(((), s.send(input * 2)?))
}

fn main() {
    let Roles(mut a, mut b, mut c) = Roles::default();
    executor::block_on(async {
        try_join!(
            try_session(&mut a, ring_a),
            try_session(&mut b, ring_b),
            try_session(&mut c, ring_c),
        )
        .unwrap();
    });
}
