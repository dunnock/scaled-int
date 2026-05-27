# Math-Ops Performance Design — Cycle 04

**Date:** 2026-05-27  
**Branch:** 04-math-ops-perf  
**Target:** ≤ 2 ns `mul`, ≤ 2 ns `div` for typical financial inputs (Decimal64, scale 4)

---

## 1. Baseline and Goals

### 1.1 Current Measurements

From cycle 02 final benchmark on i9-12900K (`CARGO_TARGET_DIR=/work/cargo-target-ralph cargo bench`):

| Operation     | Decimal64 (ns) | UDecimal64 (ns) | Estimated cycles (@ 3.6 GHz) |
|---------------|---------------:|----------------:|------------------------------:|
| `i64_add`     | 0.27           | —               | ~1                            |
| `add`         | 0.339          | 0.363           | ~1.2–1.3                     |
| `mul`         | 4.398          | 3.883           | ~14–16                        |
| `div`         | 4.474          | 3.971           | ~14–16                        |

The task spec baseline for cycle 01 was 4.73 ns mul / 4.46 ns div. Cycle 02 showed slight variation
(~0.4 ns) from run-to-run noise; treat 4.4–4.8 ns as the true baseline range.

`add` is essentially free — it compiles to a single `add` + conditional branch (branch not taken).
The bottleneck for `mul` and `div` is the **128-bit intermediate division step**.

### 1.2 Bottleneck Analysis

**Multiplication (`checked_mul`):**
```rust
let product = self.0 as i128 * rhs.0 as i128;   // i128 × i128
let scale   = const_pow10(S) as i128;             // compile-time constant
let result  = product / scale;                    // i128 ÷ constant  ← bottleneck
```

**Division (`checked_div`):**
```rust
let num    = self.0 as i128 * const_pow10(S) as i128;  // i128 × constant
let result = num / rhs.0 as i128;                       // i128 ÷ i128  ← bottleneck
```

x86-64 has no hardware 128-bit divide instruction. LLVM emits a software routine
(`__divti3` from compiler-rt, ~30–60 instructions) or, for division by a
compile-time constant, attempts the Granlund-Montgomery magic-constant technique
on 128-bit operands (requires 256-bit intermediate products, also multi-instruction).
Either way, the i128 division path is unavoidably expensive — roughly 12–18 cycles.

The `i128 × i128` multiply itself costs ~4–6 cycles on modern x86-64 (two `mulq`
instructions plus bookkeeping). So the mul path is: 4 (multiply) + 14 (divide) = 18 cycles.

### 1.3 Numerical Range Analysis

Understanding when operands overflow i64 is crucial for the fast-path design.

For scale S=4, `10^S = 10_000`. A raw `i64` can hold values up to `±9.22 × 10^18`.
For multiplication: the product of two raw values overflows i64 when
`|a| × |b| > 9.22 × 10^18`.

At scale 4 this means values whose raw magnitude exceeds `~96,000,000` (i.e.,
actual values beyond `±9,600.0000`) can produce i64 overflow in their product.
For typical financial prices (< $10,000) at scale 4, **the product always fits in i64**.

For the benchmark operands (`LHS_RAW = 1_234_567`, `RHS_RAW = 9_876_543`):
- Product = `1.219 × 10^13` — fits comfortably in i64 (max `9.22 × 10^18`)
- Final result = `1.219 × 10^13 / 10_000 = 1_219_300_000` — also fits in i64

For division: the numerator is `self.0 × 10^S`. For S=4, overflow occurs when
`|self.0| > 922_337_203_685_477` (i.e., actual value > `92,233,720,368.5477`).
Values below ~$92 billion at scale 4 are safe for i64 division.

---

## 2. Hypothesis Evaluation

### H1: Magic-Constant Division by 10^S

**Claim:** LLVM might already replace `i128 / 10^S` with multiply-by-magic + shift.

**Analysis:**

For 64-bit division by constant, LLVM reliably applies Granlund-Montgomery (multiply by
reciprocal approximation, then shift and correct). This is fast: ~4–6 cycles, comparable
to a multiply.

