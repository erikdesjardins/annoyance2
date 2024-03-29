use crate::config;
use crate::math::{amplitude_sqrt, amplitude_squared, Truncate};
use crate::panic::OptionalExt;
use core::mem;
use num_complex::Complex;

pub mod analysis;
pub mod equalizer;
mod imp;
pub mod window;

/// Run in-place Radix-2 FFT.
///
/// Results are as follows:
/// - index 0 to N/2: positive frequencies, with DC at 0 and Nyquist frequency at N/2
/// - index N/2 to N: negative frequencies (generally can be ignored)
pub fn run(
    samples: &mut [i16; config::fft::BUF_LEN_REAL],
) -> &mut [Complex<i16>; config::fft::BUF_LEN_COMPLEX_REAL] {
    let bins = complex_from_adjacent_values(samples);

    imp::radix2(bins);

    // ignore bins N/2 to N (negative frequencies)
    let (bins, _) = bins.split_at_mut(config::fft::BUF_LEN_COMPLEX_REAL);
    let bins: &mut [_; config::fft::BUF_LEN_COMPLEX_REAL] = bins.try_into().unwrap_infallible();

    bins
}

pub fn log_amplitudes_prelude() {
    if config::debug::LOG_ALL_FFT_AMPLITUDES {
        defmt::println!(".vz 1 cn FFT");
        defmt::println!(".vz 1 xn Frequency (Hz)");
        defmt::println!(".vz 1 yn Amplitude");
        let mut freqs = [0u16; config::fft::BUF_LEN_COMPLEX_REAL];
        for (i, freq) in freqs.iter_mut().enumerate() {
            *freq = (config::fft::FREQ_RESOLUTION_X1000 * i / 1000).truncate();
        }
        defmt::println!(".vz 1 xs {}", freqs);
    }
}

pub fn log_amplitudes(bins: &[Complex<i16>; config::fft::BUF_LEN_COMPLEX_REAL]) {
    if config::debug::LOG_ALL_FFT_AMPLITUDES {
        let mut amplitudes = [0u16; config::fft::BUF_LEN_COMPLEX_REAL];
        for (amp, bin) in amplitudes.iter_mut().zip(bins) {
            *amp = amplitude_sqrt(amplitude_squared(*bin));
        }
        defmt::println!(".vz 1 ys {}", amplitudes);
    }
}

fn complex_from_adjacent_values<T>(
    x: &mut [T; config::fft::BUF_LEN_REAL],
) -> &mut [Complex<T>; config::fft::BUF_LEN_COMPLEX] {
    const _: () = assert!(config::fft::BUF_LEN_REAL == 2 * config::fft::BUF_LEN_COMPLEX);
    // Safety: Complex<T> is layout-compatible with [T; 2]
    unsafe { mem::transmute(x) }
}
