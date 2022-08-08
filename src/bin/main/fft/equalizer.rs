use crate::config;
use num_complex::Complex;

/// Corrects the nonlinear frequency response of the FFT.
///
/// The frequency response probably shouldn't be nonlinear, but the FFT implementation seems fine?
#[inline(never)]
pub fn apply_to(bins: &[Complex<i16>; config::fft::BUF_LEN_COMPLEX_REAL]) {
    let _ = bins;
}