For 128-bit division by constant, LLVM is *less aggressive*. The magic constant for a
128-bit divisor requires a 256-bit intermediate, which has no hardware support. LLVM may:
1. Call `__divti3` from compiler-rt (software division, ~30–60 cycles).
2. Apply a partial optimization: shift out factors of 2 first, then call `__divti3` on the
   reduced value. `10^S = 2^S × 5^S`, so the factor-of-2 shift is free, but `5^S` still
   requires software division.
3. In some cases (small divisors), LLVM may unroll the 128-bit division into a sequence
   of 64-bit operations that avoids `__divti3` entirely.

**Verdict:** Uncertain without inspecting assembly. The implement task MUST check the
generated assembly with `CARGO_TARGET_DIR=/work/cargo-target-ralph cargo show-asm decimal64 checked_mul`
(or `objdump -d`). If `__divti3` is called, H2 is the primary fix. If LLVM already
generates magic-constant code, H1 is addressed automatically and the remaining cost is
in the multiply overhead.

**Action:** Verify in assembly. If `__divti3` is present, eliminate the i128 division
via H2. Do not hand-roll magic constants for i128 — the benefit is marginal if H2 already
covers typical inputs, and correctness of hand-rolled 128-bit magic constants is difficult
to verify.

### H2: i64 Intermediate When S Small (Primary Optimization)

**Claim:** For S≤9 and typical financial values, use i64 arithmetic and only fall back to
i128 when overflow is detected. Expected speedup: 2–3× for typical inputs.

**Design for `checked_mul`:**

```
Fast path (typical):
  1. product = a.checked_mul(b)      // i64::checked_mul → 1 add + 1 jo
  2. If Some(product):
       return Some(product / 10^S)   // i64 ÷ constant → magic multiply ~4 cycles
  3. If None (overflow → rare):
       fall through to i128 path

Slow path (overflow):
  4. product128 = a as i128 * b as i128
  5. result128  = product128 / 10^S as i128
  6. range-check → return Some/None
```

For the benchmark inputs, step 1 succeeds and the fast path takes ~5–7 cycles ≈ 1.5–2 ns.
The slow path (step 4–6) is identical to today's implementation and incurs the full ~18 cycles.

**Design for `checked_div`:**

```
Fast path (typical, S ≤ 18):
  1. scale = const_pow10(S)                // compile-time constant, i64
  2. num   = self.0.checked_mul(scale)     // detect if scaling overflows
  3. If Some(num):
       return Some(num / rhs.0)            // i64 ÷ i64 → ~4–6 cycles
  4. If None (overflow → rare):
       fall through to i128 path

Slow path (overflow):
  5. num128    = self.0 as i128 * scale as i128
  6. result128 = num128 / rhs.0 as i128
  7. range-check → return Some/None
```

**Why `const_pow10(S)` is safe to cast to i64:** `const_pow10` already returns `i64`
(the function signature is `const fn const_pow10(s: u32) -> i64` with a compile-time
assert that `s <= 18`). `i64::MAX = 9.22 × 10^18 > 10^18`, so all valid scales fit.

**Unsigned (`UDecimal64`) fast path:** Replace i64 checks with u64 checks. The logic is
symmetric; `u64::MAX = 1.84 × 10^19 > 10^18`, so all scales still fit. The unsigned
path avoids sign-extension overhead (as observed in cycle 02: 11–14% faster).

**Correctness invariant:** The fast path produces the same result as the i128 path when
`checked_mul` succeeds, because both compute `a * b / 10^S` with truncation toward zero,
and the i64 truncation and i128 truncation of the same value are identical when the i64
product doesn't overflow.

**Edge case — `i64::MIN × (-1)`:** `i64::checked_mul(i64::MIN, -1)` returns `None`
(because `i64::MIN × -1 = i64::MAX + 1` overflows i64). This is correctly handled: the
fast path falls to the i128 slow path, which handles it correctly (i64::MIN = -2^63,
-1 × -2^63 = 2^63 which overflows i64 but fits i128; result after dividing by 10^S
may or may not fit in i64 depending on S).

### H3: Split Mul into Low/High i64 Halves via `widening_mul`

**Claim:** Rust 1.86+ stabilised `u64::widening_mul` (returns `(lo: u64, hi: u64)`).
Use this to avoid the i128 type and get better codegen.

**Analysis:**

