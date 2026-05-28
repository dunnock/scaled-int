# Cycle 07 — no-std

**status: done**
**completed_at: 2026-05-28**
**branch:** 07-no-std

---

## Objective

Add `#![no_std]` compatibility to the `decimal64` crate. Introduce an explicit
`std` cargo feature (enabled by default) and an `alloc` feature for heap-dependent
APIs. All existing behaviour and public APIs are preserved when `std` is active.

See `docs/no-std-design.md` for the full design rationale.

---

## Member tasks

| # | Task                                    | Status  | Notes                                                    |
|---|-----------------------------------------|---------|----------------------------------------------------------|
| 1 | `no-std-design-and-plan`               | done    | Produces `docs/no-std-design.md`                         |
| 2 | `no-std-implement`                     | done    | ~29-line mechanical change; depends on (1)               |
| 3 | `no-std-benchmark-and-profile`         | done    | Regression check; depends on (2)                         |
| 4 | `no-std-reeval-and-improve`            | done    | No change needed; all within ±10% of cycle 05            |
| 5 | `no-std-final-benchmark`               | done    | Re-run confirms zero regression; cycle complete          |

---

## Key Design Decisions

- **`std` default-enabled:** existing callers require zero changes.
- **`alloc` additive:** unlocks `Scientific<D>::Display` and serde serialization.
- **`serde` implies `alloc`:** serialize path calls `to_string()`; alloc is the
  minimum honest requirement.
- **`Scientific::Display` gated on alloc:** the only alloc-dependent API outside
  of serde; pure no_std builds lose only this Display impl.
- **No logic changes:** all arithmetic, parsing, and core formatting are in `core`.
  The migration is purely mechanical `std::` → `core::` substitutions plus two
  `cfg` gates.

---

## Deliverables

- `docs/no-std-design.md` — full design rationale (cycle 07)
- Modified `Cargo.toml` — `std` / `alloc` features + serde dep fix
- Modified `src/lib.rs` — `cfg_attr(no_std)`, `Error` gate, `core::fmt` refs
- Modified `src/decimal64.rs` — `core::` use lines
- Modified `src/udecimal64.rs` — `core::` use lines
- Modified `src/scientific.rs` — `core::` use lines + `cfg` gates on `Display`
- Modified `src/serde_impls.rs` — `core::fmt::Formatter` refs
- `docs/no-std-bench-results.md` — benchmark regression evidence

---

## Acceptance

- `cargo build` unchanged.
- `cargo build --no-default-features` succeeds.
- `cargo build --no-default-features --features alloc` succeeds.
- `cargo test` passes with no regressions.
- `cargo test --no-default-features --features alloc` passes.
