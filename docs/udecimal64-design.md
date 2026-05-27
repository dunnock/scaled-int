# UDecimal64 Design — Cycle 02

**Date:** 2026-05-27  
**Author:** code-designer agent  
**Status:** Accepted

---

## 1. Overview and Motivation

`UDecimal64<const S: u32>` is the unsigned counterpart to `Decimal64<S>`. It stores
non-negative fixed-point decimal values in a `u64` using the same scale-as-const-generic
discipline: the raw `u64` represents the mathematical value multiplied by `10^S`.

### 1.1 Why a Separate Unsigned Type?

The primary motivation is **domain safety**, not raw speed:

- Many financial and scientific quantities are inherently non-negative: market quotes,
  order quantities, timestamps, weights, balances. Encoding this constraint in the type
  eliminates an entire class of logic bugs at zero runtime cost.
- The unsigned domain doubles the representable positive range (u64::MAX ≈ 1.84×10¹⁹
  vs i64::MAX ≈ 9.22×10¹⁸).
- The parser skips the sign-analysis branch entirely, yielding a measurable speedup
  on inputs where branch-prediction misses matter (benchmarks will quantify this).

### 1.2 Relationship to Decimal64

`UDecimal64<S>` shares:
- The same const-generic scale discipline.
- The same scale range (S ∈ 0..=18).
- The same arithmetic semantics for same-scale operations.
- The same `Round` enum from `lib.rs`.
- The same `ParseError` enum (with one message update noted in §6).

`UDecimal64<S>` differs in:
- Storage: `u64` instead of `i64`.
- Constants: `MAX = u64::MAX`, no `MIN` (minimum is always `ZERO`).
- Parse: `+` and `-` are `InvalidChar`, not valid prefixes.
- Subtraction: `Sub` trait returns `Option<Self>` to prevent silent underflow.
- `from_f64`: negative and NaN inputs clamp to `ZERO` instead of clamping to `MIN`.

---

## 2. Internal Representation

```rust
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UDecimal64<const S: u32>(u64);
```

### 2.1 Why `repr(transparent)` over `u64`

Identical justification to `Decimal64<S>`:
- Allows safe transmutation between `u64` slices and `UDecimal64<S>` slices in
  performance-critical code (e.g. SIMD pipelines, shared-memory IPC).
- Guarantees the ABI matches a plain `u64` parameter in FFI.
- Zero overhead: the compiler generates the same code as raw `u64` arithmetic.

### 2.2 Scale and Invariant

The invariant is: `self.0 == mathematical_value * 10^S` (exact, no rounding).

`const_pow10_u64(s: u32) -> u64` is a compile-time helper that asserts `s <= 18`.
At scale 18: `10^18 = 1_000_000_000_000_000_000`, `u64::MAX = 18_446_744_073_709_551_615`.
`ONE = 10^18` still fits; the maximum representable value is ~18.44 at scale 18.

Scale 19 would also technically fit (`10^19 < u64::MAX`), but for consistency with
`Decimal64<S>` (which enforces `S <= 18` because `10^19 > i64::MAX`) the limit stays 18.

### 2.3 Representable Range Summary

| Scale | ONE raw    | Max value (approx)       |
|-------|-----------|--------------------------|
| 0     | 1         | 18,446,744,073,709,551,615 |
| 2     | 100       | 184,467,440,737,095,516.15 |
| 4     | 10,000    | 1,844,674,407,370,955.16   |
| 6     | 1,000,000 | 18,446,744,073,709.55      |
| 9     | 10^9      | 18,446,744,073.71          |
| 18    | 10^18     | 18.44                      |

---

## 3. Public API Surface

### 3.1 Type Alias and Module

- Type: `pub struct UDecimal64<const S: u32>(u64)` in `src/udecimal64.rs`
- Re-exported from `lib.rs`: `pub use udecimal64::UDecimal64`
- Module: `pub(crate) mod parse_unsigned` for the fast unsigned parser

### 3.2 Constants

```rust
impl<const S: u32> UDecimal64<S> {
    pub const SCALE: u32 = S;
    pub const ZERO: Self = Self(0);
    pub const ONE: Self = Self(const_pow10_u64(S));  // 10^S as u64
    pub const MAX: Self = Self(u64::MAX);
}
```

