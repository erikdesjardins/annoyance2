//! Utilities for fixed-point calculations.

use crate::panic::OptionalExt;
use fixed::types::{I32F32, U32F32};
use fixed_sqrt::FixedSqrt;
use num_complex::Complex;

/// Squared amplitude of a complex number.
///
/// Since integer exponentiation / rooting is monotonic,
/// comparing squared amplitudes is equivalent to comparing amplitudes,
/// and is more efficient, since it avoids a `sqrt` to compute the amplitude.
pub fn amplitude_squared(x: Complex<i16>) -> u32 {
    let re_2: u32 = i32::from(x.re).pow(2).try_into().unwrap_infallible();
    let im_2: u32 = i32::from(x.im).pow(2).try_into().unwrap_infallible();
    re_2.checked_add(im_2).unwrap_infallible()
}

/// Phase of a complex number.
///
/// Return value is 0..2pi.
pub fn phase(x: Complex<i16>) -> I32F32 {
    let y = I32F32::from_num(x.im);
    let x = I32F32::from_num(x.re);

    let angle = cordic::atan2(y, x);

    // convert from -pi..pi to 0..2pi
    if angle >= 0 {
        angle
    } else {
        angle + (2 * I32F32::PI)
    }
}

/// Fixed point square root.
pub fn sqrt(x: u32) -> U32F32 {
    let x = U32F32::from_num(x);
    FixedSqrt::sqrt(x)
}

/// Fixed point scaling.
pub const fn scale_by(x: i16, scale_factor: u16 /* u16::MAX ~ 1.0 */) -> i16 {
    ((x as i32 * scale_factor as i32) >> 16) as i16
}
