//! Operation recognition uses the native serde enum declarations. Payload errors
//! cannot be mistaken for additions to the operation set.
use serde::de::{DeserializeOwned, Deserializer, Visitor, value::Error};
use serde_json::value::RawValue;

use super::DaemonError;

pub(super) fn tag(raw: &str) -> Option<String> {
    // An externally tagged enum is either a unit string or a one-entry map.
    // Borrow the payload so even a large request does not build a second tree.
    if raw.starts_with('"') {
        return serde_json::from_str(raw).ok();
    }
    let fields: std::collections::HashMap<String, &RawValue> = serde_json::from_str(raw).ok()?;
    if fields.len() == 1 { fields.into_keys().next() } else { None }
}

pub(super) fn payload(raw: &str) -> Option<&RawValue> {
    let fields: std::collections::HashMap<String, &RawValue> = serde_json::from_str(raw).ok()?;
    if fields.len() == 1 { fields.into_values().next() } else { None }
}

pub(super) fn recognize<T: DeserializeOwned>(raw: &str) -> Result<String, DaemonError> {
    let operation = tag(raw).ok_or_else(|| DaemonError::InvalidInput("expected one native operation tag".into()))?;
    let mut known = false;
    let _ = T::deserialize(EnumTags { name: &operation, known: &mut known });
    if known { Ok(operation) } else { Err(DaemonError::Unsupported(operation)) }
}

pub(super) fn recognize_leaf<T: DeserializeOwned>(raw: &RawValue, family: &str) -> Result<String, DaemonError> {
    let raw = payload(raw.get()).ok_or_else(|| DaemonError::InvalidInput(format!("missing {family} operation")))?;
    recognize::<T>(raw.get()).map_err(|error| match error {
        DaemonError::Unsupported(operation) => DaemonError::Unsupported(format!("{family}.{operation}")),
        other => other,
    })
}

pub(super) fn decode<T: DeserializeOwned>(raw: &RawValue) -> Result<T, DaemonError> {
    serde_json::from_str(raw.get()).map_err(|error| DaemonError::InvalidInput(error.to_string()))
}

/// A serde-derived enum supplies its accepted tags before visiting the
/// variant. Stop there; no fake parameter values or duplicate leaf lists.
struct EnumTags<'a> {
    name: &'a str,
    known: &'a mut bool,
}

impl<'de> Deserializer<'de> for EnumTags<'_> {
    type Error = Error;

    fn deserialize_enum<V: Visitor<'de>>(self, _: &'static str, variants: &'static [&'static str], _: V) -> Result<V::Value, Error> {
        *self.known = variants.contains(&self.name);
        Err(serde::de::Error::custom("operation tag inspected"))
    }

    fn deserialize_any<V: Visitor<'de>>(self, _: V) -> Result<V::Value, Error> {
        Err(serde::de::Error::custom("expected externally tagged native enum"))
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes byte_buf
        option unit unit_struct newtype_struct seq tuple tuple_struct map struct
        identifier ignored_any
    }
}
