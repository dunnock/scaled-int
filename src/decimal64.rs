use std::fmt;
use std::ops::{Add, Div, Mul, Neg, Sub};
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
    pub fn parse(s: &str) -> Result<Self, crate::ParseError> {
        crate::parse::parse::<S>(s)
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

// ── Arithmetic trait impls ────────────────────────────────────────────────────

impl<const S: u32> Add for Decimal64<S> {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self(self.0.checked_add(rhs.0).expect("Decimal64 addition overflow"))
    }
}

impl<const S: u32> Sub for Decimal64<S> {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self(self.0.checked_sub(rhs.0).expect("Decimal64 subtraction overflow"))
    }
}

impl<const S: u32> Neg for Decimal64<S> {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Self(self.0.checked_neg().expect("Decimal64 negation overflow"))
    }
}

impl<const S: u32> Mul for Decimal64<S> {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Self) -> Self {
        self.checked_mul(rhs).expect("Decimal64 multiplication overflow")
    }
}

impl<const S: u32> Div for Decimal64<S> {
    type Output = Self;
    #[inline]
    fn div(self, rhs: Self) -> Self {
        self.checked_div(rhs).expect("Decimal64 division by zero")
    }
}

// ── Checked / saturating / rounding variants ──────────────────────────────────

impl<const S: u32> Decimal64<S> {
    #[inline]
    pub fn checked_add(self, rhs: Self) -> Option<Self> {
        self.0.checked_add(rhs.0).map(Self)
    }

    #[inline]
    pub fn checked_sub(self, rhs: Self) -> Option<Self> {
        self.0.checked_sub(rhs.0).map(Self)
    }

    pub fn checked_mul(self, rhs: Self) -> Option<Self> {
        let product = self.0 as i128 * rhs.0 as i128;
        let scale = const_pow10(S) as i128;
        let result = product / scale;
        if result >= i64::MIN as i128 && result <= i64::MAX as i128 {
            Some(Self(result as i64))
        } else {
            None
        }
    }

    pub fn checked_div(self, rhs: Self) -> Option<Self> {
        if rhs.0 == 0 {
            return None;
        }
        let num = self.0 as i128 * const_pow10(S) as i128;
        let result = num / rhs.0 as i128;
        if result >= i64::MIN as i128 && result <= i64::MAX as i128 {
            Some(Self(result as i64))
        } else {
            None
        }
    }

    #[inline]
    pub fn saturating_add(self, rhs: Self) -> Self {
        Self(self.0.saturating_add(rhs.0))
    }

    #[inline]
    pub fn saturating_sub(self, rhs: Self) -> Self {
        Self(self.0.saturating_sub(rhs.0))
    }

    pub fn saturating_mul(self, rhs: Self) -> Self {
        let product = self.0 as i128 * rhs.0 as i128;
        let scale = const_pow10(S) as i128;
        let result = product / scale;
        Self(result.clamp(i64::MIN as i128, i64::MAX as i128) as i64)
    }

    pub fn div_round(self, rhs: Self, mode: crate::Round) -> Self {
        self.checked_div_round(rhs, mode)
            .expect("Decimal64 div_round: division by zero or overflow")
    }

    pub fn checked_div_round(self, rhs: Self, mode: crate::Round) -> Option<Self> {
        if rhs.0 == 0 {
            return None;
        }
        let num = self.0 as i128 * const_pow10(S) as i128;
        let den = rhs.0 as i128;
        let result = div_round_i128(num, den, mode);
        if result >= i64::MIN as i128 && result <= i64::MAX as i128 {
            Some(Self(result as i64))
        } else {
            None
        }
    }

    pub fn rescale_into<const OUT: u32>(self) -> Option<Decimal64<OUT>> {
        if OUT > S {
            let factor = const_pow10(OUT - S);
            self.0.checked_mul(factor).map(Decimal64::from_raw)
        } else if OUT < S {
            let factor = const_pow10(S - OUT) as i128;
            let val = self.0 as i128;
            if val % factor != 0 {
                None
            } else {
                Some(Decimal64::from_raw((val / factor) as i64))
            }
        } else {
            Some(Decimal64::from_raw(self.0))
        }
    }

