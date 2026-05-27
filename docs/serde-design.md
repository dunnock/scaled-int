# Serde Feature — Cycle 06 Design

**Date:** 2026-05-27
**Branch:** 06-serde-feature

---

## 1. Objective

Add an optional `serde` cargo feature that provides `Serialize` and `Deserialize`
implementations for `Decimal64<S>`, `UDecimal64<S>`, and `Scientific<D>` (the
newtype wrapper from cycle 05). The design must produce zero compile-time overhead
when the feature is disabled, expose a minimal stable public surface, and document
all format trade-offs the caller must understand.

---

## 2. Feature Gate

```toml
[features]
serde = ["dep:serde"]

[dependencies]
serde = { version = "1.0", optional = true }
```

The `dep:` prefix is mandatory. Without it, Cargo treats the optional dependency
name as an implicit feature, which means any crate that adds `decimal64` as a
dependency can accidentally activate serde by writing `decimal64/serde` even
without explicitly requesting it. `dep:` breaks that implicit link: the dependency
stays private, and only the named feature `serde` enables it.

We do **not** request `features = ["derive"]`. We implement `Serialize` and
`Deserialize` by hand for all three types; derive macros are not needed and would
add procedural-macro compilation time.

---

## 3. Wire Format: Default String

### 3.1 Decision

All three types serialize as a **decimal string** by default.

| Type | Example value | Serialized form |
|------|--------------|-----------------|
| `Decimal64<4>` | `1234567` raw | `"123.4567"` |
| `UDecimal64<2>` | `1000` raw | `"10.00"` |
| `Scientific<Decimal64<4>>` | `1234567` raw | `"1.2345e2"` |

`Decimal64` and `UDecimal64` use their `Display` output (fixed-point notation).
`Scientific<D>` uses its own `Display` output (always-normalized scientific notation
as specified in cycle 05: `"1.2345e2"` not `"123.4567"`).

### 3.2 Rationale

**JSON safety.** JSON numbers are IEEE 754 doubles. Any value whose magnitude
exceeds 2^53 (≈9×10^15) cannot be represented exactly as a JSON number. For
financial data at scale 4, values above ≈9×10^11 already fall outside the safe
integer range. Serializing as a string sidesteps this entirely.

**Lossless round-trip.** `Display` → `FromStr` is exact for both types: no
floating-point conversion occurs. The raw internal integer is reproduced exactly.

**Ecosystem consistency.** `rust_decimal`, `bigdecimal`, and `chrono` (for dates)
all default to string serialization. A crate that defaults to integers would
surprise users migrating from those crates.

**Human-readable formats (JSON, TOML, YAML).** String is readable without
knowledge of the scale parameter. The number `"123.4567"` communicates its value;
the integer `1234567` does not (the reader must know scale=4 to interpret it).

### 3.3 Trade-offs

| Concern | String (default) | Raw integer (opt-in) |
|---------|-----------------|---------------------|
| JSON safety | Always safe | Unsafe above 2^53 |
| Compact binary (postcard, msgpack) | ~12–20 bytes | 8 bytes |
| Human-readable | Yes | No |
| Schema self-describing | Yes | Requires out-of-band scale |
| Migration on scale change | No change | Data migration needed |
| Parse overhead on deserialize | `from_str` cost (~15 ns) | `i64` copy |

For internal binary protocols where both ends share a schema, raw integer is
the right choice. It saves 4–12 bytes per field and eliminates the parsing step.
See §5 for how to opt in.

---

## 4. Deserialize: Error Contract

Deserialization failure propagates `ParseError` as a `serde::de::Error::custom`
message. The failure modes:

| `ParseError` variant | Trigger condition | Example input |
|---------------------|-------------------|---------------|
| `Empty` | Empty string | `""` |
| `InvalidChar` | Non-numeric byte | `"12x.34"` |
| `Overflow` | Value exceeds `i64`/`u64` max at scale S | `"999999999999999999.0"` at S=4 |
| `TooManyFractional` | (currently unused; reserved) | — |
| `Underflow` | (cycle 05; `Scientific` only) | `"1.0e-1000"` |

String deserialization also accepts formats that `Display` does not emit (e.g.,
`Decimal64<4>` from the string `"+1.23"` or `"1.2"` — these parse successfully
because `FromStr` is more permissive than `Display`). Callers can rely on
round-trip invariance: `serialize(x).deserialize() == x`, but not on format
normalization: `serialize(parse(s)).display() == s`.

---

## 5. Raw Integer Format (Opt-in)

For callers that need compact binary representation, two adapter modules will be
published under `decimal64::serde_as`:

```rust
// Serialize Decimal64<S> as i64 raw value
#[serde(with = "decimal64::serde_as::raw_i64")]
price: Decimal64<4>,

// Serialize UDecimal64<S> as u64 raw value
#[serde(with = "decimal64::serde_as::raw_u64")]
qty: UDecimal64<2>,
```

### 5.1 Module Signatures

