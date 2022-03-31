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
    Failure(Failure),
    Success(Success),
    Abort(Abort),
}

struct Retry(i32);

struct Password(i32);

struct Failure(i32);

struct Success(i32);

struct Abort(i32);

#[session(Name, Value)]
type ProtoC = Receive<S, Retry, Tautology<Name, Value>, Constant<Name, Value>, Send<S, Password, Tautology<Name, Value>, Constant<Name, Value>, Branch<S, Tautology<Name, Value>, Constant<Name, Value>, ProtoC3>>>;

#[session(Name, Value)]
enum ProtoC3 {
    Abort(Abort, End),
    Success(Success, End),
    Failure(Failure, ProtoC),
}

#[session(Name, Value)]
type ProtoS = Send<C, Retry, Tautology<Name, Value>, Constant<Name, Value>, Receive<C, Password, LTnConst<'r', 0>, Incr<'r', 1>, Select<C, Tautology<Name, Value>, Constant<Name, Value>, ProtoS3>>>;

#[session(Name, Value)]
enum ProtoS3 {
    Abort(Abort, End),
    Success(Success, End),
    Failure(Failure, ProtoS),
}

async fn C(role: &mut C) -> Result<(), Box<dyn Error>> {
    let mut map = HashMap::new();
    try_session(role, map, |s: ProtoC<'_, _>| async {
        let mut s = s;
        loop {
            let (_, s_rec) = s.receive().await?;
            let s_send = s_rec.send(Password(1)).await?;
            match s_send.branch().await? {
                ProtoC3::Abort(_, s_bra) => {
                    println!("Aborted");
                    return Ok(((), s_bra));
                }
                ProtoC3::Success(_, s_bra) => {
                    println!("Success");
                    return Ok(((), s_bra));
                }
                ProtoC3::Failure(_, s_bra) => {
                    println!("Failure");
                    s = s_bra;
                }
            }
        }
    }).await
}
async fn S(role: &mut S) -> Result<(), Box<dyn Error>> {
    let mut map = HashMap::new();
    map.insert('r', -10);
    try_session(role, map, |mut s: ProtoS<'_, _>| async {
        loop {
            let s_send = s.send(Retry(10)).await?;
            let (Password(n), s_rec) = s_send.receive().await?;
            if n == 42 {
                let s_end = s_rec.select(Success(0)).await?;
                return Ok(((), s_end));
            } else {
                s = s_rec.select(Failure(-1)).await?;
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
