#![allow(clippy::type_complexity)]

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use futures::{
    channel::mpsc,
    executor::{self, ThreadPool},
    try_join, SinkExt, Stream, StreamExt,
};
use rand::{rngs::SmallRng, Rng, SeedableRng};
use session::{
    choice::Choice,
    role::{Nil, Role, Roles, Route, ToFrom},
    try_session, Branch, End, Label, Receive, Result, Select, Send,
};
use std::time::{Duration, Instant};

const VALUES: u64 = 50;

type Seed = <SmallRng as SeedableRng>::Seed;

#[derive(Roles)]
struct Roles(A, B, S, T);

#[derive(Role)]
#[message(Message)]
struct A(
    #[route(B)] Nil<B>,
    #[route(S)] ToFrom<S>,
    #[route(T)] ToFrom<T>,
);

#[derive(Role)]
#[message(Message)]
struct B(
    #[route(A)] Nil<A>,
    #[route(S)] ToFrom<S>,
    #[route(T)] ToFrom<T>,
);

#[derive(Role)]
#[message(Message)]
struct S(
    #[route(A)] ToFrom<A>,
    #[route(B)] ToFrom<B>,
    #[route(T)] Nil<T>,
);

#[derive(Role)]
#[message(Message)]
struct T(
    #[route(A)] ToFrom<A>,
    #[route(B)] ToFrom<B>,
    #[route(S)] Nil<S>,
);

#[derive(Label)]
enum Message {
    Ready(Ready),
    Stop(Stop),
    Copy(Copy),
}

struct Ready;
struct Stop;
struct Copy(u64);

#[rustfmt::skip]
type Buffer<'r, R> = Send<'r, R, S, Ready, Branch<'r, R, S, BufferChoice<'r, R>>>;

