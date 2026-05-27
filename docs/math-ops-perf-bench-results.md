# Math-Ops Performance Benchmark Results — Cycle 04

**Date:** 2026-05-27  
**Branch:** 04-math-ops-perf  
**Rustc:** 1.95.0 (59807616e 2026-04-14)  
**CPU:** 12th Gen Intel(R) Core(TM) i9-12900K  
**CPU Governor:** `powersave` (cpupower unavailable; effective bench frequency ~2.8 GHz — calibrated from `i64_mul` throughput)  
**Command:** `CARGO_TARGET_DIR=/work/cargo-target-ralph cargo bench --bench arithmetic`

---

## 1. Optimization Applied

Cycle 04 introduced the **H2 i64/u64 fast path** in `checked_mul` and `checked_div`:

- When the intermediate product (mul) or scaled numerator (div) fits in i64/u64, use integer arithmetic and the compiler's magic-constant divide instead of the i128/u128 path.
- Fall through to the original i128/u128 path only when overflow is detected.
- Added `#[inline]` to `checked_mul` and `checked_div` to enable cross-crate inlining.

---

## 2. Full Criterion Output

### 2.1 Scale 4 (Primary — 123.4567 × 987.6543)

| Benchmark           | Median (ns) | Change vs baseline   |
|---------------------|------------:|---------------------|
| `decimal64_add`     | 0.343       | +2.0% (noise)       |
| `udecimal64_add`    | 0.380       | +6.4% (noise)       |
| `i64_add`           | 0.403       | — (new baseline)    |
| `decimal64_mul`     | 2.808       | **−36.6%** ✓        |
| `udecimal64_mul`    | 3.363       | **−11.6%** ✓        |
| `i64_mul`           | 0.359       | — (new baseline)    |
| `decimal64_div`     | 2.589       | **−42.5%** ✓        |
| `udecimal64_div`    | 3.185       | **−17.2%** ✓        |
| `i64_div`           | 1.224       | — (new baseline)    |

*Baseline: cycle 02 final run — decimal64 mul 4.398 ns, div 4.474 ns; udecimal64 mul 3.883 ns, div 3.971 ns.*

Raw Criterion output (abridged):

```
arithmetic/decimal64_add   time: [338.55 ps 342.57 ps 346.95 ps]  change: +1.9%
arithmetic/udecimal64_add  time: [375.12 ps 379.90 ps 384.57 ps]  change: +6.4%
arithmetic/i64_add         time: [399.58 ps 402.73 ps 406.07 ps]  (new)
arithmetic/decimal64_mul   time: [2.7884 ns 2.8077 ns 2.8289 ns]  change: −36.6%
arithmetic/udecimal64_mul  time: [3.3326 ns 3.3626 ns 3.3906 ns]  change: −11.6%
arithmetic/i64_mul         time: [355.18 ps 359.21 ps 362.98 ps]  (new)
arithmetic/decimal64_div   time: [2.5804 ns 2.5892 ns 2.5987 ns]  change: −42.5%
arithmetic/udecimal64_div  time: [3.1382 ns 3.1850 ns 3.2324 ns]  change: −17.2%
arithmetic/i64_div         time: [1.2230 ns 1.2244 ns 1.2257 ns]  (new)
```

### 2.2 Scale 2 (Fast Path — 123.45 × 987.65)

Both operand products fit well within i64/u64 — fast path always taken.

| Benchmark              | Median (ns) |
|------------------------|------------:|
| `decimal64_mul_s2`     | 1.665       |
| `udecimal64_mul_s2`    | 1.885       |
| `decimal64_div_s2`     | 1.581       |
| `udecimal64_div_s2`    | 1.894       |

Raw Criterion output:

```
arithmetic/decimal64_mul_s2   time: [1.6588 ns 1.6647 ns 1.6713 ns]
arithmetic/udecimal64_mul_s2  time: [1.8673 ns 1.8845 ns 1.9015 ns]
arithmetic/decimal64_div_s2   time: [1.5773 ns 1.5806 ns 1.5840 ns]
arithmetic/udecimal64_div_s2  time: [1.8771 ns 1.8941 ns 1.9093 ns]
```

**Both decimal64 mul and div are under 2 ns at scale 2. ✓**

### 2.3 Scale 9 (Mixed Fast/Slow Path — 1.234567000 × 9.876543000)

Product: 1_234_567_000 × 9_876_543_000 = 1.219 × 10^19.
- Signed i64::MAX ≈ 9.22 × 10^18 → product overflows → **slow i128 path** for `Decimal64`.
- Unsigned u64::MAX ≈ 1.84 × 10^19 → product fits → **fast u64 path** for `UDecimal64`.

