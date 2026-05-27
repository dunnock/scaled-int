# Scientific Notation — Cycle 05 Design

**Date:** 2026-05-27  
**Branch:** 05-scientific-notation  

---

## 1. Objective

Add a thin `Scientific<D>` newtype wrapper that accepts scientific-notation strings in its `FromStr` implementation while leaving the base `Decimal64<S>` and `UDecimal64<S>` parsers entirely unchanged. The base parsers' performance characteristics (cycle 03 benchmark results) must not regress.

---

## 2. Representation and Public API

### 2.1 Type Definition

```rust
/// Wrapper around any `Decimal64<S>` or `UDecimal64<S>` that adds
/// scientific-notation parsing and display.
///
/// The inner value is accessible via `into_inner()` or the public field.
#[repr(transparent)]
pub struct Scientific<D>(pub D);
```

`repr(transparent)` is used so that the wrapper has zero overhead in terms of size, alignment, and ABI. The public inner field allows direct access, matching the ergonomics of similar newtype wrappers in the Rust ecosystem.

### 2.2 Signed Interface

```rust
impl<const S: u32> FromStr for Scientific<Decimal64<S>> {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err>;
}

impl<const S: u32> fmt::Display for Scientific<Decimal64<S>> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
}

impl<const S: u32> Scientific<Decimal64<S>> {
    /// Unwrap to the inner `Decimal64<S>`.
    pub fn into_inner(self) -> Decimal64<S>;
}
```

### 2.3 Unsigned Interface

The identical interface is provided for `UDecimal64<S>`:

```rust
impl<const S: u32> FromStr for Scientific<UDecimal64<S>> {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err>;
}

impl<const S: u32> fmt::Display for Scientific<UDecimal64<S>> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
}

impl<const S: u32> Scientific<UDecimal64<S>> {
    pub fn into_inner(self) -> UDecimal64<S>;
}
```

**Boundary decision:** Two separate `impl` blocks rather than a single generic `impl<D: SomeMarkerTrait>`. The generic approach would require a sealed marker trait with associated type accessors (`from_raw`, `into_raw`, scale) — a non-trivial abstraction with no current payoff. Two concrete impls are clear, compile-time verified, and do not leak implementation structure.

### 2.4 Deriving Standard Traits

`Scientific<D>` should derive `Clone`, `Copy`, `Debug`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`, and `Hash` by forwarding to `D`. Since `D` already implements all of these, `#[derive(...)]` is sufficient.

---

## 3. Grammar and Format

### 3.1 Accepted Input Grammar

```
scientific_str  ::= decimal_str [exponent_part]
decimal_str     ::= sign? integer_part ('.' fractional_part)?
exponent_part   ::= ('e' | 'E') sign? exponent_digits
sign            ::= '+' | '-'
integer_part    ::= digit*
fractional_part ::= digit*
exponent_digits ::= digit+
digit           ::= '0'..'9'
```

Notes:
- If `exponent_part` is absent, `from_str` delegates directly to the base parser with no added overhead beyond one scan for `e`/`E`.
- The mantissa sub-grammar is identical to what the base parser accepts: bare sign, integer-only, fractional-only (leading dot), integer + fractional (trailing dot is OK), and mixed. The existing parser invariants (`has_digits` guard, overflow detection, scale-S padding) all apply.
- The exponent is a bare signed integer; no decimal point is allowed in the exponent.
- `"1e"` (empty exponent digits) → `ParseError::Empty`.
- `"1e+"`  (sign with no digits) → `ParseError::Empty`.
- `"e5"` (empty mantissa) → `ParseError::Empty` (mantissa parse fails first).

### 3.2 Unsigned Restriction

For `Scientific<UDecimal64<S>>`, a negative sign in the mantissa (`"-1.5e2"`) is rejected with `ParseError::InvalidChar` by the existing `parse_unsigned::parse` function — no additional check is needed.

---

## 4. Parse Algorithm

### 4.1 High-Level Flow

