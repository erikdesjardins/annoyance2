use crate::config;
use crate::fixed::{amplitude_squared, phase, scale_by, sqrt};
use core::mem;
use num_complex::Complex;

// Fixed point FFT
// Based on:
// - NXP application note: https://www.nxp.com/docs/en/application-note/AN2114.pdf
// - fix_fft.c: https://gist.github.com/Tomwi/3842231

/// Run in-place Radix-2 FFT.
///
/// Results are as follows:
/// - index 0 to N/2: positive frequencies, with DC at 0 and Nyquist frequency at N/2
/// - index N/2 to N: negative frequencies (generally can be ignored)
pub fn run(
    samples: &mut [i16; config::fft::BUF_LEN_REAL],
) -> &mut [Complex<i16>; config::fft::BUF_LEN_COMPLEX] {
    let bins = complex_from_adjacent_values(samples);
    radix2(bins);
    bins
}

fn complex_from_adjacent_values<T>(
    x: &mut [T; config::fft::BUF_LEN_REAL],
) -> &mut [Complex<T>; config::fft::BUF_LEN_COMPLEX] {
    const _: () = assert!(config::fft::BUF_LEN_REAL == 2 * config::fft::BUF_LEN_COMPLEX);
    // Safety: Complex<T> is layout-compatible with [T; 2]
    unsafe { mem::transmute(x) }
}

const N: usize = config::fft::BUF_LEN_COMPLEX;
const N_LOG2: usize = usize::BITS as usize - 1 - N.leading_zeros() as usize;
const _: () = assert!(N.is_power_of_two());

/// Twiddle factors, used in the Radix-2 FFT algorithm.
static W: [Complex<i16>; N / 2] = {
    const SIN_TABLE: [i16; N] = include!(concat!(env!("OUT_DIR"), "/fft_sin_table.rs"));

    let mut twiddle = [Complex::new(0, 0); N / 2];

    let mut iw = 0;
    while iw < twiddle.len() {
        let wr = SIN_TABLE[iw + N / 4] >> 1;
        let wi = -SIN_TABLE[iw] >> 1;
        let w = Complex::new(wr, wi);
        twiddle[iw] = w;

        iw += 1;
    }

    twiddle
};

#[inline(never)]
fn radix2(f: &mut [Complex<i16>; N]) {
    // decimation in time - re-order data
    let mut mr = 0;
    for m in 1..N {
        let l = isolate_highest_set_bit(N - 1 - mr);
        mr = (mr & (l - 1)) + l;
        if mr > m {
            f.swap(m, mr);
        }
    }

    for stage in 0..N_LOG2 {
        let k = N_LOG2 - 1 - stage;
        let l = 1 << stage;
        let istep = l << 1;
        for m in 0..l {
            let iw = m << k;
            let w = W[iw];
            for i in (m..N).into_iter().step_by(istep) {
                let j = i + l;
                let tr = fix_mpy(w.re, f[j].re) - fix_mpy(w.im, f[j].im);
                let ti = fix_mpy(w.re, f[j].im) + fix_mpy(w.im, f[j].re);
                // fixed scaling, for proper normalization --
                // there will be log2(n) passes, so this results
                // in an overall factor of 1/n, distributed to
                // maximize arithmetic accuracy.
                let qr = f[i].re >> 1;
                let qi = f[i].im >> 1;
                f[j].re = qr - tr;
                f[j].im = qi - ti;
                f[i].re = qr + tr;
                f[i].im = qi + ti;
            }
        }
    }
}

fn isolate_highest_set_bit(x: usize) -> usize {
    (1 << usize::BITS - 1) >> x.leading_zeros()
}

fn fix_mpy(a: i16, b: i16) -> i16 {
    let product = i32::from(a) * i32::from(b);
    // round up based on the last bit that's about to be shifted out
    // this matches behavior of fix_fft.c, and is equivalent (https://alive2.llvm.org/ce/z/6TGPCe),
    // but why? it's not clear why rounding should be preferred over simple truncation here
    let rounded = product + (1 << 14);
    (rounded >> 15) as i16
}

#[inline(never)]
pub fn compute_stats(bins: &mut [Complex<i16>; config::fft::BUF_LEN_COMPLEX]) {
    let mut max_amplitude_squared = 0;
    let mut i_at_max = 0;
    let mut val_at_max = Complex::new(0, 0);

    // only look at positive, non-DC frequencies in first half of array
    for i in 2..bins.len() / 2 {
        let amplitude_squared = amplitude_squared(bins[i]);
        if amplitude_squared > max_amplitude_squared {
            max_amplitude_squared = amplitude_squared;
            i_at_max = i;
            val_at_max = bins[i];
        }
    }

    let max_amplitude = sqrt(max_amplitude_squared);
    let freq_at_max = i_at_max * config::fft::FREQ_RESOLUTION_X1000 / 1000;
    let phase_at_max = phase(val_at_max);
    let deg_at_max = scale_by(360, (phase_at_max >> 16) as u16);

    defmt::info!(
        "Max amplitude = {} @ freq = {} Hz, phase = {}/{} cycles (~{} deg)",
        max_amplitude,
        freq_at_max,
        phase_at_max,
        u32::MAX,
        deg_at_max,
    );

    if config::debug::LOG_ALL_FFT_AMPLITUDES {
        let mut amplitudes = [0; config::fft::BUF_LEN_COMPLEX / 2];
        for (amp, bin) in amplitudes.iter_mut().zip(bins) {
            *amp = (sqrt(amplitude_squared(*bin)) >> 32) as u16;
        }
        defmt::println!(
            "FFT ({}.{} Hz per each of {} buckets): {}",
            config::fft::FREQ_RESOLUTION_X1000 / 1000,
            config::fft::FREQ_RESOLUTION_X1000 % 1000,
            amplitudes.len(),
            amplitudes
        );
    }
}
