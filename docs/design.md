# decimal64 — Design Document

**Cycle:** 01-design  
**Date:** 2026-05-26  
**Status:** accepted

---

## 1. Public API Surface

### Core type

```rust
/// Fixed-scale 64-bit signed decimal.  The raw value is an `i64` whose
/// unit is `10^(-S)`.  Scale is a compile-time const; no runtime overhead.
#[repr(transparent)]
pub struct Decimal64<const S: u32>(i64);
```

### Associated constants

```rust
impl<const S: u32> Decimal64<S> {
    pub const SCALE: u32 = S;

    pub const ZERO: Self = Self(0);

    /// 1.0 at scale S.  Stored as 10^S.
    pub const ONE: Self = Self(const_pow10(S));

    pub const MAX: Self = Self(i64::MAX);
    pub const MIN: Self = Self(i64::MIN);
}
```

`const_pow10(S)` is a `const fn` returning `10i64.pow(S)`.  Panics at
compile time if `S > 18` (would overflow i64).  Practical cap is S ≤ 18.

### Raw access

```rust
impl<const S: u32> Decimal64<S> {
    #[inline(always)]
    pub const fn from_raw(raw: i64) -> Self { Self(raw) }

    #[inline(always)]
    pub const fn raw(self) -> i64 { self.0 }
}
```

`repr(transparent)` + `#[inline(always)]` ⇒ zero machine-code overhead.

### Parsing

```rust
impl<const S: u32> Decimal64<S> {
    pub fn parse(s: &str) -> Result<Self, ParseError>;
}

impl<const S: u32> FromStr for Decimal64<S> {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, ParseError> {
        Self::parse(s)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    Empty,
    InvalidChar { byte: u8, pos: usize },
    Overflow,
    TooManyFractional { got: u32, max: u32 },
}
```

### f64 conversions

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Round {
    NearestEven,   // banker's rounding — the default
    Nearest,       // round half away from zero
    TruncateTowardZero,
    TowardPosInf,
    TowardNegInf,
}

impl<const S: u32> Decimal64<S> {
    /// Default: `Round::NearestEven`.
    pub fn from_f64(x: f64) -> Self;
    pub fn from_f64_round(x: f64, mode: Round) -> Self;
    pub fn to_f64(self) -> f64;
}
```

`from_f64` returns `Decimal64::MAX`/`MIN` (clamped) for overflow rather than
panicking.  Rationale: f64 inputs come from user data or JSON; a panic there
is hostile.  The analogous constructor from `&str` returns an `Err`, but f64
has no error channel in the existing ecosystem convention — `f32::from(x as
f32)` saturates silently.  Clamping is the lesser evil.

`NaN` → `ZERO`.

### Arithmetic — trait impls (same-scale, panicking on overflow)

```rust
impl<const S: u32> Add for Decimal64<S> {
    type Output = Self;
    fn add(self, rhs: Self) -> Self { /* panics on overflow */ }
}
impl<const S: u32> Sub for Decimal64<S>  { … }  // same pattern
impl<const S: u32> Neg for Decimal64<S>  { … }
```

**Add/Sub scale rule: same-scale only.**  `Decimal64<A> + Decimal64<B>` where
`A ≠ B` is a compile error.  Justification: implicit scale coercion hides
precision loss (you silently truncate the higher-scale operand to match the
lower-scale one).  Explicit coercion via `rescale_into::<OUT>()` is provided
for callers who need it.

**Default overflow behaviour for `+`, `-`, `*`: panic.**  Matches `i64`'s
operator semantics in the standard library; checked and saturating variants
are provided for callers who need them.

```rust
impl<const S: u32> Mul for Decimal64<S> {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self { /* panics on overflow */ }
}
```

**Mul scale rule: same-scale `*` is rescaled, scale-promoting `*` is a
separate method.** See §1.1 below.

```rust
impl<const S: u32> Div for Decimal64<S> {
    type Output = Self;
    fn div(self, rhs: Self) -> Self { /* truncate toward zero; panics div-by-zero */ }
}
```

**Div default rounding: truncate toward zero.**  Simplest and fastest (integer
division).  `div_round(rhs, mode)` covers other modes.

### Arithmetic — checked / saturating variants

```rust
impl<const S: u32> Decimal64<S> {
    pub fn checked_add(self, rhs: Self) -> Option<Self>;
    pub fn checked_sub(self, rhs: Self) -> Option<Self>;
    pub fn checked_mul(self, rhs: Self) -> Option<Self>;
    pub fn checked_div(self, rhs: Self) -> Option<Self>;  // None for div-by-zero

