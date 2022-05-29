use crate::config;
use crate::fixed::{amplitude_squared, phase, scale_by, sqrt};
use num_complex::Complex;

#[inline(never)]
pub fn find_peaks(bins: &[Complex<i16>; config::fft::BUF_LEN_COMPLEX / 2]) {
    let mut max_amplitude_squared = 0;
    let mut i_at_max = 0;
    let mut val_at_max = Complex::new(0, 0);

    for (i, bin) in bins.iter().copied().enumerate().skip(1 /* skip DC */) {
        let amplitude_squared = amplitude_squared(bin);
        if amplitude_squared > max_amplitude_squared {
            max_amplitude_squared = amplitude_squared;
            i_at_max = i;
            val_at_max = bin;
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
}