```rust
// decimal64::serde_as::raw_i64
pub fn serialize<const S: u32, Ser: Serializer>(
    val: &Decimal64<S>,
    ser: Ser,
) -> Result<Ser::Ok, Ser::Error>;

pub fn deserialize<'de, const S: u32, D: Deserializer<'de>>(
    de: D,
) -> Result<Decimal64<S>, D::Error>;

// decimal64::serde_as::raw_u64
pub fn serialize<const S: u32, Ser: Serializer>(
    val: &UDecimal64<S>,
    ser: Ser,
) -> Result<Ser::Ok, Ser::Error>;

pub fn deserialize<'de, const S: u32, D: Deserializer<'de>>(
    de: D,
) -> Result<UDecimal64<S>, D::Error>;
```

### 5.2 Design rationale for `serde_as` module name

The name `decimal64::serde_as` avoids two collision hazards:
1. `decimal64::serde` would shadow the imported `serde` crate name in user code.
2. `decimal64::raw` is ambiguous (raw could mean raw pointer, raw bytes, or raw
   integer in different contexts).

`serde_as` is consistently used by the `serde_with` ecosystem for this pattern
and will be familiar to users.

### 5.3 No raw adapter for `Scientific<D>`

`Scientific<D>` is a display/parse wrapper, not a numeric type. Its "raw" form
is the inner `D`'s raw value. Callers who want raw binary for a `Scientific<D>`
field should unwrap it first and apply `raw_i64`/`raw_u64` to the inner type.
This keeps the interface surface minimal and avoids a combinatorial explosion.

---

## 6. `Scientific<D>` Serde Specifics

`Scientific<D>` is a transparent newtype over `D`. For serde purposes it has
**string format only** (no raw variant). The string format differs from D's
default serde format:

- `Decimal64<4>` (default serde) → `"123.4567"` (fixed-point)
- `Scientific<Decimal64<4>>` (serde) → `"1.23457e2"` (normalized scientific)

This is intentional: `Scientific` exists precisely to represent data in
scientific notation format. A user who serializes `Scientific<Decimal64<4>>`
has opted into the scientific representation explicitly.

On deserialization, `Scientific::from_str` is called, which accepts both
`"1.23457e2"` and `"123.457"` (the base `decimal_str` grammar from cycle 05).
So `Scientific<D>` round-trips losslessly, and can deserialize values that were
originally serialized as `Decimal64<D>` (fixed-point strings). This is a
one-way compatibility: fixed-point serialized data can be read via `Scientific`,
but scientific serialized data cannot be read via `Decimal64` (would fail with
`InvalidChar` on the `e`).

---

## 7. Module Layout

```
src/
  lib.rs              ← adds `#[cfg(feature = "serde")] pub mod serde_impls;`
                            `#[cfg(feature = "serde")] pub mod serde_as;`
  serde_impls.rs      ← Serialize/Deserialize for Decimal64, UDecimal64, Scientific
  serde_as.rs         ← raw_i64, raw_u64 adapter modules
```

All new code is gated behind `#[cfg(feature = "serde")]`. No existing modules
change their public API.

### 7.1 `serde_impls.rs` sketch

```rust
#![cfg(feature = "serde")]

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use crate::{Decimal64, UDecimal64, ParseError};

impl<const S: u32> Serialize for Decimal64<S> {
    fn serialize<Ser: Serializer>(&self, ser: Ser) -> Result<Ser::Ok, Ser::Error> {
        ser.serialize_str(&self.to_string())
    }
}

impl<'de, const S: u32> Deserialize<'de> for Decimal64<S> {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        let s = <&str>::deserialize(de)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

// Identical structure for UDecimal64<S> (s.parse() returns UDecimal64).

// For Scientific<Decimal64<S>> and Scientific<UDecimal64<S>>:
// Display uses scientific notation format; from_str parses scientific notation.
```

### 7.2 Visitor for owned-string formats

The `<&str>::deserialize(de)` approach fails for formats that always provide
owned strings (e.g., serde_json when input is not a `&str` slice). The
implementation must use a `Visitor` that handles both `visit_str` and
`visit_string`:

```rust
struct StrVisitor<const S: u32>(std::marker::PhantomData<Decimal64<S>>);

impl<'de, const S: u32> serde::de::Visitor<'de> for StrVisitor<S> {
    type Value = Decimal64<S>;
    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "a decimal string like \"123.4567\"")
    }
    fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Decimal64<S>, E> {
        v.parse().map_err(E::custom)
    }
}
```

This is the correct pattern: borrow when available, fall through to owned when
not. It also enables numeric formats (e.g., CBOR, msgpack) that provide the
value as a string variant of their type system.

---

## 8. `no_std` Compatibility Note

The `serde` crate itself supports `no_std` when compiled without the `std` feature.
`decimal64` currently depends on `std::error::Error` for `ParseError`. If
`no_std` support is added in cycle 07, the serde impl must also gate on
`#[cfg(feature = "std")]` appropriately, or use a `core::fmt::Display` bound
instead of `std::error::Error`. This is a known future interaction; it does not
affect cycle 06 implementation.

---

## 9. Testing Plan

