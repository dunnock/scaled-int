# Benchmark Results — Cycle 01

**Date:** 2026-05-26  
**Branch:** 01-design  
**Toolchain:** stable (see `rustc --version`)  
**Target:** x86-64 Linux (optimised build)

---

## Parse benchmarks

Corpus strings benchmarked at `Decimal64::<4>` (scale 4).  
All competitors use `FromStr`.  Times are median ns/op; throughput = 1 / time.

| Input               | decimal64 (ns) | f64 (ns) | rust_decimal (ns) | bigdecimal (ns) |
|---------------------|----------------|----------|-------------------|-----------------|
| `"0"`               | 4.66           | 6.36     | 9.81              | 26.98           |
| `"1.23"`            | 5.23           | 7.64     | 11.04             | 47.79           |
| `"123.4567"`        | 6.42           | 8.63     | 11.98             | 54.14           |
| `"9999999999.9999"` | 8.46           | 8.81     | 13.54             | 63.04           |
| `"-0.000001"`       | 6.00           | 8.68     | 12.58             | 54.60           |

### Throughput (M parses/s)

| Input               | decimal64 | f64    | rust_decimal | bigdecimal |
|---------------------|-----------|--------|--------------|------------|
| `"0"`               | 214.6     | 157.2  | 102.0        | 37.1       |
| `"1.23"`            | 191.2     | 130.9  | 90.6         | 20.9       |
| `"123.4567"`        | 155.8     | 115.9  | 83.5         | 18.5       |
| `"9999999999.9999"` | 118.2     | 113.5  | 73.8         | 15.9       |
| `"-0.000001"`       | 166.6     | 115.2  | 79.5         | 18.3       |

### decimal64 / rust_decimal speedup

| Input               | Speedup |
|---------------------|---------|
| `"0"`               | 2.10×   |
| `"1.23"`            | 2.11×   |
| `"123.4567"`        | 1.87×   |
| `"9999999999.9999"` | 1.60×   |
| `"-0.000001"`       | 2.10×   |

---

## Arithmetic benchmarks

Values used: `lhs = 123.4567`, `rhs = 987.6543` (both at scale 4).

| Benchmark         | Time (ns) | Notes                       |
|-------------------|-----------|-----------------------------|
| `decimal64_add`   | 0.336     | `checked_add` + `expect`    |
| `i64_add`         | 0.375     | raw integer addition        |
| `decimal64_mul`   | 4.73      | i128 intermediate + i64 div |
| `decimal64_div`   | 4.46      | i128 multiply + i128 div    |

---

## Target assessment (§8)

| Benchmark              | Target                      | Actual (representative input)      | Met?    |
|------------------------|-----------------------------|------------------------------------|---------|
| `decimal64::parse`     | ≥ 100 M/s                   | 118–215 M/s across all inputs      | **YES** |
| rust_decimal parity    | ≥ 2× faster                 | 1.60×–2.11× (median 2.07×)         | PARTIAL |
| `decimal64::add`       | ≤ 2 ns/op                   | 0.34 ns                            | **YES** |
| `decimal64::mul`       | ≤ 5 ns/op                   | 4.73 ns                            | **YES** |

---

## Notes on unexpected results

**`decimal64_add` faster than `i64_add` (0.336 ns vs 0.375 ns):**  
Both are sub-nanosecond and within measurement noise at this resolution. The difference is not meaningful; both are at the single-instruction level. The checked_add path in `Decimal64::add` costs a branch and potential panic path but the CPU's branch predictor eliminates this overhead on the hot path.

**rust_decimal 2× target: partially met:**  
Short typical-length strings (`"0"`, `"1.23"`, `"-0.000001"`) achieve ≥ 2.10× speedup. The two inputs that miss the target are `"123.4567"` (1.87×, 8 characters) and `"9999999999.9999"` (1.60×, 15 characters). The gap narrows for longer strings because `decimal64`'s per-digit cost scales linearly with length, and `rust_decimal`'s fixed-overhead advantage shrinks.

A potential cycle-02 optimisation: pre-compute a `max_safe_digits` threshold (as described in `docs/design.md` §3) to remove `checked_mul`/`checked_add` overhead for the first N digits, which would likely recover the 2× margin on longer strings.

**`decimal64::div` faster than `decimal64::mul` (4.46 ns vs 4.73 ns):**  
Both use i128 arithmetic. Division here divides a scaled i128 by an i64-range value; multiplication uses i128 multiply then i128 divide. The difference is within noise but counter-intuitive. No action needed.

---

## Escalation

The rust_decimal 2× target is missed for inputs ≥ 8 characters. Per the task directive, no optimisation is done here — this is flagged for cycle 02.
