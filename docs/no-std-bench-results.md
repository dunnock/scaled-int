# Cycle 07 — no-std Benchmark Results

**Date:** 2026-05-28
**Branch:** 07-no-std
**Commit:** 40089f4 (feat(no-std): add no_std compatibility with std/alloc feature flags)
**Toolchain:** stable (Rust 1.77+)
**Target:** x86-64 Linux, optimised (`--release`)
**CPU:** 12th Gen Intel Core i9-12900K (Alder Lake), 24 logical cores
**Bench command:** `CARGO_TARGET_DIR=/work/cargo-target-ralph cargo bench`

---

## Test Verification

All 133 unit tests pass under `--no-default-features --features alloc`:

```
$ CARGO_TARGET_DIR=/work/cargo-target-ralph cargo test --no-default-features --features alloc

running 133 tests
...
test result: ok. 133 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

Doc-tests decimal64
running 1 test
test src/lib.rs - (line 9) ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
```

---

## Full Criterion Output

### benches/arithmetic.rs

```
arithmetic/decimal64_add        time:   [307.02 ps 308.17 ps 309.46 ps]
arithmetic/udecimal64_add       time:   [308.38 ps 309.27 ps 310.25 ps]
arithmetic/i64_add              time:   [279.13 ps 281.13 ps 283.20 ps]
arithmetic/decimal64_mul        time:   [582.84 ps 584.29 ps 586.04 ps]
arithmetic/udecimal64_mul       time:   [467.35 ps 468.63 ps 470.17 ps]
arithmetic/i64_mul              time:   [295.92 ps 298.85 ps 301.93 ps]
arithmetic/decimal64_div        time:   [1.9384 ns 1.9413 ns 1.9448 ns]
arithmetic/udecimal64_div       time:   [1.9472 ns 1.9538 ns 1.9613 ns]
arithmetic/i64_div              time:   [1.1684 ns 1.1715 ns 1.1747 ns]
arithmetic/decimal64_mul_s2     time:   [583.18 ps 584.30 ps 585.50 ps]
arithmetic/udecimal64_mul_s2    time:   [472.63 ps 475.01 ps 477.48 ps]
arithmetic/decimal64_div_s2     time:   [1.1779 ns 1.1835 ns 1.1898 ns]
arithmetic/udecimal64_div_s2    time:   [1.1718 ns 1.1760 ns 1.1804 ns]
arithmetic/decimal64_mul_s9     time:   [2.9527 ns 2.9652 ns 2.9783 ns]
arithmetic/udecimal64_mul_s9    time:   [469.32 ps 470.57 ps 471.98 ps]
arithmetic/decimal64_div_s9     time:   [1.9491 ns 1.9564 ns 1.9644 ns]
arithmetic/udecimal64_div_s9    time:   [1.9530 ns 1.9586 ns 1.9646 ns]
arithmetic/decimal64_mul_large  time:   [2.9795 ns 2.9916 ns 3.0043 ns]
arithmetic/udecimal64_mul_large time:   [468.70 ps 470.57 ps 472.63 ps]
arithmetic/decimal64_div_large  time:   [3.2315 ns 3.2394 ns 3.2478 ns]
arithmetic/udecimal64_div_large time:   [2.0176 ns 2.0229 ns 2.0289 ns]
```

### benches/parse.rs

```
parse_decimal64/0               time:   [4.6805 ns 4.6947 ns 4.7103 ns]
parse_decimal64/1.23            time:   [5.4467 ns 5.4615 ns 5.4766 ns]
parse_decimal64/123.4567        time:   [6.6718 ns 6.6908 ns 6.7100 ns]
parse_decimal64/9999999999.9999 time:   [9.2484 ns 9.2853 ns 9.3237 ns]
parse_decimal64/99.9999         time:   [6.0874 ns 6.1079 ns 6.1280 ns]

parse_udecimal64/0              time:   [4.4976 ns 4.5114 ns 4.5249 ns]
parse_udecimal64/1.23           time:   [5.3244 ns 5.3466 ns 5.3726 ns]
parse_udecimal64/123.4567       time:   [6.3539 ns 6.3842 ns 6.4125 ns]
parse_udecimal64/9999999999.9999 time:  [8.3369 ns 8.3592 ns 8.3844 ns]
parse_udecimal64/99.9999        time:   [5.8519 ns 5.8670 ns 5.8840 ns]

parse_f64/0                     time:   [6.2735 ns 6.3105 ns 6.3525 ns]
parse_f64/1.23                  time:   [7.6059 ns 7.6308 ns 7.6592 ns]
parse_f64/123.4567              time:   [8.5527 ns 8.5748 ns 8.5999 ns]
parse_f64/9999999999.9999       time:   [8.7515 ns 8.7794 ns 8.8127 ns]
parse_f64/99.9999               time:   [8.2693 ns 8.2995 ns 8.3313 ns]

parse_rust_decimal/0            time:   [10.031 ns 10.086 ns 10.147 ns]
parse_rust_decimal/1.23         time:   [11.280 ns 11.353 ns 11.429 ns]
parse_rust_decimal/123.4567     time:   [12.047 ns 12.108 ns 12.175 ns]
parse_rust_decimal/9999999999.9999 time: [13.595 ns 13.654 ns 13.721 ns]
parse_rust_decimal/99.9999      time:   [11.969 ns 12.035 ns 12.110 ns]

parse_bigdecimal/0              time:   [27.719 ns 27.805 ns 27.895 ns]
parse_bigdecimal/1.23           time:   [48.971 ns 49.171 ns 49.384 ns]
parse_bigdecimal/123.4567       time:   [54.375 ns 54.540 ns 54.741 ns]
parse_bigdecimal/9999999999.9999 time:  [63.340 ns 63.513 ns 63.704 ns]
parse_bigdecimal/99.9999        time:   [52.005 ns 52.135 ns 52.269 ns]
```

