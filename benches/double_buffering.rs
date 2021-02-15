#![allow(clippy::type_complexity)]

use criterion::{criterion_group, criterion_main, Criterion};
use futures::{executor, try_join};
use rumpsteak::{
    role::{Nil, Role, Roles, ToFrom},
    try_session, End, Label, Receive, Result, Send,
};

#[derive(Roles)]
struct Roles(S, K, T);

#[derive(Role)]
#[message(Message)]
struct S(#[route(K)] ToFrom<K>, #[route(T)] Nil<T>);

#[derive(Role)]
#[message(Message)]
struct K(#[route(S)] ToFrom<S>, #[route(T)] ToFrom<T>);

#[derive(Role)]
#[message(Message)]
struct T(#[route(S)] Nil<S>, #[route(K)] ToFrom<K>);

#[derive(Label)]
enum Message {
    Ready(Ready),
    Copy(Copy),
}

struct Ready;
struct Copy(i32);

#[rustfmt::skip]
type Source<'s> = Receive<'s, S, K, Ready, Send<'s, S, K, Copy, Receive<'s, S, K, Ready, Send<'s, S, K, Copy, End<'s>>>>>;

#[rustfmt::skip]
type Kernel<'k> = Send<'k, K, S, Ready, Receive<'k, K, S, Copy, Receive<'k, K, T, Ready, Send<'k, K, T, Copy, Send<'k, K, S, Ready, Receive<'k, K, S, Copy, Receive<'k, K, T, Ready, Send<'k, K, T, Copy, End<'k>>>>>>>>>;

#[rustfmt::skip]
type KernelOptimizedWeak<'k> = Send<'k, K, S, Ready, Receive<'k, K, S, Copy, Send<'k, K, S, Ready, Receive<'k, K, T, Ready, Send<'k, K, T, Copy, Receive<'k, K, S, Copy, Receive<'k, K, T, Ready, Send<'k, K, T, Copy, End<'k>>>>>>>>>;

#[rustfmt::skip]
type KernelOptimized<'k> = Send<'k, K, S, Ready, Send<'k, K, S, Ready, Receive<'k, K, S, Copy, Receive<'k, K, T, Ready, Send<'k, K, T, Copy, Receive<'k, K, S, Copy, Receive<'k, K, T, Ready, Send<'k, K, T, Copy, End<'k>>>>>>>>>;

#[rustfmt::skip]
type Sink<'t> = Send<'t, T, K, Ready, Receive<'t, T, K, Copy, Send<'t, T, K, Ready, Receive<'t, T, K, Copy, End<'t>>>>>;

async fn source(role: &mut S, input: (i32, i32)) -> Result<()> {
    try_session(role, |s: Source<'_>| async {
        let (Ready, s) = s.receive().await?;
        let s = s.send(Copy(input.0))?;

        let (Ready, s) = s.receive().await?;
        let s = s.send(Copy(input.1))?;

        Ok(((), s))
    })
    .await
}

async fn kernel(role: &mut K) -> Result<()> {
    try_session(role, |s: Kernel<'_>| async {
        let s = s.send(Ready)?;
        let (Copy(x), s) = s.receive().await?;
        let (Ready, s) = s.receive().await?;
        let s = s.send(Copy(x))?;

        let s = s.send(Ready)?;
        let (Copy(y), s) = s.receive().await?;
        let (Ready, s) = s.receive().await?;
        let s = s.send(Copy(y))?;

        Ok(((), s))
    })
    .await
}

async fn kernel_optimized_weak(role: &mut K) -> Result<()> {
    try_session(role, |s: KernelOptimizedWeak<'_>| async {
        let s = s.send(Ready)?;
        let (Copy(x), s) = s.receive().await?;
        let s = s.send(Ready)?;
        let (Ready, s) = s.receive().await?;
        let s = s.send(Copy(x))?;

        let (Copy(y), s) = s.receive().await?;
        let (Ready, s) = s.receive().await?;
        let s = s.send(Copy(y))?;

        Ok(((), s))
    })
    .await
}

async fn kernel_optimized(role: &mut K) -> Result<()> {
    try_session(role, |s: KernelOptimized<'_>| async {
        let s = s.send(Ready)?;
        let s = s.send(Ready)?;

        let (Copy(x), s) = s.receive().await?;
        let (Ready, s) = s.receive().await?;
        let s = s.send(Copy(x))?;

        let (Copy(y), s) = s.receive().await?;
        let (Ready, s) = s.receive().await?;
        let s = s.send(Copy(y))?;

        Ok(((), s))
    })
    .await
}

async fn sink(role: &mut T) -> Result<(i32, i32)> {
    try_session(role, |s: Sink<'_>| async {
        let s = s.send(Ready)?;
        let (Copy(x), s) = s.receive().await?;

        let s = s.send(Ready)?;
        let (Copy(y), s) = s.receive().await?;

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
