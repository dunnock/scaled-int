use core::fmt;
use core::ops::{Add, Div, Mul, Sub};
use core::str::FromStr;

#[inline(always)]
const fn const_pow10_u64(s: u32) -> u64 {
    assert!(s <= 18, "UDecimal64 scale must be <= 18");
    let mut result: u64 = 1;
    let mut i = 0u32;
    while i < s {
        result *= 10;
        i += 1;
    }
    result
}

/// Fixed-scale 64-bit unsigned decimal.
///
/// The raw value is a `u64` whose unit is `10^(-S)`.
/// Scale `S` is a compile-time const; no runtime overhead.
///
/// Only non-negative values are representable; the type system enforces this statically.
/// The full `u64` range doubles the positive capacity vs [`crate::Decimal64`].
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UDecimal64<const S: u32>(u64);

impl<const S: u32> UDecimal64<S> {
    /// The scale parameter `S`.
    pub const SCALE: u32 = S;
    /// Additive identity: `0`.
    pub const ZERO: Self = Self(0);
    /// Multiplicative identity: `1.0` stored as `10^S`.
    pub const ONE: Self = Self(const_pow10_u64(S));
    /// Largest representable value (`u64::MAX` raw).
    pub const MAX: Self = Self(u64::MAX);

    /// Wrap a raw `u64` without any scaling — caller manages the scale invariant.
    #[inline(always)]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Return the raw `u64` storage value (the mathematical value × `10^S`).
    #[inline(always)]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

// ── Parse ────────────────────────────────────────────────────────────────────

impl<const S: u32> UDecimal64<S> {
    /// Parse a decimal string. Signs (`+`, `-`) are rejected. Extra fractional digits
    /// beyond `S` are silently truncated toward zero.
    pub fn parse(s: &str) -> Result<Self, crate::ParseError> {
        crate::parse_unsigned::parse::<S>(s)
    }
}

impl<const S: u32> FromStr for UDecimal64<S> {
    type Err = crate::ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        crate::parse_unsigned::parse::<S>(s)
    }
}

// ── f64 conversions ──────────────────────────────────────────────────────────

impl<const S: u32> UDecimal64<S> {
    /// Convert from `f64` using nearest-even (banker's) rounding.
    ///
    /// `NaN` and negative inputs map to `ZERO`; overflow clamps to `MAX`.
    #[cfg(feature = "std")]
    #[inline]
    pub fn from_f64(x: f64) -> Self {
        Self::from_f64_round_impl::<{ crate::RoundFlag::NEAREST_EVEN }>(x)
    }

    /// Convert from `f64` using nearest-even (banker's) rounding.
    #[cfg(feature = "std")]
    #[inline]
    pub fn from_f64_nearest_even(x: f64) -> Self {
        Self::from_f64_round_impl::<{ crate::RoundFlag::NEAREST_EVEN }>(x)
    }

    /// Convert from `f64` using nearest, ties away from zero.
    #[cfg(feature = "std")]
    #[inline]
    pub fn from_f64_nearest(x: f64) -> Self {
        Self::from_f64_round_impl::<{ crate::RoundFlag::NEAREST }>(x)
    }

    /// Convert from `f64` by truncating toward zero.
    #[cfg(feature = "std")]
    #[inline]
    pub fn from_f64_zero(x: f64) -> Self {
        Self::from_f64_round_impl::<{ crate::RoundFlag::ZERO }>(x)
    }

    /// Convert from `f64` by rounding toward positive infinity.
    #[cfg(feature = "std")]
    #[inline]
    pub fn from_f64_ceil(x: f64) -> Self {
        Self::from_f64_round_impl::<{ crate::RoundFlag::CEIL }>(x)
    }

    /// Convert from `f64` by rounding toward negative infinity.
    #[cfg(feature = "std")]
    #[inline]
    pub fn from_f64_floor(x: f64) -> Self {
        Self::from_f64_round_impl::<{ crate::RoundFlag::FLOOR }>(x)
    }

