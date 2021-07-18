use num_complex::Complex32;
use rustfft::{algorithm::butterflies::Butterfly8, Fft, FftDirection};
use std::sync::Arc;

pub fn run(input: Arc<[Complex32]>) -> Vec<Complex32> {
    let butterfly = Butterfly8::new(FftDirection::Forward);
    let mut output = input.as_ref().to_owned();
    butterfly.process(&mut output);
    output
}
