use crate::config;
use num_complex::Complex;

const N: usize = config::fft::BUF_LEN_COMPLEX;
const N_LOG2: usize = usize::BITS as usize - 1 - N.leading_zeros() as usize;
const _: () = assert!(N.is_power_of_two());

static SIN_TABLE: [i16; N * 3 / 4] = {
    const SIN_TABLE: [i16; N] = include!(concat!(env!("OUT_DIR"), "/fft_sin_table.rs"));

    let mut sin = [0; N * 3 / 4];

    let mut i = 0;
    while i < sin.len() {
        sin[i] = SIN_TABLE[i];

        i += 1;
    }

    sin
};

/// Fixed point FFT
/// Based on fix_fft.c: https://gist.github.com/Tomwi/3842231
#[inline(never)]
pub fn radix2(f: &mut [Complex<i16>; N]) {
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
        let inverse_stage = N_LOG2 - 1 - stage;
        let stride = 1 << stage;
        let step = stride << 1;
        for m in 0..stride {
            // compute twiddle factors
            let iw = m << inverse_stage;
            let wr = i32::from(SIN_TABLE[iw + N / 4] >> 1);
            let wi = i32::from(-SIN_TABLE[iw] >> 1);
            #[allow(clippy::cast_possible_truncation)]
            for i in (m..N).into_iter().step_by(step) {
                let j = i + stride;
                // apply twiddle factors
                // round up based on the last bit that's about to be shifted out
                let round = 1 << 14;
                let tr =
                    (((wr * i32::from(f[j].re) - wi * i32::from(f[j].im)) + round) >> 15) as i16;
                let ti =
                    (((wr * i32::from(f[j].im) + wi * i32::from(f[j].re)) + round) >> 15) as i16;
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
    (1 << (usize::BITS - 1)) >> x.leading_zeros()
}
