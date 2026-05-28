# Scientific Notation — Final Benchmark Results — Cycle 05

**Date:** 2026-05-28  
**Branch:** 05-scientific-notation  
**Commits:** 469fbc7 (OR-mask scan + inline(always) on scientific parse helpers)  
**Toolchain:** stable (Rust 1.77+, MSRV per Cargo.toml)  
**Target:** x86-64 Linux, optimised (`--release`)  
**CPU:** 12th Gen Intel Core i9-12900K (Alder Lake), 24 logical cores  
**Bench command:** `CARGO_TARGET_DIR=/work/cargo-target-ralph cargo bench`

---

## Full Criterion Output

### benches/arithmetic.rs

```
arithmetic/decimal64_add        time:   [325.91 ps 327.54 ps 329.16 ps]
arithmetic/udecimal64_add       time:   [316.31 ps 318.35 ps 320.40 ps]
arithmetic/i64_add              time:   [377.37 ps 381.45 ps 385.11 ps]
arithmetic/decimal64_mul        time:   [594.66 ps 596.70 ps 598.71 ps]
arithmetic/udecimal64_mul       time:   [476.18 ps 477.80 ps 479.31 ps]
arithmetic/i64_mul              time:   [267.33 ps 268.15 ps 269.03 ps]
arithmetic/decimal64_div        time:   [1.9675 ns 1.9746 ns 1.9818 ns]
arithmetic/udecimal64_div       time:   [1.9623 ns 1.9697 ns 1.9770 ns]
arithmetic/i64_div              time:   [1.1726 ns 1.1760 ns 1.1798 ns]
arithmetic/decimal64_mul_s2     time:   [590.23 ps 592.70 ps 595.32 ps]
arithmetic/udecimal64_mul_s2    time:   [472.25 ps 473.99 ps 475.69 ps]
arithmetic/decimal64_div_s2     time:   [1.1842 ns 1.1878 ns 1.1915 ns]
arithmetic/udecimal64_div_s2    time:   [1.1805 ns 1.1841 ns 1.1877 ns]
arithmetic/decimal64_mul_s9     time:   [2.9790 ns 2.9988 ns 3.0202 ns]
arithmetic/udecimal64_mul_s9    time:   [482.70 ps 484.28 ps 485.87 ps]
arithmetic/decimal64_div_s9     time:   [1.9643 ns 1.9709 ns 1.9784 ns]
arithmetic/udecimal64_div_s9    time:   [1.9833 ns 2.0396 ns 2.1061 ns]
arithmetic/decimal64_mul_large  time:   [3.2615 ns 3.3617 ns 3.4913 ns]
arithmetic/udecimal64_mul_large time:   [499.02 ps 504.25 ps 510.58 ps]
arithmetic/decimal64_div_large  time:   [4.1037 ns 4.2224 ns 4.3466 ns]
arithmetic/udecimal64_div_large time:   [2.0127 ns 2.0153 ns 2.0184 ns]
```

### benches/parse.rs

```
parse_decimal64/0               time:   [4.8011 ns 5.2188 ns 6.1344 ns]
parse_decimal64/1.23            time:   [5.5245 ns 5.5298 ns 5.5370 ns]
parse_decimal64/123.4567        time:   [6.6444 ns 6.6557 ns 6.6688 ns]
parse_decimal64/9999999999.9999 time:   [8.9549 ns 9.0179 ns 9.0808 ns]
parse_decimal64/99.9999         time:   [6.0601 ns 6.0817 ns 6.1054 ns]

parse_udecimal64/0              time:   [4.4847 ns 4.4984 ns 4.5136 ns]
parse_udecimal64/1.23           time:   [5.2891 ns 5.3053 ns 5.3224 ns]
parse_udecimal64/123.4567       time:   [6.3927 ns 6.4104 ns 6.4279 ns]
parse_udecimal64/9999999999.9999 time:  [8.4157 ns 8.4441 ns 8.4724 ns]
parse_udecimal64/99.9999        time:   [5.8707 ns 5.8915 ns 5.9129 ns]

parse_f64/0                     time:   [6.3224 ns 6.3563 ns 6.3931 ns]
parse_f64/1.23                  time:   [7.6175 ns 7.6427 ns 7.6717 ns]
parse_f64/123.4567              time:   [8.6400 ns 8.6662 ns 8.6947 ns]
parse_f64/9999999999.9999       time:   [8.7923 ns 8.8164 ns 8.8427 ns]
parse_f64/99.9999               time:   [8.2856 ns 8.3082 ns 8.3337 ns]

parse_rust_decimal/0            time:   [9.9071 ns 9.9883 ns 10.077  ns]
parse_rust_decimal/1.23         time:   [11.070 ns 11.124 ns 11.179  ns]
parse_rust_decimal/123.4567     time:   [11.976 ns 12.048 ns 12.127  ns]
parse_rust_decimal/9999999999.9999 time: [13.544 ns 13.615 ns 13.690 ns]
parse_rust_decimal/99.9999      time:   [11.854 ns 11.922 ns 11.994  ns]

parse_bigdecimal/0              time:   [27.893 ns 28.016 ns 28.143 ns]
parse_bigdecimal/1.23           time:   [48.106 ns 48.253 ns 48.402 ns]
parse_bigdecimal/123.4567       time:   [54.215 ns 54.433 ns 54.661 ns]
parse_bigdecimal/9999999999.9999 time:  [63.495 ns 63.750 ns 64.023 ns]
parse_bigdecimal/99.9999        time:   [52.147 ns 52.319 ns 52.507 ns]
```

