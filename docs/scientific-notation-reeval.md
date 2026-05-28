# Scientific Notation — Reeval and Improvement — Cycle 05

**Date:** 2026-05-28  
**Branch:** 05-scientific-notation  
**Toolchain:** stable (Rust 1.77+, MSRV per Cargo.toml)  
**CPU:** 12th Gen Intel Core i9-12900K (Alder Lake)

---

## 1. Measured vs Predicted Overhead

Design §9.1 predicted:

| Path               | Predicted overhead |
|--------------------|:-----------------:|
| With exponent      | +15–25%           |
| Without exponent   | +5–10%            |

### Baseline (cycle 05 benchmark run, pre-optimisation)

| Benchmark                          | Median (ns) | Overhead vs base |
|------------------------------------|:-----------:|:----------------:|
| `decimal64_parse` (base reference) |    8.947    |        —         |
| `scientific_parse/noexp`           |   14.685    |    **+64%**      |
| `scientific_parse/pos_exp`         |   14.288    |    **+60%**      |
| `scientific_parse/neg_exp`         |   10.121    |    (shorter input)  |
| `scientific_parse/zero`            |    7.328    |    (shorter input)  |

Both the no-exponent and positive-exponent paths missed the ≤25% target significantly.

---

## 2. Root Cause of Excess Overhead

### Two-pass architecture

`Scientific::from_str` performs two sequential O(n) scans of the byte slice:

1. **Exponent scan** — `bytes.iter().position(|&b| b == b'e' || b == b'E')` scans all bytes.
2. **Base parse** — `crate::parse::parse::<S>(s)?` re-scans the same bytes.

For `"9999999999.9999"` (14 bytes, no `e`), the exponent scan added ~5.74 ns.
This was ~0.41 ns/byte, consistent with an unvectorized byte-by-byte comparison.

The design doc predicted LLVM would auto-vectorize the scan; in practice it did not for the closure-based `.position()` call on these short strings.

The positive-exponent overhead (+60%) has the same structure: after finding `e` at position 15 in `"9.9999999999999e9"` (17 bytes), both mantissa and exponent substrings are parsed separately, resulting in a full second pass.

---

## 3. Optimisations Applied

Two of the three task options were applicable and were applied.

### Option (a) — OR-mask scan (`find_exp_marker`)

Replaced the closure-based `.iter().position(|&b| b == b'e' || b == b'E')` call
with a named helper:

```rust
#[inline(always)]
fn find_exp_marker(bytes: &[u8]) -> Option<usize> {
    for (i, &b) in bytes.iter().enumerate() {
        if b | 0x20 == b'e' {
            return Some(i);
        }
    }
    None
}
```

The OR-mask trick (`b | 0x20 == b'e'`) covers both `b'e'` (0x65) and `b'E'` (0x45)
with a single comparison per byte (OR + compare vs two compares + OR).
This form is more amenable to LLVM auto-vectorization than a two-equality closure.

### Option (b) — `#[inline(always)]` on helper functions

Added `#[inline(always)]` to all private helpers:

- `find_exp_marker`
- `pow10_i64`, `pow10_u64`
- `parse_exponent`
- `apply_exponent_i64`, `apply_exponent_u64`
- `apply_positive_exp_i64`, `apply_negative_exp_i64`
- `apply_positive_exp_u64`, `apply_negative_exp_u64`

This guarantees the optimizer sees the full call context from `from_str` through
exponent parsing and application in one inlined body.

### Option (c) — pow10 table

Already implemented in the original code. Both `pow10_i64` and `pow10_u64` use
compile-time constant `TABLE` arrays; no changes required.

---

## 4. Updated Benchmark Results (post-optimisation)

```
scientific_parse/noexp      time: [12.728 ns  12.768 ns  12.810 ns]  change: −12.4%
scientific_parse/pos_exp    time: [14.197 ns  14.268 ns  14.343 ns]  change:  +0.0% (no change)
scientific_parse/neg_exp    time: [ 8.8731 ns  8.9578 ns  9.0419 ns]  change:  −9.5%
scientific_parse/zero       time: [ 6.7788 ns  6.8103 ns  6.8432 ns]  change:  −7.5%

decimal64_parse/9999999999.9999
                            time: [ 8.6853 ns  8.7097 ns  8.7343 ns]  (reference)
```

### Overhead table (post-optimisation, using fresh base measurement)

| Case      | Scientific (ns) | Base (ns) | Overhead (%) | Change from baseline |
|-----------|:---------------:|:---------:|:------------:|:--------------------:|
| noexp     |     12.768      |   8.710   |   **+47%**   | −17 pp (was +64%)    |
| pos_exp   |     14.268      |   8.710   |   **+64%**   | unchanged            |

Both paths still exceed the ≤25% target.

---

## 5. Why the Target Was Not Met

The inline and scan-form improvements reduced the **noexp** overhead by 17 percentage
points (from +64% to +47%). The remaining ~4 ns cost is the scan over 14 bytes at
~0.29 ns/byte — LLVM still does not auto-vectorize the loop even with the OR-mask form
on this short input length.

The **pos_exp** overhead is unchanged because the bottleneck is not the scan but the
**two-pass structure**: after finding `e`, both the mantissa (`"9.9999999999999"`,
15 chars) and exponent substrings are parsed in separate calls. The combined cost of
two O(n) traversals over a 17-byte input is nearly double that of one traversal, which
is why +60–64% overhead persists regardless of scan optimisation.

The design doc correctly identified the merged single-pass parser as the primary fix:

> **Merged single-pass parser**: scan for `e`/`E` and parse the mantissa digits in
> one pass, eliminating the double-scan. Expected to cut overhead to ~5–10%.

This approach was not applied in this cycle because it requires refactoring the base
parse logic (`src/parse.rs`) rather than the `Scientific` wrapper, which is a larger
change with broader blast radius.

---

## 6. Summary

| Item                          | Status                                             |
|-------------------------------|----------------------------------------------------|
| noexp overhead vs target      | +47% — above ≤10% target; improved from +64%       |
| pos_exp overhead vs target    | +64% — above ≤25% target; unchanged                |
| Options (a) and (b) applied   | Yes — inline annotations + OR-mask scan            |
| Option (c) (pow10 table)      | Already implemented; no changes needed             |
| All tests pass                | Yes — 144 tests pass                               |
| Recommended next step         | Merged single-pass parser (cycle N+1)              |

The absolute numbers remain fast enough for financial-string ingestion use cases:
68–78 M/s for scientific parse vs 115 M/s for base parse. The overhead is documented
and the path to eliminating it (single-pass merge) is clear.
