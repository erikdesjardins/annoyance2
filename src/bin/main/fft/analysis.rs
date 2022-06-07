use crate::config;
use crate::math::{amplitude_sqrt, amplitude_squared, phase, DivRound, ScaleBy, Truncate};
use crate::panic::OptionalExt;
use fugit::{Duration, Hertz, RateExtU32};
use heapless::Vec;
use num_complex::Complex;

const FIRST_NON_DC_BIN: usize = 1;

#[inline(never)]
pub fn find_peaks(
    bins: &[Complex<i16>; config::fft::BUF_LEN_COMPLEX / 2],
    peaks_out: &mut Vec<Peak, { config::fft::analysis::MAX_PEAKS }>,
) {
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

    let mut peaks = [ZERO_PEAK; config::fft::analysis::MAX_PEAKS];
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

    'next_peak: for i_peak in 0..peaks.len() {
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
            for peak in &peaks[..i_peak] {
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
        peaks[i_peak].i = i_at_max;
        peaks[i_peak].left = left;
        peaks[i_peak].right = right;
        actual_peaks = i_peak + 1;
    }

    let peaks = &peaks[..actual_peaks];

    // Phase 2: refine the peak frequency based on shape of the peak
    //
    // For example:
    //
    //       \/ --- the real peak frequency is actually a bit to the right of the maximum-amplitude bin,
    // amp          since the peak has higher amplitudes on the right side
    // ^
    // |     .
    // |      .
    // |    .  .
    // | ...    ...
    // +-----------> freq
    //
    // If we look just at the 3 closest bins:
    //
    // ...here, the peak frequency is exactly the middle bin
    // ^
    // |  .
    // | . .
    // +----->
    //
    // ...here, the peak frequency is exactly halfway between the middle bin and right bin
    // ^
    // |  ..
    // | .
    // +----->
    //
    // ...here, the peak frequency is somewhere between the middle bin and halfway to the right bin
    //    (which we approximate, linearly, as being 1/4 to the right bin)
    // ^
    // |  .
    // |   .
    // | .
    // +----->

    let mut peak_freqs = [0; config::fft::analysis::MAX_PEAKS];

    for i_peak in 0..peaks.len() {
        let peak = &peaks[i_peak];

        // Step 1: compute amplitudes
        let center = amplitude_sqrt(amplitude_squared(bins[peak.i]));
        let sides = [peak.i - 1, peak.i + 1].map(|i| match bins.get(i) {
            Some(bin) => amplitude_sqrt(amplitude_squared(*bin)),
            // at extreme values, duplicate the center amplitude
            None => center,
        });

        // Step 2: determine whether to adjust the frequency positively (right) or negatively (left)
        let is_positive = sides[0] < sides[1];
        let (small_side, large_side) = if is_positive {
            (sides[0], sides[1])
        } else {
            (sides[1], sides[0])
        };

        // Step 3: normalize amplitudes so the small side is at 0
        let center = center - small_side;
        let large_side = large_side - small_side;
        #[allow(unused_variables)]
        let small_side = ();

        // Step 4: compute adjustment (from 0 to 1/2 of bin resolution)
        // e.g. at this point, we have
        //   ^
        // 4 |  .     <- center
        // 2 |   .    <- large_side
        // 0 | .      <- small_side
        //   +----->
        // in which case the frequency should be scaled by 2/4 * (1/2 * resolution)
        let center: usize = center.try_into().unwrap_infallible();
        let large_side: usize = large_side.try_into().unwrap_infallible();
        // adjustment = large_side/center * 1/2 * resolution
        let adjustment_x1000 = large_side * config::fft::FREQ_RESOLUTION_X1000 / center / 2;

        // Step 5: apply adjustment
        let center_freq_x1000 = peak.i * config::fft::FREQ_RESOLUTION_X1000;
        let real_freq_x1000 = if is_positive {
            center_freq_x1000 + adjustment_x1000
        } else {
            center_freq_x1000 - adjustment_x1000
        };
        let real_freq = real_freq_x1000.div_round(1000);
        // truncate frequency: we expect to only be working with < 10 kHz, which is less than u16::MAX
        let real_freq: u16 = real_freq.truncate();

        // Step 6: store adjusted frequency
        peak_freqs[i_peak] = real_freq;
    }

    // Phase 3: store peaks

    assert!(peaks_out.capacity() == config::fft::analysis::MAX_PEAKS);
    assert!(peaks_out.capacity() >= peaks.len());
    peaks_out.clear();
    peaks_out.extend((0..peaks.len()).map(|i_peak| {
        let peak = &peaks[i_peak];
        let bin = bins[peak.i];
        let peak_freq = peak_freqs[i_peak];
        Peak::from_bin_and_freq(bin, peak_freq)
    }));

    // Phase 4: log peaks

    if config::debug::LOG_FFT_PEAKS {
        for i_peak in 0..peaks.len() {
            let peak = &peaks[i_peak];
            let bin = bins[peak.i];
            let max_amplitude = amplitude_sqrt(amplitude_squared(bin));
            let deg_at_max = 360.scale_by(phase(bin));

            let peak_freq = peak_freqs[i_peak];
            let center_freq = i_to_freq(peak.i);
            let left_freq = i_to_freq(peak.left);
            let right_freq = i_to_freq(peak.right);

            defmt::println!(
                "Peak amplitude = {} @ freq = {} (mid {}, lo {}, hi {}) Hz, phase = {} deg",
                max_amplitude,
                peak_freq,
                center_freq,
                left_freq,
                right_freq,
                deg_at_max,
            );
        }
    }
}

fn i_to_freq(i: usize) -> u16 {
    let freq = (i * config::fft::FREQ_RESOLUTION_X1000).div_round(1000);
    // truncate frequency: we expect to only be working with < 10 kHz, which is less than u16::MAX
    let freq: u16 = freq.truncate();
    freq
}

/// Represents one peak frequency from the FFT, with frequency and scale factor
pub struct Peak {
    amplitude: u16,
    freq: u16,
    phase_scale_factor: u16,
}

impl Peak {
    fn from_bin_and_freq(bin: Complex<i16>, freq: u16) -> Self {
        let amplitude = amplitude_sqrt(amplitude_squared(bin));
        let phase_scale_factor = phase(bin);
        Self {
            amplitude,
            freq,
            phase_scale_factor,
        }
    }

    pub fn amplitude(&self) -> u16 {
        self.amplitude
    }

    pub fn freq(&self) -> Hertz<u32> {
        u32::from(self.freq).Hz()
    }

    pub fn period<const DENOM: u32>(&self) -> Duration<u32, 1, DENOM> {
        self.freq().into_duration()
    }

    pub fn phase_offset<const DENOM: u32>(&self) -> Duration<u32, 1, DENOM> {
        let period_ticks = self.period::<DENOM>().ticks();
        let phase_offset_ticks = period_ticks.scale_by(self.phase_scale_factor);
        Duration::<u32, 1, DENOM>::from_ticks(phase_offset_ticks)
    }
}
