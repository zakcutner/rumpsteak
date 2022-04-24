use ::futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
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
        LTnVar
    },
};

use std::collections::HashMap;
use std::error::Error;

type Channel = Bidirectional<UnboundedSender<Label>, UnboundedReceiver<Label>>;
type Name = char;
type Value = u32;

#[derive(Roles)]
#[allow(dead_code)]
struct Roles {
    a: A,
    c: C,
}

#[derive(Role)]
#[message(Label)]
struct A {
    #[route(C)]
    c: Channel,
}

#[derive(Role)]
#[message(Label)]
struct C {
    #[route(A)]
    a: Channel,
}

#[derive(Message)]
enum Label {
    Empty1(Empty1),
    Valid(Valid),
    Quit(Quit),
}

struct Empty1(u32);

struct Valid(u32);

struct Quit;

// global protocol TwoBuyer(role A, role C)
// {
//     empty1(i32) from A to C; @ x < 10                                    
//     choice at C
//     {
//         valid(i32) from C to A; @ y < 20
//     }
//     or
//     {
//         quit() from C to A;
//     }
// }

// R, L, P, U, S(branch)
// predicate(Tautology) side effect(constant)
#[session(Name, Value)]
type TwoBuyersA = Send<C, Empty1, LTnVar<Value, 'x', 'y'>, Constant<Name, Value>, Branch<C, Tautology<Name, Value>, Constant<Name, Value>, TwoBuyersA1>>;

#[session(Name, Value)]
enum TwoBuyersA1 {
    Quit(Quit, End),
    Valid(Valid, End),
}

#[session(Name, Value)]
type TwoBuyersC = Receive<A, Empty1, Tautology<Name, Value>, Constant<Name, Value>, Select<A, Tautology<Name, Value>, Constant<Name, Value>, TwoBuyersC1>>;

#[session(Name, Value)]
enum TwoBuyersC1 {
    Quit(Quit, End),
    Valid(Valid, End),
}


async fn TwoBuyersA(role: &mut A) -> Result<(), Box<dyn Error>> {
    let mut map = HashMap::new();
    map.insert('x', 0);
    map.insert('y', 10);

    try_session(role, map,
    |mut s: TwoBuyersA<'_, _>| async {
        let s = s.send(Empty1(5)).await?;
        
        match s.branch().await? {
            TwoBuyersA1::Quit(_, s) => {
                Ok(((), s))
            }
            TwoBuyersA1::Valid(_, s) => {
                Ok(((), s))
            }
        }
    })
    .await
}

async fn TwoBuyersC(role: &mut C) -> Result<(), Box<dyn Error>> {
    let mut map = HashMap::new();
    map.insert('x', 0);
    map.insert('y', 10);

    try_session(role, map,
    |mut s: TwoBuyersC<'_, _>| async {
        let (Empty1(m), s) = s.receive().await?;
        let s = s.select(Valid(7)).await?;
        Ok(((), s))
    })
    .await
}

fn main() {
    let mut roles = Roles::default();
    executor::block_on(async {
        try_join!(
            TwoBuyersA(&mut roles.a),
            TwoBuyersC(&mut roles.c),
        )
        .unwrap();
    });
}