`widening_mul` produces the exact same result as casting to i128 and multiplying, but
the compiler sees two u64 values instead of one i128 value. On x86-64, i128 operations
already compile to two `mulq` instructions, so `widening_mul` produces identical machine
code — no benefit over H2 from a codegen perspective.

The advertised benefit is when combined with a magic-constant divide on the 128-bit
product: if `hi == 0`, the 128-bit product fits in u64 and can be divided with a single
`divq` or magic-constant `mulq + shr`. This is equivalent to H2's `checked_mul` check but
using different Rust API.

**Verdict:** H3 is a more explicit form of H2 for the unsigned case. It uses `widening_mul`
to check overflow, then falls back to the pair-of-u64 form for the division. The expected
speedup is the same as H2. Since H2 is simpler to implement and achieves the same result,
H3 is deferred unless H2 shows unexpected issues on the specific LLVM version in use.

**Note on MSRV:** The project MSRV is Rust 1.77 (for `f64::round_ties_even`). `widening_mul`
requires Rust 1.86. Promoting MSRV for H3 is therefore needed; gate it behind a compile
check or promote the MSRV to 1.86 and update `Cargo.toml`.

### H4: Avoid `expect`/Panic Checks on Hot Path

**Claim:** Replace `checked_add().expect()` with `unsafe { a.unchecked_add(b) }` for a
small latency gain.

**Analysis:**

In release builds (`-C opt-level=3`), `i64::checked_add(a, b).expect("msg")` compiles to:
```asm
add  rax, rdi         ; i64 addition
jo   .overflow_panic  ; conditional jump if overflow flag set
```

The `jo` (jump-on-overflow) instruction is predicted "not taken" by the CPU's branch
predictor since overflow is a rare condition. On modern out-of-order CPUs, the cost of a
correctly-predicted branch is effectively zero (the branch is retired speculatively).

`i64::unchecked_add` (available as stable `wrapping_add` for `add` semantics, or as the
`unchecked_add` intrinsic on nightly) produces:
```asm
add  rax, rdi         ; identical to above but no overflow branch
```

For `add`, the difference is ≤ 0.05 ns (well within noise at 0.3 ns total). The checked
variant already runs at ~1.3 cycles, which is essentially a single-issue throughput.

For `mul` and `div`, the dominant cost is the i128 division (H2 addresses this). The
overflow range check is a comparison + conditional branch added after the division, adding
≤ 1 cycle. This is not the bottleneck.

**Verdict:** Provide `unsafe fn unchecked_add / unchecked_sub / unchecked_mul / unchecked_div`
as opt-in variants for callers who need maximum throughput and can guarantee no overflow.
Do NOT change the default trait implementations — the current `Add`/`Sub` behaviour (panic
on overflow) is correct and safe. The `unsafe` variants are a niche optimization.

**Feature design:**
```rust
impl<const S: u32> Decimal64<S> {
    /// Unchecked add — UB if overflow occurs.
    /// Safety: caller must ensure `self.0 + rhs.0` does not overflow i64.
    #[inline(always)]
    pub unsafe fn unchecked_add(self, rhs: Self) -> Self {
        // SAFETY: caller guarantees no overflow
        Self(unsafe { self.0.unchecked_add(rhs.0) })
    }
    // ... similarly for sub, mul, div
}
```

This requires nightly for `i64::unchecked_add` (intrinsic). On stable, use
`self.0.wrapping_add(rhs.0)` which generates the same assembly but different semantics
(wrapping vs UB on overflow). Gate behind `#[cfg(feature = "unchecked")]` or provide
both wrapping and unchecked variants.

### H5: Const-Evaluate 10^S Table

**Claim:** A `const POW10: [i128; 19]` lookup array might be optimised better by the
compiler than the current `const fn const_pow10`.

**Analysis:**

`const_pow10(S)` is a `const fn` — the Rust compiler evaluates it at compile time and
substitutes the resulting literal at every call site. This is semantically equivalent to
indexing a `const` array. Both produce the same LLVM IR: a literal integer constant in
the instruction.

The real question is whether materializing `const_pow10(S) as i128` causes any overhead.
Since `S` is a `const` generic parameter, the value is known at monomorphization time, and
LLVM will inline the constant directly into the division instruction. There is no runtime
lookup.

