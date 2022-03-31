// global protocol Counter(role A, role B) 
// {
//     choice at A {
//         continue(i32) from A to B; @ x < 10 ;;; x := x + 1
//         stop(i32) from A to B; @ x >= 10
//     }
// }

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
        LTnVar
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
    a: A,
    b: B,
}

#[derive(Role)]
#[message(Label)]
struct A
{
    #[route(B)]
    b: Channel,
}

#[derive(Role)]
#[message(Label)]
struct B {
    #[route(A)]
    a: Channel,
}

#[derive(Message)]
enum Label {
    Cons(Cons),
    Nil(Nil),
}

struct Cons(i32);

struct Nil(i32); // For the sake of the example, let have a payload to Nil.

#[session(Name, Value)]
type CounterA = Select<B, LTnVar<Value, 'x', 'y'>, Incr<'x', 1>, CounterA0>;

#[session(Name, Value)]
enum CounterA0 {
    Nil(Nil, End),
    Cons(Cons, Select<B, LTnVar<Value, 'x', 'y'>, Incr<'x', 1>, CounterA0>),
}

#[session(Name, Value)]
type CounterB = Branch<A, Tautology<Name, Value>, Constant<Name, Value>, CounterB0>;

#[session(Name, Value)]
enum CounterB0 {
    Nil(Nil, End),
    Cons(Cons, Branch<A, Tautology<Name, Value>, Constant<Name, Value>, CounterB0>),
}

async fn a(role: &mut A) -> Result<(), Box<dyn Error>> {
    let mut map = HashMap::new();
    map.insert('x', 0);
    map.insert('y', 10);

    try_session(role, map,
    |mut s: CounterA<'_, _>| async {
        let mut i = 0;
        let s = loop {
            s = if i <= 15 {
                s.select(Cons(i)).await?
            } else {
                break s;
            };
            i += 1;
        };
        let s = s.select(Nil(i)).await?;
        Ok(((), s))
    })
    .await
}

async fn b(role: &mut B) -> Result<(), Box<dyn Error>> {
    try_session(role, HashMap::new(),
        |s: CounterB<'_, _>| async {
            let mut s = s;
            loop{
                match s.branch().await? {
                    CounterB0::Cons(_, s2) => {
                        s = s2 
                    }
                    CounterB0::Nil(_, end) => {
                        println!("Terminated");
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
        try_join!(a(&mut roles.a), b(&mut roles.b)).unwrap();
    });
}
