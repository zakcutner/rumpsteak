// digraph C {
//   1;
//   2;
//   3;
//   6;
//   8;
//
//
//   1 -> 2 [label="S!Request(u32)", ];
//   2 -> 3 [label="S!Data(u32)", ];
//   3 -> 1 [label="S?KO()", ];
//   3 -> 6 [label="S?OK(u32)", ];
//   3 -> 8 [label="S?Fault()", ];
//
//   }
//
// digraph S {
//   0;
//   1;
//   2;
//   3;
//
//
//   0 -> 1 [label="C?Request(u32)", ];
//   1 -> 2 [label="C?Data(u32)", ];
//   2 -> 0 [label="C!KO()", ];
//   2 -> 3 [label="C!OK(u32)", ];
//   3 -> 3 [label="L!Log(u32)", ];
//
//   }
//
// digraph L {
// 	1;
//
// 	1 -> 1 [label="S?Log(u32)", ];
// }

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
    c: C,
    l: L,
    s: S,
}

#[derive(Role)]
#[message(Label)]
struct C {
    #[route(L)]
    l: Channel,
    #[route(S)]
    s: Channel,
}

#[derive(Role)]
#[message(Label)]
struct L {
    #[route(C)]
    c: Channel,
    #[route(S)]
    s: Channel,
}

#[derive(Role)]
#[message(Label)]
struct S {
    #[route(C)]
    c: Channel,
    #[route(L)]
    l: Channel,
}

#[derive(Message)]
enum Label {
    Request(Request),
    Data(Data),
    K(K),
    O(O),
    Fault(Fault),
    Log(Log),
}

struct Request(u32);

struct Data(u32);

struct K;

struct O(u32);

struct Fault;

struct Log(u32);

#[session]
type CslC = Send<S, Request, Send<S, Data, Branch<S, CslC3>>>;

#[session]
enum CslC3 {
    Fault(Fault, End),
    O(O, End),
    K(K, CslC),
}

#[session]
struct CslL(Receive<S, Log, CslL>);

#[session]
type CslS = Receive<C, Request, Receive<C, Data, Select<C, CslS2>>>;

#[session]
enum CslS2 {
    O(O, CslS3),
    K(K, CslS),
}

#[session]
struct CslS3(Send<L, Log, CslS3>);

async fn c(role: &mut C) -> Result<(), Box<dyn Error>> {
    try_session(role, |init_s: CslC<'_, _>| async {
        let mut s = init_s;
        loop {
            let s_dat = s.send(Request(0)).await?;
            let s_branch = s_dat.send(Data(42)).await?;
            match s_branch.branch().await? {
                CslC3::Fault(_, end) => return Ok(((), end)),
                CslC3::O(_, end) => return Ok(((), end)),
                CslC3::K(_, s_loop) => {
                    s = s_loop;
                }
            };
        }
    })
    .await
}

async fn s(role: &mut S) -> Result<(), Box<dyn Error>> {
    try_session(role, |s: CslS<'_, _>| async {
        let mut s = s;
        loop {
            let (Request(i), s_req) = s.receive().await?;
            let (Data(d), s_dat) = s_req.receive().await?;
            if i == 0 {
                // Success
                let CslS3(mut s) = s_dat.select(O(i)).await?;
                loop {
                    let CslS3(s_loop) = s.send(Log(d)).await?;
                    s = s_loop;
                }
            } else {
                s = s_dat.select(K).await?;
            }
        }
    })
    .await
}

async fn l(role: &mut L) -> Result<(), Box<dyn Error>> {
    try_session(role, |s: CslL<'_, _>| async {
        let mut s = s;
        loop {
            let (Log(_), s_rec) = s.0.receive().await?;
            s = s_rec;
        }
    })
    .await
}

fn main() {
    let mut roles = Roles::default();
    executor::block_on(async {
        try_join!(c(&mut roles.c), s(&mut roles.s), l(&mut roles.l)).unwrap();
    });
}