```
fn from_str(s: &str) -> Result<Scientific<Decimal64<S>>, ParseError>
    1. Scan s for the first 'e' or 'E' byte.
    2. If not found → return Scientific(parse::<S>(s)?)
                       [pure delegation; zero overhead beyond the scan]
    3. mantissa_str = s[..exp_pos]
    4. exponent_str = s[exp_pos+1..]
    5. mantissa_raw = parse::<S>(mantissa_str)?   [i64]
    6. exponent     = parse_exponent(exponent_str)?  [i32]
    7. raw          = apply_exponent(mantissa_raw, exponent)?  [i64]
    8. return Ok(Scientific(Decimal64::from_raw(raw)))
```

### 4.2 `parse_exponent` Detail

```rust
fn parse_exponent(s: &str) -> Result<i32, ParseError> {
    // s is the substring after 'e'/'E'
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return Err(ParseError::Empty);
    }
    let (negative, start) = match bytes[0] {
        b'-' => (true,  1),
        b'+' => (false, 1),
        _    => (false, 0),
    };
    if start == bytes.len() {
        return Err(ParseError::Empty);  // sign with no digits
    }
    let mut acc: i32 = 0;
    for &b in &bytes[start..] {
        let d = b.wrapping_sub(b'0');
        if d > 9 {
            return Err(ParseError::InvalidChar { byte: b, pos: /* byte offset */ });
        }
        acc = acc
            .checked_mul(10)
            .and_then(|v| v.checked_add(d as i32))
            .ok_or(ParseError::Overflow)?;
    }
    Ok(if negative { -acc } else { acc })
}
```

Using `i32` for the exponent is sufficient: any exponent whose absolute value exceeds ~19 will either overflow or underflow immediately in `apply_exponent`, so the i32 range (-2_147_483_648 to 2_147_483_647) is more than enough.

### 4.3 `apply_exponent` Detail

The key insight: the base parser at scale `S` already stores the mantissa as `mantissa_mathematical × 10^S` in a raw `i64`. Applying an exponent of `E` to the mathematical value multiplies the raw storage by `10^E`.

```
mathematical_value = mantissa_mathematical × 10^E
raw_result         = mathematical_value × 10^S
                   = (mantissa_mathematical × 10^S) × 10^E
                   = mantissa_raw × 10^E
```

This derivation means `apply_exponent` is a single multiply or divide of the raw value:

```rust
fn apply_exponent(raw: i64, exponent: i32) -> Result<i64, ParseError> {
    match exponent.cmp(&0) {
        Ordering::Equal   => Ok(raw),
        Ordering::Greater => apply_positive_exp(raw, exponent as u32),
        Ordering::Less    => apply_negative_exp(raw, (-exponent) as u32),
    }
}

fn apply_positive_exp(raw: i64, exp: u32) -> Result<i64, ParseError> {
    // exp > 18 always overflows any non-zero i64 (10^19 > i64::MAX)
    if exp > 18 {
        return if raw == 0 { Ok(0) } else { Err(ParseError::Overflow) };
    }
    raw.checked_mul(pow10_i64(exp)).ok_or(ParseError::Overflow)
}

fn apply_negative_exp(raw: i64, neg_exp: u32) -> Result<i64, ParseError> {
    // neg_exp > 18: 10^19 > i64::MAX, so any raw / 10^neg_exp == 0
    // Return Underflow only when mantissa was nonzero (information loss is total)
    if neg_exp > 18 {
        return if raw == 0 { Ok(0) } else { Err(ParseError::Underflow) };
    }
    // For neg_exp in [1, 18]: divide with truncation toward zero.
    // This may silently produce 0 for small raw values — same behaviour
    // as the base parser's fractional-digit truncation.
    Ok(raw / pow10_i64(neg_exp))
}
```

**Worked examples (Decimal64<4>, scale = 4, 10^S = 10_000):**