No `MIN` constant: the minimum is always `ZERO`. Adding an alias `MIN = ZERO` would
be confusing; omit it.

### 3.3 Raw Access

```rust
#[inline(always)] pub const fn from_raw(raw: u64) -> Self
#[inline(always)] pub const fn raw(self) -> u64
```

Same zero-cost contract as `Decimal64<S>`. Caller is responsible for the scale invariant.

### 3.4 Parsing

```rust
pub fn parse(s: &str) -> Result<Self, ParseError>
impl<const S: u32> FromStr for UDecimal64<S> { type Err = ParseError; }
```

See §5 for the full parse algorithm. Key contract: `+` and `-` both return
`ParseError::InvalidChar`. Otherwise semantics match `Decimal64<S>::parse`.

### 3.5 f64 Conversions

```rust
pub fn from_f64(x: f64) -> Self               // NearestEven rounding
pub fn from_f64_round(x: f64, mode: Round) -> Self
pub fn to_f64(self) -> f64
```

See §8 for conversion details. Summary: `NaN → ZERO`, negative → `ZERO`, positive
overflow → `MAX`.

### 3.6 Signed/Unsigned Interop

```rust
// On UDecimal64<S>:
pub fn as_signed(self) -> Option<Decimal64<S>>

// On Decimal64<S> (added via impl block in udecimal64.rs):
pub fn as_unsigned(self) -> Option<UDecimal64<S>>
```

`as_signed` returns `None` when `self.0 > i64::MAX as u64` (raw value overflows i64).
`as_unsigned` returns `None` when `self.0 < 0` (negative values have no unsigned equivalent).

These are the **only** conversion points between the two types. There is no implicit
coercion: callers must explicitly cross the boundary.

**Boundary decision:** The `as_unsigned` method is implemented in `src/udecimal64.rs`
as an additional `impl<const S: u32> Decimal64<S>` block. This avoids a circular
dependency (if placed in `decimal64.rs`, it would import `UDecimal64` before it's
defined; placing it in `udecimal64.rs` lets it import both types naturally).

### 3.7 Arithmetic

```rust
// Add — panics on overflow (matches signed convention)
impl<S> Add for UDecimal64<S> { type Output = Self; }

// Sub — returns Option to prevent silent underflow
impl<S> Sub for UDecimal64<S> { type Output = Option<Self>; }

// Mul — panics on overflow, uses u128 intermediate
impl<S> Mul for UDecimal64<S> { type Output = Self; }

// Div — panics on division by zero, truncates toward zero
impl<S> Div for UDecimal64<S> { type Output = Self; }

// Checked variants — all return Option<Self>
pub fn checked_add(self, rhs: Self) -> Option<Self>
pub fn checked_sub(self, rhs: Self) -> Option<Self>  // same as Sub trait
pub fn checked_mul(self, rhs: Self) -> Option<Self>
pub fn checked_div(self, rhs: Self) -> Option<Self>

// Saturating variants — all return Self
pub fn saturating_add(self, rhs: Self) -> Self  // clamps to MAX
pub fn saturating_sub(self, rhs: Self) -> Self  // clamps to ZERO
pub fn saturating_mul(self, rhs: Self) -> Self  // clamps to MAX

// Division with rounding
pub fn div_round(self, rhs: Self, mode: Round) -> Self
pub fn checked_div_round(self, rhs: Self, mode: Round) -> Option<Self>
```

See §7 for full arithmetic semantics.

### 3.8 Rescaling

```rust
pub fn rescale_into<const OUT: u32>(self) -> Option<UDecimal64<OUT>>
pub fn rescale_round_into<const OUT: u32>(self, mode: Round) -> Option<UDecimal64<OUT>>
```

Identical semantics to `Decimal64<S>`: `rescale_into` is lossless (None on fractional
loss or overflow); `rescale_round_into` applies a rounding mode and returns None only
on overflow.

### 3.9 Display and Debug

```rust
impl<S> Display for UDecimal64<S> // "1.2345" — no sign prefix ever
impl<S> Debug for UDecimal64<S>   // "UDecimal64<4>(12345)"
```

`Display` follows the same formatting as `Decimal64`: always emit exactly `S` fractional
digits with zero-padding. Because the raw is `u64`, no sign handling is needed;
`unsigned_abs()` calls are unnecessary.

---

## 4. Traits and Derives

`UDecimal64<S>` derives: `Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash`

