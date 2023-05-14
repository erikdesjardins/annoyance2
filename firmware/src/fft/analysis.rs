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

#[derive(Copy, Clone)]
pub struct ScratchPeak {
    center: Option<NonZeroU16>,
}

impl ScratchPeak {
    fn new(center: usize) -> Self {
        const _: () = assert!(
            config::fft::BUF_LEN_COMPLEX_REAL - 1 <= u16::MAX as usize,
            "indexes can fit into u16",
        );
        Self {
            center: NonZeroU16::new(center.truncate()),
        }
    }

    // The index of the bin containing the highest amplitude in the peak,
    // or none if the peak has been consumed.
    fn i(self) -> Option<usize> {
        Some(self.center?.get() as usize)
    }

    fn consume(&mut self) {
        self.center = None;
    }
}

#[inline(never)]
pub fn find_peaks(
    bins: &[Complex<i16>; config::fft::BUF_LEN_COMPLEX_REAL],
    scratch_peaks: &mut Vec<ScratchPeak, { config::fft::analysis::MAX_SCRATCH_PEAKS }>,
    amplitude_threshold: control::Sample,
    peaks_out: &mut Vec<Peak, { config::fft::analysis::MAX_PEAKS }>,
) {
    // Phase 1: find scratch peaks (all peaks regardless of amplitude)
    //
    // For example:
    //
    // ...   1    2        3
    // --|---+--|-+-------|+|
    //   |      |         | |
    //   |   .  |         | |
    //   |  . . | .       | |
    //   | .   .|. .      | |
    //   |.     .   .     |.|
    // ...           ...... ...
    //
    {
        scratch_peaks.clear();

        let mut left = FIRST_NON_DC_BIN;
        loop {
            if left + 1 >= bins.len() {
                break;
            }

            // Step 1: ascend to peak
            let mut i = left;
            let mut last_amplitude_squared = amplitude_squared(bins[i]);
            let center = loop {
                if i + 1 >= bins.len() {
                    break i;
                }
                let next_amplitude_squared = amplitude_squared(bins[i + 1]);
                if next_amplitude_squared < last_amplitude_squared {
                    break i;
                }
                last_amplitude_squared = next_amplitude_squared;
                i += 1;
            };

            // Step 2: descend to trough
            let mut i = center;
            let mut last_amplitude_squared = amplitude_squared(bins[i]);
            let right = loop {
                if i + 1 >= bins.len() {
                    break i;
                }
                let next_amplitude_squared = amplitude_squared(bins[i + 1]);
                if next_amplitude_squared > last_amplitude_squared {
                    break i;
                }
                last_amplitude_squared = next_amplitude_squared;
                i += 1;
            };

            // Step 3: append peak
            scratch_peaks
                .push(ScratchPeak::new(center))
                .unwrap_or_else(|_| panic!("too many scratch peaks found (impossible)"));

            // Step 4: left side of next peak is right side of last peak
            left = right;
        }

        log_scratch_peaks(scratch_peaks);
    }

    // Phase 2: extract and process highest peaks
    {
        peaks_out.clear();

        let mut absolute_highest_amplitude = None;

        // For each slot in our output buffer...
        for _ in 0..peaks_out.capacity() {
            // Step 1: find highest non-consumed peak
            let mut scratch_iter = scratch_peaks.iter_mut();
            let mut max_peak = match scratch_iter.next() {
                Some(peak) => peak,
                // no more peaks, stop looking
                None => break,
            };
            let mut max_peak_i = match max_peak.i() {
                Some(i) => i,
                // if the first peak has been consumed, just use 0, since it'll be overwritten or culled before this is used
                None => 0,
            };
            let mut max_amplitude_squared = match max_peak.i() {
                Some(i) => amplitude_squared(bins[i]),
                // if the first peak has been consumed, set its amplitude to 0 to ensure it'll be overwritten or culled
                None => 0,
            };
            for peak in scratch_iter {
                let i = match peak.i() {
                    Some(i) => i,
                    // this peak has been consumed
                    None => continue,
                };
                let amplitude_squared = amplitude_squared(bins[i]);
                if amplitude_squared > max_amplitude_squared {
                    max_peak = peak;
                    max_peak_i = i;
                    max_amplitude_squared = amplitude_squared;
                }
            }

            // Step 2: consume highest peak
            max_peak.consume();
            #[allow(unused_variables)]
            let max_peak = ();

            // Step 3: cull peaks below min frequency
            let min_freq_bin = (config::fft::analysis::MIN_FREQ * 1000)
                .div_round(config::fft::FREQ_RESOLUTION_X1000);
            if max_peak_i <= min_freq_bin {
                continue;
            }

            // Step 4: compute non-squared max amplitude
            let max_amplitude = amplitude_sqrt(max_amplitude_squared);

            // Step 5: cull peaks below noise floor
            if max_amplitude < config::fft::analysis::NOISE_FLOOR_AMPLITUDE {
                break;
            }

            // Step 6: cull peaks below threshold
            match absolute_highest_amplitude {
                None => {
                    // no highest peak yet, set it to this peak (the first and highest peak)
                    absolute_highest_amplitude = Some(max_amplitude);
                }
                Some(absolute_highest_amplitude) => {
                    let amplitude_threshold = amplitude_threshold.to_value_in_range(
                        config::fft::analysis::NOISE_FLOOR_AMPLITUDE..absolute_highest_amplitude,
                    );

                    if max_amplitude < amplitude_threshold {
                        break;
                    }
                }
            }

            // Step 7: refine the peak frequency based on shape of the peak
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
            let freq = {
                // Step 6.1: compute amplitudes
                let center = max_amplitude;
                let sides = [max_peak_i - 1, max_peak_i + 1].map(|i| match bins.get(i) {
                    Some(bin) => amplitude_sqrt(amplitude_squared(*bin)),
                    // at extreme values, duplicate the center amplitude
                    None => center,
                });

                // Step 6.2: determine whether to adjust the frequency positively (right) or negatively (left)
                let is_positive = sides[0] < sides[1];
                let (small_side, large_side) = if is_positive {
                    (sides[0], sides[1])
                } else {
                    (sides[1], sides[0])
                };

                // Step 6.3: normalize amplitudes so the small side is at 0
                let center = center - small_side;
                let large_side = large_side - small_side;
                #[allow(unused_variables)]
                let small_side = ();

                // Step 6.4: compute adjustment (from 0 to 1/2 of bin resolution)
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

                // Step 6.5: apply adjustment
                let center_freq_x1000 = max_peak_i * config::fft::FREQ_RESOLUTION_X1000;
                let real_freq_x1000 = if is_positive {
                    center_freq_x1000 + adjustment_x1000
                } else {
                    center_freq_x1000 - adjustment_x1000
                };
                let real_freq = real_freq_x1000.div_round(1000);
                // truncate frequency: we expect to only be working with < 10 kHz, which is less than u16::MAX
                let real_freq: u16 = real_freq.truncate();

                // Step 6.6: apply nightcore adjustment
                let real_freq = real_freq + real_freq.scale_by(config::fft::analysis::NIGHTCORE);

                // Step 6.7: ensure frequency is valid
                if real_freq > config::fft::analysis::MAX_FREQ {
                    continue;
                }
                let Some(real_freq) = NonZeroU16::new(real_freq) else {
                    continue;
                };

                // Step 6.6: store adjusted frequency
                real_freq
            };

            // Step 8: store peak
            peaks_out
                .push(Peak::from_bin_and_freq(bins[max_peak_i], freq))
                .unwrap_or_else(|_| panic!("too many peaks found (impossible)"));
        }
    }
}

