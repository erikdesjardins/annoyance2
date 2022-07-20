use crate::config;
use crate::math::ScaleBy;
use core::convert::identity;
use core::ops::{Add, Range, Sub};

/// Uses an ADC sample from a control to pick a value from a given range.
pub fn adc_sample_to_value_in_range(sample: u16, range: Range<u16>) -> u16 {
    adc_sample_to_value_in_range_via(sample, range, identity, identity)
}

/// Uses an ADC sample from a control to pick a value from a given range.
/// Performs the computation in a different type, then converts back to the original type via the given conversions.
pub fn adc_sample_to_value_in_range_via<Orig, Scalable>(
    sample: u16,
    range: Range<Orig>,
    into: impl Fn(Orig) -> Scalable,
    from: impl Fn(Scalable) -> Orig,
) -> Orig
where
    Orig: Copy,
    Scalable: Add<Output = Scalable> + Sub<Output = Scalable> + ScaleBy<u16>,
{
    // Step 1: scale up sample from ADC range to full u16 range
    let scaling_factor = sample << (u16::BITS - u32::from(config::adc::RESOLUTION_BITS));

    // Step 2: split range into base value + additional size
    let base = into(range.start);
    let size = into(range.end) - into(range.start);

    // Step 3: pick a value in the range
    let value = from(base + size.scale_by(scaling_factor));

    value
}
