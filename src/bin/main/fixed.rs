//! Utilities for fixed-point calculations.

use fixed::types::{I32F32, U32F32};
use fixed_sqrt::FixedSqrt;
use num_complex::Complex;

/// Squared amplitude of a complex number.
///
/// Since integer exponentiation / rooting is monotonic,
/// comparing squared amplitudes is equivalent, and more efficient,
/// since it avoids a `sqrt` to compute the true amplitude.
pub fn amplitude_squared(x: Complex<i16>) -> u32 {
    let re_2 = i32::from(x.re).pow(2) as u32;
    let im_2 = i32::from(x.im).pow(2) as u32;
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
    let angle = angle + I32F32::PI;
    // convert from 0..2pi to 0..1
    let angle = angle / I32F32::PI / I32F32::from_num(2);
    // extract fractional bits
    angle.to_bits() as u32
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
