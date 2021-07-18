use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use fft::{mpstthree, rumpsteak, rustfft};
use num_complex::{Complex, Complex32};
use rand::{thread_rng, Rng};
use std::sync::Arc;
use tokio::runtime;

fn gen_complex(rng: &mut impl Rng) -> Complex32 {
    Complex::new(rng.gen(), rng.gen())
}

pub fn criterion_benchmark(criterion: &mut Criterion) {
    let mut rng = thread_rng();
    let rt = runtime::Builder::new_multi_thread().build().unwrap();
    let mut group = criterion.benchmark_group("fft");

    for size in [8, 32, 56, 80, 104] {
        let input = (0..size).map(|_| gen_complex(&mut rng)).collect::<Arc<_>>();
        let expected = rustfft::run(input.clone());

        group.bench_function(BenchmarkId::new("mpstthree", size), |bencher| {
            bencher.iter(|| {
                let actual = mpstthree::run(input.clone());
                assert_eq!(actual, expected);
            });
        });

        group.bench_function(BenchmarkId::new("rumpsteak", size), |bencher| {
            bencher.iter(|| {
                let actual = rt.block_on(rumpsteak::run(input.clone()));
                assert_eq!(actual, expected);
            });
        });

        group.bench_function(BenchmarkId::new("rumpsteak_optimized", size), |bencher| {
            bencher.iter(|| {
                let actual = rt.block_on(rumpsteak::run_optimized(input.clone()));
                assert_eq!(actual, expected);
            });
        });

        group.bench_function(BenchmarkId::new("rustfft", size), |bencher| {
            bencher.iter(|| {
                let actual = rustfft::run(input.clone());
                assert_eq!(actual, expected);
            });
        });
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