    pub fn saturating_add(self, rhs: Self) -> Self;
    pub fn saturating_sub(self, rhs: Self) -> Self;
    pub fn saturating_mul(self, rhs: Self) -> Self;

    pub fn div_round(self, rhs: Self, mode: Round) -> Self;  // panics div-by-zero
    pub fn checked_div_round(self, rhs: Self, mode: Round) -> Option<Self>;
}
```

### Scale rescaling

```rust
impl<const S: u32> Decimal64<S> {
    /// Convert to a different scale.  Returns None on overflow or if
    /// fractional digits would be lost (use Round variant for lossy).
    pub fn rescale_into<const OUT: u32>(self) -> Option<Decimal64<OUT>>;
    pub fn rescale_round_into<const OUT: u32>(self, mode: Round) -> Option<Decimal64<OUT>>;
}
```

### Scale-promoting multiplication

```rust
impl<const A: u32> Decimal64<A> {
    /// Returns the mathematically exact product at scale A+B.
    /// Requires callers to name the output type explicitly or use type
    /// inference where the expected type is known.
    ///
    /// Gated on the `scale-promote` crate feature which enables
    /// `#![feature(generic_const_exprs)]` (nightly).
    #[cfg(feature = "scale-promote")]
    pub fn mul_promote<const B: u32>(self, rhs: Decimal64<B>) -> Decimal64<{ A + B }>;
}
```

See §1.1 for the const-generic limitation and escalation.

### Comparison and hashing

```rust
impl<const S: u32> PartialEq  for Decimal64<S> { … }  // delegates to i64 ==
impl<const S: u32> Eq         for Decimal64<S> {}
impl<const S: u32> PartialOrd for Decimal64<S> { … }  // delegates to i64 cmp
impl<const S: u32> Ord        for Decimal64<S> { … }
impl<const S: u32> Hash       for Decimal64<S> { … }  // hashes self.0
```

Cross-scale comparison is intentionally absent from the trait impls.  Use
`rescale_into` to align scales, then compare.

### Display / Debug

```rust
impl<const S: u32> fmt::Display for Decimal64<S> { … }  // "123.4567"
impl<const S: u32> fmt::Debug   for Decimal64<S> { … }  // "Decimal64<4>(1234567)"
```

`Display` writes integer part, optional `.`, then exactly `S` fractional digits
(zero-padded on the right).  No trailing-zero stripping — scale is exact.

---

### 1.1 Const-generic limitation: scale-promoting Mul

The expression `Decimal64<{ A + B }>` in a trait `Output` position requires
`generic_const_exprs`, which is a nightly-only feature as of Rust 1.87 (stable
as of this writing, May 2026).  The standard `Mul` trait cannot be implemented
for `Decimal64<A> * Decimal64<B> → Decimal64<{A+B}>` on stable Rust.

**Decision for v0.1.0:**

1. The `Mul` operator (`*`) is implemented for `Decimal64<S> * Decimal64<S>`
   only (same-scale), returning `Decimal64<S>`.  It divides the intermediate
   `i128` product by `10^S` before truncating to `i64`.  Panics on overflow.

2. `mul_promote` is a method (not the `*` operator), gated on the
   `scale-promote` crate feature, enabling nightly's `generic_const_exprs`.

3. An escalation has been filed at
   `.escalations/decimal64-const-generic-block.md` describing the issue and
   asking the operator whether to:
   - Ship v0.1.0 with stable-only (`*` = same-scale), documenting the gap
   - Require nightly for the `scale-promote` feature
   - Track stabilisation of `generic_const_exprs` and flip it in v0.2

For the project.md example (`price * qty → total`) to compile on stable, the
user would write `price.mul_promote(qty)` or wait for v0.2.

---

## 2. Internal Representation

```
stored_value = mathematical_value × 10^S
```

`repr(transparent)` wrapper around `i64`:

```
value              S=2              S=4              S=6              S=9
──────────────────────────────────────────────────────────────────────────
1.00            100             10_000           1_000_000       1_000_000_000
i64::MAX  9_223_372_036_854_775_807 →
  mathematical   ≈ 92.2 quadrillion  ≈ 922.3 trillion  ≈ 9.22 trillion  ≈ 9.22 billion
