#![allow(clippy::many_single_char_names)]

use criterion::{criterion_group, criterion_main, Criterion};
use futures::{
    channel::mpsc::{UnboundedReceiver, UnboundedSender},
    executor, try_join,
};
use rumpsteak::{channel::Bidirectional, try_session, End, Message, Receive, Role, Roles, Send};
use std::{error::Error, result};

type Result<T> = result::Result<T, Box<dyn Error>>;

type Channel = Bidirectional<UnboundedSender<Label>, UnboundedReceiver<Label>>;

#[derive(Roles)]
struct Roles(A, B, C);

#[derive(Role)]
#[message(Label)]
struct A(#[route(B)] Channel, #[route(C)] Channel);

#[derive(Role)]
#[message(Label)]
struct B(#[route(A)] Channel, #[route(C)] Channel);

#[derive(Role)]
#[message(Label)]
struct C(#[route(A)] Channel, #[route(B)] Channel);

#[derive(Message)]
enum Label {
    Add(Add),
    Sum(Sum),
}

struct Add(i32);
struct Sum(i32);

type AdderA = Send<B, Add, Receive<B, Add, Send<C, Add, Receive<C, Sum, End>>>>;

type AdderB = Receive<A, Add, Send<A, Add, Send<C, Add, Receive<C, Sum, End>>>>;

type AdderC = Receive<A, Add, Receive<B, Add, Send<A, Sum, Send<B, Sum, End>>>>;

async fn adder_a(role: &mut A) -> Result<()> {
    try_session(|s: AdderA| async {
        let x = 2;
        let s = s.send(role, Add(x)).await?;
        let (Add(y), s) = s.receive(role).await?;
        let s = s.send(role, Add(y)).await?;
        let (Sum(z), s) = s.receive(role).await?;
        assert_eq!(z, 5);
        Ok(((), s))
    })
    .await
}

async fn adder_b(role: &mut B) -> Result<()> {
    try_session(|s: AdderB| async {
        let (Add(y), s) = s.receive(role).await?;
        let x = 3;
        let s = s.send(role, Add(x)).await?;
        let s = s.send(role, Add(y)).await?;
        let (Sum(z), s) = s.receive(role).await?;
        assert_eq!(z, 5);
        Ok(((), s))
    })
    .await
}

async fn adder_c(role: &mut C) -> Result<()> {
    try_session(|s: AdderC| async {
        let (Add(x), s) = s.receive(role).await?;
        let (Add(y), s) = s.receive(role).await?;
        let z = x + y;
        let s = s.send(role, Sum(z)).await?;
        Ok(((), s.send(role, Sum(z)).await?))
    })
    .await
}

pub fn criterion_benchmark(criterion: &mut Criterion) {
    {
        let mut group = criterion.benchmark_group("three_adder");

        group.bench_function("with_channels", |bencher| {
            bencher.iter(|| {
                let Roles(mut a, mut b, mut c) = Roles::default();
                executor::block_on(async {
                    try_join!(adder_a(&mut a), adder_b(&mut b), adder_c(&mut c)).unwrap();
                });
            });
        });

        group.bench_function("without_channels", |bencher| {
            let Roles(mut a, mut b, mut c) = Roles::default();
            bencher.iter(|| {
                executor::block_on(async {
                    try_join!(adder_a(&mut a), adder_b(&mut b), adder_c(&mut c)).unwrap();
                });
            });
        });
    }

    oneshot::criterion_benchmark(criterion);
    blocking::criterion_benchmark(criterion);
    blocking_oneshot::criterion_benchmark(criterion);
}

mod oneshot {
    use criterion::Criterion;
    use futures::executor;
    use rumpsteak_oneshot::{session3, End, Left, Receive, Right, Send, SessionPair};

    type AdderAToB = Send<i32, Receive<i32, End>>;
    type AdderAToC = Send<i32, Receive<i32, End>>;
    type AdderAQueue = Left<Left<Right<Right<End>>>>;

    type AdderBToA = Receive<i32, Send<i32, End>>;
    type AdderBToC = Send<i32, Receive<i32, End>>;
    type AdderBQueue = Left<Left<Right<Right<End>>>>;

    type AdderCToA = Receive<i32, Send<i32, End>>;
    type AdderCToB = Receive<i32, Send<i32, End>>;
    type AdderCQueue = Left<Right<Left<Right<End>>>>;

    async fn adder_a(
        s: SessionPair<AdderAToB, AdderAToC, AdderAQueue>,
    ) -> SessionPair<End, End, End> {
        let x = 2;
        let s = s.send(x);
        let (y, s) = s.receive().await;
        let s = s.send(y);
        let (z, s) = s.receive().await;
        assert_eq!(z, 5);
        s
    }

    async fn adder_b(
        s: SessionPair<AdderBToA, AdderBToC, AdderBQueue>,
    ) -> SessionPair<End, End, End> {
        let (y, s) = s.receive().await;
        let x = 3;
        let s = s.send(x);
        let s = s.send(y);
        let (z, s) = s.receive().await;
        assert_eq!(z, 5);
        s
    }

    async fn adder_c(
        s: SessionPair<AdderCToA, AdderCToB, AdderCQueue>,
    ) -> SessionPair<End, End, End> {
        let (x, s) = s.receive().await;
        let (y, s) = s.receive().await;
        let z = x + y;
        let s = s.send(z);
        s.send(z)
    }

    pub fn criterion_benchmark(criterion: &mut Criterion) {
        criterion.bench_function("oneshot_three_adder", |bencher| {
            bencher.iter(|| {
                executor::block_on(session3(adder_a, adder_b, adder_c));
            });
        });
    }
}

mod blocking {
    use criterion::Criterion;
    use std::{
        error::Error,
        result,
        sync::mpsc::{self, Receiver, Sender},
        thread,
    };

    type Result<T> = result::Result<T, Box<dyn Error + Send + Sync>>;

    type Channel<T> = (Sender<T>, Receiver<T>);

    struct Roles(pub A, pub B, pub C);

    impl Default for Roles {
        fn default() -> Self {
            let (a_b_s, a_b_r) = mpsc::channel();
            let (a_c_s, a_c_r) = mpsc::channel();
            let (b_a_s, b_a_r) = mpsc::channel();
            let (b_c_s, b_c_r) = mpsc::channel();
            let (c_a_s, c_a_r) = mpsc::channel();
            let (c_b_s, c_b_r) = mpsc::channel();

            Self(
                A((a_b_s, b_a_r), (a_c_s, c_a_r)),
                B((b_a_s, a_b_r), (b_c_s, c_b_r)),
                C((c_a_s, a_c_r), (c_b_s, b_c_r)),
            )
        }
    }

    struct A(Channel<i32>, Channel<i32>);
    struct B(Channel<i32>, Channel<i32>);
    struct C(Channel<i32>, Channel<i32>);

    fn adder_a(A(b, c): &A) -> Result<()> {
        let x = 2;
        b.0.send(x)?;
        let y = b.1.recv()?;
        c.0.send(y)?;
        let z = c.1.recv()?;
        assert_eq!(z, 5);
        Ok(())
    }

    fn adder_b(B(a, c): &B) -> Result<()> {
        let y = a.1.recv()?;
        let x = 3;
        a.0.send(x)?;
        c.0.send(y)?;
        let z = c.1.recv()?;
        assert_eq!(z, 5);
        Ok(())
    }

    fn adder_c(C(a, b): &C) -> Result<()> {
        let x = a.1.recv()?;
        let y = b.1.recv()?;
        let z = x + y;
        a.0.send(z)?;
        b.0.send(z)?;
        Ok(())
    }

    pub fn criterion_benchmark(criterion: &mut Criterion) {
        let mut group = criterion.benchmark_group("blocking_three_adder");

        group.bench_function("with_channels", |bencher| {
            bencher.iter(|| {
                let Roles(a, b, c) = Roles::default();

                let a = thread::spawn(move || adder_a(&a));
                let b = thread::spawn(move || adder_b(&b));
                let c = thread::spawn(move || adder_c(&c));

                a.join().unwrap().unwrap();
                b.join().unwrap().unwrap();
                c.join().unwrap().unwrap();
            });
        });

        group.bench_function("without_channels", |bencher| {
            let mut roles = Some(Roles::default());

            bencher.iter(|| {
                let Roles(a, b, c) = roles.take().unwrap();

                let a = thread::spawn(move || {
                    adder_a(&a)?;
                    Result::<_>::Ok(a)
                });

                let b = thread::spawn(move || {
                    adder_b(&b)?;
                    Result::<_>::Ok(b)
                });

                let c = thread::spawn(move || {
                    adder_c(&c)?;
                    Result::<_>::Ok(c)
                });

                let a = a.join().unwrap().unwrap();
                let b = b.join().unwrap().unwrap();
                let c = c.join().unwrap().unwrap();

                roles = Some(Roles(a, b, c));
            });
        });
    }
}

mod blocking_oneshot {
    use criterion::Criterion;
    use mpstthree::{
        binary::{End, Recv, Send},
        fork_mpst,
        functionmpst::{
            close::close_mpst,
            recv::{
                recv_mpst_a_to_b, recv_mpst_a_to_c, recv_mpst_b_to_a, recv_mpst_b_to_c,
                recv_mpst_c_to_a, recv_mpst_c_to_b,
            },
            send::{
                send_mpst_a_to_b, send_mpst_a_to_c, send_mpst_b_to_a, send_mpst_b_to_c,
                send_mpst_c_to_a, send_mpst_c_to_b,
            },
        },
        role::{a::RoleA, b::RoleB, c::RoleC, end::RoleEnd},
        sessionmpst::SessionMpst,
    };
    use std::{error::Error, result};

    type Result<T> = result::Result<T, Box<dyn Error>>;

    type AtoB = Send<i32, Recv<i32, End>>;
    type AtoC = Send<i32, Recv<i32, End>>;
    type QueueA = RoleB<RoleB<RoleC<RoleC<RoleEnd>>>>;
    type EndpointA = SessionMpst<AtoB, AtoC, QueueA, RoleA<RoleEnd>>;

    type BtoA = Recv<i32, Send<i32, End>>;
    type BtoC = Send<i32, Recv<i32, End>>;
    type QueueB = RoleA<RoleA<RoleC<RoleC<RoleEnd>>>>;
    type EndpointB = SessionMpst<BtoA, BtoC, QueueB, RoleB<RoleEnd>>;

    type CtoA = Recv<i32, Send<i32, End>>;
    type CtoB = Recv<i32, Send<i32, End>>;
    type QueueC = RoleA<RoleB<RoleA<RoleB<RoleEnd>>>>;
    type EndpointC = SessionMpst<CtoA, CtoB, QueueC, RoleC<RoleEnd>>;

    fn adder_a(s: EndpointA) -> Result<()> {
        let x = 2;
        let s = send_mpst_a_to_b(x, s);
        let (y, s) = recv_mpst_a_to_b(s)?;
        let s = send_mpst_a_to_c(y, s);
        let (z, s) = recv_mpst_a_to_c(s)?;
        assert_eq!(z, 5);
        close_mpst(s)?;
        Ok(())
    }

    fn adder_b(s: EndpointB) -> Result<()> {
        let (y, s) = recv_mpst_b_to_a(s)?;
        let x = 3;
        let s = send_mpst_b_to_a(x, s);
        let s = send_mpst_b_to_c(y, s);
        let (z, s) = recv_mpst_b_to_c(s)?;
        assert_eq!(z, 5);
        close_mpst(s)?;
        Ok(())
    }

    fn adder_c(s: EndpointC) -> Result<()> {
        let (x, s) = recv_mpst_c_to_a(s)?;
        let (y, s) = recv_mpst_c_to_b(s)?;
        let z = x + y;
        let s = send_mpst_c_to_a(z, s);
        let s = send_mpst_c_to_b(z, s);
        close_mpst(s)?;
        Ok(())
    }

    pub fn criterion_benchmark(criterion: &mut Criterion) {
        criterion.bench_function("blocking_oneshot_three_adder", |bencher| {
            bencher.iter(|| {
                let (thread_a, thread_b, thread_c) = fork_mpst(adder_a, adder_b, adder_c);
                assert!(thread_a.is_ok());
                assert!(thread_b.is_ok());
                assert!(thread_c.is_ok());
            });
        });
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