### benches/scientific.rs

```
scientific_parse/noexp      time:   [12.580 ns 12.607 ns 12.639 ns]
scientific_parse/pos_exp    time:   [14.121 ns 14.169 ns 14.220 ns]
scientific_parse/neg_exp    time:   [ 8.8064 ns  8.9209 ns  9.0400 ns]
scientific_parse/zero       time:   [ 6.6510 ns  6.6791 ns  6.7081 ns]

scientific_display/decimal64_s4
                            time:   [61.469 ns 61.685 ns 61.904 ns]

decimal64_parse/9999999999.9999
                            time:   [ 8.6268 ns  8.6628 ns  8.6983 ns]
```

Times: low / **median** / high across 100 samples.

---

## Three-Way Comparison: Base | Initial Scientific | Final Scientific

All times are median ns/op. Comparison input: `"9999999999.9999"` (14 chars).

| Path      | Base `Decimal64` parse | Initial `Scientific` parse | Final `Scientific` parse | Initial overhead | Final overhead | Delta |
|-----------|:---------------------:|:-------------------------:|:------------------------:|:----------------:|:--------------:|:-----:|
| **noexp** |       8.663 ns        |         14.685 ns         |        12.607 ns         |    **+64%**      |   **+46%**     | −18 pp |
| **pos_exp** |     8.663 ns        |         14.288 ns         |        14.169 ns         |    **+60%**      |   **+64%**     | −0 pp (noise) |

Notes:
- Base reference (`decimal64_parse/9999999999.9999`) measured in same bench run: **8.663 ns**.
- Initial results from commit f29bd67 (pre-optimisation run, 2026-05-27).
- Final results from commit 469fbc7 (post-optimisation run, 2026-05-28).
- `pos_exp` input is `"9.9999999999999e9"` (17 chars) — slightly longer than base input.

### Shorter inputs (not directly comparable to base)

| Case     | Initial (ns) | Final (ns) | Change |
|----------|:------------:|:----------:|:------:|
| neg_exp  |    10.121    |    8.921   | −11.9% |
| zero     |     7.328    |    6.679   |  −8.9% |

---

## Final Overhead vs ≤25% Target

| Path    | Final overhead | Target | Met? |
|---------|:--------------:|:------:|:----:|
| noexp   |    **+46%**    | ≤10%   | No   |
| pos_exp |    **+64%**    | ≤25%   | No   |

**The ≤25% overhead target was not met.**

The two optimisations applied in cycle 05 (OR-mask scan, `#[inline(always)]` on helpers)
reduced `noexp` overhead from +64% to +46% (−18 percentage points). The `pos_exp` path
was not improved because the bottleneck is the two-pass architecture, not the scan cost:

- **noexp** remaining cost: ~4 ns for a 14-byte scan at ~0.29 ns/byte. LLVM does not
  auto-vectorize the short loop even with the OR-mask form.
- **pos_exp** is unchanged: after finding `e`, the 15-char mantissa and exponent are
  parsed in two separate calls over a 17-byte input; the combined cost of two O(n)
  traversals drives the +64% overhead regardless of scan optimisation.

The path to eliminating both bottlenecks is the **merged single-pass parser** identified
in the design doc (§9.1) and reeval doc (§5): scan for `e`/`E` and parse the mantissa
in one pass. This requires modifying `src/parse.rs` and is deferred to cycle N+1.

---

## Absolute Performance Context

| Metric                           | Value      |
|----------------------------------|:----------:|
| Scientific parse throughput (noexp) | 79.3 M/s |
| Scientific parse throughput (pos_exp) | 70.6 M/s |
| Base Decimal64 parse throughput  | 115.4 M/s  |
| rust_decimal parse throughput (`"9999999999.9999"`) | 73.4 M/s |
| bigdecimal parse throughput (`"9999999999.9999"`)   | 15.7 M/s |

`Scientific` parse is **faster than `rust_decimal`** on the same 14-char input
(79.3 vs 73.4 M/s), and 4× faster than `bigdecimal`. The overhead vs `Decimal64`
base is real but does not prevent practical use for financial-string ingestion.

---

## Display Benchmark

| Benchmark                    | Value        | Final (ns) |
|------------------------------|:------------:|:----------:|
| `scientific_display/decimal64_s4` | `123.4567` → `"1.234567e2"` | 61.685 ns |

Allocation-dominated (the `format!` round-trip includes a `String` heap alloc).
Direct `write!` to a `fmt::Formatter` would be substantially faster.

---

## Benchmark Methodology

- All parse inputs use `static &str` + `unsafe { std::ptr::read_volatile(&STATIC) }`
  to prevent LLVM from constant-folding the parse call.
- Display input raw value uses `static i64` + `read_volatile` for the same reason.
- `std::hint::black_box` wraps each result to prevent dead-code elimination.
- `criterion::black_box` (deprecated in Criterion 0.8) is not used.
- CPU governor was in boost mode (~3.7 GHz effective) for all measurements.
- 100 samples per benchmark, Criterion default warmup (3 s).
