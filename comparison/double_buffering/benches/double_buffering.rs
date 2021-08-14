use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use double_buffering::{ferrite, mpstthree, rumpsteak, sesh};
use std::sync::Arc;
use tokio::runtime;

pub fn criterion_benchmark(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("double_buffering");

    for size in [5_000, 10_000, 15_000, 20_000, 25_000] {
        let input = (0..size).collect::<Arc<_>>();
        group.throughput(Throughput::Elements(size as _));

        group.bench_function(BenchmarkId::new("ferrite", size), |bencher| {
            let rt = runtime::Builder::new_multi_thread().build().unwrap();
            bencher.iter(|| rt.block_on(ferrite::run(input.clone())));
        });

        group.bench_function(BenchmarkId::new("mpstthree", size), |bencher| {
            bencher.iter(|| mpstthree::run(input.clone()));
        });

        group.bench_function(BenchmarkId::new("rumpsteak", size), |bencher| {
            let rt = runtime::Builder::new_multi_thread().build().unwrap();
            bencher.iter(|| rt.block_on(rumpsteak::run(input.clone())));
        });

        group.bench_function(BenchmarkId::new("rumpsteak_optimized", size), |bencher| {
            let rt = runtime::Builder::new_multi_thread().build().unwrap();
            bencher.iter(|| rt.block_on(rumpsteak::run_optimized(input.clone())));
        });

        group.bench_function(BenchmarkId::new("sesh", size), |bencher| {
            bencher.iter(|| sesh::run(input.clone()));
        });
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
