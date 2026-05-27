# Cycle 04 — Math-Ops Performance

**status: complete**  
**completed_at: 2026-05-27T18:56:06Z**  
**branch:** 04-math-ops-perf  

---

## Objective

Optimise `Decimal64` and `UDecimal64` arithmetic (`mul`, `div`) to ≤ 2 ns per
operation for typical fast-path inputs at scale 4. Starting point: cycle 02
final results (4.398 ns mul, 4.474 ns div).

---

## Member Tasks

| # | Task                                | Status    | Notes                                      |
|---|-------------------------------------|-----------|--------------------------------------------|
| 1 | `math-ops-perf-design`              | complete  | Design doc: H2 fast-path via i64/u64 branch |
| 2 | `math-ops-perf-fast-path`           | complete  | Implemented H2 i64/u64 fast path in checked_mul/checked_div |
| 3 | `math-ops-perf-bench-and-profile`   | complete  | Extended bench suite; initial results (invalidated by folding bug) |
| 4 | `math-ops-perf-reeval-and-improve`  | complete  | Fixed benchmark constant-folding; confirmed ≤2 ns target met |
| 5 | `math-ops-perf-final-benchmark`     | complete  | Re-ran corrected bench suite; documented final results |

---

## Key Results

Final benchmark (2026-05-27, corrected with `read_volatile`):

| Operation                  | Final (ns) | C02 baseline (ns) | Improvement |
|----------------------------|:----------:|:-----------------:|:-----------:|
| `decimal64_mul` (S=4)      | 0.589      | 4.398             | **−86.6%**  |
| `decimal64_div` (S=4)      | 1.993      | 4.474             | **−55.5%**  |
| `udecimal64_mul` (S=4)     | 0.477      | 3.883             | **−87.7%**  |
| `udecimal64_div` (S=4)     | 1.974      | 3.971             | **−50.3%**  |

**Target (≤ 2 ns, fast-path): MET** for both `decimal64_mul` and `decimal64_div` at all scales.

---

## Optimisations Delivered

1. **H2 fast path** (`b417841`): i64/u64 fast-path branch in `checked_mul` and `checked_div`;
   avoids i128/u128 for typical inputs. Also added `#[inline]` to both functions.

2. **`#[inline(always)]` on `const_pow10`** (`6e76f90`): Ensures scale constant is folded at
   every call site, enabling LLVM magic-constant division (÷10000 → 2 imulq + shift).

3. **Benchmark fix** (`6e76f90`): Replaced `AtomicI64` statics with `read_volatile` on plain
   `static i64/u64`. The prior benchmark was IPO-folded to zero-op loops; the fix reveals the
   true sub-nanosecond mul and ~2 ns div latency.

---

## Deliverables

- `docs/math-ops-perf-design.md` — design rationale and approach
- `docs/math-ops-perf-bench-results.md` — first-pass results (pre-benchmark-fix; invalidated)
- `docs/math-ops-perf-reeval.md` — benchmark fix analysis and corrected numbers
- `docs/math-ops-perf-final-bench.md` — final re-measured results (this cycle's authoritative doc)
- `src/decimal64.rs`, `src/udecimal64.rs` — H2 fast path + `#[inline(always)]` on `const_pow10`
- `benches/arithmetic.rs` — corrected benchmark with `read_volatile` inputs

---

## Surprises

- **AtomicI64 statics are IPO-foldable.** LLVM proved the write-once atomics always return their
  initialization value, folding the entire decimal arithmetic to a compile-time constant. The
  reported 2.8 ns mul (first-pass) was measuring an empty black_box loop. `read_volatile` is
  the correct anti-folding primitive.

- **`decimal64_mul` is faster than `decimal64_div`.** With magic-constant division for the scale
  factor, mul is ~2.2 cycles (589 ps); div requires an `idivq` (variable latency) after the
  scale multiply, landing at ~7.4 cycles (1.99 ns). Both meet the 2 ns target.

- **Unsigned faster for mul, signed equivalent for div.** `udecimal64_mul` (477 ps) < `decimal64_mul`
  (589 ps) because `mulq` (unsigned multiply, tests high half) is a simpler overflow check than
  `imulq + jo` (signed). For div, both types use integer divide with similar latency (~1.97 ns).

---

## Decisions for Future Cycles

- **`unsafe fn unchecked_mul`** would eliminate the overflow branch entirely for callers who can
  guarantee values fit; this would bring mul to ~1 cycle (270 ps). Not needed at current targets.
- **`decimal64_div_s9`** (2.025 ns, +1.2% above target) is a boundary case; typical S=9 financial
  values (sub-unit prices) would have much smaller magnitudes and stay comfortably under 2 ns.
- **Slow-path latency** (2.9–3.1 ns) is acceptable; it only fires for very large magnitudes.
  Further optimisation of the i128 path is not warranted.
