# Cycle 07 — no-std Design

## 1  Objective

Add `#![no_std]` compatibility to the `decimal64` crate without breaking existing users.
The crate currently assumes `std` implicitly; the goal is to make `std` an explicit,
default-enabled cargo feature so that embedded and constrained targets can opt out.

After this cycle:

```
cargo build                                          # unchanged (std default)
cargo build --no-default-features                   # bare no_std, no allocator
cargo build --no-default-features --features alloc  # no_std + heap
cargo build --features serde                        # std + serde (unchanged)
cargo build --no-default-features --features alloc,serde  # no_std + alloc + serde
```

---

## 2  Feature Flag Design

### 2.1  New feature matrix

```toml
[features]
default = ["std"]
std    = ["serde?/std"]
alloc  = ["serde?/alloc"]
serde  = ["dep:serde", "alloc"]
```

| Feature | What it unlocks |
|---------|----------------|
| `std` (default) | `std::error::Error` impl for `ParseError`; propagates `std` to serde |
| `alloc` | `Scientific<D>::Display`; `alloc::ToString` via `extern crate alloc`; propagates `alloc` to serde |
| `serde` | `Serialize`/`Deserialize` for all three types; **implies `alloc`** |

**Why `serde` implies `alloc`:** All four `Serialize` impls call `self.to_string()`, which
requires heap allocation. Requiring `alloc` here is the minimal honest contract — binary
serde users (postcard, bincode) are already in an alloc environment. A future cycle could
add a zero-copy `serialize_raw` API that bypasses alloc, but that is out of scope here.

**Why `std` does NOT imply `alloc`:** In `std` builds, `alloc` is always transitively
present — the standard library includes the allocator. Making `std` depend on `alloc` would
be redundant and confusing; instead, code gated on `any(feature="std", feature="alloc")`
covers both cases explicitly.

### 2.2  Serde dependency change

The serde dependency must drop `default-features` to avoid forcing `std` on all consumers:

```toml
serde = { version = "1.0", optional = true, default-features = false }
```

Serde's own `std` / `alloc` sub-features are propagated via the feature forwarding lines
above (`serde?/std`, `serde?/alloc`).

### 2.3  Crate root preamble

```rust
#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_code)]

#[cfg(feature = "alloc")]
extern crate alloc;
```

The `cfg_attr` attribute is conditional so that `std` builds continue to compile normally
without an explicit `no_std` declaration.

---

## 3  Complete `std::` Dependency Audit

### 3.1  Qualified `std::` references (`src/lib.rs`)

| Location | Current | Action |
|----------|---------|--------|
| `lib.rs:69` | `impl std::fmt::Display for ParseError` | Replace with `core::fmt::Display` |
| `lib.rs:70` | `fn fmt(…, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result` | `core::fmt::Formatter` / `core::fmt::Result` |
| `lib.rs:87` | `impl std::error::Error for ParseError {}` | Gate with `#[cfg(feature = "std")]` |

The `Error` trait lives in `std::error` only; there is no `core::error::Error` in stable
Rust as of the MSRV targeted here. Gating it behind `std` preserves full ergonomics for
`std` users while making no_std builds succeed.

### 3.2  `use std::` imports

| File | Import | Replacement |
|------|--------|-------------|
| `decimal64.rs:1` | `use std::fmt;` | `use core::fmt;` |
| `decimal64.rs:2` | `use std::ops::{Add, Div, Mul, Neg, Sub};` | `use core::ops::{…};` |
| `decimal64.rs:3` | `use std::str::FromStr;` | `use core::str::FromStr;` |
| `udecimal64.rs:1` | `use std::fmt;` | `use core::fmt;` |
| `udecimal64.rs:2` | `use std::ops::{Add, Div, Mul, Sub};` | `use core::ops::{…};` |
| `udecimal64.rs:3` | `use std::str::FromStr;` | `use core::str::FromStr;` |
| `scientific.rs:1` | `use std::fmt;` | `use core::fmt;` |
| `scientific.rs:2` | `use std::str::FromStr;` | `use core::str::FromStr;` |

All of `core::fmt`, `core::ops::*`, and `core::str::FromStr` have been stable in `core`
since Rust 1.0. No compatibility risk.

### 3.3  Qualified `std::fmt::Formatter` in `serde_impls.rs`

Four visitor `expecting` methods use `std::fmt::Formatter`. Replace each with
`core::fmt::Formatter`. This is a two-character change per site.

---

## 4  `alloc` Dependency Audit

### 4.1  `to_string()` calls

