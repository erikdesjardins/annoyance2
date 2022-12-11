use crate::config;
use crate::math::{ScaleBy, ScalingFactor};
use core::convert::identity;
use core::ops::{Add, Range, Sub};
use defmt::Format;

#[derive(Copy, Clone, Format)]
pub struct Sample {
    value: ScalingFactor<u16>,
}

impl Sample {
    /// Create a control::Sample from an ADC sample from a control.
    pub fn new(sample: u16) -> Self {
        Self {
            value: ScalingFactor::from_sample::<{ config::adc::RESOLUTION_BITS }>(sample),
        }
    }

    /// Pick a value from a given range.
    pub fn to_value_in_range(self, range: Range<u16>) -> u16 {
        self.to_value_in_range_via(range, identity, identity)
    }

    /// Pick a value from a given range.
    /// Performs the computation in a different type, then converts back to the original type via the given conversions.
    pub fn to_value_in_range_via<Orig, Scalable>(
        self,
        range: Range<Orig>,
        into: impl Fn(Orig) -> Scalable,
        from: impl Fn(Scalable) -> Orig,
    ) -> Orig
    where
        Orig: Copy,
        Scalable: Add<Output = Scalable> + Sub<Output = Scalable> + ScaleBy<u16>,
    {
        // split range into base value + additional size
        let base = into(range.start);
        let size = into(range.end) - into(range.start);

        // pick a value in the range
        let value = from(base + size.scale_by(self.value));

        value
    }
}