**Verdict:** No change needed. H5 provides zero benefit over the current implementation
because `const_pow10` already produces compile-time constants indistinguishable from a
pre-baked table. However, a pre-baked table form (`const POW10_I64: [i64; 19]`) may be
marginally clearer and avoids the loop in `const_pow10` when reading the code. If added,
keep it internal (`pub(crate)`) and use it from all places that call `const_pow10`.

---

## 3. Design Decisions

### 3.1 Overflow Semantics

**Decision: Keep panic-on-overflow as the default; do not change trait impls.**

The `Add`, `Sub`, `Mul`, `Div` trait implementations on both `Decimal64` and `UDecimal64`
will continue to panic on overflow. The fast-path optimization (H2) is an internal
implementation detail that does not change the visible semantics — both the fast i64 path
and the slow i128 path return `None` from `checked_mul`/`checked_div` for the same inputs,
and both propagate the panic from `Mul::mul` for the same inputs.

The `unchecked_*` variants (H4) are a separate opt-in API, not a replacement for the
default behaviour.

### 3.2 Inline Hints

**Decision: `#[inline]` on trait impls; `#[inline(always)]` on inner helpers.**

The `Add`, `Sub`, `Mul`, `Div` trait impls currently have `#[inline]` on `add`/`sub`/`mul`/`div`.
This allows cross-crate inlining (the compiler decides based on heuristics). For the hot
arithmetic paths, we want the entire `checked_mul` body to be inlined into the caller.

Add `#[inline]` to `checked_mul` and `checked_div` (currently not marked). This enables
the callee's i64 fast path to be inlined and have its constant folded with the caller's
context.

For `const_pow10`: already evaluated at compile time; no inline annotation needed.

For the fast-path branch (`product / const_pow10(S)` in i64): LLVM will constant-fold
the division into a magic-multiply + shift after inlining, producing optimal code.

**Cross-crate note:** Without `#[inline]`, `checked_mul` is not available for inlining
into crates that depend on `decimal64`. With `#[inline]`, the i64 fast path can be
inlined and the branch for overflow can be eliminated entirely if the caller's types
prove the inputs are always in range (e.g., small-value financial data).

### 3.3 Scale Interaction

For same-scale multiplication (the current implementation): `Decimal64<S> × Decimal64<S> → Decimal64<S>`.
The fast path handles this correctly: the product raw value is divided by `10^S` before
comparing against i64 bounds.

For scale-promoting multiplication (gated on nightly `generic_const_exprs`):
`Decimal64<A> × Decimal64<B> → Decimal64<A+B>`. The intermediate product raw value is
NOT divided; it is returned directly. The fast path still applies: check if the product
fits in i64; if so, return it directly without the divide step. This is a simpler
operation and has even less overhead.

### 3.4 Panicking Behavior Optimization

The `Mul` impl calls `self.checked_mul(rhs).expect("Decimal64 multiplication overflow")`.
When the fast path (H2) returns `Some(x)`, the `expect` call is eliminated by the
optimizer because the `Option` is always `Some`. When the slow path is taken,
`expect` still incurs a branch.

A subtle but important point: the `Mul` impl's panic message becomes dead code on the
fast path after optimization. The optimizer will eliminate the panic call for the fast
path but retain it for the slow path. This is the correct behavior.

---

## 4. Implementation Specification

### 4.1 Changes to `src/decimal64.rs`

#### `checked_mul` — two-phase fast path

```rust
#[inline]
pub fn checked_mul(self, rhs: Self) -> Option<Self> {
    // Fast path: i64 product (covers most financial values at S <= 18)
    if let Some(product) = self.0.checked_mul(rhs.0) {
        return Some(Self(product / const_pow10(S)));
    }
    // Slow path: full i128 (rare; handles large magnitudes)
    let product = self.0 as i128 * rhs.0 as i128;
    let scale   = const_pow10(S) as i128;
    let result  = product / scale;
    if result >= i64::MIN as i128 && result <= i64::MAX as i128 {
        Some(Self(result as i64))
    } else {
        None
    }
}
```

**Note on correctness:** `i64::checked_mul` returning `None` does NOT mean the final
result overflows i64 — it means the intermediate product overflows. Example: multiply
`i64::MAX / 100_000` by `200_000` at S=5 (scale = 100_000). The product overflows i64
but the result = `i64::MAX / 100_000 * 200_000 / 100_000 = i64::MAX / 50` fits in i64.
The slow path handles this correctly via i128.