```

**Practical scales:**

| S | Unit name    | Typical use                              | Max absolute value   |
|---|--------------|------------------------------------------|----------------------|
| 2 | centis       | USD cents, stock shares (round lots)     | ~92.2 quadrillion    |
| 4 | basis points | FX rates, equity prices                  | ~922 trillion        |
| 6 | micros       | Crypto prices, nanosecond timing         | ~9.22 trillion       |
| 9 | nanos        | Gas prices (Ethereum), high-freq rates   | ~9.22 billion        |
|18 | attos        | Smallest unit Ethereum (wei)             | ~9.22 (tiny numbers) |

Note: S > 18 overflows `ONE` (`10^18 > i64::MAX`).  The constructor
`const_pow10` enforces `S ≤ 18` at compile time with a `const_assert!`.

**Why `i64` and not `u64`?**  Prices, quantities, and financial values are
signed.  `i64` avoids casting in arithmetic.  Negative zero is impossible
(`-0 == 0` for integers) which removes a comparison footgun.

**Why `repr(transparent)`?**  `from_raw`/`raw` compile to zero instructions.
The type is layout-compatible with `i64`, enabling `Vec<Decimal64<S>>` to be
cast to `&[i64]` via unsafe code in FFI / storage layers.

---

## 3. Fast Parse Algorithm

### Goal

Beat `rust_decimal::Decimal::from_str` (which allocates and handles arbitrary
precision).  Target: ≥ 2× throughput on the benchmark corpus (short financial
strings 4–12 chars).

### Input grammar (v0.1.0)

```
decimal   ::= sign? integer ('.' fractional)?
sign      ::= '+' | '-'
integer   ::= digit+
fractional::= digit+
digit     ::= '0'..'9'
```

Rejected in v0.1.0 (return `ParseError::InvalidChar`):
- Scientific notation (`1e3`, `1.5E-4`)
- Underscore separators (`1_000.00`)
- Bare dot (`.5` or `5.`)  — actually `.5` is accepted as `0.5`
  wait, no: "integer part: ≥ 0 digits" means `.5` has zero integer digits
  which is valid; `5.` has zero fractional digits which is also valid.

Actually: both `".5"` and `"5."` are accepted per the grammar (integer and
fractional parts can each be zero-length, but not both simultaneously —
`"."` alone is `ParseError::Empty`).

### Algorithm (single pass, no allocation)

```
fn parse<const S: u32>(s: &str) -> Result<Decimal64<S>, ParseError> {
    let bytes = s.as_bytes();
    if bytes.is_empty() { return Err(ParseError::Empty); }

    let mut i = 0usize;

    // 1. sign
    let negative = match bytes[0] {
        b'-' => { i += 1; true  }
        b'+' => { i += 1; false }
        _    => false,
    };

    // 2. integer digits
    let mut acc: i64 = 0;
    let start = i;
    while i < bytes.len() && bytes[i] != b'.' {
        let d = bytes[i].wrapping_sub(b'0');
        if d > 9 { return Err(ParseError::InvalidChar { byte: bytes[i], pos: i }); }
        acc = acc.checked_mul(10)
                 .and_then(|v| v.checked_add(d as i64))
                 .ok_or(ParseError::Overflow)?;
        i += 1;
    }

    // 3. fractional digits (pad/truncate to exactly S digits)
    let mut frac_digits = 0u32;
    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;  // consume '.'
        while i < bytes.len() {
            if frac_digits == S {
                // extra digits — consume and discard (truncate toward zero)
                let d = bytes[i].wrapping_sub(b'0');
                if d > 9 { return Err(ParseError::InvalidChar { byte: bytes[i], pos: i }); }
                i += 1;
                // Note: TooManyFractional is NOT an error — we truncate
            } else {
                let d = bytes[i].wrapping_sub(b'0');
                if d > 9 { return Err(ParseError::InvalidChar { byte: bytes[i], pos: i }); }
                acc = acc.checked_mul(10)
                         .and_then(|v| v.checked_add(d as i64))
                         .ok_or(ParseError::Overflow)?;
                frac_digits += 1;
                i += 1;
            }
        }
    }

    if i == start && frac_digits == 0 {
        return Err(ParseError::Empty);  // "." or "" with no digits at all
    }

    // 4. pad fractional to S digits
    let pad = S - frac_digits;
    for _ in 0..pad {
        acc = acc.checked_mul(10).ok_or(ParseError::Overflow)?;
    }

    Ok(Decimal64(if negative { -acc } else { acc }))
}
```

### Performance analysis

**Why this is fast:**

1. **No UTF-8 validation** — `as_bytes()` skips the validator; we only touch
   ASCII, and `wrapping_sub(b'0')` + `> 9` check handles all non-digit bytes.

2. **Branchless digit path** — `wrapping_sub` + `> 9` is one compare, one
   branch (for the error case, which is predicted-not-taken in normal inputs).
   The `checked_mul`/`checked_add` add two multiplications and an overflow
   flag check; modern CPUs fuse these with the branch.

3. **Single pass** — no two-pass (integer + fractional separately).  One loop
   for integer, one for fractional.

4. **No allocation** — `&str` → `i64`; no intermediate `String` or `Vec`.

5. **Constant-folded scale** — `S` is a const; the compiler sees `pad = S -
   frac_digits` as a loop from 0 to a small constant and will often unroll it.

**Overflow check cadence:** `checked_mul` and `checked_add` are called per
digit.  On x86-64 these compile to `imul` + `jo` (overflow jump), which is
cheap compared to memory traffic in `rust_decimal`'s path (which uses 128-bit
integer arithmetic and heap allocation).  Future optimisation: pre-compute
`max_safe_digits = floor(log10(i64::MAX / 10^S))` and use unchecked arithmetic
for the first N digits, switching to checked only near the boundary.  This is
a cycle-02 optimisation.

**Truncation vs rounding for excess fractional digits:** v0.1.0 silently
truncates toward zero.  `parse_round(s, mode)` is deferred to v0.2.  This
avoids adding a `Round` parameter to the hot path; `FromStr` cannot take extra
arguments anyway.

**Error type** (`ParseError`):

```rust
pub enum ParseError {
    /// Input was empty or contained only a sign or dot
    Empty,
    /// Non-digit, non-sign, non-dot byte at position `pos`
    InvalidChar { byte: u8, pos: usize },
    /// Accumulated value exceeded i64 range for this scale
    Overflow,
    /// (Reserved for future strict mode)
    TooManyFractional { got: u32, max: u32 },
}
```

`TooManyFractional` is reserved; currently extra fractional digits are silently
truncated (see above).

---

## 4. `from_f64` / `to_f64`

### `to_f64`

```rust
pub fn to_f64(self) -> f64 {
    (self.0 as f64) / SCALE_FACTOR  // SCALE_FACTOR = 10f64.powi(S as i32)
}
```

`SCALE_FACTOR` is computed once (at monomorphisation time, as a `const` or
`lazy_static` — the compiler will constant-fold it since `S` is const).  The
division is one `fdiv` instruction.

**Precision loss:** An `i64` has 63 significant bits; f64 has 53 mantissa bits.
For values requiring > 53 bits of integer precision (> 2^53 ≈ 9 × 10^15), the
conversion is lossy.  At S=2 this means values above ~90 trillion lose the
last ~1 cent of precision; at S=6 values above ~9 billion lose ~1 micro.  This
is inherent to f64 and unavoidable; it is documented and callers who care should
use `raw()` directly.

**Worst-case relative error from `to_f64`:** ε ≤ 2^-52 ≈ 2.2 × 10^-16 for
values in [2^52, 2^53) (one ULP).

### `from_f64`

Strategy: multiply `x` by `10^S`, round to integer, clamp to `[i64::MIN,
i64::MAX]`, convert to `i64`.

```rust
pub fn from_f64(x: f64) -> Self {
    Self::from_f64_round(x, Round::NearestEven)
}

