//! Utilities for fixed-point calculations.

use fixed::types::I16F16;
use num_complex::Complex;
use num_integer::sqrt;

pub fn amplitude(x: Complex<i16>) -> u32 {
    sqrt((x.re as i32).pow(2) as u32 + (x.im as i32).pow(2) as u32)
}

pub fn phase(x: Complex<i16>) -> u16 /* u16::MAX ~ 2*pi */ {
    let y = I16F16::from_num(x.im);
    let x = I16F16::from_num(x.re);

    let angle = cordic::atan2(y, x);
    // convert from -pi..pi to 0..2pi
    let angle = angle + I16F16::PI;
    // convert from 0..2pi to 0..1
    let angle = angle / I16F16::from_num(2) / I16F16::PI;
    // extract fractional bits
    angle.to_bits() as u16
}

pub fn scale_by(x: i16, scale_factor: u16 /* u16::MAX ~ 1.0 */) -> i16 {
    ((x as i32 * scale_factor as i32) >> 16) as i16
}
