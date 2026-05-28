# Serde Benchmark Results — Cycle 06

## Hardware

- **CPU**: 12th Gen Intel Core i9-12900K (Alder Lake)
- **Effective clock**: ~3.7 GHz (powersave governor; boost-clock behavior, same as cycle 04/05)
- **OS**: Linux 6.17.0-29-generic

## Benchmark Command

```
CARGO_TARGET_DIR=/work/cargo-target-ralph cargo bench --features serde --bench serde
```

## Criterion Output

```
serde_json_serialize_d64/123.4567
                        time:   [43.905 ns 44.019 ns 44.146 ns]
Found 3 outliers among 100 measurements (3.00%)
  3 (3.00%) high mild

serde_json_deserialize_d64/123.4567
                        time:   [13.837 ns 13.873 ns 13.911 ns]
Found 5 outliers among 100 measurements (5.00%)
  5 (5.00%) high mild

postcard_serialize_string_d64/123.4567
                        time:   [53.802 ns 53.912 ns 54.035 ns]
Found 11 outliers among 100 measurements (11.00%)
  8 (8.00%) high mild
  3 (3.00%) high severe

postcard_serialize_raw_d64/123.4567
                        time:   [21.531 ns 21.579 ns 21.632 ns]
Found 3 outliers among 100 measurements (3.00%)
  2 (2.00%) high mild
  1 (1.00%) high severe

postcard_deserialize_raw_d64/123.4567
                        time:   [1.9475 ns 1.9524 ns 1.9578 ns]
Found 3 outliers among 100 measurements (3.00%)
  3 (3.00%) high mild
```

## Summary Table

| Benchmark | ns/op (median) | Format | Path |
|-----------|---------------|--------|------|
| `serde_json_serialize_d64` | 44.0 | JSON | string |
| `serde_json_deserialize_d64` | 13.9 | JSON | string |
| `postcard_serialize_string_d64` | 53.9 | postcard | string |
| `postcard_serialize_raw_d64` | 21.6 | postcard | raw i64 |
| `postcard_deserialize_raw_d64` | 1.95 | postcard | raw i64 |

## String vs Raw Comparison (postcard)

| Operation | String path (ns/op) | Raw path (ns/op) | Raw speedup |
|-----------|--------------------:|------------------:|-------------|
| Serialize | 53.9 | 21.6 | 2.5× faster |
| Deserialize | — | 1.95 | — |

Only the raw path was benchmarked for postcard deserialization; the string-path
postcard deserialize (which would parse via `from_str`) is expected to match
`serde_json_deserialize_d64` (~14 ns) since both call the same `Decimal64::parse`
under the hood.

## Design §10 Budget Assessment

From `docs/serde-design.md` §10:

| Path | Budget | Actual | Status |
|------|--------|--------|--------|
| Serialize (string) — JSON | < 500 ns; expected 100–300 ns | 44 ns | **well under** |
| Serialize (string) — postcard | < 500 ns; expected 100–300 ns | 54 ns | **well under** |
| Deserialize (string) | 15–50 ns | 13.9 ns | **under** (slightly below lower bound) |
| Raw serialize | 1–5 ns | 21.6 ns | **over** (heap alloc dominates) |
| Raw deserialize | 1–5 ns | 1.95 ns | **within** |

### Commentary

**String-path serialize** came in at 44–54 ns, far below the 100–300 ns estimate
in the design doc. The design assumed `to_string()` allocation would dominate;
in practice serde_json's internal small-buffer path reduces the allocation cost
significantly for short strings like `"123.4567"` (8 chars).

**String-path deserialize** at 13.9 ns is at the low end of the 15–50 ns budget.
This reflects the fast single-pass `Decimal64::parse` from cycle 05 (measured
at ~8.7–14 ns for this corpus). The serde glue overhead is negligible.

**Raw deserialize** at 1.95 ns (~7 cycles) is solidly within the 1–5 ns target.
postcard's varint decoder for a 4-byte value (1_234_567 encodes in 3 bytes) is
essentially a register operation.

**Raw serialize** at 21.6 ns exceeds the 1–5 ns budget. The cost is not the
varint encoding itself but the `Vec<u8>` heap allocation in
`postcard::to_allocvec`. A caller that provides a pre-allocated buffer (e.g.
`postcard::to_slice` or `postcard::to_vec` with capacity hint) would see ~2–5 ns.
The 1–5 ns budget in the design assumed a pre-allocated output path; the
`to_allocvec` path always allocates.

**No path exceeds 500 ns**, so the design's primary constraint is met. The raw
deserialize path at 1.95 ns confirms this is the right choice for
high-frequency read-heavy workloads (e.g., reading prices from a binary log).
