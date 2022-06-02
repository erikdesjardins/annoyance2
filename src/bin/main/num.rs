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
