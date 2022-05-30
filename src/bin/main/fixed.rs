//! Utilities for fixed-point calculations.

use crate::panic::OptionalExt;
use fixed::types::{I16F48, U32F0};
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

/// Square root of a large amplitude value.
///
/// This is much more expensive than `amplitude_squared`.
///
/// Intended to be used with `amplitude_squared`, which returns large already-squared values.
/// Since all fractional bits are discarded, this may not produce accurate results for small values.
pub fn amplitude_sqrt(x: u32) -> u16 {
    let x = U32F0::from_num(x);
    let sqrt = FixedSqrt::sqrt(x);
    // truncate sqrt, which should fit into half the bits
    let bits: u32 = sqrt.to_bits();
    let bits: u16 = bits.try_into().unwrap_or_else(|_| {
        if cfg!(debug_assertions) {
            panic!("overflow in sqrt truncation: {}", bits);
        } else {
            u16::MAX
        }
    });
    bits
}

/// Phase of a complex number.
///
/// Returns a scale factor (representing 0..2pi), ready to pass to `scale_by`.
pub fn phase(x: Complex<i16>) -> u16 {
    let y = I16F48::from_num(x.im);
    let x = I16F48::from_num(x.re);
    let angle = cordic::atan2(y, x);
    // convert from -pi..pi to 0..2pi
    let angle = if angle >= 0 {
        angle
    } else {
        angle + (2 * I16F48::PI)
    };
    // convert from 0..2pi to 0..1
    let angle = angle / I16F48::PI / 2;
    // extract 16 most significant bits of fraction
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let angle = (angle.to_bits() >> (48 - 16)) as u16;
    angle
}

/// Fixed point scaling.
pub const fn scale_by(x: i16, scale_factor: u16 /* u16::MAX ~ 1.0 */) -> i16 {
    ((x as i32 * scale_factor as i32) >> 16) as i16
}
