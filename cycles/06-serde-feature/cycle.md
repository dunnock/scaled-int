# Cycle 06 — Serde Feature

**status: active**
**branch:** 06-serde-feature

---

## Objective

Add an optional `serde` cargo feature that provides `Serialize` and `Deserialize`
implementations for `Decimal64<S>`, `UDecimal64<S>`, and `Scientific<D>`.
Default wire format: decimal string (human-readable, JSON-safe, lossless round-trip).
Raw integer adapter modules (`serde_as::raw_i64`, `serde_as::raw_u64`) for compact
binary formats.

---

## Member tasks

| # | Task                                          | Status  | Notes                                                  |
|---|-----------------------------------------------|---------|--------------------------------------------------------|
| 1 | `serde-feature-design-and-plan`               | active  | This task; produces `docs/serde-design.md`             |
| 2 | `serde-feature-implement`                     | active  | Code only; depends on (1)                              |
| 3 | `serde-feature-benchmark-and-profile`         | active  | Criterion + format comparisons; depends on (2)         |
| 4 | `serde-feature-reeval-and-improve`            | active  | Analyse, apply optimisations; depends on (3)           |
| 5 | `serde-feature-final-benchmark`               | active  | Re-run, document delta; depends on (4)                 |

---

## Key Design Decisions

- **Default wire format:** Decimal string (`"123.4567"`) for all three types.
- **Raw opt-in:** `#[serde(with = "decimal64::serde_as::raw_i64")]` / `raw_u64`.
- **`Scientific<D>` format:** Always scientific notation string (`"1.2345e2"`).
- **Feature gate:** `serde = ["dep:serde"]`; zero overhead when feature disabled.
- **No derive:** Manual `Serialize`/`Deserialize` impls; no procedural macro deps.

---

## Deliverables

- `docs/serde-design.md` — full design rationale
- `src/serde_impls.rs` — `Serialize`/`Deserialize` for all three types
- `src/serde_as.rs` — `raw_i64` and `raw_u64` adapter modules
- `src/lib.rs` — feature-gated module declarations
- `Cargo.toml` — `[features]` + optional `serde` dependency + dev-deps
- `benches/serde.rs` — serialize/deserialize throughput benchmarks
- `docs/serde-bench-results.md` — final measured results

---

## Acceptance

- `docs/serde-design.md` committed.
- 4 sibling tasks done; this member-task list updated.
- All three types round-trip JSON and postcard without precision loss.
- Raw integer path benchmarks documented; string path overhead documented.
- `cargo test --features serde --all` passes with no regressions.