pub fn from_f64_round(x: f64, mode: Round) -> Self {
    if x.is_nan() { return Self::ZERO; }
    let scaled = x * SCALE_FACTOR;  // f64 * f64
    let rounded = match mode {
        Round::NearestEven      => scaled.round_ties_even(),  // libm: rint()
        Round::Nearest          => scaled.round(),            // round half away from zero
        Round::TruncateTowardZero => scaled.trunc(),
        Round::TowardPosInf     => scaled.ceil(),
        Round::TowardNegInf     => scaled.floor(),
    };
    // Clamp before cast to avoid UB
    let clamped = rounded.clamp(i64::MIN as f64, i64::MAX as f64);
    Self(clamped as i64)
}
```

**Why `Round::NearestEven` as the default?**  Banker's rounding minimises
cumulative error when rounding many values in the same direction (common in
financial aggregation).  It is the IEEE 754 default mode.  Most users won't
notice the difference, but the ones who care (financial reporting) will
appreciate it.

**Precision loss from `from_f64`:**  f64 has only 15–17 significant decimal
digits.  At S=9, the scale factor is `10^9 = 1e9`; multiplying by it
amplifies the f64 rounding error.  Round-trip fidelity: `from_f64(to_f64(x))`
is not guaranteed to equal `x` when `|x.raw()| > 2^52 / 10^S`.  For S=4 and
values > 450 million, each nano is representable but round-trips may have
1-unit error.  Callers doing exact storage should use `raw()`/`from_raw()`.

**`f64::round_ties_even`:** This is `f64::round_ties_even()`, stabilised in
Rust 1.77 (March 2024).  Available on our MSRV.

---

## 5. Arithmetic: detailed rules

### Add / Sub

```
result.0 = lhs.0 ± rhs.0
```

Both operands must have the same scale `S` (enforced at the type level).
Overflow: `i64` wrapping is UB in Rust; the `+`/`-` operators call
`i64::checked_add` in debug builds (assert!) and wrap in release builds
matching `i64`'s standard behaviour.  Crate authors planning release builds
without overflow should use `checked_add`/`saturating_add`.

### Mul (same-scale)

```
result.0 = (lhs.0 as i128 * rhs.0 as i128) / 10^S
```

`i128` intermediate prevents overflow for `|lhs.0|, |rhs.0| ≤ i64::MAX`.
Division by `10^S` keeps the scale.  If the `i128` result doesn't fit in
`i64`, the `*` operator panics (debug and release).

This is intentionally lossy in the fractional component of the product (we
keep scale `S`, not `2S`).  For lossless scale-promoted multiplication, use
`mul_promote` (nightly feature).

### Div (same-scale, default truncate)

```
result.0 = (lhs.0 as i128 * 10^S) / rhs.0
```

Multiply the dividend by `10^S` before dividing (integer rescaling) to
preserve scale.  Truncates toward zero.  Panics on `rhs.0 == 0`.

### Overflow summary table

| Op  | `+`/`-` operator | `checked_add/sub` | `saturating_add/sub` |
|-----|-----------------|-------------------|----------------------|
| OV  | panic           | `None`            | clamp to MAX/MIN     |

| Op  | `*` operator  | `checked_mul` | `saturating_mul` |
|-----|--------------|---------------|------------------|
| OV  | panic         | `None`         | clamp to MAX/MIN  |

---

## 6. Dependency DAG (sibling tasks)

```
design-and-plan  (this task)
        │
        ▼
  cargo-skeleton
        │
        ▼
   core-type
   ┌────┴──────────────┐
   ▼                   ▼