Because the storage is `u64`, the derived `Ord` is lexicographic on the raw value, which
is identical to numeric order for non-negative decimals at the same scale — correct by
construction.

**`Ord` cross-scale note:** Same-scale comparison is safe. Cross-scale comparison has no
provided impl (compile error), consistent with `Decimal64<S>`. Callers must
`rescale_into` first.

---

## 5. Parse Algorithm

### 5.1 Grammar

```
unsigned-decimal ::= integer ('.' fractional)?
integer           ::= digit+
fractional        ::= digit+
digit             ::= '0'..'9'
```

Signs (`+`, `-`) are explicitly excluded. The parser does **not** treat `+` as a valid
prefix (unlike `Decimal64`). This simplifies the first branch from:

```
match bytes[0] { b'-' => ..., b'+' => ..., _ => () }
```
to:
```
// no branch at all — start accumulating immediately
```

### 5.2 Algorithm Walkthrough

```
INPUT: s: &str, SCALE: S

bytes = s.as_bytes()
if bytes.is_empty() => Err(Empty)

// Reject sign characters immediately (no signed variant here)
if bytes[0] == b'-' || bytes[0] == b'+' => Err(InvalidChar { byte, pos: 0 })

acc: u64 = 0
has_digits: bool = false

// Integer part
while i < len && bytes[i] != b'.' {
    d = bytes[i].wrapping_sub(b'0')
    if d > 9 => Err(InvalidChar)
    acc = acc.checked_mul(10)
             .and_then(|v| v.checked_add(d as u64))
             .ok_or(Overflow)?
    has_digits = true
    i += 1
}

// Fractional part
frac_digits: u32 = 0
if bytes[i] == b'.' {
    i += 1
    while i < len {
        d = bytes[i].wrapping_sub(b'0')
        if d > 9 => Err(InvalidChar)
        has_digits = true
        if frac_digits < S {
            acc = acc.checked_mul(10)
                     .and_then(|v| v.checked_add(d as u64))
                     .ok_or(Overflow)?
            frac_digits += 1
        }
        i += 1
    }
}

if !has_digits => Err(Empty)

// Scale padding
for _ in frac_digits..S {
    acc = acc.checked_mul(10).ok_or(Overflow)?
}

Ok(UDecimal64::from_raw(acc))
```

### 5.3 Performance Analysis

Compared to `Decimal64::parse`, the unsigned parser:

1. **Eliminates the sign branch**: No `match bytes[0]` for `+`/`-`. On consistently
   positive inputs, the signed parser takes a predicted branch here; on the unsigned
   parser there is no branch. Modern CPUs handle this well via prediction, so the
   benefit is small but consistent.

2. **Eliminates the negation step**: No `if negative { -acc }` at the end. One fewer
   instruction in the hot path.

3. **Uses u64 arithmetic**: The digit accumulation loop uses `u64::checked_add` and
   `u64::checked_mul`. On x86-64, these compile to identical instruction sequences as
   their `i64` equivalents (both use 64-bit MULQ/ADDQ). No performance difference here.

4. **Avoids sign-extension**: The result is returned directly as `UDecimal64::from_raw(acc)`
   without a conditional negate. This saves one instruction per parse call.

**Expected delta:** 1–5% faster than `Decimal64::parse` for short strings (1–6 chars);
negligible for longer strings where the digit accumulation loop dominates. The benchmark
task will measure this precisely; if the delta is sub-noise the implementation task should
document it explicitly.

**The value is semantic, not performance**: even if benchmarks show no measurable speedup,
`UDecimal64` remains valuable as a domain type that prevents negative-value bugs.

### 5.4 Error Behavior

Matches `Decimal64::parse` except:
- `"+"` → `InvalidChar { byte: b'+', pos: 0 }`
- `"-1.5"` → `InvalidChar { byte: b'-', pos: 0 }`
- `"-0"` → `InvalidChar { byte: b'-', pos: 0 }`
- `"."` alone → `Empty`
- `""` → `Empty`
- `"1e5"` → `InvalidChar { byte: b'e', pos: 1 }`
- `"99999999999999999999"` → `Overflow` (exceeds u64::MAX)

Extra fractional digits beyond `S` are silently **truncated toward zero** (same as
`Decimal64`). `parse_round` for banker's-rounding on truncation is deferred to a later
cycle.

