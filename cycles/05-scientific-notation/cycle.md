# Cycle 05 — Scientific Notation

**status: active**  
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

| # | Task                                          | Status  | Notes                                             |
|---|-----------------------------------------------|---------|---------------------------------------------------|
| 1 | `scientific-notation-design-and-plan`         | active  | This task; produces `docs/scientific-notation-design.md` |
| 2 | `scientific-notation-implement`               | active  | Code only; depends on (1)                         |
| 3 | `scientific-notation-benchmark-and-profile`   | active  | Criterion + baseline comparison; depends on (2)   |
| 4 | `scientific-notation-reeval-and-improve`      | active  | Analyse, apply optimisations; depends on (3)      |
| 5 | `scientific-notation-final-benchmark`         | active  | Re-run, document delta; depends on (4)            |

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
- `benches/parse.rs` (or new bench file) — scientific parse benchmarks
- `docs/scientific-notation-bench-results.md` — final measured results
