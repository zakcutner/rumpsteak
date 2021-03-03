use criterion::{criterion_group, criterion_main, Criterion};
use futures::{
    channel::mpsc::{UnboundedReceiver, UnboundedSender},
    executor, try_join,
};
use rumpsteak::{
    channel::{Bidirectional, Nil},
    try_session, End, Message, Receive, Role, Roles, Send,
};
use std::{error::Error, result};

type Result<T> = result::Result<T, Box<dyn Error>>;

type Channel = Bidirectional<UnboundedSender<Label>, UnboundedReceiver<Label>>;

#[derive(Roles)]
struct Roles(S, K, T);

#[derive(Role)]
#[message(Label)]
struct S(#[route(K)] Channel, #[route(T)] Nil);

#[derive(Role)]
#[message(Label)]
struct K(#[route(S)] Channel, #[route(T)] Channel);

#[derive(Role)]
#[message(Label)]
struct T(#[route(S)] Nil, #[route(K)] Channel);

#[derive(Message)]
enum Label {
    Ready(Ready),
    Copy(Copy),
}

struct Ready;
struct Copy(i32);

type Source = Receive<K, Ready, Send<K, Copy, Receive<K, Ready, Send<K, Copy, End>>>>;

#[rustfmt::skip]
type Kernel = Send<S, Ready, Receive<S, Copy, Receive<T, Ready, Send<T, Copy, Send<S, Ready, Receive<S, Copy, Receive<T, Ready, Send<T, Copy, End>>>>>>>>;

#[rustfmt::skip]
type KernelOptimizedWeak = Send<S, Ready, Receive<S, Copy, Send<S, Ready, Receive<T, Ready, Send<T, Copy, Receive<S, Copy, Receive<T, Ready, Send<T, Copy, End>>>>>>>>;

#[rustfmt::skip]
type KernelOptimized = Send<S, Ready, Send<S, Ready, Receive<S, Copy, Receive<T, Ready, Send<T, Copy, Receive<S, Copy, Receive<T, Ready, Send<T, Copy, End>>>>>>>>;

type Sink = Send<K, Ready, Receive<K, Copy, Send<K, Ready, Receive<K, Copy, End>>>>;

async fn source(role: &mut S, input: (i32, i32)) -> Result<()> {
    try_session(|s: Source| async {
        let (Ready, s) = s.receive(role).await?;
        let s = s.send(role, Copy(input.0)).await?;

        let (Ready, s) = s.receive(role).await?;
        let s = s.send(role, Copy(input.1)).await?;

        Ok(((), s))
    })
    .await
}

async fn kernel(role: &mut K) -> Result<()> {
    try_session(|s: Kernel| async {
        let s = s.send(role, Ready).await?;
        let (Copy(x), s) = s.receive(role).await?;
        let (Ready, s) = s.receive(role).await?;
        let s = s.send(role, Copy(x)).await?;

        let s = s.send(role, Ready).await?;
        let (Copy(y), s) = s.receive(role).await?;
        let (Ready, s) = s.receive(role).await?;
        let s = s.send(role, Copy(y)).await?;

        Ok(((), s))
    })
    .await
}

async fn kernel_optimized_weak(role: &mut K) -> Result<()> {
    try_session(|s: KernelOptimizedWeak| async {
        let s = s.send(role, Ready).await?;
        let (Copy(x), s) = s.receive(role).await?;
        let s = s.send(role, Ready).await?;
        let (Ready, s) = s.receive(role).await?;
        let s = s.send(role, Copy(x)).await?;

        let (Copy(y), s) = s.receive(role).await?;
        let (Ready, s) = s.receive(role).await?;
        let s = s.send(role, Copy(y)).await?;

        Ok(((), s))
    })
    .await
}

async fn kernel_optimized(role: &mut K) -> Result<()> {
    try_session(|s: KernelOptimized| async {
        let s = s.send(role, Ready).await?;
        let s = s.send(role, Ready).await?;

        let (Copy(x), s) = s.receive(role).await?;
        let (Ready, s) = s.receive(role).await?;
        let s = s.send(role, Copy(x)).await?;

        let (Copy(y), s) = s.receive(role).await?;
        let (Ready, s) = s.receive(role).await?;
        let s = s.send(role, Copy(y)).await?;

        Ok(((), s))
    })
    .await
}

async fn sink(role: &mut T) -> Result<(i32, i32)> {
    try_session(|s: Sink| async {
        let s = s.send(role, Ready).await?;
        let (Copy(x), s) = s.receive(role).await?;

        let s = s.send(role, Ready).await?;
        let (Copy(y), s) = s.receive(role).await?;

        Ok(((x, y), s))
    })
    .await
}

pub fn criterion_benchmark(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("double_buffering");

    group.bench_function("unoptimized", |bencher| {
        let Roles(mut s, mut k, mut t) = Roles::default();
        let input = (1, 2);

        bencher.iter(|| {
            let (_, _, output) = executor::block_on(async {
                try_join!(source(&mut s, input), kernel(&mut k), sink(&mut t)).unwrap()
            });
            assert_eq!(input, output);
        });
    });

    group.bench_function("optimized_weak", |bencher| {
        let Roles(mut s, mut k, mut t) = Roles::default();
        let input = (1, 2);

        bencher.iter(|| {
            let (_, _, output) = executor::block_on(async {
                try_join!(
                    source(&mut s, input),
                    kernel_optimized_weak(&mut k),
                    sink(&mut t),
                )
                .unwrap()
            });
            assert_eq!(input, output);
        });
    });

    group.bench_function("optimized", |bencher| {
        let Roles(mut s, mut k, mut t) = Roles::default();
        let input = (1, 2);

        bencher.iter(|| {
            let (_, _, output) = executor::block_on(async {
                try_join!(
                    source(&mut s, input),
                    kernel_optimized(&mut k),
                    sink(&mut t),
                )
                .unwrap()
            });
            assert_eq!(input, output);
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
