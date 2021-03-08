use criterion::{criterion_group, criterion_main, Criterion};
use futures::channel::mpsc;
use rumpsteak::{session, try_session, End, Message, Receive, Role, Roles, Send};
use std::{error::Error, result, time::Duration};
use tokio::{runtime, time, try_join};

type Result<T> = result::Result<T, Box<dyn Error>>;

type Sender = mpsc::UnboundedSender<Value>;
type Receiver = mpsc::UnboundedReceiver<Value>;

#[derive(Roles)]
struct Roles(A, B, C);

#[derive(Role)]
#[message(Value)]
struct A(#[route(B)] Sender, #[route(C)] Receiver);

#[derive(Role)]
#[message(Value)]
struct B(#[route(A)] Receiver, #[route(C)] Sender);

#[derive(Role)]
#[message(Value)]
struct C(#[route(A)] Sender, #[route(B)] Receiver);

#[derive(Message)]
struct Value(i32);

#[session]
type RingA = Send<B, Value, Receive<C, Value, End>>;

#[session]
type RingB = Receive<A, Value, Send<C, Value, End>>;

#[session]
type RingBOptimized = Send<C, Value, Receive<A, Value, End>>;

#[session]
type RingC = Receive<B, Value, Send<A, Value, End>>;

#[session]
type RingCOptimized = Send<A, Value, Receive<B, Value, End>>;

async fn sleep() {
    const DURATION: Duration = Duration::from_millis(1);
    time::sleep(DURATION).await;
}

async fn ring_a(role: &mut A, input: i32) -> Result<i32> {
    let x = input;
    try_session(role, |s: RingA<'_, _>| async {
        let s = s.send(Value(x)).await?;
        let (Value(y), s) = s.receive().await?;
        sleep().await;
        Ok((x + y, s))
    })
    .await
}

async fn ring_b(role: &mut B, input: i32) -> Result<i32> {
    let x = input;
    try_session(role, |s: RingB<'_, _>| async {
        let (Value(y), s) = s.receive().await?;
        sleep().await;
        let s = s.send(Value(x)).await?;
        Ok((x + y, s))
    })
    .await
}

async fn ring_b_optimized(role: &mut B, input: i32) -> Result<i32> {
    let x = input;
    try_session(role, |s: RingBOptimized<'_, _>| async {
        let s = s.send(Value(x)).await?;
        let (Value(y), s) = s.receive().await?;
        sleep().await;
        Ok((x + y, s))
    })
    .await
}

async fn ring_c(role: &mut C, input: i32) -> Result<i32> {
    let x = input;
    try_session(role, |s: RingC<'_, _>| async {
        let (Value(y), s) = s.receive().await?;
        sleep().await;
        let s = s.send(Value(x)).await?;
        Ok((x + y, s))
    })
    .await
}

async fn ring_c_optimized(role: &mut C, input: i32) -> Result<i32> {
    let x = input;
    try_session(role, |s: RingCOptimized<'_, _>| async {
        let s = s.send(Value(x)).await?;
        let (Value(y), s) = s.receive().await?;
        sleep().await;
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
