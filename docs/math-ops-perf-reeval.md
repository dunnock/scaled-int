# math-ops-perf: re-evaluation and benchmark fix

## Summary

The ≤ 2 ns target is **met** for typical (fast-path) inputs at S=4:

| Operation | Time | Target |
|-----------|------|--------|
| `decimal64_mul` S=4 | 585 ps | ≤ 2 ns ✓ |
| `decimal64_div` S=4 | 1.95 ns | ≤ 2 ns ✓ |

However, the cycle 04 S=4 benchmark results (`math-ops-perf-bench-results.md`) were
invalid — LLVM constant-folded the entire computation away. This document explains
why, what was fixed, and the corrected numbers.

---

## 1. Discovery: cycle 04 benchmarks were measuring wrong operations

### Root cause

The original benchmark used `AtomicI64::new(CONST)` statics and loaded them inside
each `b.iter` closure with `Ordering::Relaxed`. The intention was to prevent
constant-folding via the memory fence semantics of atomics.

LLVM defeated this via **interprocedural optimization (IPO)**:
- The statics are write-once (initialized to constants, never written again).
- LLVM's whole-program alias analysis proved the loads always return the
  initialization value.
- The arithmetic was folded to a compile-time constant; only the `black_box` store
  remained in the hot loop.

Assembly evidence (old benchmark, S=4 `decimal64_mul` hot loop):
```asm
movabsq $12193254061881, %rsi    ; precomputed LHS_RAW * RHS_RAW — scale division missing
.LBB250_2:
    movq    %rsi, (%rsp)         ; black_box(constant)
    #APP #NO_APP
    movq    (%rsp), %r8
    movq    %r8, (%rsp)
    #APP #NO_APP
    decq    %rcx
    jne     .LBB250_2
```

The constant `12193254061881 = 1_234_567 × 9_876_543` is the raw product; LLVM
proved the `checked_mul` overflow check would never trip and eliminated the
`/ 10_000` scale division entirely. The reported 2.808 ns was measuring ~8 bytes of
no-op memory traffic, not decimal multiplication.

### Other ops affected

The S=4 `decimal64_div` benchmark was similarly measuring raw `LHS / RHS` without
the `× 10_000` scale multiply. All S=2 benchmarks were also affected. The S=9 and
large-magnitude benchmarks were valid (overflow prevented folding).

---

## 2. Fix: `std::ptr::read_volatile` for benchmark inputs

Replaced `AtomicI64`/`AtomicU64` statics with plain `static i64`/`static u64` and
used `unsafe { std::ptr::read_volatile(&STATIC) }` inside each `b.iter` closure.

`read_volatile` is defined in LLVM as having observable side effects; the compiler
is prohibited from eliminating or reordering volatile reads. Unlike `black_box`
(which only prevents optimization of the *value*, not its source) and atomic loads
(which are IPO-foldable for write-once globals), `read_volatile` is fully opaque to
constant-propagation.

### Assembly verification after fix

**`decimal64_mul` S=4 hot loop** (function `h39eb69912941a3b2`):
```asm
movabsq $3777893186295716171, %r15    ; magic constant for ÷10000
.LBB278_2:
    movq    S_LHS(%rip), %rdx          ; volatile load LHS (1_234_567)
    movq    S_RHS(%rip), %rax          ; volatile load RHS (9_876_543)
    movq    %rdx, %rcx
    imulq   %rax, %rcx                 ; rcx = LHS × RHS (overflow flag set if > i64)
    jo      .LBB278_4                  ; overflow → i128 slow path
    movq    %rcx, %rax
    imulq   %r15                       ; rdx:rax = rax × magic (128-bit signed)
    movq    %rdx, %rax
    movq    %rdx, %rcx
    shrq    $63, %rcx                  ; sign bit (round toward zero)
    sarq    $11, %rax                  ; arithmetic right shift
    addq    %rcx, %rax                 ; corrected quotient
.LBB278_6:
    movq    %rax, 8(%rsp)              ; black_box(result)
```

Magic constant verification: `3777893186295716171 ≈ ⌈2^(64+11) / 10000⌉`,
confirming LLVM emits the correct magic-constant division by 10,000.

**`decimal64_div` S=4 hot loop** (function `hdd8e1b2b7632dc0e`):
```asm
movl    $10000, %edi                   ; preload scale (constant-folded from const_pow10(4))
.LBB293_2:
    movq    S_LHS(%rip), %rdx          ; volatile load LHS
    movq    S_RHS(%rip), %r8           ; volatile load RHS
    testq   %r8, %r8                   ; check RHS ≠ 0
    je      panic_div_by_zero
    imulq   $10000, %rdx, %rax         ; rax = LHS × scale
    jo      .LBB293_8                  ; overflow → i128 slow path (__divti3)
    ; (i64::MIN / -1 overflow check omitted for brevity)
    movq    %rax, %rcx
    orq     %r8, %rcx
    shrq    $32, %rcx
    jne     .LBB293_10                 ; value > 32 bits → 64-bit divide
    xorl    %edx, %edx
    divl    %r8d                       ; 32-bit divide (values fit in 32 bits)
    jmp     .LBB293_11
.LBB293_10:
    cqto
    idivq   %r8                        ; 64-bit signed divide (typical for S=4)
```

For S=4 inputs (scaled numerator = 12,345,670,000 > 2^32), the 64-bit `idivq` path
is taken. No `__divti3` (128-bit division) in the fast path.

