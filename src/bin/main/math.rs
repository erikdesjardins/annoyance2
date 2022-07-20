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
    let bits: u16 = bits.truncate();
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
    // deliberate truncation (shouldn't happen, but if it does, 0 cycles = 1 cycle [in phase], so it doesn't matter)
    let angle = (angle.to_bits() >> (48 - 16)) as u16;
    angle
}

/// Fixed point scaling.
///
/// The `factor` argument represents scaling from 0 (at `0`) to 1 (at `T::MAX`).
pub trait ScaleBy<Factor> {
    fn scale_by(self, by: Factor) -> Self;
}

macro_rules! impl_scaleby {
    ($this:ty, by: $factor:ty, via: $intermediate:ty, $const_shim:ident) => {
        impl ScaleBy<$factor> for $this {
            fn scale_by(self, by: $factor) -> Self {
                ((self as $intermediate * by as $intermediate) >> <$factor>::BITS) as $this
            }
        }

        #[allow(dead_code)]
        pub const fn $const_shim(this: $this, by: $factor) -> $this {
            ((this as $intermediate * by as $intermediate) >> <$factor>::BITS) as $this
        }
    };
}

impl_scaleby!(i16, by: u16, via: i32, const_scale_by_i16_u16);
impl_scaleby!(i32, by: u16, via: i64, const_scale_by_i32_u16);
impl_scaleby!(u16, by: u16, via: u32, const_scale_by_u16_u16);
impl_scaleby!(u32, by: u16, via: u64, const_scale_by_u32_u16);

/// Integer truncation, checked in debug mode.
pub trait Truncate<To> {
    fn truncate(self) -> To;
}

macro_rules! impl_truncate {
    ($from:ty => $to:ty) => {
        const _: () = assert!(<$to>::BITS <= <$from>::BITS);

        impl Truncate<$to> for $from {
            fn truncate(self) -> $to {
                debug_assert!(self <= <$to>::MAX as $from);
                #[allow(clippy::cast_possible_truncation)]
                let truncated = self as $to;
                truncated
            }
        }
    };
}

impl_truncate!(usize => u16);
impl_truncate!(u32 => u16);
impl_truncate!(isize => i16);
impl_truncate!(i32 => i16);

/// Rounded integer division.
pub trait DivRound {
    fn div_round(self, by: Self) -> Self;
}

macro_rules! impl_divround {
    ($self:ty) => {
        impl DivRound for $self {
            fn div_round(self, by: Self) -> Self {
                let round = by / 2;
                #[allow(unused_comparisons)]
                if self >= 0 {
                    (self + round) / by
                } else {
                    (self - round) / by
                }
            }
        }
    };
}

impl_divround!(u16);
impl_divround!(u32);
impl_divround!(usize);
impl_divround!(i16);
impl_divround!(i32);
impl_divround!(isize);
