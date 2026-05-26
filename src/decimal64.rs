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

impl<const S: u32> Decimal64<S> {
    #[inline]
    pub fn from_f64(x: f64) -> Self {
        Self::from_f64_round(x, crate::Round::NearestEven)
    }

    pub fn from_f64_round(x: f64, mode: crate::Round) -> Self {
        if x.is_nan() {
            return Self::ZERO;
        }
        let scale_factor = 10f64.powi(S as i32);
        let scaled = x * scale_factor;
        let rounded = match mode {
            crate::Round::NearestEven => scaled.round_ties_even(),
            crate::Round::Nearest => scaled.round(),
            crate::Round::TruncateTowardZero => scaled.trunc(),
            crate::Round::TowardPosInf => scaled.ceil(),
            crate::Round::TowardNegInf => scaled.floor(),
        };
        // Clamp before cast; saturating cast handles edge cases in Rust 1.45+
        let clamped = rounded.clamp(i64::MIN as f64, i64::MAX as f64);
        Self(clamped as i64)
    }

    #[inline]
    pub fn to_f64(self) -> f64 {
        let scale_factor = 10f64.powi(S as i32);
        (self.0 as f64) / scale_factor
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

    // f64 conversion tests

    #[test]
    fn from_f64_nearest_even_1_005() {
        // 1.005 in f64 is actually ~1.00499999..., so 1.005 * 100 < 100.5
        // NearestEven rounds to 100, not 101
        assert_eq!(Decimal64::<2>::from_f64(1.005).raw(), 100);
    }

    #[test]
    fn from_f64_rounded_four_scale() {
        // 1.23456789 * 10000 = 12345.6789 → rounds to 12346
        assert_eq!(Decimal64::<4>::from_f64(1.23456789).raw(), 12346);
    }

    #[test]
    fn from_f64_nan_is_zero() {
        assert_eq!(Decimal64::<2>::from_f64(f64::NAN).raw(), 0);
    }

    #[test]
    fn from_f64_infinity_clamps_to_max() {
        assert_eq!(Decimal64::<2>::from_f64(f64::INFINITY).raw(), i64::MAX);
    }

    #[test]
    fn to_f64_basic() {
        assert_eq!(Decimal64::<4>(12345).to_f64(), 1.2345_f64);
    }

    #[test]
    fn round_trip_within_precision() {
        // Simple LCG for deterministic pseudo-random values
        let mut seed: u64 = 12345678901234567;
        for _ in 0..1000 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            // Use 40-bit range: well within f64's 53-bit mantissa at scale 4
            let raw = ((seed >> 24) as i64) - (1i64 << 39);
            let d = Decimal64::<4>::from_raw(raw);
            let rt = Decimal64::<4>::from_f64(d.to_f64());
            assert!(
                (rt.raw() - d.raw()).abs() <= 1,
                "round-trip failed: raw={}, rt={}",
                raw,
                rt.raw()
            );
        }
    }
}
