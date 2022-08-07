use crate::collections::ReplaceWithMapped;
use crate::config;
use crate::control;
use crate::math::{
    amplitude_sqrt, amplitude_squared, phase, DivRound, ScaleBy, ScalingFactor, Truncate,
};
use crate::panic::OptionalExt;
use core::num::NonZeroU16;
use fugit::{Duration, Hertz, RateExtU32};
use heapless::Vec;
use num_complex::Complex;

const FIRST_NON_DC_BIN: usize = 1;

const ONE_HZ: NonZeroU16 = match NonZeroU16::new(1) {
    Some(f) => f,
    None => unreachable!(),
};

#[inline(never)]
pub fn find_peaks(
    bins: &[Complex<i16>; config::fft::BUF_LEN_COMPLEX_REAL],
    amplitude_threshold: control::Sample,
    peaks_out: &mut Vec<Peak, { config::fft::analysis::MAX_PEAKS }>,
) {
    struct PeakLoc {
        /// Index of highest point in the peak
        i: usize,
        /// Leftmost index of peak, inclusive (samples monotonically decrease from highest point to here)
        left: usize,
        /// Rightmost index of peak, inclusive (samples monotonically decrease from highest point to here)
        right: usize,
        /// Real frequency at the peak, adjusted based on nearby samples
        freq: NonZeroU16,
    }

    let mut peaks: Vec<PeakLoc, { config::fft::analysis::MAX_PEAKS }> = Vec::new();

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

    'next_peak: for _ in 0..peaks.capacity() {
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
            for peak in &peaks {
                if i >= peak.left && i <= peak.right {
                    continue 'next_bin;
                }
            }
            // new max found
            max_amplitude_squared = amplitude_squared;
            i_at_max = i;
        }

        // Step 2: check if highest point is above the noise floor
        // (points below the user-controlled threshold will be culled below)
        if max_amplitude_squared < config::fft::analysis::NOISE_FLOOR_AMPLITUDE_SQUARED {
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
        peaks
            .push(PeakLoc {
                i: i_at_max,
                left,
                right,
                // computed in the next step
                freq: ONE_HZ,
            })
            .unwrap_or_else(|_| panic!("too many peaks found (impossible)"));
    }

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

    for peak in &mut peaks {
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
        // offset to ensure we don't divide by 0 if center would be at 0
        let offset = 1;
        let center = center + offset;
        let large_side = large_side + offset;
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
        // ensure freq is nonzero
        let real_freq = NonZeroU16::new(real_freq).unwrap_or(ONE_HZ);

        // Step 6: store adjusted frequency
        peak.freq = real_freq;
    }

    // Phase 3: cull peaks below threshold

    if let Some(highest) = peaks.first() {
        let amplitude_threshold = {
            let highest_amplitude = amplitude_sqrt(amplitude_squared(bins[highest.i]));
            // ensure highest amplitude is above noise floor (it should be, but sqrt might not be precise)
            let highest_amplitude =
                highest_amplitude.max(config::fft::analysis::NOISE_FLOOR_AMPLITUDE);

            amplitude_threshold
                .to_value_in_range(config::fft::analysis::NOISE_FLOOR_AMPLITUDE..highest_amplitude)
        };

        peaks.retain(|peak| {
            let bin = bins[peak.i];
            let amplitude = amplitude_sqrt(amplitude_squared(bin));
            amplitude >= amplitude_threshold
        });
    }

    // Phase 4: store peaks

    peaks_out.replace_with_mapped(&peaks, |peak| {
        Peak::from_bin_and_freq(bins[peak.i], peak.freq)
    });

    // Phase 5: log peaks

    if config::debug::LOG_FFT_PEAKS {
        for peak in &peaks {
            let bin = bins[peak.i];
            let amplitude = amplitude_sqrt(amplitude_squared(bin));
            let phase_deg = 360.scale_by(phase(bin));

            let peak_freq = peak.freq;
            let center_freq = i_to_freq(peak.i);
            let left_freq = i_to_freq(peak.left);
            let right_freq = i_to_freq(peak.right);

            defmt::println!(
                "Peak amplitude = {} @ freq = {} (mid {}, lo {}, hi {}) Hz, phase = {} deg",
                amplitude,
                peak_freq,
                center_freq,
                left_freq,
                right_freq,
                phase_deg,
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
    freq: NonZeroU16,
    phase: ScalingFactor<u16>,
}

impl Peak {
    fn from_bin_and_freq(bin: Complex<i16>, freq: NonZeroU16) -> Self {
        let phase = phase(bin);
        Self { freq, phase }
    }

    pub fn freq(&self) -> Hertz<u32> {
        u32::from(self.freq.get()).Hz()
    }

    pub fn period<const DENOM: u32>(&self) -> Duration<u32, 1, DENOM> {
        self.freq().into_duration()
    }

    pub fn phase_offset<const DENOM: u32>(&self) -> Duration<u32, 1, DENOM> {
        let period_ticks = self.period::<DENOM>().ticks();
        let phase_offset_ticks = period_ticks.scale_by(self.phase);
        Duration::<u32, 1, DENOM>::from_ticks(phase_offset_ticks)
    }
}
