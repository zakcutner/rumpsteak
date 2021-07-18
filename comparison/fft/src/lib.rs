use num_complex::Complex32;
use std::f32::consts::FRAC_1_SQRT_2;

pub mod mpstthree;
pub mod rumpsteak;
pub mod rustfft;

#[inline]
fn rotate_45(input: Complex32) -> Complex32 {
    (rotate_90(input) + input) * Complex32::from(FRAC_1_SQRT_2)
}

#[inline]
fn rotate_90(input: Complex32) -> Complex32 {
    Complex32::new(input.im, -input.re)
}

#[inline]
fn rotate_135(input: Complex32) -> Complex32 {
    (rotate_90(input) - input) * Complex32::from(FRAC_1_SQRT_2)
}
