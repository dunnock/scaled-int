# Task: no-std-reeval-and-improve

**Project:** decimal64  
**Cycle:** 07-no-std  
**Status:** active  
**Depends on:** `no-std-benchmark-and-profile`

---

## Objective

Review benchmark results from `no-std-benchmark-and-profile`. If regressions were
found (> 5% slowdown on any benchmark), diagnose and fix them. If no regressions
were found, write a short "no change needed" document and exit.

## Decision tree

```
Read docs/no-std-bench-results.md
│
├── "no regression" verdict?
│    └─ YES → Write docs/no-std-reeval.md: "baseline maintained, no changes"
│              Commit, exit.
│
└── Regression found on benchmark B?
     └─ YES → Diagnose root cause (diff the IR, check inlining, check cfg gates)
               Apply targeted fix to the affected file(s)
               Re-run the specific benchmark to verify improvement
               Document fix + before/after numbers in docs/no-std-reeval.md
               Commit, exit.
```

## Likely regression causes (if any)

- A `#[inline(always)]` annotation inadvertently removed.
- A `cfg` gate that shadowed a hot path (e.g., `Display` on a code path the
  compiler previously inlined into a test).
- An import reorganization that broke LLVM's ability to see through a boundary.

## Deliverable

`docs/no-std-reeval.md` containing:
- Summary: regression found (Y/N)
- If Y: root cause, fix applied, before/after numbers
- If N: one-line confirmation "baseline maintained"

## Constraint

The reeval task must NOT introduce new public APIs or change existing behaviour.
Only performance fixes are in scope.