| Input       | mantissa_raw | exponent | raw_result    | value        |
|-------------|:------------:|:--------:|:-------------:|:------------:|
| `"1.5e3"`   | 15_000       | +3       | 15_000_000    | 1500.0000    |
| `"1.5e0"`   | 15_000       | 0        | 15_000        | 1.5000       |
| `"1.5e-1"`  | 15_000       | −1       | 1_500         | 0.1500       |
| `"1.5e-3"`  | 15_000       | −3       | 15            | 0.0015       |
| `"1.5e-4"`  | 15_000       | −4       | 1             | 0.0001       |
| `"1.5e-5"`  | 15_000       | −5       | 0             | 0.0000 (silent truncation) |
| `"1.5e-19"` | 15_000       | −19      | Underflow     | —            |
| `"9.9e17"`  | 99_000       | +17      | Overflow      | —            |

**Underflow threshold rationale:** The threshold at `neg_exp > 18` is the natural arithmetic boundary: `10^19 > 9.22 × 10^18 = i64::MAX`, so for any valid `i64` raw value, dividing by `10^19` or more always yields exactly 0. Returning `Underflow` instead of `Ok(0)` at this boundary prevents silent discarding of inputs that are clearly out of meaningful range (e.g., `"1e-1000"`), while allowing values like `"0.00001"` at scale 4 to silently truncate to zero — consistent with the base parser's existing truncation of excess fractional digits.

For negative exponents in `[1, 18]`, the behaviour is intentionally the same as truncation: `"1.5e-5"` at scale 4 gives `Ok(0)`, just as `"0.000015"` would silently truncate to `0.0000`.

---

## 5. Error Contract

### 5.1 New Error Variant: `ParseError::Underflow`

```rust
pub enum ParseError {
    Empty,
    InvalidChar { byte: u8, pos: usize },
    Overflow,
    Underflow,   // ← NEW for cycle 05
    TooManyFractional { got: u32, max: u32 },
}
```

`Underflow` is returned when a nonzero mantissa is combined with a negative exponent so extreme (`|exponent| > 18`) that no representable non-zero value exists at any scale. The mathematical value is not zero, but the storage cannot distinguish it from zero.

**Note on additive compatibility:** Adding `Underflow` to `ParseError` is a non-breaking source change for `match` arms that use `_` or `..` catch-alls. Any downstream code with exhaustive `match` arms will see a compile error and must add the new arm — this is the intended behaviour for a semver-minor addition. Document in the changelog.

### 5.2 Error Summary

| Condition | Error variant |
|-----------|---------------|
| Empty input, bare sign, bare dot, empty exponent digits | `ParseError::Empty` |
| Exponent contains non-digit, non-sign character | `ParseError::InvalidChar` |
| Mantissa digit accumulation overflows `i64` | `ParseError::Overflow` |
| Positive exponent shift overflows `i64` | `ParseError::Overflow` |
| Nonzero mantissa with exponent < −18 | `ParseError::Underflow` |
| Mantissa for `UDecimal64` contains `'-'` or `'+'` | `ParseError::InvalidChar` |

---

## 6. Display Algorithm

### 6.1 Design Decision: Always-Scientific Form

`Display` for `Scientific<D>` always emits normalized scientific notation regardless of magnitude. The alternative (only-scientific outside a window such as `[1e-3, 1e6]`) adds conditional logic and surprises callers who expect `Display` to be stable across magnitudes.

Normalized form: `[-]M.MMMMMeN` where:
- Exactly one significant digit precedes the decimal point.
- Trailing zeros in the coefficient are retained so that the number of digits after the decimal point equals `S` minus the digits consumed by the leading integer part.
- The exponent `N` is signed, minimal (no leading zeros), using lowercase `e`.
- Special case: zero displays as `0e0`.

### 6.2 Algorithm

```
fn display(raw: i64, scale: u32) -> String
    1. Handle raw == 0: emit "0e0".
    2. Handle negative: record sign, work with abs(raw).
    3. Compute coefficient string = abs(raw).to_string()  [e.g., "15000" for raw=15000]
    4. The decimal value has `scale` digits to the right of the decimal point.
       Conceptually the decimal string is: coefficient with decimal point inserted
       `scale` places from the right. Example: "15000", scale=4 → "1.5000".
    5. Find p = position of first nonzero digit in the coefficient string (0-indexed).
       This gives: exponent = (coeff_len - scale - 1) - p
         ... where (coeff_len - scale - 1) is the units-digit position, and p is
         how many leading zeros precede the first significant digit.
    6. The mantissa digits start at position p.
       Emit: ['-'] + coeff[p] + '.' + coeff[p+1..] + 'e' + exponent
       If p+1 == coeff.len(): emit just coeff[p] + ".0e" + exponent (need at least
       one digit after decimal in normalized form? Design choice: no, allow "1e0".)
```

