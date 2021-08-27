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
    Empty3(Empty3),
    Empty4(Empty4),
    Valid(Valid),
    Quit(Quit),
    Empty5(Empty5),
    Empty1(Empty1),
    Empty2(Empty2),
}

struct Empty3(i32);

struct Empty4(i32);

struct Valid(i32);

struct Quit;

struct Empty5(i32);

struct Empty1(i32);

struct Empty2(i32);

#[session]
type ThreeBuyersC = Receive<S, Empty3, Receive<A, Empty4, Select<A, ThreeBuyersC2>>>;

#[session]
enum ThreeBuyersC2 {
    Quit(Quit, Send<S, Quit, End>),
    Valid(Valid, Send<S, Valid, Receive<S, Empty5, End>>),
}

#[session]
type ThreeBuyersS = Receive<A, Empty1, Send<A, Empty2, Send<C, Empty3, Branch<C, ThreeBuyersS3>>>>;

#[session]
enum ThreeBuyersS3 {
    Valid(Valid, Send<C, Empty5, End>),
    Quit(Quit, End),
}

#[session]
type ThreeBuyersA = Send<S, Empty1, Receive<S, Empty2, Send<C, Empty4, Branch<C, ThreeBuyersA3>>>>;

#[session]
enum ThreeBuyersA3 {
    Quit(Quit, End),
    Valid(Valid, End),
}