---

## 6. Error Types

`UDecimal64` reuses `ParseError` from `lib.rs` without modification to the enum
variants. However:

**Required change to `ParseError::Overflow` display:** The current message is
`"value overflows i64"`. This is incorrect for `UDecimal64` (whose storage is `u64`).
The implementation task must change the display to the type-neutral `"numeric overflow"`.
This is backward-compatible at the type level; only the display string changes.

No new error variants are needed for cycle 02.

---

## 7. Arithmetic Semantics

### 7.1 Addition

```rust
impl<const S: u32> Add for UDecimal64<S> {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        self.checked_add(rhs).expect("UDecimal64 addition overflow")
    }
}
```

Panics on `u64` overflow. Rationale: matches `Decimal64` convention where the operator
panics and the caller uses `checked_add` for fallible paths. Consistent API surface
is more important than diverging behavior.

`checked_add` uses `u64::checked_add` directly (one instruction + branch).

`saturating_add` clamps to `u64::MAX` (i.e., `Self::MAX`).

### 7.2 Subtraction

```rust
impl<const S: u32> Sub for UDecimal64<S> {
    type Output = Option<Self>;
    fn sub(self, rhs: Self) -> Option<Self> {
        self.0.checked_sub(rhs.0).map(Self)
    }
}
```

**Rationale for `Option<Self>` as Output:** Unsigned subtraction underflow is silent
wrapping, which is almost never the desired behavior for decimal types. Rather than
panicking (which `Decimal64` does), or wrapping silently (which raw `u64` does), the
`Sub` trait returns `Option<Self>`. This forces callers to handle the underflow case:

```rust
let diff = qty - fill;           // Option<UDecimal64<4>>
let diff = (qty - fill)?;        // propagates None as error
let diff = (qty - fill).unwrap_or(UDecimal64::ZERO); // clamp to zero
```

`checked_sub` is provided for symmetry with the other checked variants; it has identical
behavior to the `Sub` trait operator:
```rust
pub fn checked_sub(self, rhs: Self) -> Option<Self> {
    self.0.checked_sub(rhs.0).map(Self)
}
```

`saturating_sub` clamps to `ZERO` on underflow (since unsigned values cannot go below zero):
```rust
pub fn saturating_sub(self, rhs: Self) -> Self {
    Self(self.0.saturating_sub(rhs.0))  // saturating_sub on u64 returns 0 on underflow
}
```

**Ergonomic tradeoff:** Having `Sub` return `Option` is unusual in Rust and may surprise
users expecting `Self`. The alternative (panic on underflow, like `Decimal64`) was rejected
by the task specification because:
1. Unsigned underflow is not a programming error in the same sense as signed overflow —
   it's a domain condition (insufficient quantity, balance underflow).
2. `Option` is more expressive for financial code where `fill > qty` is a legitimate
   runtime condition, not a bug.

### 7.3 Multiplication

```rust
pub fn checked_mul(self, rhs: Self) -> Option<Self> {
    let product = self.0 as u128 * rhs.0 as u128;
    let scale = const_pow10_u64(S) as u128;
    let result = product / scale;
    if result <= u64::MAX as u128 {
        Some(Self(result as u64))
    } else {
        None
    }
}
```

The `Mul` trait panics (`expect`) on overflow; `checked_mul` and `saturating_mul`
are the non-panicking variants. Uses u128 intermediate to avoid intermediate overflow.

Truncation semantics: `product / scale` truncates toward zero (standard integer
division). This is consistent with `Decimal64::checked_mul`.

### 7.4 Division

```rust
pub fn checked_div(self, rhs: Self) -> Option<Self> {
    if rhs.0 == 0 { return None; }
    let num = self.0 as u128 * const_pow10_u64(S) as u128;
    let result = num / rhs.0 as u128;
    if result <= u64::MAX as u128 {
        Some(Self(result as u64))
    } else {
        None
    }
}
```

For unsigned division there are no negative quotients; `TowardPosInf` and
`TowardNegInf` may still differ for non-exact quotients. The `div_round_u128`
helper must be adapted for unsigned arithmetic (no sign tracking needed; all
values are non-negative so the rounding simplifies).

