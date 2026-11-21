//! Lossless browser mapping, independent of domain serde definitions.
//! Native integer values become single-key {$integer: decimal} objects.

use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

pub(crate) fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, ()> {
    let mut value = serde_json::to_value(value).map_err(|_| ())?;
    visit_output(&mut value)?;
    serde_json::to_vec(&value).map_err(|_| ())
}

fn visit_output(value: &mut Value) -> Result<(), ()> {
    match value {
        Value::Number(number) => {
            if !number.is_i64() && !number.is_u64() {
                return Err(());
            }
            *value = serde_json::json!({ "$integer": number.to_string() });
        }
        Value::Array(items) => {
            for item in items {
                visit_output(item)?;
            }
        }
        Value::Object(fields) => {
            for item in fields.values_mut() {
                visit_output(item)?;
            }
        }
        _ => {}
    }
    Ok(())
}

pub(crate) fn decode<T: DeserializeOwned + Serialize>(bytes: &[u8]) -> Result<T, ()> {
    // The duplicate-key visitor validates the whole tree before Value can erase keys.
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let Strict(mut value) = serde::Deserialize::deserialize(&mut deserializer).map_err(|_| ())?;
    deserializer.end().map_err(|_| ())?;
    visit_input(&mut value)?;
    let result: T = serde_json::from_value(value.clone()).map_err(|_| ())?;
    let canonical = serde_json::to_value(&result).map_err(|_| ())?;
    if !keys_known(&value, &canonical) {
        return Err(());
    }
    Ok(result)
}

/// Strict ordinary JSON for APIs whose native contract already uses string IDs.
pub(crate) fn decode_json<T: DeserializeOwned + Serialize>(bytes: &[u8]) -> Result<T, ()> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let Strict(value) = serde::Deserialize::deserialize(&mut deserializer).map_err(|_| ())?;
    deserializer.end().map_err(|_| ())?;
    let result: T = serde_json::from_value(value.clone()).map_err(|_| ())?;
    if !keys_known(&value, &serde_json::to_value(&result).map_err(|_| ())?) {
        return Err(());
    }
    Ok(result)
}

fn visit_input(value: &mut Value) -> Result<(), ()> {
    match value {
        Value::Number(_) => return Err(()),
        Value::Object(fields) if fields.contains_key("$integer") => {
            if fields.len() != 1 {
                return Err(());
            }
            let text = fields["$integer"].as_str().ok_or(())?;
            let number = if text.starts_with('-') { serde_json::Number::from(text.parse::<i64>().map_err(|_| ())?) } else { serde_json::Number::from(text.parse::<u64>().map_err(|_| ())?) };
            if number.to_string() != text {
                return Err(());
            }
            *value = Value::Number(number);
        }
        Value::Object(fields) => {
            for item in fields.values_mut() {
                visit_input(item)?;
            }
        }
        Value::Array(items) => {
            for item in items {
                visit_input(item)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn keys_known(input: &Value, canonical: &Value) -> bool {
    match (input, canonical) {
        (Value::Object(a), Value::Object(b)) => a.iter().all(|(key, value)| b.get(key).is_some_and(|expected| keys_known(value, expected))),
        (Value::Array(a), Value::Array(b)) => a.len() == b.len() && a.iter().zip(b).all(|(a, b)| keys_known(a, b)),
        _ => true,
    }
}

struct Strict(Value);
impl<'de> serde::Deserialize<'de> for Strict {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Strict;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("JSON with unique keys")
            }
            fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<Strict, E> {
                Ok(Strict(v.into()))
            }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Strict, E> {
                Ok(Strict(v.into()))
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Strict, E> {
                Ok(Strict(v.into()))
            }
            fn visit_f64<E: serde::de::Error>(self, _: f64) -> Result<Strict, E> {
                Err(E::custom("integers must use $integer"))
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Strict, E> {
                Ok(Strict(v.into()))
            }
            fn visit_unit<E: serde::de::Error>(self) -> Result<Strict, E> {
                Ok(Strict(Value::Null))
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut a: A) -> Result<Strict, A::Error> {
                let mut values = Vec::new();
                while let Some(Strict(value)) = a.next_element()? {
                    values.push(value);
                }
                Ok(Strict(Value::Array(values)))
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(self, mut a: A) -> Result<Strict, A::Error> {
                let mut values = serde_json::Map::new();
                while let Some(key) = a.next_key::<String>()? {
                    if values.contains_key(&key) {
                        return Err(serde::de::Error::custom("duplicate key"));
                    }
                    let Strict(value) = a.next_value()?;
                    values.insert(key, value);
                }
                Ok(Strict(Value::Object(values)))
            }
        }
        d.deserialize_any(Visitor)
    }
}
