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
// put in RAM: ~300us improvement
// #[link_section = ".data.adc::fft::PFW"]
static PFW: [Complex<i16>; N / 4] = {
    const SIN_TABLE: [i16; N] = include!(concat!(env!("OUT_DIR"), "/fft_sin_table.rs"));

    let mut twiddle = [Complex::new(0, 0); N / 4];

    let mut iw = 0;
    while iw < twiddle.len() {
        let wr = SIN_TABLE[iw + SIN_TABLE.len() / 4] >> 1;
        let wi = -SIN_TABLE[iw] >> 1;
        let w = Complex::new(wr, wi);
        twiddle[iw] = w;

        iw += 1;
    }

    twiddle
};

#[inline(never)]
#[rustfmt::skip]
#[allow(clippy::cast_possible_truncation)]
fn radix2(pfs: &mut [Complex<i16>; N]) {
    for stage in 0..N_LOG2 {
        let stride = N >> (1 + stage);
        let edirts = 1 << stage;
        for blk in (0..N).into_iter().step_by(stride * 2) {
            let pa = blk;
            let pb = blk + stride / 2;
            let qa = blk + stride;
            let qb = blk + stride / 2 + stride;
            for j in 0..stride / 2 {
                let iw = j * edirts;
                // scale inputs
                pfs[pa + j].re >>= 1;
                pfs[pa + j].im >>= 1;
                pfs[qa + j].re >>= 1;
                pfs[qa + j].im >>= 1;
                pfs[pb + j].re >>= 1;
                pfs[pb + j].im >>= 1;
                pfs[qb + j].re >>= 1;
                pfs[qb + j].im >>= 1;
                // add
                let ft1a = Complex {
                    re: pfs[pa + j].re + pfs[qa + j].re,
                    im: pfs[pa + j].im + pfs[qa + j].im,
                };
                let ft1b = Complex {
                    re: pfs[pb + j].re + pfs[qb + j].re,
                    im: pfs[pb + j].im + pfs[qb + j].im,
                };
                // sub
                let ft2a = Complex {
                    re: pfs[pa + j].re - pfs[qa + j].re,
                    im: pfs[pa + j].im - pfs[qa + j].im,
                };
                let ft2b = Complex {
                    re: pfs[pb + j].re - pfs[qb + j].re,
                    im: pfs[pb + j].im - pfs[qb + j].im,
                };
                // store adds
                pfs[pa + j] = ft1a;
                pfs[pb + j] = ft1b;
                // cmul
                let tmp = (i32::from(ft2a.re) * i32::from(PFW[iw].re)) - (i32::from(ft2a.im) * i32::from(PFW[iw].im));
                pfs[qa + j].re = (tmp >> 15) as i16;
                let tmp = (i32::from(ft2a.re) * i32::from(PFW[iw].im)) + (i32::from(ft2a.im) * i32::from(PFW[iw].re));
                pfs[qa + j].im = (tmp >> 15) as i16;
                // twiddled cmul
                let tmp = (i32::from(ft2b.re) * i32::from(PFW[iw].im)) + (i32::from(ft2b.im) * i32::from(PFW[iw].re));
                pfs[qb + j].re = (tmp >> 15) as i16;
                let tmp = (i32::from(ft2b.im) * i32::from(PFW[iw].im)) - (i32::from(ft2b.re) * i32::from(PFW[iw].re));
                pfs[qb + j].im = (tmp >> 15) as i16;
            }
        }
    }
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
