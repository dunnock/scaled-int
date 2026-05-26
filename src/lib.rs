//! Fixed-scale 64-bit signed decimal for Rust.
//!
//! Provides [`Decimal64<S>`], a `repr(transparent)` wrapper around `i64` where
//! `S` is a compile-time const scale: the raw integer value represents the
//! mathematical value multiplied by `10^S`.
//!
//! See `docs/design.md` for the full design rationale, API surface, and
//! arithmetic rules.

#![deny(unsafe_code)]

pub mod decimal64;
