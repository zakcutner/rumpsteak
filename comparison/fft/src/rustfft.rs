use num_complex::Complex32;
use rustfft::{algorithm::butterflies::Butterfly8, Fft, FftDirection};

pub fn run(input: &[[Complex32; 8]]) -> Vec<[Complex32; 8]> {
    let butterfly = Butterfly8::new(FftDirection::Forward);
    input
        .iter()
        .map(|vector| {
            let mut vector = vector.to_owned();
            butterfly.process(&mut vector);
            vector
        })
        .collect()
}
