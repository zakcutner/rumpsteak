use futures::{channel::oneshot, join};
use std::future::Future;

pub trait Queue {
    fn new() -> Self;
}

pub trait Session: Sized {
    type Dual: Session<Dual = Self>;

    fn new() -> (Self, Self::Dual);
}

pub struct End;

impl Queue for End {
    fn new() -> Self {
        Self
    }
}

impl Session for End {
    type Dual = End;

    fn new() -> (Self, Self::Dual) {
        (Self, Self)
    }
}

pub struct Left<Q: Queue>(Q);

impl<Q: Queue> Queue for Left<Q> {
    fn new() -> Self {
        Self(Q::new())
    }
}

pub struct Right<Q: Queue>(Q);

impl<Q: Queue> Queue for Right<Q> {
    fn new() -> Self {
        Self(Q::new())
    }
}

pub struct Send<T, S: Session> {
    channel: oneshot::Sender<(T, S::Dual)>,
}

impl<T, S: Session> Session for Send<T, S> {
    type Dual = Receive<T, S::Dual>;

    fn new() -> (Self, Self::Dual) {
        let (sender, receiver) = oneshot::channel();
        (Self { channel: sender }, Self::Dual { channel: receiver })
    }
}

impl<T, S: Session> Send<T, S> {
    pub fn send(self, value: T) -> S {
        let (this, other) = S::new();
        self.channel
            .send((value, other))
            .map_err(|_| oneshot::Canceled)
            .unwrap();
        this
    }
}

pub struct Receive<T, S: Session> {
    channel: oneshot::Receiver<(T, S)>,
}

impl<T, S: Session> Session for Receive<T, S> {
    type Dual = Send<T, S::Dual>;

    fn new() -> (Self, Self::Dual) {
        let (other, this) = Self::Dual::new();
        (this, other)
    }
}

impl<T, S: Session> Receive<T, S> {
    pub async fn receive(self) -> (T, S) {
        self.channel.await.unwrap()
    }
}

pub async fn session2<S: Session, O: Future<Output = (End, End)>>(f: impl FnOnce(S, S::Dual) -> O) {
    let s = S::new();

    join!(f(s.0, s.1));
}

pub struct SessionPair<S1: Session, S2: Session, Q: Queue>(S1, S2, Q);

impl<T, S1: Session, S2: Session, Q: Queue> SessionPair<Send<T, S1>, S2, Left<Q>> {
    pub fn send(self, value: T) -> SessionPair<S1, S2, Q> {
        SessionPair(self.0.send(value), self.1, (self.2).0)
    }
}

impl<T, S1: Session, S2: Session, Q: Queue> SessionPair<S1, Send<T, S2>, Right<Q>> {
    pub fn send(self, value: T) -> SessionPair<S1, S2, Q> {
        SessionPair(self.0, self.1.send(value), (self.2).0)
    }
}

impl<T, S1: Session, S2: Session, Q: Queue> SessionPair<Receive<T, S1>, S2, Left<Q>> {
    pub async fn receive(self) -> (T, SessionPair<S1, S2, Q>) {
        let (t, s) = self.0.receive().await;
        (t, SessionPair(s, self.1, (self.2).0))
    }
}

impl<T, S1: Session, S2: Session, Q: Queue> SessionPair<S1, Receive<T, S2>, Right<Q>> {
    pub async fn receive(self) -> (T, SessionPair<S1, S2, Q>) {
        let (t, s) = self.1.receive().await;
        (t, SessionPair(self.0, s, (self.2).0))
    }
}

pub async fn session3<
    S1: Session,
    S2: Session,
    S3: Session,
    P1: Queue,
    P2: Queue,
    P3: Queue,
    O1: Future<Output = SessionPair<End, End, End>>,
    O2: Future<Output = SessionPair<End, End, End>>,
    O3: Future<Output = SessionPair<End, End, End>>,
>(
    f1: impl FnOnce(SessionPair<S1, S2, P1>) -> O1,
    f2: impl FnOnce(SessionPair<S1::Dual, S3, P2>) -> O2,
    f3: impl FnOnce(SessionPair<S2::Dual, S3::Dual, P3>) -> O3,
) {
    let s1 = S1::new();
    let s2 = S2::new();
    let s3 = S3::new();

    join!(
        f1(SessionPair(s1.0, s2.0, P1::new())),
        f2(SessionPair(s1.1, s3.0, P2::new())),
        f3(SessionPair(s2.1, s3.1, P3::new()))
    );
}
