use crate::config;
use num_complex::Complex;

// Fixed point FFT
// Based on:
// - NXP application note: https://www.nxp.com/docs/en/application-note/AN2114.pdf
// - fix_fft.c: https://gist.github.com/Tomwi/3842231

const N: usize = config::fft::BUF_LEN / 2;
const N_LOG2: usize = config::fft::BUF_LEN_LOG2 - 1;

const _: () = assert!(N.is_power_of_two());

/// Twiddle factors, used in the Radix-2 FFT algorithm.
// put in RAM: ~300us improvement
#[link_section = ".data.adc::fft::PFW"]
static PFW: [Complex<i16>; N / 4] = {
    const SIN_TABLE: [i16; N] = include!(concat!(env!("OUT_DIR"), "/sin_table.rs"));

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

/// Run in-place Radix-2 FFT.
///
/// Results are as follows:
/// - index 0 to N/2: positive frequencies, with DC at 0 and Nyquist frequency at N/2
/// - index N/2 to N: negative frequencies
#[inline(never)]
pub fn radix2(pfs: &mut [Complex<i16>; N]) {
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
                let tmp =
                    (i32::from(ft2a.re) * i32::from(PFW[iw].re)) - (i32::from(ft2a.im) * i32::from(PFW[iw].im));
                pfs[qa + j].re = (tmp >> 15) as i16;
                let tmp =
                    (i32::from(ft2a.re) * i32::from(PFW[iw].im)) + (i32::from(ft2a.im) * i32::from(PFW[iw].re));
                pfs[qa + j].im = (tmp >> 15) as i16;
                // twiddled cmul
                let tmp =
                    (i32::from(ft2b.re) * i32::from(PFW[iw].im)) + (i32::from(ft2b.im) * i32::from(PFW[iw].re));
                pfs[qb + j].re = (tmp >> 15) as i16;
                let tmp =
                    (i32::from(-ft2b.re) * i32::from(PFW[iw].re)) + (i32::from(ft2b.im) * i32::from(PFW[iw].im));
                pfs[qb + j].im = (tmp >> 15) as i16;
            }
        }
    }
}
