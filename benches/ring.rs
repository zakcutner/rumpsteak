#![allow(clippy::type_complexity)]

use criterion::{criterion_group, criterion_main, Criterion};
use rumpsteak::{
    role::{From, Role, Roles, To},
    try_session, End, Label, Receive, Result, Send,
};
use std::time::Duration;
use tokio::{runtime, time, try_join};

#[derive(Roles)]
struct Roles(A, B, C);

#[derive(Role)]
#[message(Message)]
struct A(#[route(B)] To<B>, #[route(C)] From<C>);

#[derive(Role)]
#[message(Message)]
struct B(#[route(A)] From<A>, #[route(C)] To<C>);

#[derive(Role)]
#[message(Message)]
struct C(#[route(A)] To<A>, #[route(B)] From<B>);

#[derive(Label)]
enum Message {
    Value(Value),
}

struct Value(i32);

#[rustfmt::skip]
type RingA<'a> = Send<'a, A, B, Value, Receive<'a, A, C, Value, End<'a>>>;

#[rustfmt::skip]
type RingB<'b> = Receive<'b, B, A, Value, Send<'b, B, C, Value, End<'b>>>;

#[rustfmt::skip]
type RingBOptimized<'b> = Send<'b, B, C, Value, Receive<'b, B, A, Value, End<'b>>>;

#[rustfmt::skip]
type RingC<'c> = Receive<'c, C, B, Value, Send<'c, C, A, Value, End<'c>>>;

#[rustfmt::skip]
type RingCOptimized<'c> = Send<'c, C, A, Value, Receive<'c, C, B, Value, End<'c>>>;

async fn sleep() {
    const DURATION: Duration = Duration::from_millis(1);
    time::sleep(DURATION).await;
}

async fn ring_a(role: &mut A, input: i32) -> Result<i32> {
    let x = input;
    try_session(role, |s: RingA<'_>| async {
        sleep().await;
        let s = s.send(Value(x))?;
        let (Value(y), s) = s.receive().await?;
        Ok((x + y, s))
    })
    .await
}

async fn ring_b(role: &mut B, input: i32) -> Result<i32> {
    let x = input;
    try_session(role, |s: RingB<'_>| async {
        let (Value(y), s) = s.receive().await?;
        sleep().await;
        let s = s.send(Value(x))?;
        Ok((x + y, s))
    })
    .await
}

async fn ring_b_optimized(role: &mut B, input: i32) -> Result<i32> {
    let x = input;
    try_session(role, |s: RingBOptimized<'_>| async {
        sleep().await;
        let s = s.send(Value(x))?;
        let (Value(y), s) = s.receive().await?;
        Ok((x + y, s))
    })
    .await
}

async fn ring_c(role: &mut C, input: i32) -> Result<i32> {
    let x = input;
    try_session(role, |s: RingC<'_>| async {
        let (Value(y), s) = s.receive().await?;
        sleep().await;
        let s = s.send(Value(x))?;
        Ok((x + y, s))
    })
    .await
}

async fn ring_c_optimized(role: &mut C, input: i32) -> Result<i32> {
    let x = input;
    try_session(role, |s: RingCOptimized<'_>| async {
        sleep().await;
        let s = s.send(Value(x))?;
        let (Value(y), s) = s.receive().await?;
        Ok((x + y, s))
    })
    .await
}

pub fn criterion_benchmark(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("ring");

    let mut builder = runtime::Builder::new_current_thread();
    let rt = builder.enable_time().build().unwrap();

    let Roles(mut a, mut b, mut c) = Roles::default();
    let input = (1, 2, 3);
    let expected = (4, 3, 5);

    group.bench_function("unoptimized", |bencher| {
        bencher.iter(|| {
            rt.block_on(async {
                let output = try_join!(
                    ring_a(&mut a, input.0),
                    ring_b(&mut b, input.1),
                    ring_c(&mut c, input.2),
                )
                .unwrap();
                assert_eq!(output, expected);
            })
        });
    });

    group.bench_function("optimized_b", |bencher| {
        bencher.iter(|| {
            rt.block_on(async {
                let output = try_join!(
                    ring_a(&mut a, input.0),
                    ring_b_optimized(&mut b, input.1),
                    ring_c(&mut c, input.2),
                )
                .unwrap();
                assert_eq!(output, expected);
            })
        });
    });

    group.bench_function("optimized_c", |bencher| {
        bencher.iter(|| {
            rt.block_on(async {
                let output = try_join!(
                    ring_a(&mut a, input.0),
                    ring_b(&mut b, input.1),
                    ring_c_optimized(&mut c, input.2),
                )
                .unwrap();
                assert_eq!(output, expected);
            })
        });
    });

    group.bench_function("optimized", |bencher| {
        bencher.iter(|| {
            rt.block_on(async {
                let output = try_join!(
                    ring_a(&mut a, input.0),
                    ring_b_optimized(&mut b, input.1),
                    ring_c_optimized(&mut c, input.2),
                )
                .unwrap();
                assert_eq!(output, expected);
            })
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