Worked examples (scale = 4):

| raw       | coeff str | decimal view | display      |
|-----------|-----------|--------------|--------------|
| 12345     | "12345"   | "1.2345"     | `1.2345e0`   |
| 100       | "100"     | "0.0100"     | `1e-2`       |
| 1234500   | "1234500" | "123.4500"   | `1.234500e2` |
| 10000     | "10000"   | "1.0000"     | `1e0`        |
| 1         | "1"       | "0.0001"     | `1e-4`       |

Trailing zeros after the last significant digit are stripped (e.g., raw=10000 → `1e0`, not `1.0000e0`). This keeps Display concise. Callers needing fixed width can use `format_args` with width/precision specifiers.

**Negative example:** raw = -12345, scale = 4 → `-1.2345e0`

### 6.3 Unsigned Display

Identical logic with `u64` arithmetic and no sign branch.

---

## 7. Module Layout and lib.rs Changes

### 7.1 New File: `src/scientific.rs`

All code for `Scientific<D>` lives in a single new module:

```
src/
  scientific.rs          ← new: Scientific<D> type, impls for Decimal64 and UDecimal64
  lib.rs                 ← export Scientific; add Underflow to ParseError
  parse.rs               ← unchanged
  parse_unsigned.rs      ← unchanged
  decimal64.rs           ← unchanged
  udecimal64.rs          ← unchanged
```

**Boundary decision:** Keeping all Scientific logic in one file avoids creating helper modules for what is ultimately a small surface area. The file will be roughly 200–300 lines including tests.

### 7.2 Internal Helpers: `parse_scientific_signed` and `parse_scientific_unsigned`

Two private free functions inside `scientific.rs`:

```rust
pub(crate) fn parse_scientific_signed<const S: u32>(s: &str)
    -> Result<Decimal64<S>, ParseError>;

pub(crate) fn parse_scientific_unsigned<const S: u32>(s: &str)
    -> Result<UDecimal64<S>, ParseError>;
```

These call the existing `parse::parse::<S>` and `parse_unsigned::parse::<S>` for the mantissa — no duplication of parse logic.

### 7.3 `lib.rs` Changes

1. Add `pub mod scientific;` and `pub use scientific::Scientific;`.
2. Add `Underflow` variant to `ParseError`.
3. Update `ParseError::Display` match arm for `Underflow`.
4. Update `ParseError`'s doc comment to mention `Scientific`.

No changes to `Cargo.toml`.

---

## 8. Test Matrix

The implement task must cover at minimum the following cases:

| Category | Input | Scale | Expected |
|----------|-------|-------|----------|
| No exponent (delegation) | `"1.2345"` | 4 | `Ok(raw=12345)` |
| Positive exponent, exact | `"1.5e3"` | 4 | `Ok(raw=15_000_000)` |
| Positive exponent, zero | `"0e100"` | 4 | `Ok(raw=0)` |
| Positive exponent, overflow | `"9.9e18"` | 4 | `Err(Overflow)` |
| Positive exponent, boundary (exp=18) | `"1e18"` | 0 | `Ok(raw=10^18)` if ≤ i64::MAX, else Overflow |
| Negative exponent, truncate | `"1.5e-5"` | 4 | `Ok(raw=0)` |
| Negative exponent, exact | `"1.5e-3"` | 4 | `Ok(raw=15)` |
| Negative exponent, boundary (neg_exp=18) | `"1e-18"` | 0 | `Ok(raw=0)` |
| Negative exponent, Underflow | `"1e-19"` | 0 | `Err(Underflow)` |
| Negative exponent, zero mantissa | `"0e-100"` | 4 | `Ok(raw=0)` |
| Uppercase E | `"1.5E3"` | 4 | `Ok(raw=15_000_000)` |
| Signed positive exponent | `"1.5e+3"` | 4 | `Ok(raw=15_000_000)` |
| Empty exponent | `"1e"` | 4 | `Err(Empty)` |
| Sign-only exponent | `"1e+"` | 4 | `Err(Empty)` |
| Invalid exponent char | `"1e3.0"` | 4 | `Err(InvalidChar)` |
| Empty mantissa | `"e5"` | 4 | `Err(Empty)` |
| Negative mantissa, signed | `"-1.5e3"` | 4 | `Ok(raw=-15_000_000)` |
| Negative mantissa, unsigned | `"-1.5e3"` | 4 | `Err(InvalidChar)` (UDecimal) |
| Round-trip Display→parse | various | 4 | `Ok(original_raw)` |