    #[cfg(feature = "std")]
    fn from_f64_round_impl<const MODE: crate::RoundFlagEnum>(x: f64) -> Self {
        if x.is_nan() || x < 0.0 {
            return Self::ZERO;
        }
        let scale_factor = 10f64.powi(S as i32);
        let scaled = x * scale_factor;
        let rounded = match crate::RoundFlag::from_u8(MODE) {
            crate::RoundFlag::NearestEven => scaled.round_ties_even(),
            crate::RoundFlag::Nearest => scaled.round(),
            crate::RoundFlag::Zero => scaled.trunc(),
            crate::RoundFlag::Ceil => scaled.ceil(),
            crate::RoundFlag::Floor => scaled.floor(),
        };
        // Saturating cast (Rust 1.45+): any f64 >= u64::MAX saturates to u64::MAX.
        let clamped = rounded.clamp(0.0, u64::MAX as f64);
        Self(clamped as u64)
    }

    /// Convert to `f64`. Lossless for `raw < 2^53`; larger values lose the last few ULPs.
    #[inline]
    pub fn to_f64(self) -> f64 {
        (self.0 as f64) / (const_pow10_u64(S) as f64)
    }
}

// ── Signed/unsigned interop ──────────────────────────────────────────────────

impl<const S: u32> UDecimal64<S> {
    /// Convert to `Decimal64<S>`. Returns `None` when the raw value exceeds `i64::MAX`.
    pub fn as_signed(self) -> Option<crate::Decimal64<S>> {
        if self.0 > i64::MAX as u64 {
            None
        } else {
            Some(crate::Decimal64::from_raw(self.0 as i64))
        }
    }
}

/// Extension on `Decimal64<S>` to convert to the unsigned counterpart.
impl<const S: u32> crate::Decimal64<S> {
    /// Convert to `UDecimal64<S>`. Returns `None` for negative values.
    pub fn as_unsigned(self) -> Option<UDecimal64<S>> {
        if self.raw() < 0 {
            None
        } else {
            Some(UDecimal64::from_raw(self.raw() as u64))
        }
    }
}

// ── Arithmetic trait impls ───────────────────────────────────────────────────

impl<const S: u32> Add for UDecimal64<S> {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        self.checked_add(rhs).expect("UDecimal64 addition overflow")
    }
}

/// Subtraction returns `Option<Self>` to prevent silent underflow.
///
/// Use `saturating_sub` to clamp to `ZERO` instead of propagating `None`.
impl<const S: u32> Sub for UDecimal64<S> {
    type Output = Option<Self>;
    #[inline]
    fn sub(self, rhs: Self) -> Option<Self> {
        self.0.checked_sub(rhs.0).map(Self)
    }
}

impl<const S: u32> Mul for UDecimal64<S> {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Self) -> Self {
        self.checked_mul(rhs)
            .expect("UDecimal64 multiplication overflow")
    }
}

impl<const S: u32> Div for UDecimal64<S> {
    type Output = Self;
    #[inline]
    fn div(self, rhs: Self) -> Self {
        self.checked_div(rhs).expect("UDecimal64 division by zero")
    }
}

// ── Checked / saturating / rounding variants ─────────────────────────────────

impl<const S: u32> UDecimal64<S> {
    /// Returns `None` on overflow.
    #[inline]
    pub fn checked_add(self, rhs: Self) -> Option<Self> {
        self.0.checked_add(rhs.0).map(Self)
    }

    /// Returns `None` on underflow (same behavior as the `Sub` trait).
    #[inline]
    pub fn checked_sub(self, rhs: Self) -> Option<Self> {
        self.0.checked_sub(rhs.0).map(Self)
    }

    /// Returns `None` on overflow.
    #[inline(always)]
    pub fn checked_mul(self, rhs: Self) -> Option<Self> {
        // Fast path: u64 product covers most financial values at S <= 5
        if let Some(product) = self.0.checked_mul(rhs.0)
            && S <= 7
        {
            return Some(Self(product / const_pow10_u64(S)));
        }
        // Slow path: full u128 handles large magnitudes
        let product = self.0 as u128 * rhs.0 as u128;
        let scale = const_pow10_u64(S) as u128;
        let result = product / scale;
        Some(Self(result.try_into().ok()?))
    }

