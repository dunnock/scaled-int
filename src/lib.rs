//! Fixed-scale 64-bit signed decimal for Rust.
//!
//! Provides [`Decimal64<S>`], a `repr(transparent)` wrapper around `i64` where
//! `S` is a compile-time const scale: the raw integer value represents the
//! mathematical value multiplied by `10^S`.
//!
//! See `docs/design.md` for the full design rationale, API surface, and
//! arithmetic rules.

#![deny(unsafe_code)]

pub(crate) mod parse;
pub mod decimal64;
pub use decimal64::Decimal64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Round {
    NearestEven,
    Nearest,
    TruncateTowardZero,
    TowardPosInf,
    TowardNegInf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    Empty,
    InvalidChar { byte: u8, pos: usize },
    Overflow,
    TooManyFractional { got: u32, max: u32 },
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::Empty => write!(f, "empty or missing digits"),
            ParseError::InvalidChar { byte, pos } => {
                write!(f, "invalid character {:?} at position {}", *byte as char, pos)
            }
            ParseError::Overflow => write!(f, "value overflows i64"),
            ParseError::TooManyFractional { got, max } => {
                write!(f, "too many fractional digits: got {}, max {}", got, max)
            }
        }
    }
}

impl std::error::Error for ParseError {}
