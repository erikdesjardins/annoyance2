//! Utilities for fixed-point calculations.

use crate::panic::OptionalExt;
use fixed::types::{I32F32, U32F32};
use fixed_sqrt::FixedSqrt;
use num_complex::Complex;

/// Squared amplitude of a complex number.
///
/// Since integer exponentiation / rooting is monotonic,
/// comparing squared amplitudes is equivalent, and more efficient,
/// since it avoids a `sqrt` to compute the true amplitude.
pub fn amplitude_squared(x: Complex<i16>) -> u32 {
    let re_2: u32 = i32::from(x.re).pow(2).try_into().unwrap_infallible();
    let im_2: u32 = i32::from(x.im).pow(2).try_into().unwrap_infallible();
    re_2 + im_2
}

/// Phase of a complex number.
///
/// Return value is scaled up such that u32::MAX is approximately 2pi rad.
pub fn phase(x: Complex<i16>) -> u32 {
    let y = I32F32::from_num(x.im);
    let x = I32F32::from_num(x.re);

    let angle = cordic::atan2(y, x);
    // convert from -pi..pi to 0..2pi
    let angle = if angle >= 0 {
        angle
    } else {
        angle + 2 * I32F32::PI
    };
    // convert from 0..2pi to 0..1
    let angle = angle / I32F32::PI / 2;
    // extract fractional bits
    let full_bits: i64 = angle.to_bits();
    let positive_bits: u64 = full_bits.try_into().unwrap();
    debug_assert!(positive_bits <= u64::from(u32::MAX));
    let fractional_bits: u32 = positive_bits.try_into().unwrap_or(u32::MAX);
    fractional_bits
}

/// Fixed point square root.
///
/// Return value is scaled up by a factor of 2^32, giving 32 bits of fractional precision.
pub fn sqrt(x: u32) -> u64 {
    let x = U32F32::from_num(x);
    let sqrt = FixedSqrt::sqrt(x);
    sqrt.to_bits()
}

/// Fixed point scaling.
pub fn scale_by(x: i16, scale_factor: u16 /* u16::MAX ~ 1.0 */) -> i16 {
    ((i32::from(x) * i32::from(scale_factor)) >> 16) as i16
}