    /// Returns `None` on division by zero or result overflow.
    #[inline(always)]
    pub fn checked_div(self, rhs: Self) -> Option<Self> {
        if rhs.0 == 0 {
            return None;
        }
        // Fast path: if scaled numerator fits in u64, use u64 division
        let scale = const_pow10_u64(S);
        if let Some(num) = self.0.checked_mul(scale) {
            return Some(Self(num / rhs.0));
        }
        // Slow path: full u128
        let num = self.0 as u128 * scale as u128;
        let result = num / rhs.0 as u128;
        Some(Self(result.try_into().ok()?))
    }

    /// Clamps to `MAX` on overflow instead of panicking.
    #[inline]
    pub fn saturating_add(self, rhs: Self) -> Self {
        Self(self.0.saturating_add(rhs.0))
    }

    /// Clamps to `ZERO` on underflow instead of wrapping.
    #[inline]
    pub fn saturating_sub(self, rhs: Self) -> Self {
        Self(self.0.saturating_sub(rhs.0))
    }

    /// Clamps to `MAX` on overflow instead of panicking.
    #[inline]
    pub fn saturating_mul(self, rhs: Self) -> Self {
        self.checked_mul(rhs).unwrap_or(Self::MAX)
    }

    /// Divide using nearest-even (banker's) rounding. Panics on division by zero or overflow.
    #[inline]
    pub fn div_round_nearest_even(self, rhs: Self) -> Self {
        self.div_round_impl::<{ crate::RoundFlag::NEAREST_EVEN }>(rhs)
    }

    /// Divide using nearest, ties away from zero. Panics on division by zero or overflow.
    #[inline]
    pub fn div_round_nearest(self, rhs: Self) -> Self {
        self.div_round_impl::<{ crate::RoundFlag::NEAREST }>(rhs)
    }

    /// Divide by truncating toward zero. Panics on division by zero or overflow.
    #[inline]
    pub fn div_round_zero(self, rhs: Self) -> Self {
        self.div_round_impl::<{ crate::RoundFlag::ZERO }>(rhs)
    }

    /// Divide by rounding toward positive infinity. Panics on division by zero or overflow.
    #[inline]
    pub fn div_round_ceil(self, rhs: Self) -> Self {
        self.div_round_impl::<{ crate::RoundFlag::CEIL }>(rhs)
    }

    /// Divide by rounding toward negative infinity. Panics on division by zero or overflow.
    #[inline]
    pub fn div_round_floor(self, rhs: Self) -> Self {
        self.div_round_impl::<{ crate::RoundFlag::FLOOR }>(rhs)
    }

    /// Divide using nearest-even (banker's) rounding. Returns `None` on division by zero or overflow.
    #[inline]
    pub fn checked_div_round_nearest_even(self, rhs: Self) -> Option<Self> {
        self.checked_div_round_impl::<{ crate::RoundFlag::NEAREST_EVEN }>(rhs)
    }

    /// Divide using nearest, ties away from zero. Returns `None` on division by zero or overflow.
    #[inline]
    pub fn checked_div_round_nearest(self, rhs: Self) -> Option<Self> {
        self.checked_div_round_impl::<{ crate::RoundFlag::NEAREST }>(rhs)
    }

    /// Divide by truncating toward zero. Returns `None` on division by zero or overflow.
    #[inline]
    pub fn checked_div_round_zero(self, rhs: Self) -> Option<Self> {
        self.checked_div_round_impl::<{ crate::RoundFlag::ZERO }>(rhs)
    }

    /// Divide by rounding toward positive infinity. Returns `None` on division by zero or overflow.
    #[inline]
    pub fn checked_div_round_ceil(self, rhs: Self) -> Option<Self> {
        self.checked_div_round_impl::<{ crate::RoundFlag::CEIL }>(rhs)
    }

    /// Divide by rounding toward negative infinity. Returns `None` on division by zero or overflow.
    #[inline]
    pub fn checked_div_round_floor(self, rhs: Self) -> Option<Self> {
        self.checked_div_round_impl::<{ crate::RoundFlag::FLOOR }>(rhs)
    }

    #[inline]
    fn div_round_impl<const MODE: crate::RoundFlagEnum>(self, rhs: Self) -> Self {
        self.checked_div_round_impl::<MODE>(rhs)
            .expect("UDecimal64 div_round: division by zero or overflow")
    }

