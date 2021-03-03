use futures::{
    channel::mpsc::{UnboundedReceiver, UnboundedSender},
    executor, try_join,
};
use rumpsteak::{
    channel::Bidirectional, try_session, Branch, Choice, End, Message, Receive, Role, Roles,
    Select, Send,
};
use std::{error::Error, result};

type Result<T> = result::Result<T, Box<dyn Error>>;

type Channel = Bidirectional<UnboundedSender<Label>, UnboundedReceiver<Label>>;

#[derive(Roles)]
struct Roles(C, S);

#[derive(Role)]
#[message(Label)]
struct C(#[route(S)] Channel);

#[derive(Role)]
#[message(Label)]
struct S(#[route(C)] Channel);

#[derive(Message)]
enum Label {
    Add(Add),
    Bye(Bye),
    Hello(Hello),
    Sum(Sum),
}

struct Add(i32);
struct Bye;
struct Hello(i32);
struct Sum(i32);

type Client = Send<S, Hello, Select<S, ClientChoice>>;

#[derive(Choice)]
#[message(Label)]
enum ClientChoice {
    Add(Add, Send<S, Add, Receive<S, Sum, Select<S, ClientChoice>>>),
    Bye(Bye, Receive<S, Bye, End>),
}

type Server = Receive<C, Hello, Branch<C, ServerChoice>>;

#[derive(Choice)]
#[message(Label)]
enum ServerChoice {
    Add(Add, Receive<C, Add, Send<C, Sum, Branch<C, ServerChoice>>>),
    Bye(Bye, Send<C, Bye, End>),
}

async fn client(role: &mut C) -> Result<()> {
    try_session(|s: Client| async {
        let s = s.send(role, Hello(1)).await?;

        let s = s.select(role, Add(2)).await?;
        let s = s.send(role, Add(3)).await?;
        let (Sum(f), s) = s.receive(role).await?;
        println!("1 + 2 + 3 = {}", f);
        assert_eq!(f, 6);

        let s = s.select(role, Add(4)).await?;
        let s = s.send(role, Add(5)).await?;
        let (Sum(f), s) = s.receive(role).await?;
        println!("1 + 4 + 5 = {}", f);
        assert_eq!(f, 10);

        let s = s.select(role, Add(6)).await?;
        let s = s.send(role, Add(7)).await?;
        let (Sum(f), s) = s.receive(role).await?;
        println!("1 + 6 + 7 = {}", f);
        assert_eq!(f, 14);

        let s = s.select(role, Bye).await?;
        let (Bye, s) = s.receive(role).await?;

        Ok(((), s))
    })
    .await
}

async fn server(role: &mut S) -> Result<()> {
    try_session(|s: Server| async {
        let (Hello(u), mut s) = s.receive(role).await?;
        let s = loop {
            s = match s.branch(role).await? {
                ServerChoice::Add(Add(v), s) => {
                    let (Add(w), s) = s.receive(role).await?;
                    s.send(role, Sum(u + v + w)).await?
                }
                ServerChoice::Bye(Bye, s) => break s.send(role, Bye).await?,
            };
        };

        Ok(((), s))
    })
    .await
}

fn main() {
    let Roles(mut c, mut s) = Roles::default();
    executor::block_on(async {
        try_join!(client(&mut c), server(&mut s)).unwrap();
    });
}