---

## 9. Performance Expectations

### 9.1 Parse Performance

The `Scientific` parser adds two overheads to the base parser path:

1. **Exponent scan:** One forward scan of the byte slice looking for `b'e'` or `b'E'`. This is a tight loop over ASCII bytes and is branch-predicted as "not found" for inputs that are already valid plain decimals. LLVM may auto-vectorize it.

2. **Exponent parse + apply:** Typically 3–6 digits for the exponent (`checked_mul` loop), then one `i64::checked_mul` or `i64::checked_div`. This is O(1) work.

Expected overhead for scientific-notation inputs: +15–25% vs the base parser.  
Expected overhead for non-scientific inputs (no `e`/`E`): the scan cost only, predicted at +5–10%.

### 9.2 Display Performance

The `Display` implementation is allocation-free (writes directly to `fmt::Formatter`) and involves:
- One integer `abs()` and one `to_string()`-equivalent digit loop
- Finding the first nonzero digit position (loop over coefficient string)
- Writing coefficient + `'e'` + exponent digits

This is comparable in cost to the base `Display` (which also formats the integer and fractional parts with a digit loop). No significant regression expected.

### 9.3 Benchmark Strategy

The benchmark task (`scientific-notation-benchmark-and-profile`) must:
- Baseline: `"9999999999.9999"` and `"0"` parsed via base `Decimal64` (from cycle 03 results).
- New cases: `"9.9999e9"`, `"1.0e-5"`, `"0"`, `"1234567.89"` (no exponent) via `Scientific`.
- Use `std::hint::black_box` and `static` + `read_volatile` inputs (per cycle 04 lessons).
- Report M/s (millions per second) for direct comparison.

---

## 10. Non-Goals and Deferred Decisions

- **`no_std` compatibility:** `Scientific` uses only `core` types; it is naturally `no_std` compatible once `ParseError` switches to `core::error::Error`. Deferred to cycle 07.
- **`serde` support:** Cycle 06 adds `Serialize`/`Deserialize` to `Decimal64` and `UDecimal64`. `Scientific` should get `serde` impls at the same time (emit scientific notation in `Serialize`, accept it in `Deserialize`). Deferred.
- **`parse_round` for scientific notation:** The base parser silently truncates extra fractional digits; a future `Scientific::parse_round(s, mode)` method could apply banker's rounding. Deferred.
- **Scale promotion in arithmetic on `Scientific<D>`:** Arithmetic delegates entirely to the inner `D` type. No cross-scale promotion is added by this cycle.
- **Alternate Display modes:** A `fixed_display()` method returning the standard decimal representation (same as base `Display`) could be added without changing `Display`. Not in this cycle.
- **Exponent notation in `ParseError` messages:** The `InvalidChar` variant does not carry position context for errors inside the exponent sub-string. The `pos` field will reflect the byte offset within the original input. This is a known limitation; a richer error type is out of scope.

---

## 11. Phase Decomposition

This cycle has four execution phases after this design task:

| Phase | Task slug | Role | Depends on |
|-------|-----------|------|------------|
| 1 | `scientific-notation-implement` | rust-implementer | this task |
| 2 | `scientific-notation-benchmark-and-profile` | rust-performance | implement |
| 3 | `scientific-notation-reeval-and-improve` | rust-performance | benchmark |
| 4 | `scientific-notation-final-benchmark` | rust-performance | reeval |