| Location | Context | Required feature |
|----------|---------|-----------------|
| `scientific.rs:228` | `abs_raw.to_string()` inside `fmt_scientific_u64` | `alloc` or `std` |
| `serde_impls.rs:9` | `self.to_string()` in `Decimal64::serialize` | `alloc` (implied by `serde`) |
| `serde_impls.rs:37` | `self.to_string()` in `UDecimal64::serialize` | `alloc` (implied by `serde`) |
| `serde_impls.rs:65` | `self.to_string()` in `Scientific<Decimal64<S>>::serialize` | `alloc` (implied by `serde`) |
| `serde_impls.rs:92` | `self.to_string()` in `Scientific<UDecimal64<S>>::serialize` | `alloc` (implied by `serde`) |

### 4.2  `Scientific<D>` Display — alloc gate

`fmt_scientific_u64` converts the raw integer to a heap-allocated `String` via
`u64::to_string()` before doing digit surgery. This is the only reason `Scientific`'s
`Display` impl needs `alloc`.

**Gate the three affected items** with `#[cfg(any(feature = "std", feature = "alloc"))]`:
- `fn fmt_scientific_u64` (private helper)
- `impl fmt::Display for Scientific<Decimal64<S>>`
- `impl fmt::Display for Scientific<UDecimal64<S>>`

In a pure no_std environment without alloc, `Scientific` remains usable for parsing and
arithmetic; only `Display` (and hence `to_string()` and serde serialization) is absent.
This is an acceptable trade-off — the type is still fully functional for internal
computation.

---

## 5  Floating-Point Methods in no_std

`Decimal64::from_f64_round` uses these `f64` methods:

| Method | Core availability |
|--------|-----------------|
| `f64::is_nan()` | `core` since 1.0 |
| `f64::powi(n: i32)` | `core` since 1.0 (compiler intrinsic) |
| `f64::round_ties_even()` | `core` since 1.77.0 |
| `f64::round()` | `core` since 1.0 |
| `f64::trunc()` | `core` since 1.0 |
| `f64::ceil()` | `core` since 1.0 |
| `f64::floor()` | `core` since 1.0 |
| `f64::clamp(min, max)` | `core` since 1.50.0 |

All eight methods are available unconditionally in `core` via LLVM intrinsics. No `libm`
dependency is required on supported targets (x86_64, AArch64, RISC-V, ARM with FPU).

**Caveat for bare-metal targets:** On Cortex-M0 (soft-float only), `powi`, `round`, etc.
require a software-float library (e.g., `compiler_rt`). This is a linker-level concern
for the final binary, not a compilation concern for this crate. The crate makes no
promises beyond successful `cargo build` on the host triple.

---

## 6  Serde Feature Compatibility

Serde supports no_std natively since version 1.0.57 (released 2017). With
`default-features = false`, serde compiles without `std`. Its `alloc` feature gate enables
`alloc::string::String` in error messages and deserializers.

The `serde_impls.rs` changes are minimal:
1. Replace four `std::fmt::Formatter` references with `core::fmt::Formatter`.
2. No structural changes to `Serialize`/`Deserialize` logic.
3. The `serde_as.rs` file (raw integer adapters) uses no `std`-specific items; no changes needed.

Tests in `serde_impls.rs` use `serde_json` and `postcard`, which are dev-dependencies that
require `std`. These tests continue to run under `cargo test --features serde` as before.
There is no requirement to run serde tests under no_std.

---

## 7  Compatibility Matrix

| Build command | `ParseError::source()` | `Scientific::Display` | Serde |
|---------------|------------------------|----------------------|-------|
| `cargo build` (default `std`) | ✓ | ✓ | optional feature |
| `--no-default-features --features alloc` | ✗ | ✓ | optional feature |
| `--no-default-features` | ✗ | ✗ | ✗ |
| `--no-default-features --features alloc,serde` | ✗ | ✓ | ✓ |
| `--features serde` (default `std`) | ✓ | ✓ | ✓ |

Legend: ✓ = available, ✗ = not compiled in (not an error, just absent).

---

## 8  Per-File Implementation Plan

### 8.1  `Cargo.toml`

```toml
[features]
default = ["std"]
std    = ["serde?/std"]
alloc  = ["serde?/alloc"]
serde  = ["dep:serde", "alloc"]

[dependencies]
serde = { version = "1.0", optional = true, default-features = false }
```

Remove `features = ["derive"]` from the serde dep — we use manual impls, not derive.
(Confirm the current Cargo.toml does not use derive; it does not.)

### 8.2  `src/lib.rs`

1. Add at the top (before `#![deny(unsafe_code)]`):
   ```rust
   #![cfg_attr(not(feature = "std"), no_std)]
   ```
2. After the attribute block:
   ```rust
   #[cfg(feature = "alloc")]
   extern crate alloc;
   ```
