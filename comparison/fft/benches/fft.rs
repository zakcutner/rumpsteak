use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use fft::{ferrite, mpstthree, rumpsteak, rustfft, sesh};
use num_complex::{Complex, Complex32};
use rand::{thread_rng, Rng};
use std::sync::Arc;
use tokio::runtime;

fn generate(rng: &mut impl Rng, size: usize) -> Vec<[Complex32; 8]> {
    (0..size)
        .map(|_| {
            let mut column = [Default::default(); 8];
            for value in &mut column {
                *value = Complex::new(rng.gen(), rng.gen());
            }

            column
        })
        .collect()
}

fn transpose(columns: &[[Complex32; 8]]) -> [Arc<[Complex32]>; 8] {
    let mut rows = <[Vec<_>; 8]>::default();
    for column in columns {
        for (j, &value) in column.iter().enumerate() {
            rows[j].push(value);
        }
    }

    [
        rows[0].iter().copied().collect(),
        rows[1].iter().copied().collect(),
        rows[2].iter().copied().collect(),
        rows[3].iter().copied().collect(),
        rows[4].iter().copied().collect(),
        rows[5].iter().copied().collect(),
        rows[6].iter().copied().collect(),
        rows[7].iter().copied().collect(),
    ]
}

pub fn criterion_benchmark(criterion: &mut Criterion) {
    let mut rng = thread_rng();
    let mut group = criterion.benchmark_group("fft");

    for size in [1_000, 2_000, 3_000, 4_000, 5_000] {
        group.throughput(Throughput::Elements(size as _));

        let input_columns = generate(&mut rng, size);
        let input_rows = transpose(&input_columns);

        let expected_columns = rustfft::run(&input_columns);
        let expected_rows = transpose(&expected_columns);

        group.bench_function(BenchmarkId::new("ferrite", size), |bencher| {
            let rt = runtime::Builder::new_multi_thread().build().unwrap();
            bencher.iter(|| {
                let actual = rt.block_on(ferrite::run(&input_rows));
                assert_eq!(actual, expected_rows);
            });
        });

        group.bench_function(BenchmarkId::new("mpstthree", size), |bencher| {
            bencher.iter(|| {
                let actual = mpstthree::run(&input_rows);
                assert_eq!(actual, expected_rows);
            });
        });

        group.bench_function(BenchmarkId::new("rumpsteak", size), |bencher| {
            let rt = runtime::Builder::new_multi_thread().build().unwrap();
            bencher.iter(|| {
                let actual = rt.block_on(rumpsteak::run(&input_rows));
                assert_eq!(actual, expected_rows);
            });
        });

        group.bench_function(BenchmarkId::new("rumpsteak_optimized", size), |bencher| {
            let rt = runtime::Builder::new_multi_thread().build().unwrap();
            bencher.iter(|| {
                let actual = rt.block_on(rumpsteak::run_optimized(&input_rows));
                assert_eq!(actual, expected_rows);
            });
        });

        group.bench_function(BenchmarkId::new("rustfft", size), |bencher| {
            bencher.iter(|| {
                let actual = rustfft::run(&input_columns);
                assert_eq!(actual, expected_columns);
            });
        });

        group.bench_function(BenchmarkId::new("sesh", size), |bencher| {
            bencher.iter(|| {
                let actual = sesh::run(&input_rows);
                assert_eq!(actual, expected_rows);
            });
        });
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