    #[inline]
    fn checked_div_round_impl<const MODE: crate::RoundFlagEnum>(self, rhs: Self) -> Option<Self> {
        if rhs.0 == 0 {
            return None;
        }
        let num = self.0 as u128 * const_pow10_u64(S) as u128;
        let result = div_round_u128::<MODE>(num, rhs.0 as u128);
        Some(Self(result.try_into().ok()?))
    }

    /// Lossless rescale. Returns `None` if fractional digits would be lost or on overflow.
    pub fn rescale_into<const OUT: u32>(self) -> Option<UDecimal64<OUT>> {
        if OUT > S {
            let factor = const_pow10_u64(OUT - S);
            self.0.checked_mul(factor).map(UDecimal64::from_raw)
        } else if OUT < S {
            let factor = const_pow10_u64(S - OUT) as u128;
            let val = self.0 as u128;
            if !val.is_multiple_of(factor) {
                None
            } else {
                Some(UDecimal64::from_raw((val / factor) as u64))
            }
        } else {
            Some(UDecimal64::from_raw(self.0))
        }
    }

    /// Rescale using nearest-even (banker's) rounding. Returns `None` only on overflow.
    #[inline]
    pub fn rescale_round_into_nearest_even<const OUT: u32>(self) -> Option<UDecimal64<OUT>> {
        self.rescale_round_into_impl::<OUT, { crate::RoundFlag::NEAREST_EVEN }>()
    }

    /// Rescale using nearest, ties away from zero. Returns `None` only on overflow.
    #[inline]
    pub fn rescale_round_into_nearest<const OUT: u32>(self) -> Option<UDecimal64<OUT>> {
        self.rescale_round_into_impl::<OUT, { crate::RoundFlag::NEAREST }>()
    }

    /// Rescale by truncating toward zero. Returns `None` only on overflow.
    #[inline]
    pub fn rescale_round_into_zero<const OUT: u32>(self) -> Option<UDecimal64<OUT>> {
        self.rescale_round_into_impl::<OUT, { crate::RoundFlag::ZERO }>()
    }

    /// Rescale by rounding toward positive infinity. Returns `None` only on overflow.
    #[inline]
    pub fn rescale_round_into_ceil<const OUT: u32>(self) -> Option<UDecimal64<OUT>> {
        self.rescale_round_into_impl::<OUT, { crate::RoundFlag::CEIL }>()
    }

    /// Rescale by rounding toward negative infinity. Returns `None` only on overflow.
    #[inline]
    pub fn rescale_round_into_floor<const OUT: u32>(self) -> Option<UDecimal64<OUT>> {
        self.rescale_round_into_impl::<OUT, { crate::RoundFlag::FLOOR }>()
    }

    #[inline]
    fn rescale_round_into_impl<const OUT: u32, const MODE: crate::RoundFlagEnum>(
        self,
    ) -> Option<UDecimal64<OUT>> {
        if OUT > S {
            let factor = const_pow10_u64(OUT - S);
            self.0.checked_mul(factor).map(UDecimal64::from_raw)
        } else if OUT < S {
            let factor = const_pow10_u64(S - OUT) as u128;
            let result = div_round_u128::<MODE>(self.0 as u128, factor);
            if result <= u64::MAX as u128 {
                Some(UDecimal64::from_raw(result as u64))
            } else {
                None
            }
        } else {
            Some(UDecimal64::from_raw(self.0))
        }
    }
}

/// Rounding integer division for non-negative u128 values. `den` must be non-zero.
fn div_round_u128<const MODE: crate::RoundFlagEnum>(num: u128, den: u128) -> u128 {
    debug_assert!(den != 0);
    let q = num / den;
    let r = num % den;
    if r == 0 {
        return q;
    }
    match crate::RoundFlag::from_u8(MODE) {
        crate::RoundFlag::Zero => q,
        crate::RoundFlag::Ceil => q + 1,
        crate::RoundFlag::Floor => q,
        crate::RoundFlag::Nearest => {
            if r * 2 >= den {
                q + 1
            } else {
                q
            }
        }
        crate::RoundFlag::NearestEven => {
            let r2 = r * 2;
            if r2 > den {
                q + 1
            } else if r2 == den {
                if !q.is_multiple_of(2) { q + 1 } else { q }
            } else {
                q
            }
        }
    }
}

