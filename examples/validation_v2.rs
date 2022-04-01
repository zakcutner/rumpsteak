use futures::{
    channel::mpsc::{UnboundedReceiver, UnboundedSender},
    executor, try_join,
};
#[allow(unused_imports)]
use ::rumpsteak::{
    channel::Bidirectional,
    session,
    Branch,
    End,
    Message,
    Receive,
    Role,
    Roles,
    Select,
    Send,
    effect::{
        SideEffect,
        Constant,
        Incr,
    },
    try_session,
    predicate::{
        Tautology,
        LTnVar,
        Eq
    },
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
    Password(Password),
    Failure(Failure),
    Success(Success),
    Abort(Abort),
}

struct Password(i32);

struct Failure(i32);

struct Success(i32);

struct Abort(i32);

#[session(Name, Value)]
// type AuthenticationC = Branch<S, Tautology<Name, Value>, Constant<Name, Value>, AuthenticationC2>;
type AuthenticationC = Send<S, Password, Tautology<Name, Value>, Constant<Name, Value>, Branch<S, Tautology<Name, Value>, Constant<Name, Value>, AuthenticationC2>>;

#[session(Name, Value)]
enum AuthenticationC2 {
    Abort(Abort, End),
    Success(Success, End),
    Failure(Failure, Branch<S, Tautology<Name, Value>, Constant<Name, Value>, AuthenticationC2>),
}

#[session(Name, Value)]
enum AuthenticationC2 {
    Abort(Abort, End),
    Success(Success, End),
    Failure(Failure, Branch<S, Tautology<Name, Value>, Constant<Name, Value>, AuthenticationC2>),
}

#[session(Name, Value)]
// type AuthenticationS = Select<C, LTnVar<Value, 'x', 'y'>, Incr<'x', 1>, AuthenticationS2>;
type AuthenticationS = Receive<C, Password, Tautology<Name, Value>, Constant<Name, Value>, Select<C, LTnVar<Value, 'x', 'y'>, Incr<'x', 1>, AuthenticationS2>>;

#[session(Name, Value)]
enum AuthenticationS2 {
    Abort(Abort, End),
    Success(Success, End),
    Failure(Failure, Select<C, LTnVar<Value, 'x', 'y'>, Incr<'x', 1>, AuthenticationS2>),
}

async fn s(role: &mut S) -> Result<(), Box<dyn Error>> {
    let mut map = HashMap::new();
    map.insert('x', 0);
    map.insert('y', 10);

    try_session(role, map,
        |mut s: AuthenticationS<'_, _>| async {
            let (Password(password), s) = s.receive().await?;
            let mut i = 0;
            let s = loop {
                s = if i <= 9 {
                    s.select(Failure(i)).await?
                } else {
                    break s;
                };
                i += 1;
            };
            let s = s.select(Abort(i)).await?;
            Ok(((), s))
        })
        .await
}

async fn c(role: &mut C) -> Result<(), Box<dyn Error>> {
    try_session(role, HashMap::new(),
        |s: AuthenticationC<'_, _>| async {
            let mut s = s;
            loop{
                match s.branch().await? {
                    AuthenticationC2::Failure(_, s2) => {
                        s = s2 
                    },
                    AuthenticationC2::Abort(_, end) => {
                        println!("Terminated");
                        return Ok(((), end))
                    },
                    AuthenticationC2::Success(_, end) => {
                        println!("Sucess log in");
                        return Ok(((), end))
                    },
                }
            }
        })
    .await
}

fn main() {
    let mut roles = Roles::default();
    executor::block_on(async {
        try_join!(s(&mut roles.s), c(&mut roles.c)).unwrap();
    });
}