#### `checked_div` — scaled-numerator fast path

```rust
#[inline]
pub fn checked_div(self, rhs: Self) -> Option<Self> {
    if rhs.0 == 0 {
        return None;
    }
    // Fast path: if numerator (self * 10^S) fits in i64, use i64 division
    let scale = const_pow10(S);
    if let Some(num) = self.0.checked_mul(scale) {
        return Some(Self(num / rhs.0));
    }
    // Slow path: full i128
    let num    = self.0 as i128 * scale as i128;
    let result = num / rhs.0 as i128;
    if result >= i64::MIN as i128 && result <= i64::MAX as i128 {
        Some(Self(result as i64))
    } else {
        None
    }
}
```

**Note:** The fast path result `num / rhs.0` always fits in i64 because:
- `num` fits in i64 (checked_mul succeeded)
- Dividing a value that fits in i64 by a non-zero i64 produces a value that fits in i64

#### `saturating_mul` — update to use fast path

```rust
pub fn saturating_mul(self, rhs: Self) -> Self {
    if let Some(product) = self.0.checked_mul(rhs.0) {
        return Self(product / const_pow10(S));
    }
    let product = self.0 as i128 * rhs.0 as i128;
    let scale   = const_pow10(S) as i128;
    let result  = product / scale;
    Self(result.clamp(i64::MIN as i128, i64::MAX as i128) as i64)
}
```

#### `#[inline]` additions

Add `#[inline]` to `checked_mul`, `checked_div`, `checked_div_round`, and
`saturating_mul`. These are currently not marked for inlining; without the annotation
they are not inlined cross-crate.

### 4.2 Changes to `src/udecimal64.rs`

Apply symmetric changes using `u64::checked_mul` and `u128`:

```rust
// checked_mul fast path
if let Some(product) = self.0.checked_mul(rhs.0) {
    return Some(Self(product / const_pow10_u64(S)));
}

// checked_div fast path
let scale = const_pow10_u64(S);
if let Some(num) = self.0.checked_mul(scale) {
    return Some(Self(num / rhs.0));
}
```

### 4.3 Assembly Verification Checklist

After implementing H2, verify the generated assembly:

1. Run `CARGO_TARGET_DIR=/work/cargo-target-ralph cargo show-asm decimal64 'decimal64::decimal64::Decimal64<4_u32>::checked_mul'`
   - **Expected:** fast path uses `imulq` + `jo` (branch-on-overflow), then `imulq` + `shrq`
     (magic divide for constant 10000). No `__divti3` call in the fast path.
   - **If `__divti3` present:** the i128 slow path is present but should only be reached
     after the jo branch. Verify the fast path does NOT call `__divti3`.

2. Benchmark with `CARGO_TARGET_DIR=/work/cargo-target-ralph cargo bench -- arithmetic`
   - **Expected:** `decimal64_mul` ≤ 2.5 ns, `decimal64_div` ≤ 2.5 ns.
   - Target ≤ 2 ns requires the fast path to be ~6–7 cycles, which is achievable on i9-12900K.

### 4.4 `unchecked_*` Variants (H4, Optional)

If the benchmark result after H2 still exceeds 2 ns, add unsafe fast-path variants:

```rust
impl<const S: u32> Decimal64<S> {
    /// Unchecked multiply — undefined behaviour if overflow occurs.
    /// Safety: caller guarantees `self.0 * rhs.0 / 10^S` fits in i64 without
    ///         intermediate overflow (i.e., `self.0 * rhs.0` fits in i64).
    #[inline(always)]
    pub unsafe fn unchecked_mul(self, rhs: Self) -> Self {
        // SAFETY: caller ensures no overflow; wrapping_mul matches unchecked semantics
        // when overflow does not occur.
        Self(self.0.wrapping_mul(rhs.0) / const_pow10(S))
    }
}
```

This removes the `jo` branch entirely and produces 2 instructions (multiply + magic divide).
Expected latency: ~2 cycles ≈ 0.55 ns. Gate behind `#[cfg(feature = "unchecked")]` to
keep the safe API as the default.

---

## 5. Expected Performance Outcomes

