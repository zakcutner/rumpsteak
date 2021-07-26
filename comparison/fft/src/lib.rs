use num_complex::Complex32;
use std::{f32::consts::FRAC_1_SQRT_2, sync::Arc};

pub mod mpstthree;
pub mod rumpsteak;
pub mod rustfft;

fn zip_with(
    x: Arc<[Complex32]>,
    y: Arc<[Complex32]>,
    f: impl Fn(Complex32, Complex32) -> Complex32,
) -> Arc<[Complex32]> {
    x.into_iter()
        .zip(y.into_iter())
        .map(|(&x, &y)| f(x, y))
        .collect()
}

fn rotate_45(input: Complex32) -> Complex32 {
    (rotate_90(input) + input) * Complex32::from(FRAC_1_SQRT_2)
}

fn rotate_90(input: Complex32) -> Complex32 {
    Complex32::new(input.im, -input.re)
}

fn rotate_135(input: Complex32) -> Complex32 {
    (rotate_90(input) - input) * Complex32::from(FRAC_1_SQRT_2)
}
