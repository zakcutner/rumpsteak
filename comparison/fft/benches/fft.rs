use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use fft::{mpstthree, rumpsteak, rustfft};
use num_complex::{Complex, Complex32};
use rand::{thread_rng, Rng};
use std::sync::Arc;
use tokio::runtime;

fn generate(rng: &mut impl Rng, size: usize) -> Arc<[Complex32]> {
    (0..size)
        .map(|_| Complex::new(rng.gen(), rng.gen()))
        .collect()
}

pub fn criterion_benchmark(criterion: &mut Criterion) {
    let mut rng = thread_rng();
    let rt = runtime::Builder::new_current_thread().build().unwrap();
    let mut group = criterion.benchmark_group("fft");

    for size in [8 * 64, 8 * 128, 8 * 256, 8 * 512, 8 * 1024] {
        let input_rows = [
            generate(&mut rng, size),
            generate(&mut rng, size),
            generate(&mut rng, size),
            generate(&mut rng, size),
            generate(&mut rng, size),
            generate(&mut rng, size),
            generate(&mut rng, size),
            generate(&mut rng, size),
        ];

        let mut input_columns = vec![<[_; 8]>::default(); input_rows[0].len()];
        for (i, row) in input_rows.iter().enumerate() {
            for (j, &value) in row.iter().enumerate() {
                input_columns[j][i] = value;
            }
        }

        let expected_columns = rustfft::run(&input_columns);
        let mut expected_rows = [
            vec![Default::default(); size],
            vec![Default::default(); size],
            vec![Default::default(); size],
            vec![Default::default(); size],
            vec![Default::default(); size],
            vec![Default::default(); size],
            vec![Default::default(); size],
            vec![Default::default(); size],
        ];

        for (i, column) in expected_columns.iter().enumerate() {
            for (j, &value) in column.iter().enumerate() {
                expected_rows[j][i] = value;
            }
        }

        group.bench_function(BenchmarkId::new("mpstthree", size), |bencher| {
            bencher.iter(|| {
                let actual = mpstthree::run(&input_rows);
                for (actual, expected) in actual.iter().zip(expected_rows.iter()) {
                    for (actual, expected) in actual.iter().zip(expected.iter()) {
                        assert_eq!(actual, expected);
                    }
                }
            });
        });

        group.bench_function(BenchmarkId::new("rumpsteak", size), |bencher| {
            bencher.iter(|| {
                let actual = rt.block_on(rumpsteak::run(&input_rows));
                for (actual, expected) in actual.iter().zip(expected_rows.iter()) {
                    for (actual, expected) in actual.iter().zip(expected.iter()) {
                        assert_eq!(actual, expected);
                    }
                }
            });
        });

        group.bench_function(BenchmarkId::new("rumpsteak_optimized", size), |bencher| {
            bencher.iter(|| {
                let actual = rt.block_on(rumpsteak::run_optimized(&input_rows));
                for (actual, expected) in actual.iter().zip(expected_rows.iter()) {
                    for (actual, expected) in actual.iter().zip(expected.iter()) {
                        assert_eq!(actual, expected);
                    }
                }
            });
        });

        group.bench_function(BenchmarkId::new("rustfft", size), |bencher| {
            bencher.iter(|| {
                let actual = rustfft::run(&input_columns);
                for (actual, expected) in actual.iter().zip(expected_columns.iter()) {
                    for (actual, expected) in actual.iter().zip(expected.iter()) {
                        assert_eq!(actual, expected);
                    }
                }
            });
        });
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
