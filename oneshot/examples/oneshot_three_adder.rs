use futures::executor;
use session_oneshot::{session3, End, Left, Receive, Right, Send, SessionPair};

type AdderAToB = Send<i32, Receive<i32, End>>;
type AdderAToC = Send<i32, Receive<i32, End>>;
type AdderAQueue = Left<Left<Right<Right<End>>>>;

type AdderBToA = Receive<i32, Send<i32, End>>;
type AdderBToC = Send<i32, Receive<i32, End>>;
type AdderBQueue = Left<Left<Right<Right<End>>>>;

type AdderCToA = Receive<i32, Send<i32, End>>;
type AdderCToB = Receive<i32, Send<i32, End>>;
type AdderCQueue = Left<Right<Left<Right<End>>>>;

async fn adder_a(s: SessionPair<AdderAToB, AdderAToC, AdderAQueue>) -> SessionPair<End, End, End> {
    let x = 2;
    let s = s.send(x);
    let (y, s) = s.receive().await;
    let s = s.send(y);
    let (z, s) = s.receive().await;
    println!("{} + {} = {}", x, y, z);
    assert_eq!(z, 5);
    s
}

async fn adder_b(s: SessionPair<AdderBToA, AdderBToC, AdderBQueue>) -> SessionPair<End, End, End> {
    let (y, s) = s.receive().await;
    let x = 3;
    let s = s.send(x);
    let s = s.send(y);
    let (z, s) = s.receive().await;
    println!("{} + {} = {}", x, y, z);
    assert_eq!(z, 5);
    s
}

async fn adder_c(s: SessionPair<AdderCToA, AdderCToB, AdderCQueue>) -> SessionPair<End, End, End> {
    let (x, s) = s.receive().await;
    let (y, s) = s.receive().await;
    let z = x + y;
    let s = s.send(z);
    s.send(z)
}

fn main() {
    executor::block_on(session3(adder_a, adder_b, adder_c));
}
