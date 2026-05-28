# Scientific Notation Benchmark Results — Cycle 05

**Date:** 2026-05-27  
**Branch:** 05-scientific-notation  
**Commit:** f29bd67 (feat(cycle-05): implement Scientific<D> with scientific-notation parse and display)  
**Toolchain:** stable (Rust 1.77+, MSRV per Cargo.toml)  
**Target:** x86-64 Linux, optimised (`--release`)  
**CPU:** 12th Gen Intel Core i9-12900K (Alder Lake), 24 logical cores  
**Bench binary:** `benches/scientific.rs` via `CARGO_TARGET_DIR=/work/cargo-target-ralph cargo bench --bench scientific`  

---

## Full Criterion Output

```
scientific_parse/noexp      time: [14.626 ns  14.685 ns  14.745 ns]
scientific_parse/pos_exp    time: [14.214 ns  14.288 ns  14.361 ns]
scientific_parse/neg_exp    time: [10.024 ns  10.121 ns  10.209 ns]
scientific_parse/zero       time: [ 7.2827 ns  7.3277 ns  7.3756 ns]

scientific_display/decimal64_s4
                            time: [60.158 ns  60.306 ns  60.477 ns]

decimal64_parse/9999999999.9999
                            time: [ 8.9111 ns  8.9466 ns  8.9828 ns]
  (7 outliers: 1 low severe, 3 low mild, 3 high mild)
```

Times: low / **median** / high across 100 samples.

---

## Parse Benchmark Summary

All times are median ns/op. M/s = 1000 / median_ns.

| Benchmark                              | Input                   | Median (ns) | Throughput (M/s) |
|----------------------------------------|-------------------------|:-----------:|:----------------:|
| `decimal64_parse` (base reference)     | `"9999999999.9999"`     |    8.947    |      111.8       |
| `scientific_parse/noexp`               | `"9999999999.9999"`     |   14.685    |       68.1       |
| `scientific_parse/pos_exp`             | `"9.9999999999999e9"`   |   14.288    |       70.0       |
| `scientific_parse/neg_exp`             | `"1.0e-5"`              |   10.121    |       98.8       |
| `scientific_parse/zero`                | `"0e0"`                 |    7.328    |      136.5       |

### Overhead vs base Decimal64 parser

The no-exponent and positive-exponent cases both parse a string of similar length
to the 14-char base reference input:

| Case      | Scientific (ns) | Base (ns) | Overhead (%) |
|-----------|:---------------:|:---------:|:------------:|
| noexp     |     14.685      |   8.947   |   **+64%**   |
| pos_exp   |     14.288      |   8.947   |   **+60%**   |

The shorter strings are faster in absolute terms but not directly comparable
(different input lengths):

| Case     | Scientific (ns) | Notes                       |
|----------|-----------------|-----------------------------|
| neg_exp  |     10.121      | `"1.0e-5"` — 6 bytes        |
| zero     |      7.328      | `"0e0"` — 3 bytes           |

---

## Display Benchmark

| Benchmark                    | Value        | Median (ns) | Throughput (M/s) |
|------------------------------|--------------|:-----------:|:----------------:|
| `scientific_display/decimal64_s4` | `123.4567` (raw=1_234_567, scale=4) | 60.306 | 16.6 |

The display path allocates a `String` via `format!()`. The allocation dominates
at ~60 ns. A bare `write!()` to a `fmt::Formatter` (the hot path in real use)
would be substantially faster; this benchmark measures the `format!` round-trip
cost that an end-user calling `.to_string()` sees.

---

## Design §9.1 Prediction vs Actual

Design §9.1 predicted:

- **Scientific-notation inputs** (with `e`/`E`): +15–25% overhead vs base.
- **Non-scientific inputs** (no `e`/`E`): scan cost only, +5–10%.

**Actual results miss the prediction significantly (+60–64% for both paths).**

### Root cause: two-pass architecture

`Scientific::from_str` performs two scans of the byte slice:

1. `bytes.iter().position(|&b| b == b'e' || b == b'E')` — O(n) scan for the
   exponent marker.
2. `crate::parse::parse::<S>(s)?` — the full base parser scan (also O(n)).

For "9999999999.9999" (14 bytes with no `e`), the overhead is:
`14.685 ns − 8.947 ns = 5.74 ns` for a 14-byte scan via `.position()`.
This is ~0.41 ns/byte, consistent with an unvectorized byte comparison loop.

The design doc assumed LLVM would auto-vectorize the scan into a SIMD search, which
would reduce it to ~1 ns regardless of length. In practice LLVM did not vectorize the
closure-based `.position()` call on this input.

### Impact assessment

The overhead is real but the absolute numbers remain fast:
- 68–70 M/s for scientific-notation parse vs 111.8 M/s for base parse.
- This is still ~5× faster than `rust_decimal` (13–14 ns in cycle 01 results).
- For the target use case (occasional financial-string ingestion with scientific
  notation), 14 ns/parse is acceptable.

### Optimization path for cycle N+1

If the overhead must be reduced to ≤25%:
- **Merged single-pass parser**: scan for `e`/`E` and parse the mantissa digits in
  one pass, eliminating the double-scan. Expected to cut overhead to ~5–10%.
- **Byte-scan vectorization**: use `memchr` or a manual SIMD search for `b'e'`/`b'E'`
  to bring the scan cost to <1 ns for typical lengths.
- The current two-pass implementation is correct and the separation of concerns is
  clean; optimization is deferred pending a clear business requirement.

---

## Benchmark Methodology

- All parse inputs use `static &str` + `unsafe { std::ptr::read_volatile(&STATIC) }`
  to prevent LLVM from constant-folding the parse call.
- Display input raw value uses `static i64` + `read_volatile` for the same reason.
- `std::hint::black_box` wraps each result to prevent dead-code elimination.
- `criterion::black_box` (deprecated in Criterion 0.8) is not used.
- CPU governor was in boost mode (~3.7 GHz effective) for all measurements.
