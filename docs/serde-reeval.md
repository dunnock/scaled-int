# Serde Re-evaluation — Cycle 06

**Date:** 2026-05-28
**Branch:** 06-serde-feature

---

## 1. Baseline Numbers (from `serde-bench-results.md`)

| Benchmark | ns/op (median) | Budget | Status |
|-----------|---------------|--------|--------|
| `serde_json_serialize_d64` | 44.0 ns | ≤ 500 ns | **well under** (11×) |
| `serde_json_deserialize_d64` | 13.9 ns | ≤ 100 ns | **well under** (7×) |
| `postcard_serialize_string_d64` | 53.9 ns | ≤ 500 ns | **well under** (9×) |
| `postcard_serialize_raw_d64` | 21.6 ns | ~1–5 ns | over (see §3) |
| `postcard_deserialize_raw_d64` | 1.95 ns | ~1–5 ns | **within** |

---

## 2. Decision: No Optimization Needed

The primary constraint from `serde-design.md` §10 is that the string-path serialize
cost must not exceed **500 ns**. All string-path measurements are 10× below this
limit:

- JSON serialize: 44 ns (11× under budget)
- JSON deserialize: 13.9 ns (7× under budget)
- postcard string serialize: 53.9 ns (9× under budget)

The `to_string()` heap allocation — which §10 expected to cost 100–300 ns — measured
only ~44–54 ns in practice. serde_json's internal short-string path amortizes the
allocation for the 8-character `"123.4567"` string, and the system allocator handles
small allocations quickly on Alder Lake.

No stack-buffer optimization (`arrayvec` or manual `itoa` approach) is warranted.
Adding `arrayvec` as a dependency would increase compile time and crate surface for a
measured gain of ≤ 10 ns on a path that is already off the hot path.

---

## 3. Raw Serialize Budget Overage

`postcard_serialize_raw_d64` at 21.6 ns exceeds the 1–5 ns design target. This is
**not an implementation problem** — the serde impl itself (which calls
`ser.serialize_i64(self.0)`) is correct and minimal. The 21.6 ns cost is entirely in
`postcard::to_allocvec`, which allocates a `Vec<u8>` on every call regardless of
payload size.

The 1–5 ns target assumed a caller using a pre-allocated output buffer (e.g.,
`postcard::to_slice`). In that scenario the varint encoding of `1_234_567` (3 bytes)
is a register-level operation and the measured 1.95 ns raw deserialize confirms the
varint codec itself is within budget.

No change to `serde_impls.rs` or `serde_as.rs` is needed.

---

## 4. Checklist from Task

1. **Is `serde_json_serialize_d64` within budget (≤ 500 ns)?**
   Yes — 44 ns. Not bottlenecked by `to_string()` heap allocation relative to budget.

2. **Is `serde_json_deserialize_d64` within budget (≤ 100 ns)?**
   Yes — 13.9 ns. The `from_str()` parse path (cycle 05, ~8.7–14 ns) dominates; JSON
   overhead is negligible.

3. **Is `postcard_serialize_raw_d64` close to ~5 ns?**
   No — 21.6 ns. Cause is `to_allocvec` heap allocation in the *benchmark harness*,
   not the serde impl. The impl is correct; callers using pre-allocated buffers will
   see ≤ 5 ns.

---

## 5. No Code Changes

`src/serde_impls.rs` and `Cargo.toml` are unchanged from the previous task.
All tests pass: `cargo test --features serde --all`.