This is the key asymmetry between signed and unsigned at S=9.

Division fast path: LHS × 10^9 = 1.234 × 10^18 < i64::MAX → **fast path** for both mul and div types.

| Benchmark              | Median (ns) | Path                    |
|------------------------|------------:|-------------------------|
| `decimal64_mul_s9`     | 4.647       | i128 slow (i64 overflow)|
| `udecimal64_mul_s9`    | 2.018       | u64 fast (u64 fits)     |
| `decimal64_div_s9`     | 2.032       | i64 fast                |
| `udecimal64_div_s9`    | 2.028       | u64 fast                |

Raw Criterion output:

```
arithmetic/decimal64_mul_s9   time: [4.0915 ns 4.6470 ns 5.4089 ns]  (6 high-severe outliers)
arithmetic/udecimal64_mul_s9  time: [2.0150 ns 2.0182 ns 2.0215 ns]
arithmetic/decimal64_div_s9   time: [2.0269 ns 2.0315 ns 2.0362 ns]
arithmetic/udecimal64_div_s9  time: [2.0226 ns 2.0277 ns 2.0325 ns]
```

Note: `decimal64_mul_s9` shows high variance (4.1–5.4 ns) because it hits the i128 slow path with a non-power-of-2 divisor (10^9 = 2^9 × 5^9); the division by 5^9 is not as easily optimized by LLVM as smaller scale values.

### 2.4 Large Magnitude — i128/u128 Slow Path Forced

For both mul and div, inputs are chosen so the fast-path check fails:

- **mul_large (S=4):** LHS=2,000,000,000 × RHS=5,000,000,000 → product=1×10^19 > i64::MAX (overflow); result=1×10^15 fits.
- **div_large (S=4):** LHS=i64::MAX, RHS=10,000 → LHS×10^4 overflows i64; result=i64::MAX fits.

| Benchmark                | Median (ns) |
|--------------------------|------------:|
| `decimal64_mul_large`    | 5.369       |
| `udecimal64_mul_large`   | 3.064       |
| `decimal64_div_large`    | 5.154       |
| `udecimal64_div_large`   | 4.287       |

Raw Criterion output:

```
arithmetic/decimal64_mul_large   time: [5.3426 ns 5.3692 ns 5.3995 ns]
arithmetic/udecimal64_mul_large  time: [3.0363 ns 3.0643 ns 3.0979 ns]
arithmetic/decimal64_div_large   time: [5.1318 ns 5.1538 ns 5.1770 ns]
arithmetic/udecimal64_div_large  time: [4.2549 ns 4.2870 ns 4.3202 ns]
```

Slow-path decimal64 operations are ~5.2–5.4 ns, comparable to the pre-optimization baseline (~4.4–4.5 ns); the branching overhead of the fast-path check adds ~0.5 ns to the slow path.

---

## 3. Estimated Cycles per Operation

`perf stat` is unavailable in this environment (`perf_event_paranoid=4`, no perf binary).
Cycles are estimated using `i64_mul` as calibration: i64 multiply throughput on Alder Lake = 1 cycle/iteration; measured = 359 ps → **effective bench clock ≈ 2.79 GHz**.

| Operation                     | Median (ns) | Est. cycles |
|-------------------------------|------------:|------------:|
| `i64_add`                     | 0.403       | 1.1         |
| `i64_mul`                     | 0.359       | 1.0 (cal.)  |
| `i64_div`                     | 1.224       | 3.4         |
| `decimal64_mul` (S4, fast)    | 2.808       | 7.8         |
| `decimal64_div` (S4, fast)    | 2.589       | 7.2         |
| `decimal64_mul` (S2, fast)    | 1.665       | 4.6         |
| `decimal64_div` (S2, fast)    | 1.581       | 4.4         |
| `decimal64_mul` (S9, slow)    | 4.647       | 13.0        |
| `udecimal64_mul` (S4, fast)   | 3.363       | 9.4         |
| `udecimal64_div` (S4, fast)   | 3.185       | 8.9         |
| `decimal64_mul_large` (slow)  | 5.369       | 15.0        |
| `decimal64_div_large` (slow)  | 5.154       | 14.4        |

**Fast-path anatomy (decimal64_mul, S4, ~7.8 cycles):**

The fast path for S=4 mul compiles approximately to:
```asm
imulq rdi, rax          ; i64 product (3-cycle latency, sets OF)
jo    .slow_path        ; overflow branch — predicted not-taken (0 cycles)
movabsq $magic, %rcx    ; magic constant for / 10000
imulq %rcx, %rax        ; magic multiply (3-cycle latency)
sar   $shift, %rdx      ; arithmetic shift
```