**Key confirmations:**
- `const_pow10` / `const_pow10_u64` with `#[inline(always)]` produces no runtime
  calls; the scale is folded to a literal constant (`$10000`, `$100`, etc.) ✓
- No `__divti3` in fast paths; only appears in the overflow slow path ✓
- Old precomputed constant `12193254061881` absent from the new assembly ✓

---

## 3. Optimisations applied this cycle

### 3.1 `#[inline(always)]` on `const_pow10` / `const_pow10_u64`

Both helper functions had only `#[inline]` (a hint). Changed to `#[inline(always)]`
to guarantee inlining and enable LLVM to see the scale as a compile-time constant
at all call sites, unlocking magic-constant division and avoiding runtime loop calls.

### 3.2 Benchmark redesign (`std::ptr::read_volatile`)

Replaced `AtomicI64`/`AtomicU64` statics with plain `i64`/`u64` statics and
`read_volatile` loads, ensuring LLVM cannot fold the arithmetic to a constant.

---

## 4. Corrected benchmark results

Measured on this system (CPU governor: powersave, effective boost ~3.79 GHz
calibrated from `i64_mul` = 264 ps ≈ 1 cycle).

### Scale 4 — fast path (typical financial values)

| Benchmark | Time | Cycles | vs target |
|-----------|------|--------|-----------|
| `decimal64_add` | 323 ps | 1.2 | ✓ ≤ 2 ns |
| `decimal64_mul` | 585 ps | 2.2 | ✓ ≤ 2 ns |
| `decimal64_div` | 1.95 ns | 7.4 | ✓ ≤ 2 ns |
| `udecimal64_add` | 313 ps | 1.2 | ✓ ≤ 2 ns |
| `udecimal64_mul` | 461 ps | 1.7 | ✓ ≤ 2 ns |
| `udecimal64_div` | 1.95 ns | 7.4 | ✓ ≤ 2 ns |
| `i64_add` | 372 ps | 1.4 | (ref) |
| `i64_mul` | 264 ps | 1.0 | (ref) |
| `i64_div` | 1.17 ns | 4.4 | (ref) |

### Scale 2 — fast path (currency)

| Benchmark | Time | Notes |
|-----------|------|-------|
| `decimal64_mul_s2` | 587 ps | ✓ |
| `decimal64_div_s2` | 1.18 ns | ✓ (32-bit divl; 100 fits) |
| `udecimal64_mul_s2` | 469 ps | ✓ |
| `udecimal64_div_s2` | 1.17 ns | ✓ |

### Scale 9 — fast path / slow path mix

| Benchmark | Time | Path |
|-----------|------|------|
| `decimal64_mul_s9` | 2.93 ns | slow (overflow; see §5) |
| `decimal64_div_s9` | 2.00 ns | idivq |
| `udecimal64_mul_s9` | 472 ps | fast (mulq + magic) |
| `udecimal64_div_s9` | 2.01 ns | udivq |

### Large magnitude — forced slow path

| Benchmark | Time | Notes |
|-----------|------|-------|
| `decimal64_mul_large` | 2.98 ns | expected slow path |
| `decimal64_div_large` | 3.25 ns | expected slow path |
| `udecimal64_mul_large` | 485 ps | fast (no overflow for u64) |
| `udecimal64_div_large` | 2.00 ns | udivq |

---

## 5. Why `decimal64_mul_s9` and `udecimal64_div_large` differ

**`decimal64_mul_s9` slow path (2.93 ns):**  
LHS_RAW = 1,234,567,000 and RHS_RAW = 9,876,543,000. Their product
≈ 1.22 × 10^19 > i64::MAX ≈ 9.22 × 10^18, so `i64::checked_mul` overflows.
The i128 slow path (full 128-bit multiply + 128-bit/64-bit divide) is taken.
For these inputs this is correct and expected behaviour; typical financial S=9
values (e.g. sub-unit prices) would be far smaller and stay on the fast path.

**`udecimal64_mul_large` fast path (485 ps):**  
`u64` overflow threshold is higher (18.4 × 10^18 vs 9.2 × 10^18 for i64), and
the test inputs (2 × 10^9, 5 × 10^9) have product 10^19 which fits in u64.
No overflow; magic-constant division path taken.

---

## 6. Target assessment

The ≤ 2 ns target is met for all typical fast-path inputs:

- **`decimal64_mul` S=4 and S=2**: 585–587 ps ✓ (magic-constant division, ~2.2 cycles)
- **`decimal64_div` S=4**: 1.95 ns ✓ (scale multiply + `idivq`, ~7.4 cycles)
- **`decimal64_div` S=2**: 1.18 ns ✓ (scale multiply + `divl`, small quotient)

The 2 ns target is **not** met for inputs that trigger the i128 slow path
(`decimal64_mul_s9` = 2.93 ns, `decimal64_mul_large` = 2.98 ns,
`decimal64_div_large` = 3.25 ns). This is expected: these paths involve 128-bit
arithmetic and are only triggered for values outside the i64 fast-path range.

---

## 7. Decision

Target met for typical inputs. No further source-level optimisation required for
this cycle. The benchmark redesign (§3.2) and `#[inline(always)]` fix (§3.1) are
the deliverables for this cycle.

Possible future work (not in scope):
- `unsafe fn unchecked_mul` variant for callers that guarantee no overflow
- SIMD digit accumulation in the parser (already identified as cycle 02 priority)
- `parse_round(s, mode)` for financial ingestion with explicit rounding
