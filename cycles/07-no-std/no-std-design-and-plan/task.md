# Task: no-std-design-and-plan

**Project:** decimal64  
**Cycle:** 07-no-std  
**Status:** done  
**Depends on:** (none — first task in cycle)

---

## Objective

Design the `no_std` migration for the `decimal64` crate and produce a detailed
design document. Create four sibling tasks covering implementation, benchmarking,
re-evaluation, and final documentation.

## Deliverables

- `docs/no-std-design.md` (≥2 pages) covering:
  - Feature flag design (`std`, `alloc`, `serde` feature matrix)
  - Complete `std::` import audit across all source files
  - `alloc` dependency audit (`to_string()`, `Display` gating)
  - Floating-point method compatibility in `core`
  - Per-file implementation plan
  - Testing strategy and acceptance criteria
- `cycles/07-no-std/cycle.md` with all 5 member tasks listed
- Four sibling task directories under `cycles/07-no-std/`

## Acceptance

- Design doc committed.
- Four sibling tasks created with task.md + checklist.json.
- `cycle.md` member list updated to show all 5 tasks.
