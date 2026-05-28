//! Fixed-scale 64-bit signed decimal for Rust.
//!
//! Provides [`Decimal64<S>`], a `repr(transparent)` wrapper around `i64` where
//! `S` is a compile-time const scale: the raw integer value represents the
//! mathematical value multiplied by `10^S`.
//!
//! # Quick start
//!
//! ```rust
//! use decimal64::Decimal64;
//!
//! let price: Decimal64<4> = "123.4567".parse().unwrap();
//! let qty: Decimal64<4> = "10.0000".parse().unwrap();
//! let total = price * qty;
//! assert_eq!(total.to_string(), "1234.5670");
//! ```
//!
//! See `docs/design.md` for the full design rationale, API surface, and
//! arithmetic rules.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]

#[cfg(feature = "alloc")]
extern crate alloc;

pub(crate) mod parse;
pub(crate) mod parse_unsigned;
pub mod decimal64;
pub mod udecimal64;
pub mod scientific;
pub use decimal64::Decimal64;
pub use udecimal64::UDecimal64;
pub use scientific::Scientific;

#[cfg(feature = "serde")]
pub mod serde_impls;
#[cfg(feature = "serde")]
pub mod serde_as;

/// Rounding mode for `from_f64_round`, `div_round`, and `rescale_round_into`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Round {
    /// IEEE 754 default: round to nearest, ties go to the even digit (banker's rounding).
    NearestEven,
    /// Round to nearest, ties go away from zero.
    Nearest,
    /// Truncate toward zero (same as integer division).
    TruncateTowardZero,
    /// Round toward positive infinity (ceiling).
    TowardPosInf,
    /// Round toward negative infinity (floor).
    TowardNegInf,
}

/// Errors returned by [`Decimal64::parse`], [`Scientific`], and the
/// [`FromStr`](std::str::FromStr) impls.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// Input was empty, or contained only a sign or a bare dot.
    Empty,
    /// A byte that is not a digit, sign, or dot was encountered.
    InvalidChar { byte: u8, pos: usize },
    /// The accumulated value exceeded `i64` range for this scale.
    Overflow,
    /// Nonzero value combined with a negative exponent so extreme that no
    /// representable non-zero value exists (returned by [`Scientific`] parsing).
    Underflow,
    /// Reserved for a future strict-mode parse; currently unused (extra digits are truncated).
    TooManyFractional { got: u32, max: u32 },
}

impl core::fmt::Display for ParseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ParseError::Empty => write!(f, "empty or missing digits"),
            ParseError::InvalidChar { byte, pos } => {
                write!(f, "invalid character {:?} at position {}", *byte as char, pos)
            }
            ParseError::Overflow => write!(f, "numeric overflow"),
            ParseError::Underflow => {
                write!(f, "value too small to represent (underflow)")
            }
            ParseError::TooManyFractional { got, max } => {
                write!(f, "too many fractional digits: got {}, max {}", got, max)
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ParseError {}
