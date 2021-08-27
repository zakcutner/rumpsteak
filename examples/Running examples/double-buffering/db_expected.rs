use ::futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
#[allow(unused_imports)]
use ::rumpsteak::{
    channel::Bidirectional, session, Branch, End, Message, Receive, Role, Roles, Select, Send,
};

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
struct DbK(Send<S, Ready, Receive<S, Copy, Receive<T, Ready, Send<T, Copy, DbK>>>>);

#[session]
struct DbS(Receive<K, Ready, Send<K, Copy, DbS>>);

#[session]
struct DbT(Send<K, Ready, Receive<K, Copy, DbT>>);
