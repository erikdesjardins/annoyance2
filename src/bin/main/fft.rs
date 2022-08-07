use crate::config;
use crate::math::{amplitude_sqrt, amplitude_squared};
use crate::panic::OptionalExt;
use core::mem;
use num_complex::Complex;

pub mod analysis;
mod imp;
pub mod window;

/// Run in-place Radix-2 FFT.
///
/// Results are as follows:
/// - index 0 to N/2: positive frequencies, with DC at 0 and Nyquist frequency at N/2
/// - index N/2 to N: negative frequencies (generally can be ignored)
pub fn run(
    samples: &mut [i16; config::fft::BUF_LEN_REAL],
) -> &[Complex<i16>; config::fft::BUF_LEN_COMPLEX_REAL] {
    let bins = complex_from_adjacent_values(samples);

    imp::radix2(bins);

    // ignore bins N/2 to N (negative frequencies)
    let bins: &[_; config::fft::BUF_LEN_COMPLEX_REAL] = bins[..config::fft::BUF_LEN_COMPLEX_REAL]
        .try_into()
        .unwrap_infallible();

    if config::debug::LOG_ALL_FFT_AMPLITUDES {
        let mut amplitudes = [0; config::fft::BUF_LEN_COMPLEX_REAL];
        for (amp, bin) in amplitudes.iter_mut().zip(bins) {
            *amp = amplitude_sqrt(amplitude_squared(*bin));
        }
        defmt::println!(
            "FFT ({}.{} Hz per each of {} buckets): {}",
            config::fft::FREQ_RESOLUTION_X1000 / 1000,
            config::fft::FREQ_RESOLUTION_X1000 % 1000,
            amplitudes.len(),
            amplitudes
        );
    }

    bins
}

fn complex_from_adjacent_values<T>(
    x: &mut [T; config::fft::BUF_LEN_REAL],
) -> &mut [Complex<T>; config::fft::BUF_LEN_COMPLEX] {
    const _: () = assert!(config::fft::BUF_LEN_REAL == 2 * config::fft::BUF_LEN_COMPLEX);
    // Safety: Complex<T> is layout-compatible with [T; 2]
    unsafe { mem::transmute(x) }
}