The `implement` task (sibling 1) must add the following tests. They are specified
here so that the implementer cannot skip them.

### 9.1 Dev dependencies to add

```toml
[dev-dependencies]
serde_json  = "1"           # JSON human-readable round-trip
postcard    = { version = "1", features = ["alloc"] }  # binary compact round-trip
serde_test  = "1"           # token-based assertion helper
```

### 9.2 Required test matrix

| Test name | Type | Format | Assertion |
|-----------|------|--------|-----------|
| `d64_json_roundtrip` | `Decimal64<4>` | JSON | serialize then deserialize equals original |
| `d64_raw_json_roundtrip` | `Decimal64<4>` + `raw_i64` | JSON | i64 value preserved |
| `d64_postcard_roundtrip` | `Decimal64<4>` | postcard | bytes roundtrip |
| `d64_raw_postcard_roundtrip` | `Decimal64<4>` + `raw_i64` | postcard | 8 bytes for i64 |
| `ud64_json_roundtrip` | `UDecimal64<2>` | JSON | serialize then deserialize |
| `ud64_raw_json_roundtrip` | `UDecimal64<2>` + `raw_u64` | JSON | u64 preserved |
| `scientific_json_roundtrip` | `Scientific<Decimal64<4>>` | JSON | scientific string preserved |
| `d64_deser_invalid` | `Decimal64<4>` | JSON | invalid string → `Err` |
| `d64_deser_overflow` | `Decimal64<4>` | JSON | overflow string → `Err` |
| `d64_serde_token_zero` | `Decimal64<4>` | `serde_test` tokens | `Str("0.0000")` |
| `d64_serde_token_negative` | `Decimal64<4>` | `serde_test` tokens | `Str("-1.0000")` |

### 9.3 JSON wire format assertions

```rust
let v: Decimal64<4> = "123.4567".parse().unwrap();
let json = serde_json::to_string(&v).unwrap();
assert_eq!(json, r#""123.4567""#);  // quoted string, not bare number

let back: Decimal64<4> = serde_json::from_str(&json).unwrap();
assert_eq!(back, v);
```

### 9.4 Postcard size assertion for raw format

```rust
#[derive(Serialize, Deserialize)]
struct Row {
    #[serde(with = "decimal64::serde_as::raw_i64")]
    price: Decimal64<4>,
}
let r = Row { price: "123.4567".parse().unwrap() };
let bytes = postcard::to_allocvec(&r).unwrap();
// postcard varint-encodes i64; 1234567 encodes in 3 bytes (varint), not 8
// but the significant claim is it's far smaller than the string "123.4567" (8 chars)
assert!(bytes.len() < 8);
```

---

## 10. Performance Budget

Serde serialize/deserialize is not on the hot path for this library (arithmetic
and parse benchmarks are). However, the `benchmark-and-profile` task (sibling 2)
must measure:

| Benchmark | Description |
|-----------|-------------|
| `serde_json_serialize_d64` | `serde_json::to_string(&d64_val)` throughput |
| `serde_json_deserialize_d64` | `serde_json::from_str::<Decimal64<4>>(s)` throughput |
| `postcard_serialize_raw` | `postcard::to_allocvec(&row_with_raw_d64)` throughput |
| `postcard_deserialize_raw` | `postcard::from_bytes::<Row>(&bytes)` throughput |

Expected baseline (no optimization, string path):
- Serialize: dominated by `to_string()` allocation, ~100–300 ns.
- Deserialize: dominated by `from_str()` parse, ~15–50 ns.

The raw path should be ~1–5 ns (just `i64` read/write with a varint encoder).

If the string path is > 500 ns, the `reeval-and-improve` task should investigate
stack-allocated formatting (avoid heap allocation for the string). A fixed-size
buffer of 30 bytes covers all `Decimal64` values at any scale.

---

## 11. Stability Guarantees

The serde wire format **is** part of the public API from cycle 06 onward.
Changing the default serialization from string to integer in a future cycle would
be a **semver-major break**. The raw adapters (`serde_as::raw_i64`,
`serde_as::raw_u64`) are also stable once published.

The only escape hatch: users can always wrap `Decimal64` in their own newtype
and implement custom `Serialize`/`Deserialize`, bypassing the library defaults.

---

## 12. Out of Scope

- JSON Schema / `schemars` integration — deferred indefinitely.
- `serde_as` from the `serde_with` crate — the built-in `decimal64::serde_as`
  module covers the primary use case; `serde_with` integration can be added as
  a separate optional feature later.
- Versioned format migration — no version tag in the wire format.
- Custom serialize for `Round` — not a stored type, not needed.

---

## 13. Dependency Chain

```
serde-feature-design-and-plan   (this task)
  └─> serde-feature-implement
        └─> serde-feature-benchmark-and-profile
              └─> serde-feature-reeval-and-improve
                    └─> serde-feature-final-benchmark
```

Each task inherits all Cargo.toml and source changes from its predecessor.
No task modifies `Cargo.toml` except `implement` (which adds the `serde`
dependency and dev-dependencies).
