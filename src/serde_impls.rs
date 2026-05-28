use serde::{Deserialize, Deserializer, Serialize, Serializer};
#[cfg(not(feature = "std"))]
use alloc::string::ToString;

use crate::{Decimal64, Scientific, UDecimal64};

// ── Decimal64<S> ──────────────────────────────────────────────────────────────

impl<const S: u32> Serialize for Decimal64<S> {
    fn serialize<Ser: Serializer>(&self, ser: Ser) -> Result<Ser::Ok, Ser::Error> {
        ser.serialize_str(&self.to_string())
    }
}

struct D64Visitor<const S: u32>;

impl<'de, const S: u32> serde::de::Visitor<'de> for D64Visitor<S> {
    type Value = Decimal64<S>;

    fn expecting(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "a decimal string like \"123.4567\"")
    }

    fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Decimal64<S>, E> {
        v.parse().map_err(E::custom)
    }
}

impl<'de, const S: u32> Deserialize<'de> for Decimal64<S> {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        de.deserialize_str(D64Visitor::<S>)
    }
}

// ── UDecimal64<S> ─────────────────────────────────────────────────────────────

impl<const S: u32> Serialize for UDecimal64<S> {
    fn serialize<Ser: Serializer>(&self, ser: Ser) -> Result<Ser::Ok, Ser::Error> {
        ser.serialize_str(&self.to_string())
    }
}

struct UD64Visitor<const S: u32>;

impl<'de, const S: u32> serde::de::Visitor<'de> for UD64Visitor<S> {
    type Value = UDecimal64<S>;

    fn expecting(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "an unsigned decimal string like \"10.00\"")
    }

    fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<UDecimal64<S>, E> {
        v.parse().map_err(E::custom)
    }
}

impl<'de, const S: u32> Deserialize<'de> for UDecimal64<S> {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        de.deserialize_str(UD64Visitor::<S>)
    }
}

// ── Scientific<Decimal64<S>> ──────────────────────────────────────────────────

impl<const S: u32> Serialize for Scientific<Decimal64<S>> {
    fn serialize<Ser: Serializer>(&self, ser: Ser) -> Result<Ser::Ok, Ser::Error> {
        ser.serialize_str(&self.to_string())
    }
}

struct SciD64Visitor<const S: u32>;

impl<'de, const S: u32> serde::de::Visitor<'de> for SciD64Visitor<S> {
    type Value = Scientific<Decimal64<S>>;

    fn expecting(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "a scientific notation decimal string like \"1.2345e2\"")
    }

    fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Scientific<Decimal64<S>>, E> {
        v.parse().map_err(E::custom)
    }
}

impl<'de, const S: u32> Deserialize<'de> for Scientific<Decimal64<S>> {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        de.deserialize_str(SciD64Visitor::<S>)
    }
}

// ── Scientific<UDecimal64<S>> ─────────────────────────────────────────────────

impl<const S: u32> Serialize for Scientific<UDecimal64<S>> {
    fn serialize<Ser: Serializer>(&self, ser: Ser) -> Result<Ser::Ok, Ser::Error> {
        ser.serialize_str(&self.to_string())
    }
}

struct SciUD64Visitor<const S: u32>;

impl<'de, const S: u32> serde::de::Visitor<'de> for SciUD64Visitor<S> {
    type Value = Scientific<UDecimal64<S>>;

    fn expecting(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "a scientific notation decimal string like \"1.23e2\"")
    }

    fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Scientific<UDecimal64<S>>, E> {
        v.parse().map_err(E::custom)
    }
}

impl<'de, const S: u32> Deserialize<'de> for Scientific<UDecimal64<S>> {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        de.deserialize_str(SciUD64Visitor::<S>)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[test]
    fn d64_json_roundtrip() {
        let v: Decimal64<4> = "123.4567".parse().unwrap();
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, r#""123.4567""#);
        let back: Decimal64<4> = serde_json::from_str(&json).unwrap();
        assert_eq!(back, v);
    }

    #[test]
    fn d64_raw_json_roundtrip() {
        #[derive(Serialize, Deserialize)]
        struct Row {
            #[serde(with = "crate::serde_as::raw_i64")]
            price: Decimal64<4>,
        }
        let r = Row { price: "123.4567".parse().unwrap() };
        let json = serde_json::to_string(&r).unwrap();
        let back: Row = serde_json::from_str(&json).unwrap();
        assert_eq!(back.price.raw(), r.price.raw());
    }

    #[test]
    fn d64_postcard_roundtrip() {
        let v: Decimal64<4> = "123.4567".parse().unwrap();
        let bytes = postcard::to_allocvec(&v).unwrap();
        let back: Decimal64<4> = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(back, v);
    }

    #[test]
    fn d64_raw_postcard_roundtrip() {
        #[derive(Serialize, Deserialize)]
        struct Row {
            #[serde(with = "crate::serde_as::raw_i64")]
            price: Decimal64<4>,
        }
        let r = Row { price: "123.4567".parse().unwrap() };
        let bytes = postcard::to_allocvec(&r).unwrap();
        // postcard varint-encodes i64; 1234567 encodes in fewer bytes than the string "123.4567"
        assert!(bytes.len() < 8);
        let back: Row = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(back.price.raw(), r.price.raw());
    }

    #[test]
    fn ud64_json_roundtrip() {
        let v: UDecimal64<2> = "10.00".parse().unwrap();
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, r#""10.00""#);
        let back: UDecimal64<2> = serde_json::from_str(&json).unwrap();
        assert_eq!(back, v);
    }

    #[test]
    fn ud64_raw_json_roundtrip() {
        #[derive(Serialize, Deserialize)]
        struct Row {
            #[serde(with = "crate::serde_as::raw_u64")]
            qty: UDecimal64<2>,
        }
        let r = Row { qty: "10.00".parse().unwrap() };
        let json = serde_json::to_string(&r).unwrap();
        let back: Row = serde_json::from_str(&json).unwrap();
        assert_eq!(back.qty.raw(), r.qty.raw());
    }

    #[test]
    fn scientific_json_roundtrip() {
        let v: Scientific<Decimal64<4>> = "1.2345e2".parse().unwrap();
        let json = serde_json::to_string(&v).unwrap();
        // Display emits normalized scientific notation
        let back: Scientific<Decimal64<4>> = serde_json::from_str(&json).unwrap();
        assert_eq!(back.0.raw(), v.0.raw());
    }

    #[test]
    fn d64_deser_invalid() {
        let result: Result<Decimal64<4>, _> = serde_json::from_str(r#""12x.34""#);
        assert!(result.is_err());
    }

    #[test]
    fn d64_deser_overflow() {
        let result: Result<Decimal64<4>, _> =
            serde_json::from_str(r#""999999999999999999.0""#);
        assert!(result.is_err());
    }

    #[test]
    fn d64_serde_token_zero() {
        let v: Decimal64<4> = Decimal64::ZERO;
        serde_test::assert_ser_tokens(&v, &[serde_test::Token::Str("0.0000")]);
    }

    #[test]
    fn d64_serde_token_negative() {
        let v: Decimal64<4> = "-1.0000".parse().unwrap();
        serde_test::assert_ser_tokens(&v, &[serde_test::Token::Str("-1.0000")]);
    }
}
