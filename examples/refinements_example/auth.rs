use ::futures::channel::mpsc::{UnboundedReceiver, UnboundedSender,};
use futures::{executor, try_join};
#[allow(unused_imports)]
use ::rumpsteak::{
    channel::Bidirectional, session, Branch, End, Message, Receive, Role, Roles, Select, Send, 
    effect::Constant, predicate::Tautology, effect::Incr, predicate::LTnConst,
    try_session,
};
use std::collections::HashMap;
use std::error::Error;

type Channel = Bidirectional<UnboundedSender<Label>, UnboundedReceiver<Label>>;

type Name = char;
type Value = i32;

#[derive(Roles)]
#[allow(dead_code)]
struct Roles {
    c: C,
    s: S,
}

#[derive(Role)]
#[message(Label)]
struct C {
    #[route(S)]
    s: Channel,
}

#[derive(Role)]
#[message(Label)]
struct S {
    #[route(C)]
    c: Channel,
}

#[derive(Message)]
enum Label {
    Retry(Retry),
    Password(Password),
    Fail(Fail),
    Succeed(Succeed),
    Abort(Abort),
}

struct Retry(i32);

struct Password(i32);

struct Fail(i32);

struct Succeed(i32);

struct Abort(i32);

#[session(Name, Value)]
type AuthC = Receive<S, Retry, Tautology<Name, Value>, Constant<Name, Value>, Send<S, Password, Tautology<Name, Value>, Constant<Name, Value>, Branch<S, Tautology<Name, Value>, Constant<Name, Value>, AuthC3>>>;

#[session(Name, Value)]
enum AuthC3 {
    Abort(Abort, End),
    Succeed(Succeed, End),
    Fail(Fail, AuthC),
}

#[session(Name, Value)]
type AuthS = Send<C, Retry, Tautology<Name, Value>, Constant<Name, Value>, Receive<C, Password, LTnConst<'r', 0>, Incr<'r', 1>, Select<C, Tautology<Name, Value>, Constant<Name, Value>, AuthS3>>>;

#[session(Name, Value)]
enum AuthS3 {
    Abort(Abort, End),
    Succeed(Succeed, End),
    Fail(Fail, AuthS),
}

async fn C(role: &mut C) -> Result<(), Box<dyn Error>> {
    let mut map = HashMap::new();
    try_session(role, map, |s: AuthC<'_, _>| async {
        let mut s = s;
        loop {
            let (_, s_rec) = s.receive().await?;
            let s_send = s_rec.send(Password(1)).await?;
            match s_send.branch().await? {
                AuthC3::Abort(_, s_bra) => {
                    println!("Login aborted");
                    return Ok(((), s_bra));
                }
                AuthC3::Succeed(_, s_bra) => {
                    println!("Login succeeded");
                    return Ok(((), s_bra));
                }
                AuthC3::Fail(_, s_bra) => {
                    println!("Login failed");
                    s = s_bra;
                }
            }
        }
    }).await
}
async fn S(role: &mut S) -> Result<(), Box<dyn Error>> {
    let mut map = HashMap::new();
    map.insert('r', -10);
    try_session(role, map, |mut s: AuthS<'_, _>| async {
        loop {
            let s_send = s.send(Retry(10)).await?;
            let (Password(n), s_rec) = s_send.receive().await?;
            if n == 42 {
                let s_end = s_rec.select(Succeed(0)).await?;
                return Ok(((), s_end));
            } else {
                s = s_rec.select(Fail(-1)).await?;
            }
        }
    }).await
}

fn main() {
    let mut roles = Roles::default();
    executor::block_on(async{
        try_join!(C(&mut roles.c), S(&mut roles.s)).unwrap();
    });
}
