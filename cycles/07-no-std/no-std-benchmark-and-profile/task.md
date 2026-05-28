# Task: no-std-benchmark-and-profile

**Project:** decimal64  
**Cycle:** 07-no-std  
**Status:** active  
**Depends on:** `no-std-implement`

---

## Objective

Verify that the no_std migration did not introduce performance regressions.
Run the existing benchmark suite and compare against the pre-migration baseline
stored in prior cycle bench docs.

The no_std changes are purely `use` line substitutions and `cfg` gates; zero-cost
impact is expected. This task exists to confirm that expectation with numbers and
to detect any accidental regression (e.g., an optimizer hint lost due to a
changed code path).

## Prior baseline

| Benchmark | Source doc | Representative result |
|-----------|-----------|----------------------|
| `parse` throughput | `docs/bench-results.md` | ~70 M/s |
| `arithmetic/mul` | `docs/math-ops-perf-final-bench.md` | ~0.6 ns |
| `arithmetic/div` | `docs/math-ops-perf-final-bench.md` | ~2.0 ns |
| `scientific/parse` | `docs/scientific-notation-final-bench.md` | ~9–13 ns |
| `serde/serialize` | `docs/serde-final-bench.md` | ~44 ns |
| `serde/deserialize` | `docs/serde-final-bench.md` | ~14 ns |

## Steps

1. Run the full benchmark suite with default features (std):
   ```bash
   CARGO_TARGET_DIR=/work/cargo-target-ralph cargo bench
   ```
2. Collect results and compare to the baselines above.
3. Document findings in `docs/no-std-bench-results.md`.

## Regression threshold

≤5% slowdown vs the baseline on any benchmark is acceptable (within measurement noise).
Any regression > 5% must be investigated before the reeval task proceeds.

## Deliverable

`docs/no-std-bench-results.md` containing:
- Raw Criterion output (or a summary table)
- Comparison to prior baseline with delta %
- Pass/fail verdict per benchmark
- Conclusion: "no regression" or list of regressions for reeval

## Out of scope

- Do not run benchmarks without std (bench harness requires std).
- Do not change source code.
- Do not add new benchmarks.
