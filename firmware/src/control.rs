use crate::config;
use crate::math::{ScaleBy, ScalingFactor};
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
    pub fn to_value_in_range<T>(self, range: Range<T>) -> T
    where
        T: Add<Output = T> + Sub<Output = T> + ScaleBy<u16> + Copy,
    {
        // split range into base value + additional size
        let base = range.start;
        let size = range.end - range.start;

        // pick a value in the range
        let value = base + size.scale_by(self.value);

        value
    }
}