pub fn log_scratch_peaks_prelude() {
    if config::debug::LOG_FFT_SCRATCH_PEAKS {
        defmt::println!(".vz 2 cn FFT Scratch Peaks");
        defmt::println!(".vz 2 xn Frequency (Hz)");
        let mut freqs = [0u16; config::fft::BUF_LEN_COMPLEX_REAL];
        for (i, freq) in freqs.iter_mut().enumerate() {
            *freq = (config::fft::FREQ_RESOLUTION_X1000 * i / 1000).truncate();
        }
        defmt::println!(".vz 2 xs {}", freqs);
    }
}

pub fn log_scratch_peaks(
    scratch_peaks: &Vec<ScratchPeak, { config::fft::analysis::MAX_SCRATCH_PEAKS }>,
) {
    if config::debug::LOG_FFT_SCRATCH_PEAKS {
        let mut is_scratch_peak = [0u8; config::fft::BUF_LEN_COMPLEX_REAL];
        for peak in scratch_peaks.iter() {
            if let Some(i) = peak.i() {
                is_scratch_peak[i] = 1;
            }
        }
        defmt::println!(".vz 2 ys {}", is_scratch_peak);
    }
}

pub fn log_peaks(peaks: &[Peak]) {
    if config::debug::LOG_FFT_PEAKS {
        for peak in peaks {
            defmt::println!(
                "Peak amplitude = {}, freq = {}, phase = {} deg",
                peak.amplitude(),
                peak.freq().to_Hz(),
                360.scale_by(peak.phase()),
            );
        }
    }
}

/// Represents one peak frequency from the FFT, with frequency and scale factor
pub struct Peak {
    amplitude: u16,
    freq: NonZeroU16,
    phase: ScalingFactor<u16>,
}

impl Peak {
    fn from_bin_and_freq(bin: Complex<i16>, freq: NonZeroU16) -> Self {
        let amplitude = amplitude_sqrt(amplitude_squared(bin));
        let phase = phase(bin);
        Self {
            amplitude,
            freq,
            phase,
        }
    }

    pub fn amplitude(&self) -> u16 {
        self.amplitude
    }

    pub fn freq(&self) -> Hertz<u32> {
        u32::from(self.freq.get()).Hz()
    }

    pub fn period<const DENOM: u32>(&self) -> Duration<u32, 1, DENOM> {
        self.freq().into_duration()
    }

    pub fn phase(&self) -> ScalingFactor<u16> {
        self.phase
    }

    pub fn phase_offset<const DENOM: u32>(&self) -> Duration<u32, 1, DENOM> {
        let period_ticks = self.period::<DENOM>().ticks();
        let phase_offset_ticks = period_ticks.scale_by(self.phase);
        Duration::<u32, 1, DENOM>::from_ticks(phase_offset_ticks)
    }
}
