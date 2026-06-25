//! Canonical JSON serialization for content-addressed Agoramesh messages.
//!
//! The canonical form is used to compute object IDs and signatures, so two
//! messages with the same logical content produce identical bytes regardless
//! of the serialization path.

use serde::Serialize;
use serde_json::Value;

/// Serializes a value to a deterministic JSON byte string.
///
/// The output is compact (no whitespace) and sorts object keys recursively.
/// This is the only canonical format used in Phase 1; CBOR is intentionally
/// not supported yet.
///
/// # Errors
///
/// Returns an error if the value cannot be serialized to JSON.
pub fn to_vec<T>(value: &T) -> Result<Vec<u8>, serde_json::Error>
where
    T: Serialize + ?Sized,
{
    let value = serde_json::to_value(value)?;
    let canonical = sort_value(value);
    let mut bytes = Vec::new();
    let mut serializer = serde_json::Serializer::new(&mut bytes);
    canonical.serialize(&mut serializer)?;
    Ok(bytes)
}

fn sort_value(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut entries: Vec<_> = map.into_iter().collect();
            entries.sort_by(|(a, _), (b, _)| a.as_str().cmp(b.as_str()));
            Value::Object(
                entries
                    .into_iter()
                    .map(|(k, v)| (k, sort_value(v)))
                    .collect(),
            )
        }
        Value::Array(array) => Value::Array(array.into_iter().map(sort_value).collect()),
        scalar => scalar,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn canonical_json_is_sorted_and_compact() {
        let mut map = BTreeMap::new();
        map.insert("b", 2);
        map.insert("a", 1);
        let bytes = to_vec(&map).expect("serialize");
        assert_eq!(String::from_utf8(bytes).expect("utf8"), r#"{"a":1,"b":2}"#);
    }

    #[test]
    fn canonical_json_sorts_nested_keys() {
        let input = r#"{"z":{"b":1,"a":2},"a":3}"#;
        let value: Value = serde_json::from_str(input).expect("parse");
        let bytes = to_vec(&value).expect("serialize");
        assert_eq!(
            String::from_utf8(bytes).expect("utf8"),
            r#"{"a":3,"z":{"a":2,"b":1}}"#
        );
    }
}