3. Change `impl std::fmt::Display for ParseError` and interior `std::fmt::Formatter`
   references to use `core::fmt`.
4. Gate `impl std::error::Error for ParseError {}` with `#[cfg(feature = "std")]`.

### 8.3  `src/decimal64.rs`

Change the three use lines at the top:
```rust
use core::fmt;
use core::ops::{Add, Div, Mul, Neg, Sub};
use core::str::FromStr;
```
No other changes needed; all arithmetic uses `i64`/`i128` primitives which are in `core`.

### 8.4  `src/udecimal64.rs`

Change the three use lines at the top:
```rust
use core::fmt;
use core::ops::{Add, Div, Mul, Sub};
use core::str::FromStr;
```

### 8.5  `src/scientific.rs`

1. Change use lines:
   ```rust
   use core::fmt;
   use core::str::FromStr;
   ```
2. Gate the Display infrastructure on `alloc` or `std`:
   ```rust
   #[cfg(any(feature = "std", feature = "alloc"))]
   fn fmt_scientific_u64(…) { … }

   #[cfg(any(feature = "std", feature = "alloc"))]
   impl<const S: u32> fmt::Display for Scientific<Decimal64<S>> { … }

   #[cfg(any(feature = "std", feature = "alloc"))]
   impl<const S: u32> fmt::Display for Scientific<UDecimal64<S>> { … }
   ```
3. The `to_string()` call inside `fmt_scientific_u64` requires alloc; since the
   function itself is now gated, the call is safe.

### 8.6  `src/serde_impls.rs`

Replace four occurrences of `std::fmt::Formatter` with `core::fmt::Formatter`.
No structural changes.

### 8.7  `src/parse.rs` and `src/parse_unsigned.rs`

No changes needed. These files use no `std::` imports — they operate entirely on
byte slices and return `Result` types defined in `crate::`.

### 8.8  `src/serde_as.rs`

No changes needed. This file uses `serde::{Serializer, Deserializer}` directly and
no `std::` items.

---

## 9  Testing Strategy

### 9.1  Build verification (CI-equivalent shell commands)

```bash
# 1. Default build must be unchanged
cargo build

# 2. Pure no_std must compile (no allocator, no std)
cargo build --no-default-features

# 3. no_std + alloc must compile
cargo build --no-default-features --features alloc

# 4. no_std + alloc + serde must compile
cargo build --no-default-features --features alloc,serde

# 5. Existing test suite must pass (with std)
cargo test

# 6. no_std + alloc tests must pass
# (test harness always links std; this tests the library code under alloc feature)
cargo test --no-default-features --features alloc
```

### 9.2  Test scope under no_std + alloc

All existing unit tests are valid under `--no-default-features --features alloc`:
- `decimal64.rs` tests: use `to_string()` (alloc) + `parse()` (core) → ✓
- `parse.rs` tests: use `parse()` only → ✓
- `parse_unsigned.rs` tests: use `parse()` only → ✓
- `scientific.rs` tests: use `to_string()` (alloc, gated `Display`) + `parse()` → ✓
- `serde_impls.rs` tests: use `serde_json`/`postcard` dev-deps (std) → run only under `--features serde` with std

No new tests are required in the implement task. The existing suite, passing under
`--no-default-features --features alloc`, is sufficient proof of correctness.

### 9.3  Regression prevention for std builds

`cargo test` (default std + no serde) must continue to pass with no changes to test
results. The benchmark suite must also compile (`cargo bench --no-run`).

---

## 10  Estimated Implementation Cost

| Task | Estimated lines changed | Risk |
|------|------------------------|------|
| Cargo.toml feature table | 6 lines | Low |
| lib.rs preamble + Error gate | 5 lines | Low |
| decimal64.rs use lines | 3 lines | Low |
| udecimal64.rs use lines | 3 lines | Low |
| scientific.rs use lines + cfg gates | 8 lines | Low |
| serde_impls.rs Formatter fix | 4 lines | Low |
| **Total** | **~29 lines** | **Low** |

This is a purely mechanical transformation — no logic changes, no new abstractions,
no API breaks. The only non-trivial decision (gating `Scientific::Display` on alloc)
is documented in §4.2 above.

---

## 11  Acceptance Criteria

1. `cargo build` succeeds with no warnings (default features).
2. `cargo build --no-default-features` succeeds.
3. `cargo build --no-default-features --features alloc` succeeds.
4. `cargo test` passes with no regressions.
5. `cargo test --no-default-features --features alloc` passes.
6. `cargo bench --no-run` succeeds (benchmarks still compile with std).
7. No public API is removed; existing users see no breaking change.
