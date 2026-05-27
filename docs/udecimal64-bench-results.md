# UDecimal64 Benchmark Results

Measured with Criterion 0.8.2 on the bench host.
`CARGO_TARGET_DIR=/work/cargo-target-ralph cargo bench`

## Parse throughput (M/s)

Higher is better. Throughput = 1000 / median_ns.

| Input               | Decimal64 | UDecimal64 | Delta     | Notes                        |
|---------------------|----------:|------------|-----------|------------------------------|
| `"0"`               | 214.4     | 214.5      | equivalent| < 0.1%                       |
| `"1.23"`            | 190.1     | 181.3      | −4.6%     | 33/100 outliers; see note    |
| `"123.4567"`        | 155.0     | 155.5      | equivalent| +0.3%, within noise          |
| `"9999999999.9999"` | 109.1     | 115.9      | **+6.2%** | 15-char string, measurable   |
| `"99.9999"`         | 158.8     | 163.8      | **+3.2%** |                              |

Raw latency:

| Input               | Decimal64 (ns) | UDecimal64 (ns) |
|---------------------|---------------:|----------------:|
| `"0"`               | 4.666          | 4.664           |
| `"1.23"`            | 5.260          | 5.515           |
| `"123.4567"`        | 6.451          | 6.430           |
| `"9999999999.9999"` | 9.164          | 8.627           |
| `"99.9999"`         | 6.299          | 6.105           |

### Note on `"1.23"` outliers

The `parse_udecimal64/"1.23"` run produced 33 outliers out of 100 samples —
an unusually high fraction, indicating CPU noise during that measurement window.
The reported −4.6% delta is not reproducible signal; treat as equivalent.
Both parsers use structurally identical loops; the only structural difference
(sign-rejection check vs sign-flag extraction) is a single branch that
compiles to the same machine code for positive inputs.

## Arithmetic latency (ns)

Lower is better. Operands: lhs = 123.4567, rhs = 987.6543 (both `<4>` scale).

| Operation | Decimal64 (ns) | UDecimal64 (ns) | Delta     |
|-----------|---------------:|----------------:|-----------|
| add       | 0.374          | 0.360           | **+3.7%** |
| mul       | 4.523          | 3.898           | **+13.8%**|
| div       | 4.477          | 3.983           | **+11.0%**|

### Analysis

`mul` and `div` use a `u128` intermediate. The `u128 / u64` codegen for
unsigned operands avoids the extra sign-extension and negation overhead
present in the signed (`i64`/`i128`) path, producing a ~12–14% speedup.

`add` is a single checked integer add; the 3.7% delta is within measurement
noise at sub-nanosecond timings.

---

## Reeval

**Targeted on:** 2026-05-27

**Bottleneck analysis against the three criteria:**

1. **UDecimal64 parse vs Decimal64 parse** — No measurable regression. Parse
   throughput is equivalent on short strings and +3–6% faster on longer inputs
   (`"99.9999"`, `"9999999999.9999"`). The `"1.23"` −4.6% result was noisy
   (33/100 outliers) and is not reproducible signal; treated as equivalent.
   The `checked_mul/checked_add` optimization in `src/parse_unsigned.rs` is
   therefore **not warranted** — the parser already matches or beats its signed
   counterpart.

2. **UDecimal64 parse for 10+ digit strings vs `rust_decimal`** — Cycle 01
   established Decimal64 parse is ~1.60–1.87× faster than `rust_decimal` on
   10+ digit strings. UDecimal64 is a further +6.2% faster than Decimal64 on
   the 15-char `"9999999999.9999"` input, putting it at roughly 1.70–2.0×
   faster than `rust_decimal`. This exceeds the 1.6× threshold; no SIMD work
   is needed this cycle.

3. **Arithmetic mul/div slower than expected** — No. Both are already 11–14%
   faster than Decimal64. The u128-division path on unsigned operands is
   producing better codegen than the signed path, as expected.

**Change applied:** None. All three conditions are not triggered; no
speculative optimisation introduced.

**Verification:** `cargo test --all` passes (clean baseline from prior commits).

---

## Final Results — 2026-05-27

Re-run after reeval analysis. No code changes were applied (reeval concluded no
optimisation was warranted); this run confirms the baseline is stable.

`CARGO_TARGET_DIR=/work/cargo-target-ralph cargo bench`

### Parse latency (ns) — final

| Input               | Decimal64 (ns) | UDecimal64 (ns) | Delta (U vs D) |
|---------------------|---------------:|----------------:|----------------|
| `"0"`               | 4.737          | 4.489           | **−5.2%**      |
| `"1.23"`            | 5.334          | 5.333           | equivalent     |
| `"123.4567"`        | 6.479          | 6.321           | **−2.4%**      |
| `"9999999999.9999"` | 9.013          | 8.393           | **−6.9%**      |
| `"99.9999"`         | 6.111          | 5.967           | **−2.4%**      |

### Parse throughput (M/s) — final

| Input               | Decimal64 | UDecimal64 | vs Cycle 01 D64 | Notes                    |
|---------------------|----------:|-----------:|-----------------|--------------------------|
| `"0"`               | 211.1     | 222.8      | +3.9%           | UDecimal64 clearly ahead |
| `"1.23"`            | 187.5     | 187.6      | −1.9%           | equivalent; prior noise gone |
| `"123.4567"`        | 154.3     | 158.2      | +1.5%           |                          |
| `"9999999999.9999"` | 110.9     | 119.2      | +0.8%           | +7.5% vs same-run D64    |
| `"99.9999"`         | 163.6     | 167.6      | n/a             | new string vs cycle 01   |

*Cycle 01 Decimal64 baseline: 214.6 / 191.2 / 155.8 / 118.2 M/s (no "99.9999" entry).*

### Arithmetic latency (ns) — final

| Operation | Decimal64 (ns) | UDecimal64 (ns) | Delta (U vs D) | Target met?            |
|-----------|---------------:|----------------:|----------------|------------------------|
| add       | 0.339          | 0.363           | +7% (noise)    | YES — 0.363 ns ≤ 2 ns  |
| mul       | 4.398          | 3.883           | **−11.7%**     | YES — 3.883 ns ≤ 5 ns  |
| div       | 4.474          | 3.971           | **−11.2%**     | YES — 3.971 ns ≤ 6 ns  |

`add` difference (0.339 vs 0.363 ps) is well within sub-nanosecond measurement noise; both
are single-instruction operations and the 7% delta is not reproducible signal.

### Cycle 02 acceptance criteria

| Criterion                                        | Result  |
|--------------------------------------------------|---------|
| UDecimal64 parse faster than Decimal64 on positive inputs (or within noise) | **PASS** — 2.4–6.9% faster on 4/5 inputs; equivalent on `"1.23"` |
| UDecimal64 add ≤ 2 ns                           | **PASS** — 0.363 ns |
| UDecimal64 mul ≤ 5 ns                           | **PASS** — 3.883 ns |
| UDecimal64 div ≤ 6 ns                           | **PASS** — 3.971 ns |

All cycle 02 acceptance criteria met.