parse-impl       f64-conversions    arithmetic
   └──────────────┬────────────────┘
                  ▼
              benchmark
                  │
                  ▼
          docs-and-readme
```

`parse-impl`, `f64-conversions`, and `arithmetic` are independent of each
other and can execute in parallel after `core-type` completes.  `benchmark`
waits for both `parse-impl` and `arithmetic`.

---

## 7. Out of scope (cycle 01)

| Feature                         | Reason deferred          | Target   |
|---------------------------------|--------------------------|----------|
| SIMD parse                      | Bench first; may not matter | Cycle 02 |
| `no_std`                        | `Display` needs `std::fmt` | Cycle 02 |
| serde feature                   | Trivial add-on            | Cycle 02 |
| Cross-scale `+`/`-`             | Footgun; explicit cast API first | Later |
| Scale-promote `*` on stable     | Needs `generic_const_exprs` | Post-stabilisation |
| `parse_round(s, mode)`          | Hot path purity            | Cycle 02 |
| Scientific notation             | Unneeded for finance       | Maybe    |
| `_` separators                  | Unneeded for v0.1          | Maybe    |

---

## 8. Performance targets

| Benchmark              | Target                          | Baseline             |
|------------------------|----------------------------------|----------------------|
| `decimal64::parse`     | ≥ 100 M parses/sec               | f64::from_str ~80 M/s  |
| `rust_decimal` parity  | ≥ 2× faster than rust_decimal    | rust_decimal ~30 M/s   |
| `decimal64::add`       | same as `i64::add` (1 instr)     | —                    |
| `decimal64::mul`       | ≤ 5 ns/op (i128 mul + i64 div)   | —                    |

These are rough targets; the `benchmark` task will establish actuals.

---

## Escalation filed

`/work/ralph-self-improvement/workspace/.escalations/decimal64-const-generic-block.md`
— operator review needed for `mul_promote` nightly dependency decision.
