// Here we use this FSM for k instead of the naive one.
// digraph k {
//   0;
//   1;
//   2;
//   3;
//   4;
//
//
//   0 -> 1 [label="s!ready()", ];
//   1 -> 2 [label="s!ready()", ];
//   2 -> 3 [label="s?copy()", ];
//   3 -> 4 [label="t?ready()", ];
//   4 -> 1 [label="t!copy()", ];
//
//   }

use ::futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use ::futures::{executor, try_join};
#[allow(unused_imports)]
use ::rumpsteak::{
    channel::Bidirectional, session, try_session, Branch, End, Message, Receive, Role, Roles,
    Select, Send,
};
use std::error::Error;

type Channel = Bidirectional<UnboundedSender<Label>, UnboundedReceiver<Label>>;

#[derive(Roles)]
#[allow(dead_code)]
struct Roles {
    k: K,
    s: S,
    t: T,
}

#[derive(Role)]
#[message(Label)]
struct K {
    #[route(S)]
    s: Channel,
    #[route(T)]
    t: Channel,
}

#[derive(Role)]
#[message(Label)]
struct S {
    #[route(K)]
    k: Channel,
    #[route(T)]
    t: Channel,
}

#[derive(Role)]
#[message(Label)]
struct T {
    #[route(K)]
    k: Channel,
    #[route(S)]
    s: Channel,
}

#[derive(Message)]
enum Label {
    Ready(Ready),
    Copy(Copy),
}

struct Ready;

struct Copy;

#[session]
type DbK = Send<S, Ready, DbK1>;

#[session]
struct DbK1(Send<S, Ready, Receive<S, Copy, Receive<T, Ready, Send<T, Copy, DbK1>>>>);

#[session]
struct DbS(Receive<K, Ready, Send<K, Copy, DbS>>);

#[session]
struct DbT(Send<K, Ready, Receive<K, Copy, DbT>>);

async fn s(role: &mut S) -> Result<(), Box<dyn Error>> {
    try_session(role, |s: DbS<'_, _>| async {
        let mut s_rec = s.0;
        loop {
            let (Ready, s_snd) = s_rec.receive().await?;
            s_rec = s_snd.send(Copy).await?.0;
        }
    })
    .await
}

async fn k(role: &mut K) -> Result<(), Box<dyn Error>> {
    try_session(role, |s: DbK<'_, _>| async {
        let s = s.send(Ready).await?;
        let mut s_snd = s.0;
        loop {
            let s = s_snd.send(Ready).await?;
            let (Copy, s) = s.receive().await?;
            let (Ready, s) = s.receive().await?;
            s_snd = s.send(Copy).await?.0;
        }
    })
    .await
}

async fn t(role: &mut T) -> Result<(), Box<dyn Error>> {
    try_session(role, |s: DbT<'_, _>| async {
        let mut s_snd = s.0;
        loop {
            let s = s_snd.send(Ready).await?;
            let (Copy, s) = s.receive().await?;
            s_snd = s.0;
        }
    })
    .await
}

fn main() {
    let mut roles = Roles::default();
    executor::block_on(async {
        try_join!(s(&mut roles.s), t(&mut roles.t), k(&mut roles.k)).unwrap();
    });
}
