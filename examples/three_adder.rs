use futures::{executor, try_join};
use session::{
    role::{Role, Roles, ToFrom},
    try_session, End, Label, Receive, Result, Send,
};

#[derive(Roles)]
struct Roles(A, B, C);

#[derive(Role)]
#[message(Message)]
struct A(#[route(B)] ToFrom<B>, #[route(C)] ToFrom<C>);

#[derive(Role)]
#[message(Message)]
struct B(#[route(A)] ToFrom<A>, #[route(C)] ToFrom<C>);

#[derive(Role)]
#[message(Message)]
struct C(#[route(A)] ToFrom<A>, #[route(B)] ToFrom<B>);

#[derive(Label)]
enum Message {
    Add(Add),
    Sum(Sum),
}

struct Add(i32);
struct Sum(i32);

#[rustfmt::skip]
type AdderA<'a> = Send<'a, A, B, Add, Receive<'a, A, B, Add, Send<'a, A, C, Add, Receive<'a, A, C, Sum, End>>>>;

#[rustfmt::skip]
type AdderB<'b> = Receive<'b, B, A, Add, Send<'b, B, A, Add, Send<'b, B, C, Add, Receive<'b, B, C, Sum, End>>>>;

#[rustfmt::skip]
type AdderC<'c> = Receive<'c, C, A, Add, Receive<'c, C, B, Add, Send<'c, C, A, Sum, Send<'c, C, B, Sum, End>>>>;

async fn adder_a(s: AdderA<'_>) -> Result<((), End)> {
    let x = 2;
    let s = s.send(Add(x))?;
    let (Add(y), s) = s.receive().await?;
    let s = s.send(Add(y))?;
    let (Sum(z), s) = s.receive().await?;
    println!("{} + {} = {}", x, y, z);
    assert_eq!(z, 5);
    Ok(((), s))
}

async fn adder_b(s: AdderB<'_>) -> Result<((), End)> {
    let (Add(y), s) = s.receive().await?;
    let x = 3;
    let s = s.send(Add(x))?;
    let s = s.send(Add(y))?;
    let (Sum(z), s) = s.receive().await?;
    println!("{} + {} = {}", x, y, z);
    assert_eq!(z, 5);
    Ok(((), s))
}

async fn adder_c(s: AdderC<'_>) -> Result<((), End)> {
    let (Add(x), s) = s.receive().await?;
    let (Add(y), s) = s.receive().await?;
    let z = x + y;
    let s = s.send(Sum(z))?;
    Ok(((), s.send(Sum(z))?))
}

fn main() {
    let Roles(mut a, mut b, mut c) = Roles::default();
    executor::block_on(async {
        try_join!(
            try_session(&mut a, adder_a),
            try_session(&mut b, adder_b),
            try_session(&mut c, adder_c),
        )
        .unwrap();
    });
}
