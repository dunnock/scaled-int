use std::fmt;
use std::str::FromStr;

const fn const_pow10(s: u32) -> i64 {
    assert!(s <= 18, "Decimal64 scale must be <= 18");
    let mut result: i64 = 1;
    let mut i = 0u32;
    while i < s {
        result *= 10;
        i += 1;
    }
    result
}

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Decimal64<const S: u32>(i64);

impl<const S: u32> Decimal64<S> {
    pub const SCALE: u32 = S;
    pub const ZERO: Self = Self(0);
    pub const ONE: Self = Self(const_pow10(S));
    pub const MAX: Self = Self(i64::MAX);
    pub const MIN: Self = Self(i64::MIN);

    #[inline(always)]
    pub const fn from_raw(raw: i64) -> Self {
        Self(raw)
    }

    #[inline(always)]
    pub const fn raw(self) -> i64 {
        self.0
    }
}

impl<const S: u32> fmt::Display for Decimal64<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if S == 0 {
            return write!(f, "{}", self.0);
        }
        let neg = self.0 < 0;
        let abs = self.0.unsigned_abs();
        let divisor = const_pow10(S) as u64;
        let integer = abs / divisor;
        let frac = abs % divisor;
        if neg {
            write!(f, "-{}.{:0>width$}", integer, frac, width = S as usize)
        } else {
            write!(f, "{}.{:0>width$}", integer, frac, width = S as usize)
        }
    }
}

impl<const S: u32> fmt::Debug for Decimal64<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Decimal64<{}>({})", S, self.0)
    }
}

impl<const S: u32> FromStr for Decimal64<S> {
    type Err = crate::ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        crate::parse::parse::<S>(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_raw_equals_pow10() {
        assert_eq!(Decimal64::<4>::ONE.raw(), 10_000);
    }

    #[test]
    fn display_basic() {
        assert_eq!(Decimal64::<2>(123).to_string(), "1.23");
    }

    #[test]
    fn display_zero_scale() {
        assert_eq!(Decimal64::<0>(42).to_string(), "42");
    }

    #[test]
    fn max_raw() {
        assert_eq!(Decimal64::<4>::MAX.raw(), i64::MAX);
    }

    #[test]
    fn negative_less_than_zero() {
        assert!(Decimal64::<2>(-100) < Decimal64::<2>(0));
    }

    #[test]
    fn display_zero() {
        assert_eq!(Decimal64::<2>(0).to_string(), "0.00");
    }

    #[test]
    fn display_negative() {
        assert_eq!(Decimal64::<2>(-100).to_string(), "-1.00");
    }

    #[test]
    fn display_fractional_padding() {
        assert_eq!(Decimal64::<4>(1234567).to_string(), "123.4567");
    }
}
