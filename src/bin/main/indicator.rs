use crate::config;
use crate::fft::analysis::Peak;
use crate::math::Truncate;
use crate::panic::OptionalExt;
use heapless::Vec;

/// Number of indicators to distribute the scaling factor between
const N: usize = 4;

/// Compute scaling factors for amplitude indicator, based on raw ADC samples.
#[inline(never)]
pub fn amplitude_scaling_factors(input: &[u16; config::adc::BUF_LEN_RAW]) -> [u16; N] {
    // Step 1: find min and max samples

    let mut min_sample = u16::MAX;
    let mut max_sample = 0;

    for &sample in input {
        min_sample = min_sample.min(sample);
        max_sample = max_sample.max(sample);
    }

    // Step 2: find the closer of the two maximal samples to clipping

    let max_possible_sample = config::adc::MAX_POSSIBLE_SAMPLE;
    let min_possible_sample = 0;

    let close_to_max = max_possible_sample - max_sample;
    let close_to_min = min_sample - min_possible_sample;

    let closest_to_clipping = close_to_max.min(close_to_min);

    // Step 3: compute scale factor in resolution bits

    // because we check from both sides, the closest sample to clipping can be at most `max_possible_sample/2` away from an extremity
    // so this value is in `max_possible_sample/2..=max_possible_sample`
    let closeness_to_max_possible_sample = max_possible_sample - closest_to_clipping;

    // Step 4: compute scale factor in full u16 range

    // shift down from `max_possible_sample/2..=max_possible_sample` to `0..=max_possible_sample/2`
    // and then scale up to `0..=max_possible_sample`
    let adjusted_closeness_to_half_max_sample =
        (closeness_to_max_possible_sample - (max_possible_sample / 2)) * 2;
    // scale up from ADC sample range to full u16 range
    let overall_scale_factor = adjusted_closeness_to_half_max_sample
        << (u16::BITS - u32::from(config::adc::RESOLUTION_BITS));

    // Step 5: distribute scale factor

    distribute_scale_factor(overall_scale_factor)
}

/// Compute scaling factors for "above threshold" indicator, based on FFT peaks.
#[inline(never)]
pub fn threshold_scaling_factors(
    peaks: &Vec<Peak, { config::fft::analysis::MAX_PEAKS }>,
) -> [u16; N] {
    // Step 1: divide peaks found by max possible peaks

    let max_peaks: u32 = config::fft::analysis::MAX_PEAKS
        .try_into()
        .unwrap_infallible();
    let overall_scale_factor: u32 = u32::from(u16::MAX) * peaks.len() as u32 / max_peaks;
    let overall_scale_factor: u16 = overall_scale_factor.truncate();

    // Step 2: distribute scale factor

    distribute_scale_factor(overall_scale_factor)
}

/// Split scale factor up into N buckets.
///
/// For example, an overall scale factor of 62.5% (5/8) would be distributed over 4 buckets to: 100% 100% 50% 0%.
fn distribute_scale_factor(overall_scale_factor: u16) -> [u16; N] {
    let mut factors = [0; N];

    for (i, factor) in factors.iter_mut().enumerate() {
        let max_factor_over_n: u16 = u16::MAX / N.truncate();
        let local_factor_over_n: u16 =
            overall_scale_factor.saturating_sub(i.truncate() * max_factor_over_n);
        let local_factor: u16 = if local_factor_over_n >= max_factor_over_n {
            u16::MAX
        } else {
            local_factor_over_n * N.truncate()
        };
        *factor = local_factor;
    }

    factors
}
