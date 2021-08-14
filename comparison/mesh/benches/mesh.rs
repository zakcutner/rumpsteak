use criterion::{criterion_group, criterion_main, Criterion};
use mesh::rumpsteak;
use tokio::runtime;

pub fn criterion_benchmark(criterion: &mut Criterion) {
    let rt = runtime::Builder::new_current_thread().build().unwrap();

    {
        let mut group = criterion.benchmark_group("mesh/2");

        group.bench_function("rumpsteak", |bencher| {
            bencher.iter(|| rt.block_on(rumpsteak::two::run()));
        });
    }

    {
        let mut group = criterion.benchmark_group("mesh/4");

        group.bench_function("rumpsteak", |bencher| {
            bencher.iter(|| rt.block_on(rumpsteak::four::run()));
        });
    }

    {
        let mut group = criterion.benchmark_group("mesh/6");

        group.bench_function("rumpsteak", |bencher| {
            bencher.iter(|| rt.block_on(rumpsteak::six::run()));
        });
    }

    {
        let mut group = criterion.benchmark_group("mesh/8");

        group.bench_function("rumpsteak", |bencher| {
            bencher.iter(|| rt.block_on(rumpsteak::eight::run()));
        });
    }

    // let mut group = criterion.benchmark_group("mesh/10");

    // group.bench_function("rumpsteak", |bencher| {
    //     bencher.iter(|| rt.block_on(rumpsteak::ten::run()));
    // });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
