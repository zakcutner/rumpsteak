//! Benchmark for the subtyping algorithm, using the video streaming example
//! shown in [A Sound Algorithm for Asynchronous Session
//! Subtyping](https://drops.dagstuhl.de/opus/volltexte/2019/10940/).

use criterion::{criterion_group, criterion_main, Criterion};
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use rumpsteak::{
    channel::Bidirectional, serialize, session, subtyping, Branch, Message, Role, Roles, Select,
    Send,
};

type Channel = Bidirectional<UnboundedSender<Label>, UnboundedReceiver<Label>>;

#[derive(Roles)]
struct Roles(C, S);

#[derive(Role)]
#[message(Label)]
struct C(#[route(S)] Channel);

#[derive(Role)]
#[message(Label)]
struct S(#[route(C)] Channel);

#[derive(Message)]
enum Label {
    HighQuality(HighQuality),
    LowQuality(LowQuality),
    Success(Success),
    Failure(Failure),
}

struct HighQuality;
struct LowQuality;
struct Success;
struct Failure;

#[session]
type Client = Select<S, ClientOutputChoice>;

#[session]
enum ClientOutputChoice {
    HighQuality(HighQuality, Branch<S, ClientInputChoice>),
    LowQuality(LowQuality, Branch<S, ClientInputChoice>),
}

#[session]
enum ClientInputChoice {
    Success(Success, Client),
    Failure(Failure, Client),
}

#[session]
type ClientOptimized = Send<S, HighQuality, Branch<S, ClientOptimizedChoice>>;

#[session]
enum ClientOptimizedChoice {
    Success(Success, ClientOptimized),
    Failure(Failure, Send<S, LowQuality, ClientOptimized>),
}

pub fn criterion_benchmark(criterion: &mut Criterion) {
    criterion.bench_function("subtyping", |bencher| {
        let client = serialize::serialize::<Client<'_, C>>();
        let client_optimized = serialize::serialize::<ClientOptimized<'_, C>>();

        bencher.iter(|| {
            assert!(!subtyping::is_subtype(&client_optimized, &client, 10));
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
