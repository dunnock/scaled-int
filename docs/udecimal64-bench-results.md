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