// ── Display / Debug ──────────────────────────────────────────────────────────

impl<const S: u32> fmt::Display for UDecimal64<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if S == 0 {
            return write!(f, "{}", self.0);
        }
        let divisor = const_pow10_u64(S);
        let integer = self.0 / divisor;
        let frac = self.0 % divisor;
        write!(f, "{}.{:0>width$}", integer, frac, width = S as usize)
    }
}

impl<const S: u32> fmt::Debug for UDecimal64<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "UDecimal64<{}>({})", S, self.0)
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Decimal64, ParseError};
    #[cfg(all(not(feature = "std"), feature = "alloc"))]
    use alloc::format;
    #[cfg(all(not(feature = "std"), feature = "alloc"))]
    use alloc::string::ToString;

    // ── Constants ────────────────────────────────────────────────────────────

    #[test]
    fn one_raw_equals_pow10() {
        assert_eq!(UDecimal64::<4>::ONE.raw(), 10_000);
    }

    #[test]
    fn max_raw_is_u64_max() {
        assert_eq!(UDecimal64::<4>::MAX.raw(), u64::MAX);
    }

    #[test]
    fn zero_raw_is_zero() {
        assert_eq!(UDecimal64::<4>::ZERO.raw(), 0);
    }

    // ── Parse ────────────────────────────────────────────────────────────────

    #[test]
    fn parse_zero() {
        let d: UDecimal64<4> = "0".parse().unwrap();
        assert_eq!(d, UDecimal64::ZERO);
    }

    #[test]
    fn parse_basic_fractional() {
        let d: UDecimal64<4> = "1.2345".parse().unwrap();
        assert_eq!(d.raw(), 12345);
    }

    #[test]
    fn parse_truncation() {
        let d: UDecimal64<4> = "1.23456".parse().unwrap();
        assert_eq!(d.raw(), 12345);
    }

    #[test]
    fn parse_plus_sign_rejected() {
        let r: Result<UDecimal64<2>, _> = "+1.00".parse();
        assert_eq!(r, Err(ParseError::InvalidChar { byte: b'+', pos: 0 }));
    }

    #[test]
    fn parse_minus_sign_rejected() {
        let r: Result<UDecimal64<2>, _> = "-1.00".parse();
        assert_eq!(r, Err(ParseError::InvalidChar { byte: b'-', pos: 0 }));
    }

    #[test]
    fn parse_minus_zero_rejected() {
        let r: Result<UDecimal64<2>, _> = "-0".parse();
        assert_eq!(r, Err(ParseError::InvalidChar { byte: b'-', pos: 0 }));
    }

    #[test]
    fn parse_empty() {
        let r: Result<UDecimal64<2>, _> = "".parse();
        assert_eq!(r, Err(ParseError::Empty));
    }

    #[test]
    fn parse_overflow() {
        let r: Result<UDecimal64<2>, _> = "99999999999999999999".parse();
        assert_eq!(r, Err(ParseError::Overflow));
    }

    #[test]
    fn parse_dot_only_is_empty() {
        let r: Result<UDecimal64<2>, _> = ".".parse();
        assert_eq!(r, Err(ParseError::Empty));
    }

    #[test]
    fn parse_leading_dot() {
        let d: UDecimal64<4> = ".5".parse().unwrap();
        assert_eq!(d.raw(), 5000);
    }

    #[test]
    fn parse_trailing_dot() {
        let d: UDecimal64<4> = "5.".parse().unwrap();
        assert_eq!(d.raw(), 50000);
    }

    #[test]
    fn parse_scientific_notation_rejected() {
        let r: Result<UDecimal64<2>, _> = "1e5".parse();
        assert!(matches!(r, Err(ParseError::InvalidChar { .. })));
    }

    #[cfg(any(feature = "std", feature = "alloc"))]
    #[test]
    fn parse_round_trip() {
        let mut seed: u64 = 0xdeadbeef_cafebabe;
        let mut count = 0;
        while count < 10_000 {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let d = UDecimal64::<4>::from_raw(seed);
            let s = d.to_string();
            let parsed: UDecimal64<4> = s.parse().unwrap_or_else(|e| {
                panic!("round-trip parse failed: raw={seed}, s={s:?}, err={e}")
            });
            assert_eq!(parsed, d, "round-trip mismatch: raw={seed}, s={s:?}");
            count += 1;
        }
    }

    // ── Display ──────────────────────────────────────────────────────────────

    #[cfg(any(feature = "std", feature = "alloc"))]
    #[test]
    fn display_basic() {
        assert_eq!(UDecimal64::<2>(123).to_string(), "1.23");
    }

    #[cfg(any(feature = "std", feature = "alloc"))]
    #[test]
    fn display_zero_scale() {
        assert_eq!(UDecimal64::<0>(42).to_string(), "42");
    }

    #[cfg(any(feature = "std", feature = "alloc"))]
    #[test]
    fn display_zero() {
        assert_eq!(UDecimal64::<2>(0).to_string(), "0.00");
    }

    #[cfg(any(feature = "std", feature = "alloc"))]
    #[test]
    fn display_fractional_padding() {
        assert_eq!(UDecimal64::<4>(1234567).to_string(), "123.4567");
    }

    #[cfg(any(feature = "std", feature = "alloc"))]
    #[test]
    fn debug_format() {
        assert_eq!(
            format!("{:?}", UDecimal64::<4>(12345)),
            "UDecimal64<4>(12345)"
        );
    }

    // ── f64 conversions ──────────────────────────────────────────────────────

    #[cfg(feature = "std")]
    #[test]
    fn from_f64_nan_is_zero() {
        assert_eq!(UDecimal64::<2>::from_f64(f64::NAN).raw(), 0);
    }

    #[cfg(feature = "std")]
    #[test]
    fn from_f64_negative_is_zero() {
        assert_eq!(UDecimal64::<2>::from_f64(-1.5).raw(), 0);
    }

    #[cfg(feature = "std")]
    #[test]
    fn from_f64_negative_zero_is_zero() {
        assert_eq!(UDecimal64::<2>::from_f64(-0.0).raw(), 0);
    }

    #[cfg(feature = "std")]
    #[test]
    fn from_f64_infinity_clamps_to_max() {
        assert_eq!(UDecimal64::<2>::from_f64(f64::INFINITY).raw(), u64::MAX);
    }

    #[cfg(feature = "std")]
    #[test]
    fn from_f64_basic() {
        assert_eq!(UDecimal64::<4>::from_f64(1.2345).raw(), 12345);
    }

    #[test]
    fn to_f64_basic() {
        assert_eq!(UDecimal64::<4>(12345).to_f64(), 1.2345_f64);
    }

    #[cfg(feature = "std")]
    #[test]
    fn f64_round_trip() {
        let mut seed: u64 = 12345678901234567;
        for _ in 0..1000 {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            // Use 40-bit range: well within f64's 53-bit mantissa at scale 4
            let raw = seed >> 24;
            let d = UDecimal64::<4>::from_raw(raw);
            let rt = UDecimal64::<4>::from_f64(d.to_f64());
            assert!(
                rt.raw().abs_diff(d.raw()) <= 1,
                "f64 round-trip failed: raw={raw}, rt={}",
                rt.raw()
            );
        }
    }

    // ── Interop ──────────────────────────────────────────────────────────────

    #[test]
    fn as_signed_small_succeeds() {
        let u = UDecimal64::<4>(10_000);
        assert_eq!(u.as_signed(), Some(Decimal64::<4>::from_raw(10_000)));
    }

    #[test]
    fn as_signed_i64_max_succeeds() {
        let u = UDecimal64::<4>(i64::MAX as u64);
        assert_eq!(u.as_signed(), Some(Decimal64::<4>::from_raw(i64::MAX)));
    }

    #[test]
    fn as_signed_above_i64_max_returns_none() {
        let u = UDecimal64::<4>(i64::MAX as u64 + 1);
        assert_eq!(u.as_signed(), None);
    }

    #[test]
    fn as_unsigned_nonneg_succeeds() {
        let d = Decimal64::<4>::from_raw(12345);
        assert_eq!(d.as_unsigned(), Some(UDecimal64::<4>(12345)));
    }

    #[test]
    fn as_unsigned_negative_returns_none() {
        let d = Decimal64::<4>::from_raw(-1);
        assert_eq!(d.as_unsigned(), None);
    }

    #[test]
    fn as_unsigned_zero_succeeds() {
        let d = Decimal64::<4>::from_raw(0);
        assert_eq!(d.as_unsigned(), Some(UDecimal64::<4>::ZERO));
    }

    // ── Addition ─────────────────────────────────────────────────────────────

    #[test]
    fn add_basic() {
        assert_eq!(
            UDecimal64::<2>(100) + UDecimal64::<2>(50),
            UDecimal64::<2>(150)
        );
    }

    #[test]
    #[should_panic(expected = "UDecimal64 addition overflow")]
    fn add_overflow_panics() {
        let _ = UDecimal64::<2>::MAX + UDecimal64::<2>(1);
    }

    #[test]
    fn checked_add_overflow_returns_none() {
        assert_eq!(UDecimal64::<2>::MAX.checked_add(UDecimal64::<2>(1)), None);
    }

    #[test]
    fn saturating_add_clamps_to_max() {
        assert_eq!(
            UDecimal64::<2>::MAX.saturating_add(UDecimal64::<2>(1)),
            UDecimal64::<2>::MAX
        );
    }

    // ── Subtraction ──────────────────────────────────────────────────────────

    #[test]
    fn sub_exact() {
        assert_eq!(
            UDecimal64::<2>(150) - UDecimal64::<2>(50),
            Some(UDecimal64::<2>(100))
        );
    }

    #[test]
    fn sub_underflow_returns_none() {
        assert_eq!(UDecimal64::<2>(50) - UDecimal64::<2>(100), None);
    }

    #[test]
    fn saturating_sub_underflow_clamps_to_zero() {
        assert_eq!(
            UDecimal64::<2>::ZERO.saturating_sub(UDecimal64::<2>::ONE),
            UDecimal64::<2>::ZERO
        );
    }

    #[test]
    fn checked_sub_underflow_returns_none() {
        assert_eq!(UDecimal64::<4>(5).checked_sub(UDecimal64::<4>(10)), None);
    }

    // ── Multiplication ───────────────────────────────────────────────────────

    #[test]
    fn mul_same_scale() {
        // 1.0000 × 2.0000 = 2.0000  (raw: 10_000 * 20_000 / 10_000 = 20_000)
        assert_eq!(
            UDecimal64::<4>(10_000) * UDecimal64::<4>(20_000),
            UDecimal64::<4>(20_000)
        );
    }

    #[test]
    fn checked_mul_overflow_returns_none() {
        assert_eq!(
            UDecimal64::<4>::MAX.checked_mul(UDecimal64::<4>(20_000)),
            None
        );
    }

    #[test]
    fn saturating_mul_clamps_to_max() {
        assert_eq!(
            UDecimal64::<4>::MAX.saturating_mul(UDecimal64::<4>(20_000)),
            UDecimal64::<4>::MAX
        );
    }

    // ── Division ─────────────────────────────────────────────────────────────

    #[test]
    fn div_same_scale() {
        // 3.0000 / 2.0000 = 1.5000  (raw: 30_000 * 10_000 / 20_000 = 15_000)
        assert_eq!(
            UDecimal64::<4>(30_000) / UDecimal64::<4>(20_000),
            UDecimal64::<4>(15_000)
        );
    }

    #[test]
    fn div_truncates_toward_zero() {
        // 1.00 / 3.00 = 0.33…  raw: (100 * 100) / 300 = 33
        assert_eq!(
            UDecimal64::<2>(100) / UDecimal64::<2>(300),
            UDecimal64::<2>(33)
        );
    }

    #[test]
    fn div_truncation_scale2() {
        // 0.10 / 0.03: raw (10 * 100) / 3 = 333
        assert_eq!(
            UDecimal64::<2>(10) / UDecimal64::<2>(3),
            UDecimal64::<2>(333)
        );
    }

    #[test]
    fn checked_div_by_zero_returns_none() {
        assert_eq!(UDecimal64::<2>(100).checked_div(UDecimal64::<2>(0)), None);
    }

    #[test]
    fn div_round_toward_pos_inf() {
        // 1.00 / 3.00: 33.33… → ceil = 34
        let result = UDecimal64::<2>(100).div_round_ceil(UDecimal64::<2>(300));
        assert_eq!(result, UDecimal64::<2>(34));
    }

    #[test]
    fn div_round_toward_neg_inf() {
        // 1.00 / 3.00: 33.33… → floor = 33 (same as trunc for positives)
        let result = UDecimal64::<2>(100).div_round_floor(UDecimal64::<2>(300));
        assert_eq!(result, UDecimal64::<2>(33));
    }

    #[test]
    fn div_round_nearest() {
        // 1.00 / 3.00 at scale 2: 33.33… → Nearest = 33
        let result = UDecimal64::<2>(100).div_round_nearest(UDecimal64::<2>(300));
        assert_eq!(result, UDecimal64::<2>(33));
    }

    #[test]
    fn div_round_nearest_half_up() {
        // 1.00 / 2.00: 50 exactly (no rounding needed)
        let result = UDecimal64::<2>(100).div_round_nearest(UDecimal64::<2>(200));
        assert_eq!(result, UDecimal64::<2>(50));
    }

    #[test]
    fn div_round_nearest_even_tie() {
        // 3 / 2 at scale 2: (300 * 100) / 200 = 150 exactly
        let result = UDecimal64::<2>(300).div_round_nearest_even(UDecimal64::<2>(200));
        assert_eq!(result, UDecimal64::<2>(150));
    }

    #[test]
    fn checked_div_round_by_zero_returns_none() {
        assert_eq!(
            UDecimal64::<2>(100).checked_div_round_nearest(UDecimal64::<2>(0)),
            None
        );
    }

    // ── Rescaling ────────────────────────────────────────────────────────────

    #[test]
    fn rescale_into_upscale() {
        // 1.23 at scale 2 → scale 6: raw 1_230_000
        let result: Option<UDecimal64<6>> = UDecimal64::<2>(123).rescale_into();
        assert_eq!(result, Some(UDecimal64::<6>(1_230_000)));
    }

    #[test]
    fn rescale_into_downscale_exact() {
        // 1.20 (raw 120 at scale 2) → scale 1: raw 12
        let result: Option<UDecimal64<1>> = UDecimal64::<2>(120).rescale_into();
        assert_eq!(result, Some(UDecimal64::<1>(12)));
    }

    #[test]
    fn rescale_into_downscale_lossy_returns_none() {
        // 1.23 cannot be represented exactly at scale 1
        let result: Option<UDecimal64<1>> = UDecimal64::<2>(123).rescale_into();
        assert_eq!(result, None);
    }

    #[test]
    fn rescale_same_scale_is_identity() {
        let d = UDecimal64::<4>(12345);
        let result: Option<UDecimal64<4>> = d.rescale_into();
        assert_eq!(result, Some(d));
    }

    #[test]
    fn rescale_round_into_downscale_rounds_up() {
        // 1.25 at scale 2 → scale 1 with Nearest: 1.25 rounds to 1.3 (raw 13)
        let result: Option<UDecimal64<1>> = UDecimal64::<2>(125).rescale_round_into_nearest::<1>();
        assert_eq!(result, Some(UDecimal64::<1>(13)));
    }

    #[test]
    fn rescale_round_into_downscale_truncates() {
        // 1.23 at scale 2 → scale 1 with TruncateTowardZero: raw 12
        let result: Option<UDecimal64<1>> = UDecimal64::<2>(123).rescale_round_into_zero::<1>();
        assert_eq!(result, Some(UDecimal64::<1>(12)));
    }

    // ── Ordering ─────────────────────────────────────────────────────────────

    #[test]
    fn ordering_is_numeric() {
        assert!(UDecimal64::<4>(100) < UDecimal64::<4>(200));
        assert!(UDecimal64::<4>(0) < UDecimal64::<4>(1));
        assert_eq!(UDecimal64::<4>(50), UDecimal64::<4>(50));
    }
}