`div_round_u128` for unsigned inputs:
- All of `q`, `r`, `den` are non-negative.
- `TowardPosInf`: if `r > 0`, add 1.
- `TowardNegInf`: never subtract (floor == trunc for positives).
- `Nearest`/`NearestEven`: same half-up / banker's logic but without sign handling.

### 7.5 Overflow / Underflow Summary

| Operation      | Trait/Method        | On overflow/underflow         |
|----------------|---------------------|-------------------------------|
| `a + b`        | `Add`               | panic                         |
| `a - b`        | `Sub`               | `None` (underflow → None)     |
| `a * b`        | `Mul`               | panic                         |
| `a / b`        | `Div`               | panic (b=0); never overflows  |
| `checked_add`  | method              | `None`                        |
| `checked_sub`  | method              | `None` (same as Sub trait)    |
| `checked_mul`  | method              | `None`                        |
| `checked_div`  | method              | `None` (b=0)                  |
| `saturating_add` | method            | `MAX`                         |
| `saturating_sub` | method            | `ZERO`                        |
| `saturating_mul` | method            | `MAX`                         |

---

## 8. f64 Conversion Details

```rust
pub fn from_f64_round(x: f64, mode: Round) -> Self {
    if x.is_nan() || x < 0.0 {
        return Self::ZERO;
    }
    let scale_factor = 10f64.powi(S as i32);
    let scaled = x * scale_factor;
    let rounded = match mode { /* same as Decimal64 */ };
    // Clamp to [0.0, u64::MAX as f64] before cast
    let clamped = rounded.clamp(0.0, u64::MAX as f64);
    Self(clamped as u64)
}
```

Key differences from `Decimal64::from_f64_round`:
1. `NaN` → `ZERO` (same as signed).
2. Negative values (including `-0.0`) → `ZERO` (unsigned cannot represent negative;
   clamping rather than panicking is the correct behavior for a conversion function).
3. Lower bound for clamp is `0.0` (not `i64::MIN as f64`).
4. Upper bound for clamp is `u64::MAX as f64`.

**Note on `u64::MAX as f64`:** `u64::MAX = 18_446_744_073_709_551_615`. In f64, this
rounds to `18_446_744_073_709_551_616.0` (one ULP higher). The saturating cast
`rounded as u64` in Rust 1.45+ handles this correctly (any f64 ≥ u64::MAX is
saturated to u64::MAX). No special handling needed.

`to_f64`:
```rust
pub fn to_f64(self) -> f64 {
    (self.0 as f64) / (const_pow10_u64(S) as f64)
}
```

Same precision caveat as `Decimal64::to_f64`: lossless for `raw < 2^53`; the last
few ULPs are lost for larger values. This is documented, not fixed.

---

## 9. Module Layout

After cycle 02 implementation, the source tree becomes:

```
src/
├── lib.rs            — exports UDecimal64; updates ParseError::Overflow message
├── decimal64.rs      — unchanged (except as_unsigned() added by udecimal64.rs)
├── parse.rs          — unchanged (signed parser)
├── udecimal64.rs     — UDecimal64 type + all impl blocks + as_unsigned() for Decimal64
└── parse_unsigned.rs — fast unsigned parser (no sign branch)
```

`lib.rs` additions:
```rust
pub(crate) mod parse_unsigned;
pub mod udecimal64;
pub use udecimal64::UDecimal64;
```

### 9.1 Internal Helper: `const_pow10_u64`

```rust
const fn const_pow10_u64(s: u32) -> u64 {
    assert!(s <= 18, "UDecimal64 scale must be <= 18");
    let mut result: u64 = 1;
    let mut i = 0u32;
    while i < s { result *= 10; i += 1; }
    result
}
```

This is private to `udecimal64.rs`. If the signed and unsigned pow10 helpers are
ever unified (e.g., via a trait), that is a later refactor.

### 9.2 `div_round_u128` for Unsigned

A private helper in `udecimal64.rs` for rounding division of non-negative u128 values.
Simpler than the signed variant: all quantities are non-negative, eliminating sign-tracking
branches.

```rust
fn div_round_u128(num: u128, den: u128, mode: Round) -> u128 {
    debug_assert!(den != 0);
    let q = num / den;
    let r = num % den;
    if r == 0 { return q; }
    match mode {
        Round::TruncateTowardZero => q,
        Round::TowardPosInf => q + 1,         // always positive remainder
        Round::TowardNegInf => q,             // floor == trunc for positives
        Round::Nearest => {
            if r * 2 >= den { q + 1 } else { q }
        }
        Round::NearestEven => {
            let r2 = r * 2;
            if r2 > den { q + 1 }
            else if r2 == den { if q % 2 != 0 { q + 1 } else { q } }
            else { q }
        }
    }
}
```

