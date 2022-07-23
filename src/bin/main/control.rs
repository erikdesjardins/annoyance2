use crate::config;
use crate::math::ScaleBy;
use core::convert::identity;
use core::ops::{Add, Range, Sub};
use defmt::Format;

#[derive(Copy, Clone, Format)]
pub struct Sample(u16);

impl Sample {
    /// Create a control::Sample from an ADC sample from a control.
    pub fn new(sample: u16) -> Self {
        Self(sample)
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
        // Step 1: scale up sample from ADC range to full u16 range
        let scaling_factor = self.0 << (u16::BITS - u32::from(config::adc::RESOLUTION_BITS));

        // Step 2: split range into base value + additional size
        let base = into(range.start);
        let size = into(range.end) - into(range.start);

        // Step 3: pick a value in the range
        let value = from(base + size.scale_by(scaling_factor));

        value
    }
}
