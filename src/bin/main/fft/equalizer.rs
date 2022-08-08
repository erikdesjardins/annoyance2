use crate::config;
use crate::math::{ScaleBy, ScalingFactor, Truncate};
use crate::panic::OptionalExt;
use num_complex::Complex;

/// Corrects the non-flat frequency response of the FFT.
///
/// This probably shouldn't be necessary?
#[inline(never)]
pub fn apply_to(bins: &mut [Complex<i16>; config::fft::BUF_LEN_COMPLEX_REAL]) {
    for (i, bin) in bins.iter_mut().enumerate() {
        // The frequency response of the FFT is decently approximated by a line
        // passing through (0, MAX_AMPLITUDE) and (MAX_FREQ, 0).

        // Scale amplitudes in order to invert this non-flat frequency response,
        // by effectively multiplying each sample by `1/<amplitude for ideal signal at this freq>`.

        let i: u16 = i.try_into().unwrap_infallible();
        let len: u16 = config::fft::BUF_LEN_COMPLEX_REAL
            .try_into()
            .unwrap_infallible();

        // Step 1: approximate freq response at this point for ideal, max amplitude signal

        let progress_in_freq_range = ScalingFactor::from_ratio(i, len);
        // Avoid using the last part of the range, since near the end we'd nearly be multiplying by infinity,
        // which would amplify noise far too much.
        // Also, it seems that this approximation isn't perfect. If we instead (to prevent bad behavior at the endpoints)
        // enforce a maximum to scale up by, but use the full range, we end up scaling amplitudes too much,
        // even at middle frequencies.
        let range_compression = ScalingFactor::from_ratio(15, 16);

        // Compute the response, starting with MAX_AMPLITUDE at 0 Hz, and decreasing as we get closer to MAX_FREQ.
        let freq_response_at_this_point = config::fft::MAX_AMPLITUDE
            - config::fft::MAX_AMPLITUDE
                .scale_by(progress_in_freq_range)
                .scale_by(range_compression);

        let apply_scaling = |x| {
            // Step 2: prescale by max potential divisor, to avoid reducing overall amplitude
            let prescaled: i32 = i32::from(x) * i32::from(config::fft::MAX_AMPLITUDE);

            // Step 3: scale down by frequency response at this point
            let downscaled: i32 = prescaled / i32::from(freq_response_at_this_point);

            // Step 4: truncate back down to i16, which should work since overall amplitude shouldn't exceed max amplitude
            let truncated: i16 = downscaled.truncate();

            truncated
        };

        bin.im = apply_scaling(bin.im);
        bin.re = apply_scaling(bin.re);
    }
}