    pub fn rescale_round_into<const OUT: u32>(self, mode: crate::Round) -> Option<Decimal64<OUT>> {
        if OUT > S {
            let factor = const_pow10(OUT - S);
            self.0.checked_mul(factor).map(Decimal64::from_raw)
        } else if OUT < S {
            let factor = const_pow10(S - OUT) as i128;
            let result = div_round_i128(self.0 as i128, factor, mode);
            if result >= i64::MIN as i128 && result <= i64::MAX as i128 {
                Some(Decimal64::from_raw(result as i64))
            } else {
                None
            }
        } else {
            Some(Decimal64::from_raw(self.0))
        }
    }
}

/// Integer division with rounding. `den` must be non-zero.
/// Uses truncating division as the base; applies `mode` to adjust.
fn div_round_i128(num: i128, den: i128, mode: crate::Round) -> i128 {
    debug_assert!(den != 0);
    let q = num / den;
    let r = num % den; // same sign as num (Rust truncates toward zero)

    if r == 0 {
        return q;
    }

    match mode {
        crate::Round::TruncateTowardZero => q,
        crate::Round::TowardPosInf => {
            // ceil: add 1 when the fractional part is positive (r and den same sign)
            if (r > 0) == (den > 0) { q + 1 } else { q }
        }
        crate::Round::TowardNegInf => {
            // floor: subtract 1 when the fractional part is negative (r and den opposite sign)
            if (r > 0) != (den > 0) { q - 1 } else { q }
        }
        crate::Round::Nearest => {
            // half away from zero
            let abs_2r = r.unsigned_abs().saturating_mul(2);
            let abs_d = den.unsigned_abs();
            if abs_2r >= abs_d {
                if (r > 0) == (den > 0) { q + 1 } else { q - 1 }
            } else {
                q
            }
        }
        crate::Round::NearestEven => {
            // banker's rounding
            let abs_2r = r.unsigned_abs().saturating_mul(2);
            let abs_d = den.unsigned_abs();
            if abs_2r > abs_d {
                if (r > 0) == (den > 0) { q + 1 } else { q - 1 }
            } else if abs_2r == abs_d {
                if q % 2 != 0 {
                    if (r > 0) == (den > 0) { q + 1 } else { q - 1 }
                } else {
                    q
                }
            } else {
                q
            }
        }
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

    // ── Arithmetic tests ──────────────────────────────────────────────────────

    #[test]
    fn add_basic() {
        assert_eq!(
            Decimal64::<2>(100) + Decimal64::<2>(50),
            Decimal64::<2>(150)
        );
    }

    #[test]
    fn sub_basic() {
        assert_eq!(
            Decimal64::<2>(100) - Decimal64::<2>(150),
            Decimal64::<2>(-50)
        );
    }

    #[test]
    fn neg_basic() {
        assert_eq!(-Decimal64::<2>(100), Decimal64::<2>(-100));
    }

    #[test]
    #[should_panic]
    fn neg_min_panics() {
        let _ = -Decimal64::<2>::MIN;
    }

    #[test]
    fn checked_add_overflow_returns_none() {
        assert_eq!(Decimal64::<2>::MAX.checked_add(Decimal64::<2>(1)), None);
    }

    #[test]
    fn saturating_add_clamps_to_max() {
        assert_eq!(
            Decimal64::<2>::MAX.saturating_add(Decimal64::<2>(1)),
            Decimal64::<2>::MAX
        );
    }

    #[test]
    fn saturating_sub_clamps_to_min() {
        assert_eq!(
            Decimal64::<2>::MIN.saturating_sub(Decimal64::<2>(1)),
            Decimal64::<2>::MIN
        );
    }

    #[test]
    fn mul_same_scale() {
        // 1.0000 × 2.0000 = 2.0000  (raw: 10_000 * 20_000 / 10_000 = 20_000)
        assert_eq!(
            Decimal64::<4>(10_000) * Decimal64::<4>(20_000),
            Decimal64::<4>(20_000)
        );
    }

    #[test]
    fn mul_checked_overflow_returns_none() {
        assert_eq!(Decimal64::<4>::MAX.checked_mul(Decimal64::<4>(20_000)), None);
    }

    #[test]
    fn saturating_mul_clamps() {
        assert_eq!(
            Decimal64::<4>::MAX.saturating_mul(Decimal64::<4>(20_000)),
            Decimal64::<4>::MAX
        );
    }

    #[test]
    fn div_same_scale() {
        // 3.0000 / 2.0000 = 1.5000  (raw: 30_000 * 10_000 / 20_000 = 15_000)
        assert_eq!(
            Decimal64::<4>(30_000) / Decimal64::<4>(20_000),
            Decimal64::<4>(15_000)
        );
    }

    #[test]
    fn div_truncates_toward_zero() {
        // 0.10 / 0.03 = 3.333…  raw: (10 * 100) / 3 = 333
        assert_eq!(
            Decimal64::<2>(10) / Decimal64::<2>(3),
            Decimal64::<2>(333)
        );
    }

    #[test]
    fn div_truncates_negative_toward_zero() {
        // -0.10 / 0.03 = -3.333…  raw: (-10 * 100) / 3 = -333
        assert_eq!(
            Decimal64::<2>(-10) / Decimal64::<2>(3),
            Decimal64::<2>(-333)
        );
    }

    #[test]
    fn checked_div_by_zero_returns_none() {
        assert_eq!(Decimal64::<2>(100).checked_div(Decimal64::<2>(0)), None);
    }

    #[test]
    fn div_round_nearest() {
        // 1.0 / 3.0 at scale 2: (100 * 100) / 300 = 33.33… → Nearest = 33
        let result = Decimal64::<2>(100)
            .div_round(Decimal64::<2>(300), crate::Round::Nearest);
        assert_eq!(result, Decimal64::<2>(33));
    }

    #[test]
    fn div_round_toward_pos_inf() {
        // 1.0 / 3.0 at scale 2: 33.33… → ceil = 34
        let result = Decimal64::<2>(100)
            .div_round(Decimal64::<2>(300), crate::Round::TowardPosInf);
        assert_eq!(result, Decimal64::<2>(34));
    }

    #[test]
    fn div_round_toward_neg_inf() {
        // -1.0 / 3.0 at scale 2: -33.33… → floor = -34
        let result = Decimal64::<2>(-100)
            .div_round(Decimal64::<2>(300), crate::Round::TowardNegInf);
        assert_eq!(result, Decimal64::<2>(-34));
    }

    #[test]
    fn div_round_nearest_even_tie() {
        // 0.05 / 0.10 at scale 2: (5 * 100) / 10 = 50 exactly → already integer, no rounding
        let result = Decimal64::<2>(5)
            .div_round(Decimal64::<2>(10), crate::Round::NearestEven);
        assert_eq!(result, Decimal64::<2>(50));
    }

    #[test]
    fn rescale_into_upscale() {
        // Decimal64::<2>(123) = 1.23 → scale 6 = raw 1_230_000
        let result: Option<Decimal64<6>> = Decimal64::<2>(123).rescale_into();
        assert_eq!(result, Some(Decimal64::<6>(1_230_000)));
    }

    #[test]
    fn rescale_into_downscale_lossy_returns_none() {
        // 1.23 cannot be represented exactly at scale 1
        let result: Option<Decimal64<1>> = Decimal64::<2>(123).rescale_into();
        assert_eq!(result, None);
    }

    #[test]
    fn rescale_into_downscale_exact() {
        // 1.20 (raw 120 at scale 2) → scale 1: raw 12 = 1.2 (exact)
        let result: Option<Decimal64<1>> = Decimal64::<2>(120).rescale_into();
        assert_eq!(result, Some(Decimal64::<1>(12)));
    }

    #[test]
    fn rescale_round_into_downscale() {
        // 1.23 at scale 2 → scale 1 with Nearest: 1.23 rounds to 1.2 (raw 12)
        let result: Option<Decimal64<1>> =
            Decimal64::<2>(123).rescale_round_into(crate::Round::Nearest);
        assert_eq!(result, Some(Decimal64::<1>(12)));
    }

    #[test]
    fn rescale_round_into_downscale_rounds_up() {
        // 1.25 at scale 2 → scale 1 with Nearest: 1.25 rounds to 1.3 (raw 13)
        let result: Option<Decimal64<1>> =
            Decimal64::<2>(125).rescale_round_into(crate::Round::Nearest);
        assert_eq!(result, Some(Decimal64::<1>(13)));
    }

    #[test]
    fn rescale_same_scale_is_identity() {
        let d = Decimal64::<4>(12345);
        let result: Option<Decimal64<4>> = d.rescale_into();
        assert_eq!(result, Some(d));
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
