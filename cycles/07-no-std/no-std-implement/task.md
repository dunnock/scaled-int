# Task: no-std-implement

**Project:** decimal64  
**Cycle:** 07-no-std  
**Status:** active  
**Depends on:** `no-std-design-and-plan`

---

## Objective

Apply the no_std migration described in `docs/no-std-design.md`. All changes are
mechanical; no logic modifications. The implementation task produces working
code only — no documentation updates.

## Spec

Read `docs/no-std-design.md` §8 (Per-File Implementation Plan) in full before
starting. Apply exactly the changes listed there, in order.

### `Cargo.toml`

1. Replace the `[features]` section:
   ```toml
   [features]
   default = ["std"]
   std    = ["serde?/std"]
   alloc  = ["serde?/alloc"]
   serde  = ["dep:serde", "alloc"]
   ```
2. Change the serde dependency to disable default features:
   ```toml
   serde = { version = "1.0", optional = true, default-features = false }
   ```

### `src/lib.rs`

1. Insert `#![cfg_attr(not(feature = "std"), no_std)]` before `#![deny(unsafe_code)]`.
2. Insert `#[cfg(feature = "alloc")] extern crate alloc;` after the attribute block.
3. Change `impl std::fmt::Display for ParseError` to use `core::fmt`.
4. Change the `fmt` method signature to use `core::fmt::Formatter` and `core::fmt::Result`.
5. Gate `impl std::error::Error for ParseError {}` with `#[cfg(feature = "std")]`.

### `src/decimal64.rs`

Change the three opening `use` lines:
```rust
use core::fmt;
use core::ops::{Add, Div, Mul, Neg, Sub};
use core::str::FromStr;
```

### `src/udecimal64.rs`

Change the three opening `use` lines:
```rust
use core::fmt;
use core::ops::{Add, Div, Mul, Sub};
use core::str::FromStr;
```

### `src/scientific.rs`

1. Change the two opening `use` lines:
   ```rust
   use core::fmt;
   use core::str::FromStr;
   ```
2. Gate `fn fmt_scientific_u64` with `#[cfg(any(feature = "std", feature = "alloc"))]`.
3. Gate `impl fmt::Display for Scientific<Decimal64<S>>` with the same attribute.
4. Gate `impl fmt::Display for Scientific<UDecimal64<S>>` with the same attribute.

### `src/serde_impls.rs`

Replace all four occurrences of `std::fmt::Formatter` with `core::fmt::Formatter`.

## Verification

Run these commands in order; all must succeed:

```bash
CARGO_TARGET_DIR=/work/cargo-target-ralph cargo build
CARGO_TARGET_DIR=/work/cargo-target-ralph cargo build --no-default-features
CARGO_TARGET_DIR=/work/cargo-target-ralph cargo build --no-default-features --features alloc
CARGO_TARGET_DIR=/work/cargo-target-ralph cargo build --no-default-features --features alloc,serde
CARGO_TARGET_DIR=/work/cargo-target-ralph cargo test
CARGO_TARGET_DIR=/work/cargo-target-ralph cargo test --no-default-features --features alloc
```

If any command fails, fix the root cause before proceeding.

## Out of scope

- No benchmark changes.
- No documentation changes.
- No new public APIs.
- Do not add or remove tests.