#[derive(Choice)]
#[role('r, R)]
enum BufferChoice<'r, R>
where
    R: Route<S, Route = ToFrom<S>> + Route<T, Route = ToFrom<T>> + Role<Message = Message>,
    S: Route<R, Route = ToFrom<R>>,
    T: Route<R, Route = ToFrom<R>>,
{
    #[rustfmt::skip]
    Stop(Stop, Receive<'r, R, T, Ready, Send<'r, R, T, Stop, End<'r>>>),
    #[rustfmt::skip]
    Copy(Copy, Receive<'r, R, T, Ready, Send<'r, R, T, Copy, Buffer<'r, R>>>),
}

#[rustfmt::skip]
type Source<'s> = Receive<'s, S, A, Ready, Select<'s, S, A, SourceChoice<'s, B, A>>>;

#[derive(Choice)]
#[role('s, S)]
enum SourceChoice<'s, Q, R>
where
    Q: Route<S, Route = ToFrom<S>> + Role<Message = Message>,
    R: Route<S, Route = ToFrom<S>> + Role<Message = Message>,
    S: Route<Q, Route = ToFrom<Q>> + Route<R, Route = ToFrom<R>>,
{
    #[rustfmt::skip]
    Stop(Stop, Receive<'s, S, Q, Ready, Send<'s, S, Q, Stop, End<'s>>>),
    #[rustfmt::skip]
    Copy(Copy, Receive<'s, S, Q, Ready, Select<'s, S, Q, SourceChoice<'s, R, Q>>>),
}

#[rustfmt::skip]
type Sink<'t> = Send<'t, T, A, Ready, Branch<'t, T, A, SinkChoice<'t, B, A>>>;

#[derive(Choice)]
#[role('t, T)]
enum SinkChoice<'t, Q, R>
where
    Q: Route<T, Route = ToFrom<T>> + Role<Message = Message>,
    R: Route<T, Route = ToFrom<T>> + Role<Message = Message>,
    T: Route<Q, Route = ToFrom<Q>> + Route<R, Route = ToFrom<R>>,
{
    #[rustfmt::skip]
    Stop(Stop, Send<'t, T, Q, Ready, Receive<'t, T, Q, Stop, End<'t>>>),
    #[rustfmt::skip]
    Copy(Copy, Send<'t, T, Q, Ready, Branch<'t, T, Q, SinkChoice<'t, R, Q>>>),
}

async fn buffer<R>(role: &mut R) -> Result<()>
where
    R: Route<S, Route = ToFrom<S>> + Route<T, Route = ToFrom<T>> + Role<Message = Message>,
    S: Route<R, Route = ToFrom<R>>,
    T: Route<R, Route = ToFrom<R>>,
{
    try_session(role, |mut s: Buffer<'_, R>| async {
        let s = loop {
            s = match s.send(Ready)?.branch().await? {
                BufferChoice::Stop(Stop, s) => {
                    let (Ready, s) = s.receive().await?;
                    break s.send(Stop)?;
                }
                BufferChoice::Copy(Copy(v), s) => {
                    let (Ready, s) = s.receive().await?;
                    s.send(Copy(v))?
                }
            };
        };

        Ok(((), s))
    })
    .await
}

fn sleep(mut rng: impl Rng) {
    let duration = Duration::from_micros(rng.gen_range(0..500));
    let start = Instant::now();
    while Instant::now() - start < duration {}
}

async fn source(
    role: &mut S,
    seed: Seed,
    mut input: impl Stream<Item = u64> + Unpin,
) -> Result<()> {
    let mut rng = SmallRng::from_seed(seed);
    try_session(role, |mut s: Source<'_>| async {
        let s = loop {
            s = {
                let (Ready, s) = s.receive().await?;
                sleep(&mut rng);
                let s = match input.next().await {
                    Some(v) => s.select(Copy(v))?,
                    None => {
                        let s = s.select(Stop)?;
                        let (Ready, s) = s.receive().await?;
                        break s.send(Stop)?;
                    }
                };

                let (Ready, s) = s.receive().await?;
                sleep(&mut rng);
                match input.next().await {
                    Some(v) => s.select(Copy(v))?,
                    None => {
                        let s = s.select(Stop)?;
                        let (Ready, s) = s.receive().await?;
                        break s.send(Stop)?;
                    }
                }
            };
        };

        Ok(((), s))
    })
    .await
}

async fn sink(role: &mut T, seed: Seed) -> Result<()> {
    let mut rng = SmallRng::from_seed(seed);
    try_session(role, |mut s: Sink<'_>| async {
        let s = loop {
            s = {
                sleep(&mut rng);
                let s = s.send(Ready)?;
                let (v, s) = match s.branch().await? {
                    SinkChoice::Stop(Stop, s) => {
                        let s = s.send(Ready)?;
                        let (Stop, s) = s.receive().await?;
                        break s;
                    }
                    SinkChoice::Copy(Copy(v), s) => (v, s),
                };
                black_box(v);

                sleep(&mut rng);
                let s = s.send(Ready)?;
                let (v, s) = match s.branch().await? {
                    SinkChoice::Stop(Stop, s) => {
                        let s = s.send(Ready)?;
                        let (Stop, s) = s.receive().await?;
                        break s;
                    }
                    SinkChoice::Copy(Copy(v), s) => (v, s),
                };
                black_box(v);

                s
            };
        };

        Ok(((), s))
    })
    .await
}

pub fn criterion_benchmark(criterion: &mut Criterion) {
    let mut rng = SmallRng::from_entropy();
    let seeds = (rng.gen::<Seed>(), rng.gen::<Seed>());

    let mut group = criterion.benchmark_group("double_buffering");
    group.throughput(Throughput::Elements(VALUES));

    group.bench_function("single_thread", |bencher| {
        let pool = ThreadPool::new().unwrap();
        let (sender, receiver) = mpsc::channel(0);
        let Roles(mut a, mut b, mut s, mut t) = Roles::default();

        pool.spawn_ok(async move {
            try_join!(
                buffer(&mut a),
                buffer(&mut b),
                source(&mut s, seeds.0, receiver),
                sink(&mut t, seeds.1)
            )
            .unwrap();
        });

        bencher.iter(|| {
            let mut sender = sender.clone();
            let mut stream = futures::stream::iter(0..VALUES).map(Result::Ok);
            executor::block_on(sender.send_all(&mut stream)).unwrap();
        });
    });

    group.bench_function("multiple_threads", |bencher| {
        let pool = ThreadPool::new().unwrap();
        let (sender, receiver) = mpsc::channel(0);
        let Roles(mut a, mut b, mut s, mut t) = Roles::default();

        pool.spawn_ok(async move { buffer(&mut a).await.unwrap() });
        pool.spawn_ok(async move { buffer(&mut b).await.unwrap() });
        pool.spawn_ok(async move { source(&mut s, seeds.0, receiver).await.unwrap() });
        pool.spawn_ok(async move { sink(&mut t, seeds.1).await.unwrap() });

        bencher.iter(|| {
            let mut sender = sender.clone();
            let mut stream = futures::stream::iter(0..VALUES).map(Result::Ok);
            executor::block_on(sender.send_all(&mut stream)).unwrap();
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