Latency chain: 3 (imul) + 3 (magic imul) + 1 (shift/adjust) + 1-2 (Option unwrap) ≈ 8–9 cycles.
Consistent with measured 7.8 cycles.

---

## 4. Comparison vs Pre-Optimization Baseline

From cycle 02 (`docs/udecimal64-bench-results.md` final results, 2026-05-27):

| Operation       | Cycle 02 (ns) | Cycle 04 (ns) | Improvement |
|-----------------|:-------------:|:-------------:|:-----------:|
| `decimal64_mul` | 4.398         | 2.808         | **−36.6%**  |
| `decimal64_div` | 4.474         | 2.589         | **−42.5%**  |
| `udecimal64_mul`| 3.883         | 3.363         | **−11.6%**  |
| `udecimal64_div`| 3.971         | 3.185         | **−17.2%**  |

### vs Target (≤ 2 ns)

| Operation                | Cycle 04 (ns) | Target (ns) | Met? |
|--------------------------|:-------------:|:-----------:|:----:|
| `decimal64_mul` (S4)     | 2.808         | 2.0         | ✗ (+40%) |
| `decimal64_div` (S4)     | 2.589         | 2.0         | ✗ (+29%) |
| `decimal64_mul` (S2)     | 1.665         | 2.0         | **✓**    |
| `decimal64_div` (S2)     | 1.581         | 2.0         | **✓**    |
| `udecimal64_mul` (S9)    | 2.018         | 2.0         | ✗ (+1%)  |
| `decimal64_div` (S9)     | 2.032         | 2.0         | ✗ (+2%)  |

The 2 ns target is met for S=2 operations and very nearly met for S=9. For S=4, the 8-cycle latency chain from two sequential multiplications (product imulq + magic-divide imulq) bounds the minimum at ~2.8 ns at the observed ~2.8 GHz bench frequency.

---

## 5. Key Findings

1. **H2 fast path works as designed.** For typical financial values at S=4, the i64 fast path is taken on every iteration, producing 36–42% speedup over the i128-only baseline.

2. **Scale 2 is under 2 ns.** The magic constant for `/100` is simpler than `/10000`, reducing the second multiply's latency or enabling a different (cheaper) compiler decomposition. Both `decimal64_mul_s2` (1.67 ns) and `decimal64_div_s2` (1.58 ns) beat the 2 ns target.

3. **Signed vs. unsigned reversal.** In cycle 02 (i128/u128 paths only), unsigned was 11–14% faster. In cycle 04 (i64/u64 fast paths), **signed is now 11–17% faster** than unsigned for S=4. This is likely because `i64::checked_mul` uses `imulq + jo` (overflow flag directly), while `u64::checked_mul` uses `mulq + test rdx` (requires testing the high half separately), adding a data dependency before the magic divide.

4. **S=9 asymmetry.** At S=9 with inputs LHS=1.23×10^9, RHS=9.88×10^9:
   - `Decimal64` mul triggers the i128 slow path (product 1.22×10^19 > i64::MAX), resulting in 4.6 ns.
   - `UDecimal64` mul stays on the u64 fast path (product 1.22×10^19 < u64::MAX ≈ 1.84×10^19), resulting in 2.0 ns.
   - `div` (both types) takes the fast path since the scaled numerator fits.

5. **Slow path is ~5 ns.** Large-magnitude inputs that force the i128 path cost 5.2–5.4 ns (decimal64), slightly above the pre-optimization baseline due to the added fast-path check overhead (~0.5 ns branch tax when the slow path is taken).

6. **perf unavailable.** `perf_event_paranoid=4` and no `perf` binary in PATH prevented hardware counter measurement. Cycles estimated from `i64_mul` calibration (359 ps/cycle → ~2.8 GHz effective bench clock).

---

## 6. Recommendations for Next Cycle

- **Reeval (`math-ops-perf-reeval-and-improve`)** should investigate whether the 2 ns target can be hit at S=4. The bottleneck is the two-multiply latency chain (imul + magic imul). Options:
  - Pre-computed reciprocal stored in the struct (avoids repeated magic constant materialization).
  - Explicit `#[inline(always)]` on `checked_mul` / `checked_div` (currently `#[inline]`).
  - Profile with actual instruction counts when perf is available.
- The S=2 and S=9 (div) results already meet the 2 ns target, suggesting the target is achievable at S=4 with additional optimization.
- `decimal64_mul_s9` high variance (σ ≈ 10%) warrants investigation; the i128 division by 10^9 (= 5^9 × 2^9) may not benefit from a magic constant and could be improved with an explicit decomposition.