### 5.1 Predicted Benchmark Results

For the benchmark inputs (`LHS_RAW = 1_234_567`, `RHS_RAW = 9_876_543` at scale 4):
- `1_234_567 × 9_876_543 = 1.219 × 10^13` — fits in i64 (`< 9.22 × 10^18`). Fast path taken.
- `1_234_567 × 10_000 = 1.234 × 10^10` — fits in i64 for div fast path.

Predicted assembly for `checked_mul` fast path (after H2):
```asm
; a in rax, b in rdi
imulq rdi, rax       ; multiply (sets overflow flag)
jo    .slow_path     ; rare; branch predicted not-taken
; magic divide by 10000: multiply by 7378697629483820647, shift >> 66
movabsq $7378697629483820647, %rcx
imulq rcx, rax
shrq  $13, rdx        ; (approximate; actual shift depends on LLVM's magic)
; ... range check elided (result always in i64 range for fast path)
ret
```

At 3 cycles per instruction, ~3 instructions → ~1 ns. Combined with loop/call overhead
in Criterion, ~1.5–2 ns total is expected.

### 5.2 Risks and Mitigations

| Risk | Probability | Mitigation |
|------|-------------|------------|
| LLVM doesn't generate magic-constant divide for i64 / 10000 | Low | Verify assembly; if not, use explicit magic constants from table |
| Branch misprediction for `checked_mul` overflow check | Very low | Overflow is rare for financial inputs; predictor will learn quickly |
| Fast path gives different rounding than i128 path | None | Both compute truncation toward zero; results are identical when i64 product doesn't overflow |
| Benchmark variance obscures real improvement | Medium | Run each bench 10× and report min/median; use `criterion` warmup settings |
| `saturating_mul` fast path returns wrong result for large inputs | None | Fast path only active when i64 product doesn't overflow; result fits in i64 by construction |

---

## 6. Phase Decomposition

This cycle has four execution phases (sibling tasks):

### Phase 1 — Implement (`math-ops-perf-implement`)

Depends on: `math-ops-perf-design-and-plan`

- Apply H2 to `checked_mul` and `checked_div` in `src/decimal64.rs`
- Apply H2 to `checked_mul` and `checked_div` in `src/udecimal64.rs`
- Update `saturating_mul` in both files
- Add `#[inline]` to `checked_mul`, `checked_div`, `checked_div_round`, `saturating_mul`
- Verify `cargo test --all` passes
- Optionally: add `unchecked_*` variants behind a feature flag (H4)
- Capture generated assembly for `checked_mul<4>` and `checked_div<4>`; include in commit

### Phase 2 — Benchmark and Profile (`math-ops-perf-benchmark-and-profile`)

Depends on: `math-ops-perf-implement`

- Run `cargo bench -- arithmetic` and record results
- Add expanded benchmark coverage: test at S=2, S=4, S=9 with overflow and non-overflow inputs
- Run `perf stat` to measure cycles per op
- Capture assembly in docs

### Phase 3 — Re-eval and Improve (`math-ops-perf-reeval-and-improve`)

Depends on: `math-ops-perf-benchmark-and-profile`

- Compare results against 2 ns target
- If target not met: investigate additional options (H3 widening_mul, explicit magic constants)
- Apply any profitable improvements; re-run tests

### Phase 4 — Final Benchmark (`math-ops-perf-final-benchmark`)

Depends on: `math-ops-perf-reeval-and-improve`

- Re-run full benchmark with optimized code
- Document delta vs cycle 01/02 baseline
- Update cycle.md and create escalation if targets met

---

## 7. Non-Goals and Deferred Decisions

- **SIMD multiply:** Not applicable to scalar integer arithmetic.
- **Parallel batching:** Out of scope; single-value operations only.
- **Newton-Raphson iterative division:** Too complex for the benefit; i64 fast path is simpler.
- **Hand-rolled 128-bit magic constants:** Deferred unless assembly inspection shows a clear need.
- **`parse_round` integration:** Unrelated to arithmetic performance.
- **`no_std` changes:** Will fall naturally from arithmetic changes since no new stdlib imports are introduced.
- **Promoting MSRV to 1.86 for `widening_mul`:** Deferred; H2 achieves the same result on MSRV 1.77.
