use crate::config;
use num_complex::Complex;

const N: usize = config::fft::BUF_LEN_COMPLEX;
const N_LOG2: usize = usize::BITS as usize - 1 - N.leading_zeros() as usize;
const _: () = assert!(N.is_power_of_two());

const SCALE: i16 = 1;

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

    // specialize code for each stage
    fn run_stage<const STAGE: usize>(f: &mut [Complex<i16>; N]) {
        let inverse_stage = N_LOG2 - 1 - STAGE;
        let stride = 1 << STAGE;
        let step = stride << 1;
        for m in 0..stride {
            // compute twiddle factors
            let iw = m << inverse_stage;
            let wr = i32::from(SIN_TABLE[iw + N / 4] >> SCALE);
            let wi = i32::from(-SIN_TABLE[iw] >> SCALE);
            #[allow(clippy::cast_possible_truncation)]
            (m..N).into_iter().step_by(step).for_each(|i| {
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
                let qr = f[i].re >> SCALE;
                let qi = f[i].im >> SCALE;
                f[j].re = qr - tr;
                f[j].im = qi - ti;
                f[i].re = qr + tr;
                f[i].im = qi + ti;
            });
        }
    }

    run_stage::<0>(f);
    run_stage::<1>(f);
    run_stage::<2>(f);
    run_stage::<3>(f);
    run_stage::<4>(f);
    run_stage::<5>(f);
    run_stage::<6>(f);
    run_stage::<7>(f);
    run_stage::<8>(f);
    run_stage::<9>(f);
    assert_eq!(9, N_LOG2 - 1);
}

fn isolate_highest_set_bit(x: usize) -> usize {
    (1 << (usize::BITS - 1)) >> x.leading_zeros()
}
