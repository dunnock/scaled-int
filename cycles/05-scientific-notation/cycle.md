# Cycle 05 — Scientific Notation

**status: complete**  
**branch:** 05-scientific-notation  

---

## Objective

Add a `Scientific<D>` newtype wrapper over `Decimal64<S>` and `UDecimal64<S>` that:
- Accepts scientific-notation strings (`"1.5e3"`, `"9.9E-4"`, etc.) in `FromStr`.
- Leaves the base parser unchanged (cycle 03 benchmark numbers are preserved).
- Emits normalized scientific notation in `Display`.
- Adds `ParseError::Underflow` for extreme negative exponents.

---

## Member Tasks

| # | Task                                          | Status   | Notes                                             |
|---|-----------------------------------------------|----------|---------------------------------------------------|
| 1 | `scientific-notation-design-and-plan`         | complete | Produced `docs/scientific-notation-design.md`     |
| 2 | `scientific-notation-implement`               | complete | `src/scientific.rs`, `ParseError::Underflow`      |
| 3 | `scientific-notation-benchmark-and-profile`   | complete | Initial results in `docs/scientific-notation-bench-results.md` |
| 4 | `scientific-notation-reeval-and-improve`      | complete | OR-mask scan + `#[inline(always)]`; `docs/scientific-notation-reeval.md` |
| 5 | `scientific-notation-final-benchmark`         | complete | Final results in `docs/scientific-notation-final-bench.md` |

---

## Key Design Decisions

- **Wrapper approach:** `Scientific<D>` is a `repr(transparent)` newtype; base parsers untouched.
- **Exponent application:** `raw_result = mantissa_raw × 10^E` (direct i64 multiply/divide).
- **Underflow threshold:** `|exponent| > 18` with nonzero mantissa → `ParseError::Underflow`.
- **Display:** Always normalized scientific notation (`1.2345e0`), trailing-zero stripped mantissa.
- **Module:** New `src/scientific.rs`; no changes to `Cargo.toml` or existing modules.

---

## Deliverables

- `docs/scientific-notation-design.md` — full design rationale
- `src/scientific.rs` — `Scientific<D>` type and impls
- `src/lib.rs` — `ParseError::Underflow` addition, `Scientific` re-export
- `benches/scientific.rs` — scientific parse + display + base reference benchmarks
- `docs/scientific-notation-bench-results.md` — initial measured results (pre-optimisation)
- `docs/scientific-notation-reeval.md` — optimisation analysis and reeval results
- `docs/scientific-notation-final-bench.md` — authoritative final benchmark results

---

## Key Results

**CPU:** 12th Gen Intel Core i9-12900K (Alder Lake), ~3.7 GHz effective  
**Reference input:** `"9999999999.9999"` (14 chars, scale=4)

### Parse overhead vs base `Decimal64`

| Path    | Base (ns) | Initial Scientific (ns) | Final Scientific (ns) | Initial overhead | Final overhead |
|---------|:---------:|:-----------------------:|:---------------------:|:----------------:|:--------------:|
| noexp   |   8.663   |          14.685         |         12.607        |     **+64%**     |    **+46%**    |
| pos_exp |   8.663   |          14.288         |         14.169        |     **+60%**     |    **+64%**    |

**Target: ≤25% overhead — NOT MET.**

### Improvements applied (commit 469fbc7)

1. OR-mask scan: replaced `.iter().position(|&b| b==b'e'||b==b'E')` with `b | 0x20 == b'e'`
2. `#[inline(always)]` on all private helpers (`find_exp_marker`, `pow10_*`, `parse_exponent`, `apply_*`)

**Noexp improved −18 pp (64% → 46%).** Pos_exp unchanged — bottleneck is two-pass
architecture (separate mantissa + exponent parse calls), not the scan cost.

### Absolute throughput

| Parser          | 14-char input (M/s) |
|-----------------|:-------------------:|
| `Decimal64`     |        115.4        |
| `Scientific` (noexp) |    79.3        |
| `Scientific` (pos_exp) |  70.6        |
| `rust_decimal`  |         73.4        |
| `bigdecimal`    |         15.7        |

`Scientific` parse is faster than `rust_decimal` on matching input length.

### Path to ≤25% overhead (cycle N+1)

Merged single-pass parser: scan for `e`/`E` while accumulating mantissa digits,
eliminating the double-scan. Requires changes to `src/parse.rs` — deferred.
