//! Native JSON envelopes retain opaque values without converting them to DTOs.

use std::fmt;

use serde::de::{MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::value::RawValue;

use super::{RouteFailure, RpcId};

#[derive(Debug, thiserror::Error)]
pub(crate) enum ProtocolFailure {
    #[error("invalid native JSON envelope: {0}")]
    InvalidFrame(String),
    #[error("native JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Route(#[from] RouteFailure),
}

/// Ordered object fields keep every opaque value in its original JSON form,
/// including numbers that cannot be represented by serde_json::Value.
#[derive(Clone, Debug)]
pub(crate) struct RawObject {
    fields: Vec<(String, Box<RawValue>)>,
}

impl RawObject {
    pub(crate) fn parse(raw: &str) -> Result<Self, ProtocolFailure> {
        Ok(serde_json::from_str(raw)?)
    }

    pub(crate) fn empty() -> Self {
        Self { fields: Vec::new() }
    }

    pub(crate) fn get(&self, key: &str) -> Option<&RawValue> {
        self.fields.iter().find(|(name, _)| name == key).map(|(_, value)| value.as_ref())
    }

    pub(crate) fn set_raw(&mut self, key: &str, value: Box<RawValue>) {
        if let Some((_, current)) = self.fields.iter_mut().find(|(name, _)| name == key) {
            *current = value;
        } else {
            self.fields.push((key.to_owned(), value));
        }
    }

    pub(crate) fn set<T: Serialize>(&mut self, key: &str, value: &T) -> Result<(), ProtocolFailure> {
        self.set_raw(key, serde_json::value::to_raw_value(value)?);
        Ok(())
    }

    pub(crate) fn overlay(&mut self, newer: &Self) -> Result<(), ProtocolFailure> {
        for (key, value) in &newer.fields {
            if let Some(previous) = self.get(key)
                && previous.get().starts_with('{')
                && value.get().starts_with('{')
            {
                let mut merged = Self::parse(previous.get())?;
                merged.overlay(&Self::parse(value.get())?)?;
                self.set(key, &merged)?;
            } else {
                self.set_raw(key, value.clone());
            }
        }
        Ok(())
    }

    pub(crate) fn render(&self) -> Result<String, ProtocolFailure> {
        Ok(serde_json::to_string(self)?)
    }
}

impl Serialize for RawObject {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.fields.len()))?;
        for (key, value) in &self.fields {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for RawObject {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ObjectVisitor;
        impl<'de> Visitor<'de> for ObjectVisitor {
            type Value = RawObject;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a native JSON object")
            }

            fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Self::Value, M::Error> {
                let mut object = RawObject::empty();
                while let Some((key, value)) = map.next_entry::<String, Box<RawValue>>()? {
                    if object.get(&key).is_some() {
                        return Err(serde::de::Error::custom(format!("duplicate JSON field {key}")));
                    }
                    object.fields.push((key, value));
                }
                Ok(object)
            }
        }
        deserializer.deserialize_map(ObjectVisitor)
    }
}

impl RpcId {
    pub(crate) fn from_raw(raw: &RawValue) -> Result<Self, ProtocolFailure> {
        if raw.get().starts_with('"') {
            Ok(Self::Text(serde_json::from_str(raw.get())?))
        } else {
            serde_json::from_str::<i64>(raw.get()).map(Self::Integer).map_err(|_| ProtocolFailure::InvalidFrame("request id must be a string or int64".to_owned()))
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum FrameKind {
    Request { id: RpcId, method: String },
    Notification { method: String },
    Response { id: RpcId },
}

/// Parsing inspects only the envelope. Unmodified forwarding uses `raw()`;
/// changing an ID re-serializes object keys while keeping raw field values.
#[derive(Clone, Debug)]
pub(crate) struct NativeFrame {
    raw: String,
    pub(crate) object: RawObject,
    pub(crate) kind: FrameKind,
}

impl NativeFrame {
    pub(crate) fn parse(raw: &str) -> Result<Self, ProtocolFailure> {
        let object = RawObject::parse(raw)?;
        let id = object.get("id").map(RpcId::from_raw).transpose()?;
        let method = object.get("method").map(|value| serde_json::from_str::<String>(value.get())).transpose()?;
        let kind = match (method, id) {
            (Some(method), Some(id)) => FrameKind::Request { id, method },
            (Some(method), None) => FrameKind::Notification { method },
            (None, Some(id)) if object.get("result").is_some() != object.get("error").is_some() => FrameKind::Response { id },
            _ => return Err(ProtocolFailure::InvalidFrame("expected a request, notification or response".to_owned())),
        };
        Ok(Self { raw: raw.to_owned(), object, kind })
    }

    pub(crate) fn raw(&self) -> &str {
        &self.raw
    }

    pub(crate) fn params(&self) -> Result<Option<RawObject>, ProtocolFailure> {
        self.object.get("params").map(|value| RawObject::parse(value.get())).transpose()
    }

    pub(crate) fn with_id(&self, id: &RpcId) -> Result<String, ProtocolFailure> {
        let mut object = self.object.clone();
        match id {
            RpcId::Integer(value) => object.set("id", value)?,
            RpcId::Text(value) => object.set("id", value)?,
        }
        object.render()
    }
}
