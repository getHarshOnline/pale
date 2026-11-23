//! Canonicalization, content digests, and strict JSON ingestion.
//!
//! Canonicalization is RFC 8785 (JCS) via the vetted [`serde_jcs`] crate; the
//! agent-surface records are float-free by design, so [`parse_strict`] rejects any
//! float on ingestion as well as duplicate object keys and oversized integers.
//! Digests are the SHA-256 of the canonical UTF-8 bytes.
//!
//! JCS preserves non-ASCII characters as UTF-8 rather than `\u`-escaping them;
//! that encoding is part of the digest contract.

use std::fmt;

use serde::de::{self, DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::Serialize;

use crate::sha256_hex;

/// Errors from canonicalization or strict ingestion.
#[derive(Debug, thiserror::Error)]
pub enum CanonError {
    /// The value could not be canonicalized (e.g. a non-string map key).
    #[error("canonicalization failed: {0}")]
    Canonicalize(#[source] serde_json::Error),
    /// The input violated the selected JSON grammar.
    ///
    /// [`parse_strict`] rejects duplicate keys, floats, oversized integers and
    /// trailing bytes. [`parse_unique`] shares the ambiguity/syntax checks but
    /// admits finite native-owner numbers.
    #[error("JSON parse failed: {0}")]
    Parse(#[source] serde_json::Error),
}

/// Return the RFC 8785 (JCS) canonical JSON string for `value`.
///
/// Typed values pass through the same float-free, safe-integer grammar as raw
/// input accepted by [`parse_strict`]. This keeps digest preimages independent
/// of whether they originated as a Rust value or JSON bytes.
///
/// # Examples
///
/// ```
/// use pale_core::canon::to_canonical_string;
///
/// let value = serde_json::json!({"z": 1, "a": [2, 1]});
/// assert_eq!(to_canonical_string(&value)?, r#"{"a":[2,1],"z":1}"#);
/// # Ok::<(), pale_core::canon::CanonError>(())
/// ```
///
/// # Errors
///
/// Returns [`CanonError`] when serialization fails or the resulting JSON
/// violates the float-free, safe-integer record contract.
pub fn to_canonical_string<T: Serialize + ?Sized>(value: &T) -> Result<String, CanonError> {
    let canonical = serde_jcs::to_string(value).map_err(CanonError::Canonicalize)?;
    // Typed callers must obey the same float-free, safe-integer contract as
    // records admitted from bytes. Validating the canonical bytes keeps the two
    // entry paths equivalent without maintaining a second number walker.
    parse_strict(canonical.as_bytes())?;
    Ok(canonical)
}

/// Render strict JSON as deterministic two-space-indented file bytes.
///
/// This is a presentation boundary, never a digest or signature preimage. The
/// input is admitted through [`parse_strict`], object members retain their
/// deterministic lexical ordering, and the returned document ends with exactly
/// one line feed. Callers that supply canonical JCS bytes must retain those
/// original bytes for record identity, hashing, and signature operations.
///
/// # Errors
///
/// Returns [`CanonError`] when the input is not strict JSON or the admitted value
/// cannot be serialized.
pub fn pretty_json_document(strict_json: &[u8]) -> Result<Vec<u8>, CanonError> {
    let value = parse_strict(strict_json)?;
    let mut bytes = serde_json::to_vec_pretty(&value).map_err(CanonError::Canonicalize)?;
    bytes.push(b'\n');
    Ok(bytes)
}

/// Return the lowercase hexadecimal SHA-256 of `value`'s canonical JSON.
///
/// This is an untyped digest primitive. Typed digest newtypes give the resulting
/// bytes their domain meaning; calling this function does not validate a
/// record's claimed self-digest or admit the record.
///
/// # Errors
///
/// Returns [`CanonError`] when `value` cannot satisfy the canonical record
/// contract.
pub fn digest<T: Serialize + ?Sized>(value: &T) -> Result<String, CanonError> {
    Ok(sha256_hex(to_canonical_string(value)?.as_bytes()))
}

/// Parse bytes into a [`serde_json::Value`] under the strict record grammar.
///
/// Duplicate object keys, floats, integers outside JSON's safe range, malformed
/// JSON, and trailing bytes are rejected.
///
/// Use this function before ordinary Serde deserialization when the exact input
/// bytes cross a trust boundary: a previously built [`serde_json::Value`] can no
/// longer reveal duplicate keys from its source text. Passing this syntactic
/// gate does not perform schema or semantic admission.
///
/// # Errors
///
/// Returns [`CanonError::Parse`] for any rejected input.
pub fn parse_strict(bytes: &[u8]) -> Result<serde_json::Value, CanonError> {
    parse(bytes, NumberDomain::PortableJcs)
}

/// Parse native-owner JSON while preserving its complete numeric domain.
///
/// Duplicate object keys, malformed JSON, and trailing bytes are rejected, but
/// finite JSON numbers are retained as [`serde_json::Number`] values.
/// This is the appropriate boundary for structurally editing a native
/// configuration whose numeric policy Pale does not own. Use [`parse_strict`]
/// for Pale contracts and digest preimages.
///
/// # Errors
///
/// Returns [`CanonError::Parse`] for duplicate keys, malformed numbers or
/// syntax, and trailing bytes.
pub fn parse_unique(bytes: &[u8]) -> Result<serde_json::Value, CanonError> {
    parse(bytes, NumberDomain::NativeJson)
}

fn parse(bytes: &[u8], number_domain: NumberDomain) -> Result<serde_json::Value, CanonError> {
    let mut de = serde_json::Deserializer::from_slice(bytes);
    let value = ValueSeed { number_domain }
        .deserialize(&mut de)
        .map_err(CanonError::Parse)?;
    de.end().map_err(CanonError::Parse)?;
    Ok(value)
}

#[derive(Clone, Copy)]
enum NumberDomain {
    NativeJson,
    PortableJcs,
}

#[derive(Clone, Copy)]
struct ValueSeed {
    number_domain: NumberDomain,
}

impl<'de> DeserializeSeed<'de> for ValueSeed {
    type Value = serde_json::Value;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_any(ValueVisitor {
            number_domain: self.number_domain,
        })
    }
}

/// Largest nonnegative integer in Pale's contiguous portable JSON domain.
///
/// Every numeric `u64` projection admitted into generated Pale contracts obeys
/// the closed domain
///
#[doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/docs/math/json-safe-integer-domain.display.mathml"))]
///
/// RFC 8785 canonicalizes numbers through the ECMAScript binary64 model. Some
/// larger integers are exactly representable, but adjacent integers cease to be
/// uniformly distinguishable above this ceiling. A portable integer domain must
/// therefore stop here; larger identities use a lossless nonnumeric
/// representation, such as the decimal strings used by machine-local filesystem
/// identities.
pub const JSON_SAFE_INTEGER_MAX: u64 = (1u64 << 53) - 1;

/// JSON Schema for an unsigned integer admitted by canonical JSON.
pub(crate) fn json_safe_u64_schema(_generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schemars::Schema::try_from(serde_json::json!({
        "type": "integer",
        "minimum": 0,
        "maximum": JSON_SAFE_INTEGER_MAX
    }))
    .expect("the static canonical unsigned-integer schema is valid")
}

/// JSON Schema for a positive unsigned integer admitted by canonical JSON.
pub(crate) fn positive_json_safe_u64_schema(
    generator: &mut schemars::SchemaGenerator,
) -> schemars::Schema {
    let mut schema = json_safe_u64_schema(generator);
    schema.insert("minimum".into(), 1.into());
    schema
}

/// JSON Schema for a positive portable integer that retains the `uint64` annotation.
///
/// Use this projection when a pre-existing wire shape exposes the Rust storage
/// width as schema metadata. The numeric admission law remains owned by
/// [`positive_json_safe_u64_schema`]; this helper adds no wider integer domain.
pub(crate) fn formatted_positive_json_safe_u64_schema(
    generator: &mut schemars::SchemaGenerator,
) -> schemars::Schema {
    let mut schema = positive_json_safe_u64_schema(generator);
    schema.insert("format".into(), "uint64".into());
    schema
}

struct ValueVisitor {
    number_domain: NumberDomain,
}

impl<'de> Visitor<'de> for ValueVisitor {
    type Value = serde_json::Value;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.number_domain {
            NumberDomain::NativeJson => {
                f.write_str("unambiguous JSON with finite native-owner numbers")
            }
            NumberDomain::PortableJcs => f.write_str(
                "strict Pale JSON (no duplicate keys, no floats, integers within the 2^53 safe bound)",
            ),
        }
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E> {
        Ok(serde_json::Value::Bool(v))
    }

    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
        if matches!(self.number_domain, NumberDomain::PortableJcs)
            && v.unsigned_abs() > JSON_SAFE_INTEGER_MAX
        {
            return Err(E::custom(format!(
                "integer {v} exceeds the JSON safe-integer bound (±2^53-1)"
            )));
        }
        Ok(serde_json::Value::Number(v.into()))
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
        if matches!(self.number_domain, NumberDomain::PortableJcs) && v > JSON_SAFE_INTEGER_MAX {
            return Err(E::custom(format!(
                "integer {v} exceeds the JSON safe-integer bound (2^53-1)"
            )));
        }
        Ok(serde_json::Value::Number(v.into()))
    }

    fn visit_i128<E: de::Error>(self, _v: i128) -> Result<Self::Value, E> {
        Err(E::custom("integer out of the supported 64-bit range"))
    }

    fn visit_u128<E: de::Error>(self, _v: u128) -> Result<Self::Value, E> {
        Err(E::custom("integer out of the supported 64-bit range"))
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Self::Value, E> {
        if matches!(self.number_domain, NumberDomain::PortableJcs) {
            return Err(E::custom(concat!(
                "floats are not allowed in agent-surface records ",
                "(and oversized integers parse as floats)"
            )));
        }
        serde_json::Number::from_f64(value)
            .map(serde_json::Value::Number)
            .ok_or_else(|| E::custom("non-finite JSON number"))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> {
        Ok(serde_json::Value::String(v.to_owned()))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(serde_json::Value::Null)
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(serde_json::Value::Null)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut arr = Vec::new();
        while let Some(v) = seq.next_element_seed(ValueSeed {
            number_domain: self.number_domain,
        })? {
            arr.push(v);
        }
        Ok(serde_json::Value::Array(arr))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut obj = serde_json::Map::new();
        while let Some(key) = map.next_key::<String>()? {
            let val = map.next_value_seed(ValueSeed {
                number_domain: self.number_domain,
            })?;
            if obj.contains_key(&key) {
                return Err(de::Error::custom(format!("duplicate object key: {key}")));
            }
            obj.insert(key, val);
        }
        Ok(serde_json::Value::Object(obj))
    }
}
