// Adapted from: Stay safe under panic: Rust programming with affine MPST
//
// global protocol Proto(role A, role C, role S)
// {
//     choice at S
//     {
//         login(i32) from S to C;
//         password(i32) from C to A;
//         choice at A
//         {
//              Auth(i32) from A to S;
//              Auth(i32) from S to C;
//         }
//         or
//         {
//              again(i32) from A to S;
//              again(i32) from S to C;
//         }
//     }
//     or
//     {
//         cancel(i32) from S to C;
//         quit(i32) from C to A;
//     }
// }

use futures::{
    channel::mpsc::{UnboundedReceiver, UnboundedSender},
    executor, try_join,
};
#[allow(unused_imports)]
use rumpsteak::{
    channel::Bidirectional, session, try_session, Branch, End, Message, Receive, Role, Roles,
    Select, Send,
};

use std::error::Error;

type Channel = Bidirectional<UnboundedSender<Label>, UnboundedReceiver<Label>>;

#[derive(Roles)]
#[allow(dead_code)]
struct Roles {
    c: C,
    s: S,
    a: A,
}

#[derive(Role)]
#[message(Label)]
struct C {
    #[route(S)]
    s: Channel,
    #[route(A)]
    a: Channel,
}

#[derive(Role)]
#[message(Label)]
struct S {
    #[route(C)]
    c: Channel,
    #[route(A)]
    a: Channel,
}

#[derive(Role)]
#[message(Label)]
struct A {
    #[route(C)]
    c: Channel,
    #[route(S)]
    s: Channel,
}

#[derive(Message)]
enum Label {
    Login(Login),
    Cancel(Cancel),
    Password(Password),
    Again(Again),
    Auth(Auth),
    Quit(Quit),
}

struct Login(i32);

struct Cancel(i32);

struct Password(i32);

struct Again(i32);

struct Auth(i32);

struct Quit(i32);

#[session]
type ProtoC = Branch<S, ProtoC0>;

#[session]
enum ProtoC0 {
    Cancel(Cancel, Send<A, Quit, End>),
    Login(Login, Send<A, Password, Branch<S, ProtoC3>>),
}

#[session]
enum ProtoC3 {
    Auth(Auth, End),
    Again(Again, End),
}

#[session]
type ProtoS = Select<C, ProtoS0>;

#[session]
enum ProtoS0 {
    Cancel(Cancel, End),
    Login(Login, Branch<A, ProtoS2>),
}

#[session]
enum ProtoS2 {
    Again(Again, Send<C, Again, End>),
    Auth(Auth, Send<C, Auth, End>),
}

#[session]
type ProtoA = Branch<C, ProtoA0>;

#[session]
enum ProtoA0 {
    Password(Password, Select<S, ProtoA4>),
    Quit(Quit, End),
}

#[session]
enum ProtoA4 {
    Again(Again, End),
    Auth(Auth, End),
}

async fn client(role: &mut C) -> Result<(), Box<dyn Error>> {
    try_session(role, |s: ProtoC<'_, _>| async {
        match s.branch().await? {
            ProtoC0::Cancel(Cancel(i), continuation) => {
                let s = continuation.send(Quit(i)).await?;
                Ok(((), s))
            }
            ProtoC0::Login(Login(_i), continuation) => {
                // Change HERE for something != 10, so that authentication fails.
                let s = continuation.send(Password(10)).await?;
                match s.branch().await? {
                    ProtoC3::Auth(_i, end) => {
                        println!("Authenticated");
                        Ok(((), end))
                    }
                    ProtoC3::Again(_i, end) => {
                        println!("Authentication failed");
                        Ok(((), end))
                    }
                }
            }
        }
    })
    .await
}

async fn auth(role: &mut A) -> Result<(), Box<dyn Error>> {
    try_session(role, |s: ProtoA<'_, _>| async {
        match s.branch().await? {
            ProtoA0::Password(Password(i), continuation) => {
                if i == 10 {
                    // Password is 10
                    let end = continuation.select(Auth(i)).await?;
                    Ok(((), end))
                } else {
                    let end = continuation.select(Again(i)).await?;
                    Ok(((), end))
                }
            }
            ProtoA0::Quit(_i, end) => Ok(((), end)),
        }
    })
    .await
}

async fn server(role: &mut S) -> Result<(), Box<dyn Error>> {
    try_session(role, |s: ProtoS<'_, _>| async {
        let i: i32 = 10;

        // For the sake of the example, let assume that we never Cancel
        match s.branch().await? {
            ProtoS2::Auth(Auth(i), cont) => {
                let end = cont.send(Auth(i)).await?;
                Ok(((), end))
            }
            ProtoS2::Again(Again(i), cont) => {
                let end = cont.send(Again(i)).await?;
                Ok(((), end))
            }
        }
    })
    .await
}

fn main() {
    let mut roles = Roles::default();
    executor::block_on(async {
        try_join!(
            client(&mut roles.c),
            server(&mut roles.s),
            auth(&mut roles.a)
        )
        .unwrap();
    });
}
