use ::futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
#[allow(unused_imports)]
use ::rumpsteak::{
    channel::Bidirectional, session, Branch, End, Message, Receive, Role, Roles, Select, Send,
};

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
