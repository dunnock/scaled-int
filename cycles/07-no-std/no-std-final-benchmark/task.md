# Task: no-std-final-benchmark

**Project:** decimal64  
**Cycle:** 07-no-std  
**Status:** active  
**Depends on:** `no-std-reeval-and-improve`

---

## Objective

Produce the final benchmark record for cycle 07. Re-run the benchmark suite after
any reeval fixes (or confirm unchanged if reeval made no changes) and write the
definitive results document that closes the cycle.

## Steps

1. Check if `no-std-reeval-and-improve` applied any source changes.
   - If no source changes: the numbers in `docs/no-std-bench-results.md` are already
     final. Copy them into the final doc without re-running. Note "no reeval changes".
   - If source changes were made: re-run `cargo bench` to capture the post-fix numbers.

2. Write `docs/no-std-final-bench.md` containing:
   - Final benchmark table (all benchmarks, post-reeval numbers)
   - Delta vs cycle-start baseline (from `docs/bench-results.md` etc.)
   - Verdict: "no regression" or "regression recovered"
   - One-sentence cycle summary

3. Update `cycles/07-no-std/cycle.md` status from `active` to `complete`.

4. Commit both files.

## Deliverable

- `docs/no-std-final-bench.md` — final numbers + cycle verdict
- `cycles/07-no-std/cycle.md` — status changed to `complete`

## Out of scope

- Do not change source code.
- Do not change test files.
- Do not add new benchmarks.
