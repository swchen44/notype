//! YAML wire codec only. Domain validation and transfer application are separate.
use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::Value;

/// Bounded transfer failure; never contains parser excerpts or document values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// Invalid, ambiguous or structurally excessive YAML.
    Invalid,
    /// Input, scalar content or rendered output exceeds the byte bound.
    TooLarge,
    /// The bounded value cannot be encoded as YAML.
    Encoding,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Invalid => "settings-yaml-invalid",
            Self::TooLarge => "settings-yaml-too-large",
            Self::Encoding => "settings-yaml-encoding-failed",
        })
    }
}
impl std::error::Error for Error {}
/// Maximum transfer document size in UTF-8 bytes.
pub const MAX_BYTES: usize = 8_388_608;
const MAX_DEPTH: usize = 64;
const MAX_NODES: usize = 200_000;

/// Parse syntax only; the caller must validate the resulting domain document.
pub fn parse(bytes: &[u8]) -> Result<Value, Error> {
    if bytes.len() > MAX_BYTES {
        return Err(Error::TooLarge);
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|_| Error::Invalid)?
        .trim_start_matches('\u{feff}');
    let options = serde_saphyr::options! {
        budget: serde_saphyr::budget! {
            max_reader_input_bytes: Some(MAX_BYTES),
            max_depth: MAX_DEPTH,
            flow_nesting_limit: MAX_DEPTH,
            max_documents: 1,
            max_nodes: MAX_NODES,
            max_events: 500_000,
            max_total_scalar_bytes: MAX_BYTES,
            max_aliases: 0,
            max_anchors: 0,
        },
        duplicate_keys: serde_saphyr::DuplicateKeyPolicy::Error,
        merge_keys: serde_saphyr::MergeKeyPolicy::Error,
        reject_unsupported_tags: true,
        strict_booleans: true,
        with_snippet: false,
    };
    // Do not format the parser error: it may contain document values or secrets.
    let value: StrictValue =
        serde_saphyr::from_str_with_options(text, options).map_err(|_| Error::Invalid)?;
    check_tree(&value.0)?;
    Ok(value.0)
}

/// Render a document as human-readable UTF-8 YAML without environment resolution.
pub fn render(value: &Value) -> Result<String, Error> {
    check_tree(value)?;
    let text = serde_saphyr::to_string(value).map_err(|_| Error::Encoding)?;
    if text.len() > MAX_BYTES {
        return Err(Error::TooLarge);
    }
    Ok(text)
}

// Iterative checks also bound caller-created values before recursive encoding.
fn check_tree(value: &Value) -> Result<(), Error> {
    let mut pending = vec![(value, 1usize)];
    let mut nodes = 0usize;
    let mut scalars = 0usize;
    while let Some((value, depth)) = pending.pop() {
        nodes += 1;
        if let Value::Object(items) = value {
            // Parser node budgets count keys as well as values.
            nodes = nodes.saturating_add(items.len());
        }
        if depth > MAX_DEPTH || nodes > MAX_NODES {
            return Err(Error::Invalid);
        }
        let children = match value {
            Value::Array(items) => items.len(),
            Value::Object(items) => items.len(),
            _ => 0,
        };
        if nodes.saturating_add(pending.len()).saturating_add(children) > MAX_NODES {
            return Err(Error::Invalid);
        }
        match value {
            Value::Array(items) => pending.extend(items.iter().map(|item| (item, depth + 1))),
            Value::Object(items) => {
                scalars = items
                    .keys()
                    .try_fold(scalars, |size, key| size.checked_add(key.len()))
                    .ok_or(Error::TooLarge)?;
                pending.extend(items.values().map(|item| (item, depth + 1)));
            }
            Value::String(text) => {
                scalars = scalars.checked_add(text.len()).ok_or(Error::TooLarge)?
            }
            _ => {}
        }
        if scalars > MAX_BYTES {
            return Err(Error::TooLarge);
        }
    }
    Ok(())
}

// YAML's duplicate identity can differ from a String map's identity (1 vs "1").
// Check the final key representation as well; never let conversion overwrite it.
struct StrictValue(Value);
impl<'de> Deserialize<'de> for StrictValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ValueVisitor;
        impl<'de> Visitor<'de> for ValueVisitor {
            type Value = StrictValue;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("a bounded YAML value")
            }
            fn visit_bool<E: de::Error>(self, value: bool) -> Result<Self::Value, E> {
                Ok(StrictValue(value.into()))
            }
            fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
                Ok(StrictValue(value.into()))
            }
            fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
                Ok(StrictValue(value.into()))
            }
            fn visit_f64<E: de::Error>(self, value: f64) -> Result<Self::Value, E> {
                serde_json::Number::from_f64(value)
                    .map(|n| StrictValue(Value::Number(n)))
                    .ok_or_else(|| E::custom("invalid number"))
            }
            fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
                Ok(StrictValue(value.into()))
            }
            fn visit_string<E: de::Error>(self, value: String) -> Result<Self::Value, E> {
                Ok(StrictValue(value.into()))
            }
            fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(StrictValue(Value::Null))
            }
            fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(StrictValue(Value::Null))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                while let Some(value) = seq.next_element::<StrictValue>()? {
                    values.push(value.0);
                }
                Ok(StrictValue(Value::Array(values)))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut values = serde_json::Map::new();
                while let Some((key, value)) = map.next_entry::<String, StrictValue>()? {
                    if values.insert(key, value.0).is_some() {
                        return Err(de::Error::custom("duplicate key"));
                    }
                }
                Ok(StrictValue(Value::Object(values)))
            }
        }
        deserializer.deserialize_any(ValueVisitor)
    }
}