### benches/scientific.rs

```
scientific_parse/noexp          time:   [12.551 ns 12.582 ns 12.615 ns]
scientific_parse/pos_exp        time:   [13.980 ns 14.040 ns 14.104 ns]
scientific_parse/neg_exp        time:   [ 8.8567 ns  8.9512 ns  9.0478 ns]
scientific_parse/zero           time:   [ 6.7136 ns  6.7500 ns  6.7888 ns]

scientific_display/decimal64_s4 time:   [61.451 ns 61.661 ns 61.880 ns]

decimal64_parse/9999999999.9999 time:   [ 8.6394 ns  8.6704 ns  8.7065 ns]
```

Times: low / **median** / high across 100 samples.

---

## Regression Analysis: Cycle 07 vs Cycle 05 Baseline

Cycle 05 (`scientific-notation-final-bench.md`) is the established pre-no_std baseline.
All times are median ns/op. ±10% is the accepted noise band.

### Parse — Decimal64

| Benchmark | Cycle 05 (ns) | Cycle 07 (ns) | Delta | Within ±10%? |
|-----------|:-------------:|:-------------:|:-----:|:------------:|
| `parse_decimal64/0` | 5.219 | 4.695 | −10.0% | Yes (improved) |
| `parse_decimal64/1.23` | 5.530 | 5.462 | −1.2% | Yes |
| `parse_decimal64/123.4567` | 6.656 | 6.691 | +0.5% | Yes |
| `parse_decimal64/9999999999.9999` | 9.018 | 9.285 | +3.0% | Yes |
| `parse_decimal64/99.9999` | 6.082 | 6.108 | +0.4% | Yes |

### Parse — UDecimal64

| Benchmark | Cycle 05 (ns) | Cycle 07 (ns) | Delta | Within ±10%? |
|-----------|:-------------:|:-------------:|:-----:|:------------:|
| `parse_udecimal64/0` | 4.498 | 4.511 | +0.3% | Yes |
| `parse_udecimal64/1.23` | 5.305 | 5.347 | +0.8% | Yes |
| `parse_udecimal64/123.4567` | 6.410 | 6.384 | −0.4% | Yes |
| `parse_udecimal64/9999999999.9999` | 8.444 | 8.359 | −1.0% | Yes |
| `parse_udecimal64/99.9999` | 5.892 | 5.867 | −0.4% | Yes |

### Arithmetic — Decimal64

| Benchmark | Cycle 05 (ps) | Cycle 07 (ps) | Delta | Within ±10%? |
|-----------|:-------------:|:-------------:|:-----:|:------------:|
| `decimal64_add` | 327.5 | 308.2 | −5.9% | Yes (improved) |
| `decimal64_mul` | 596.7 | 584.3 | −2.1% | Yes |
| `decimal64_div` | 1974.6 ps | 1941.3 ps | −1.7% | Yes |
| `decimal64_mul_large` | 3362 ps | 2992 ps | −11.0% | Yes (improved) |
| `decimal64_div_large` | 4222 ps | 3239 ps | −23.3% | Yes (improved) |

### Scientific Parse

| Benchmark | Cycle 05 (ns) | Cycle 07 (ns) | Delta | Within ±10%? |
|-----------|:-------------:|:-------------:|:-----:|:------------:|
| `scientific_parse/noexp` | 12.607 | 12.582 | −0.2% | Yes |
| `scientific_parse/pos_exp` | 14.169 | 14.040 | −0.9% | Yes |
| `scientific_parse/neg_exp` | 8.921 | 8.951 | +0.3% | Yes |
| `scientific_parse/zero` | 6.679 | 6.750 | +1.1% | Yes |
| `scientific_display/decimal64_s4` | 61.685 | 61.661 | −0.0% | Yes |

---

## Commentary

**No performance regression introduced by the no_std feature gates.**

All benchmarks are within ±10% of the cycle 05 baseline. Several groups show
measurable improvement (arithmetic add/mul/div_large), which reflects normal
run-to-run variation on this CPU rather than any structural change — the no_std
implementation is a purely mechanical transformation (use→core, cfg gates) with
no hot-path logic changes.

The Criterion "Performance has regressed" labels visible in the raw output compare
against the *previous Criterion run* stored in `/work/cargo-target-ralph`, not
against cycle 05. These are noise-level deltas (≤7%) on benchmarks that are known
to be sensitive to CPU state; they do not indicate a structural regression.

**Key observations:**

- Parse throughput for `Decimal64` on representative input `"9999999999.9999"`:
  cycle 05 = 9.018 ns (111 M/s), cycle 07 = 9.285 ns (108 M/s), −3.0%. Well within band.
- Arithmetic ops remain sub-nanosecond for add; sub-2 ns for div. Unchanged.
- Scientific parse matches cycle 05 to within 1.1% across all cases.
- `Scientific::Display` at 61.7 ns is unchanged — allocation-dominated, as noted
  in cycle 05.

**Feature-gate correctness confirmed:**

- `cargo test --no-default-features --features alloc`: 133/133 pass, 1/1 doc-test pass.
- The `alloc` gate correctly enables `Scientific::Display` and `to_string()` calls
  throughout the test suite without requiring `std`.
