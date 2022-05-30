use crate::config;
use crate::fixed::{amplitude_sqrt, amplitude_squared, phase, scale_by};
use num_complex::Complex;

const FIRST_NON_DC_BIN: usize = 1;

#[inline(never)]
pub fn find_peaks(bins: &[Complex<i16>; config::fft::BUF_LEN_COMPLEX / 2]) {
    struct PeakLoc {
        /// Index of highest point in the peak
        i: usize,
        /// Leftmost index of peak, inclusive (samples monotonically decrease from highest point to here)
        left: usize,
        /// Rightmost index of peak, inclusive (samples monotonically decrease from highest point to here)
        right: usize,
    }

    const ZERO_PEAK: PeakLoc = PeakLoc {
        i: 0,
        left: 0,
        right: 0,
    };

    let mut peaks = [ZERO_PEAK; config::fft::analysis::PEAKS];
    let mut actual_peaks = 0;

    // Phase 1: find locations of peaks
    //
    // For example:
    //
    //       1    2        3
    //   |---+--|-+--|    |+|
    //   |      |    |    | |
    //   |   .  |    |    | |
    //   |  . . | .  |    | |
    //   | .   .|. . |    | |
    //   |.     .   .|    |.|
    // ...           ...... ...
    //

    'next_peak: for peak_i in 0..peaks.len() {
        // Step 1: find highest point outside an existing peak
        let mut max_amplitude_squared = 0;
        let mut i_at_max = 0;
        'next_bin: for i in FIRST_NON_DC_BIN..bins.len() {
            let amplitude_squared = amplitude_squared(bins[i]);
            // if this isn't the highest point, continue
            if amplitude_squared <= max_amplitude_squared {
                continue 'next_bin;
            }
            // if this is already inside an existing peak, continue
            for peak in &peaks[..peak_i] {
                if i >= peak.left && i <= peak.right {
                    continue 'next_bin;
                }
            }
            // new max found
            max_amplitude_squared = amplitude_squared;
            i_at_max = i;
        }

        // Step 2: check if highest point is above the threshold
        if max_amplitude_squared < config::fft::analysis::AMPLITUDE_THRESHOLD_SQUARED {
            break 'next_peak;
        }

        // Step 3: widen peak until it stops monotonically decreasing
        let mut left = i_at_max;
        let mut left_amplitude_squared = max_amplitude_squared;
        loop {
            if left == 0 {
                break;
            }
            let i = left - 1;
            let amplitude_squared = amplitude_squared(bins[i]);
            if amplitude_squared >= left_amplitude_squared {
                break;
            }
            left = i;
            left_amplitude_squared = amplitude_squared;
        }
        let mut right = i_at_max;
        let mut right_amplitude_squared = max_amplitude_squared;
        loop {
            if right == bins.len() - 1 {
                break;
            }
            let i = right + 1;
            let amplitude_squared = amplitude_squared(bins[i]);
            if amplitude_squared >= right_amplitude_squared {
                break;
            }
            right = i;
            right_amplitude_squared = amplitude_squared;
        }

        // Step 4: store valid peak
        peaks[peak_i].i = i_at_max;
        peaks[peak_i].left = left;
        peaks[peak_i].right = right;
        actual_peaks = peak_i + 1;
    }

    let peaks = &peaks[..actual_peaks];

    // Phase 2: log peaks

    for peak in peaks {
        let bin = bins[peak.i];
        let max_amplitude = amplitude_sqrt(amplitude_squared(bin));
        let deg_at_max = scale_by(360, phase(bin));

        let center_freq = i_to_freq(peak.i);
        let left_freq = i_to_freq(peak.left);
        let right_freq = i_to_freq(peak.right);

        defmt::info!(
            "Peak amplitude = {} @ freq = {} ({} to {}) Hz, phase = {} deg",
            max_amplitude,
            center_freq,
            left_freq,
            right_freq,
            deg_at_max,
        );
    }
}

fn i_to_freq(i: usize) -> usize {
    i * config::fft::FREQ_RESOLUTION_X1000 / 1000
}
