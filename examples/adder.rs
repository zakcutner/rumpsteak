#![allow(clippy::type_complexity)]

use futures::{executor, try_join};
use session::{
    choice::Choice,
    role::{Role, Roles, ToFrom},
    try_session, Branch, End, Label, Receive, Result, Select, Send,
};

#[derive(Roles)]
struct Roles(C, S);

#[derive(Role)]
#[message(Message)]
struct C(#[route(S)] ToFrom<S>);

#[derive(Role)]
#[message(Message)]
struct S(#[route(C)] ToFrom<C>);

#[derive(Label)]
enum Message {
    Add(Add),
    Bye(Bye),
    Hello(Hello),
    Sum(Sum),
}

struct Add(i32);
struct Bye;
struct Hello(i32);
struct Sum(i32);

type Client<'c> = Send<'c, C, S, Hello, Select<'c, C, S, ClientChoice<'c>>>;

#[derive(Choice)]
#[role('c, C)]
enum ClientChoice<'c> {
    #[rustfmt::skip]
    Add(Add, Send<'c, C, S, Add, Receive<'c, C, S, Sum, Select<'c, C, S, ClientChoice<'c>>>>),
    Bye(Bye, Receive<'c, C, S, Bye, End>),
}

type Server<'s> = Receive<'s, S, C, Hello, Branch<'s, S, C, ServerChoice<'s>>>;

#[derive(Choice)]
#[role('s, S)]
enum ServerChoice<'s> {
    #[rustfmt::skip]
    Add(Add, Receive<'s, S, C, Add, Send<'s, S, C, Sum, Branch<'s, S, C, ServerChoice<'s>>>>),
    Bye(Bye, Send<'s, S, C, Bye, End>),
}

async fn client(s: Client<'_>) -> Result<((), End)> {
    let s = s.send(Hello(1))?;

    let s = s.select(Add(2))?;
    let s = s.send(Add(3))?;
    let (Sum(f), s) = s.receive().await?;
    println!("1 + 2 + 3 = {}", f);
    assert_eq!(f, 6);

    let s = s.select(Add(4))?;
    let s = s.send(Add(5))?;
    let (Sum(f), s) = s.receive().await?;
    println!("1 + 4 + 5 = {}", f);
    assert_eq!(f, 10);

    let s = s.select(Add(6))?;
    let s = s.send(Add(7))?;
    let (Sum(f), s) = s.receive().await?;
    println!("1 + 6 + 7 = {}", f);
    assert_eq!(f, 14);

    let s = s.select(Bye)?;
    let (Bye, s) = s.receive().await?;

    Ok(((), s))
}

async fn server(s: Server<'_>) -> Result<((), End)> {
    let (Hello(u), mut s) = s.receive().await?;
    let s = loop {
        s = match s.branch().await? {
            ServerChoice::Add(Add(v), s) => {
                let (Add(w), s) = s.receive().await?;
                s.send(Sum(u + v + w))?
            }
            ServerChoice::Bye(Bye, s) => break s.send(Bye)?,
        };
    };

    Ok(((), s))
}

fn main() {
    let Roles(mut c, mut s) = Roles::default();
    executor::block_on(async {
        try_join!(try_session(&mut c, client), try_session(&mut s, server)).unwrap();
    });
}