No risk of overflow in `r * 2` for the Nearest/NearestEven cases: `r < den`,
so `r * 2 < 2*den`. Since both `num` and `den` are at most u128::MAX/2 in practice
(we're dividing at most u64 values scaled by 10^18), this is safe.

---

## 10. Dependency DAG (Cycle 02 Tasks)

```
udecimal64-design-and-plan (this task)
    │
    └─► udecimal64-implement
            │
            └─► udecimal64-benchmark-and-profile
                    │
                    └─► udecimal64-reeval-and-improve
                            │
                            └─► udecimal64-final-benchmark
```

Each arrow is a hard `depends_on` — no task begins before its predecessor is `done`.

### 10.1 Task Summaries

**`udecimal64-implement`** (`rust-implementer` role)
- Implement `UDecimal64<S>` per this design doc.
- New files: `src/udecimal64.rs`, `src/parse_unsigned.rs`.
- Changes to `src/lib.rs`: export, ParseError message fix.
- No Cargo.toml changes needed (no new dependencies).
- Comprehensive unit tests covering all API surface.
- `cargo test` must pass with no warnings.

**`udecimal64-benchmark-and-profile`** (`benchmarker` role)
- Add `UDecimal64` parse benchmarks to `benches/parse.rs` alongside `Decimal64` baselines.
- Add arithmetic benchmarks (add, mul, div) to `benches/arithmetic.rs`.
- Target: show UDecimal64 parse at same or faster throughput as Decimal64.
- Capture results in `docs/udecimal64-bench-results.md`.
- If UDecimal64 parse is slower, document why with profiling evidence.

**`udecimal64-reeval-and-improve`** (`optimiser` role)
- Analyse benchmark output. Identify top bottleneck.
- If parse is the bottleneck: consider SIMD digit scan for long strings
  (the cycle 01 bench gap at 10+ digits showed 1.60×–1.87× vs rust_decimal).
- If arithmetic is the bottleneck: consider bitwise tricks for small-scale divisions.
- Apply at most one targeted optimisation; re-run `cargo bench` to confirm improvement.
- Document the change and its effect.

**`udecimal64-final-benchmark`** (`benchmarker` role)
- Re-run full benchmark suite (both `Decimal64` and `UDecimal64` sides).
- Update `docs/udecimal64-bench-results.md` with the final numbers.
- Document delta vs `Decimal64` parse (expected: ≥1% faster or "equivalent, documented").
- Signal cycle complete.

---

## 11. Out of Scope (Cycle 02)

- **Serde support** — deferred to cycle 06. No `Serialize`/`Deserialize` derives.
- **`no_std` support** — deferred to cycle 07. The same minimal `std` dependency profile
  as `Decimal64` applies.
- **Scientific notation parse** — deferred to cycle 05.
- **`parse_round` (banker's rounding on truncation)** — deferred. Extra fractional digits
  are truncated toward zero.
- **Scale-promoting `Mul`** (`UDecimal64<A> * UDecimal64<B> → UDecimal64<{A+B}>`) —
  nightly only via `generic_const_exprs`; not included. A `mul_promote` explicit method
  may be added in a later cycle.
- **Neg** — not applicable to unsigned types.

---

## 12. Design Invariants Summary

The following invariants must hold after any operation on `UDecimal64<S>`:

1. **Non-negative**: `self.0` is a `u64`; the type system enforces this statically.
2. **Scale consistency**: arithmetic on `UDecimal64<A>` and `UDecimal64<B>` where
   `A ≠ B` is a **compile error** (no implicit coercion). Only same-scale operations
   are defined on the traits.
3. **No NaN, no Inf**: not applicable (integer storage). Conversion from `f64::NAN`
   or `f64::INFINITY` produces `ZERO` or `MAX` respectively.
4. **Overflow contract**: documented per operation in §7.5. No silent wrapping anywhere
   in the public API.
5. **Parse completeness**: every valid unsigned decimal string (matching the grammar
   in §5.1) parses without error as long as the raw value ≤ `u64::MAX`.
