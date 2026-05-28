# Cycle 07 — no-std Reeval and Improve

**Task:** `no-std-reeval-and-improve`
**Date:** 2026-05-28
**Branch:** 07-no-std

---

## Decision: no change needed

All benchmarks measured in `docs/no-std-bench-results.md` are within ±10% of the
cycle 05 (`scientific-notation`) baseline. No regression fix is required.

---

## Baseline vs Expected

The no_std implementation in commit `40089f4` is a purely mechanical transformation:
`std::` imports replaced with `core::`, and alloc-dependent code gated behind
`cfg(any(feature = "std", feature = "alloc"))`. No hot-path logic was altered.

| Benchmark group | Max delta vs cycle 05 | Within budget? |
|-----------------|:--------------------:|:--------------:|
| `parse_decimal64/` | +3.0% | Yes |
| `parse_udecimal64/` | +0.8% | Yes |
| `arithmetic/decimal64_*` | −23.3% (improvement) | Yes |
| `scientific_parse/` | +1.1% | Yes |
| `scientific_display/` | −0.0% | Yes |

Several arithmetic benchmarks improved by 5–23%; these are within normal run-to-run
CPU variation (Criterion noise band on this Alder Lake host) and do not reflect a
structural change. No path was made slower.

The `#[inline(always)]` annotations on hot helpers (added in cycle 05) remain
intact in the transformed source. The cfg gates wrap only `Display` impls and the
`std::error::Error` impl — neither appears on any parse or arithmetic critical path.

---

## Test Verification (re-confirmed)

```
CARGO_TARGET_DIR=/work/cargo-target-ralph cargo test --no-default-features --features alloc

test result: ok. 133 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
Doc-tests: ok. 1 passed
```

All 133 unit tests and 1 doc-test pass under `--no-default-features --features alloc`,
confirming feature-gate correctness.

---

## Conclusion

Budget is met. No code changes made in this task.
