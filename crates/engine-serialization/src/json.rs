use crate::{MAX_DEPTH, MAX_INPUT_BYTES, MAX_OBJECTS, Result, SceneError, text};
use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::{Map, Number, Value};
use std::fmt;

struct Unique(Value);

/// Detect another entry without allocating or validating its contents.
struct RejectExtraEntry(&'static str);

impl<'de> DeserializeSeed<'de> for RejectExtraEntry {
    type Value = ();

    fn deserialize<D: Deserializer<'de>>(
        self,
        _deserializer: D,
    ) -> std::result::Result<Self::Value, D::Error> {
        Err(de::Error::custom(self.0))
    }
}

impl<'de> Deserialize<'de> for Unique {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        struct JsonValueVisitor;

        impl<'de> Visitor<'de> for JsonValueVisitor {
            type Value = Unique;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("bounded JSON with unique keys")
            }

            fn visit_bool<E: de::Error>(self, value: bool) -> std::result::Result<Unique, E> {
                Ok(Unique(Value::Bool(value)))
            }

            fn visit_i64<E: de::Error>(self, value: i64) -> std::result::Result<Unique, E> {
                Ok(Unique(value.into()))
            }

            fn visit_u64<E: de::Error>(self, value: u64) -> std::result::Result<Unique, E> {
                Ok(Unique(value.into()))
            }

            fn visit_f64<E: de::Error>(self, value: f64) -> std::result::Result<Unique, E> {
                Number::from_f64(value)
                    .map(|number| Unique(Value::Number(number)))
                    .ok_or_else(|| E::custom("nonfinite number"))
            }

            fn visit_unit<E: de::Error>(self) -> std::result::Result<Unique, E> {
                Ok(Unique(Value::Null))
            }

            fn visit_str<E: de::Error>(self, value: &str) -> std::result::Result<Unique, E> {
                text(value).map_err(E::custom)?;
                Ok(Unique(value.into()))
            }

            fn visit_string<E: de::Error>(self, value: String) -> std::result::Result<Unique, E> {
                text(&value).map_err(E::custom)?;
                Ok(Unique(value.into()))
            }

            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut access: A,
            ) -> std::result::Result<Unique, A::Error> {
                let mut values = Vec::new();
                while values.len() < MAX_OBJECTS {
                    let Some(Unique(value)) = access.next_element()? else {
                        return Ok(Unique(Value::Array(values)));
                    };
                    values.push(value);
                }

                access.next_element_seed(RejectExtraEntry("JSON array item limit"))?;
                Ok(Unique(Value::Array(values)))
            }

            fn visit_map<A: MapAccess<'de>>(
                self,
                mut access: A,
            ) -> std::result::Result<Unique, A::Error> {
                let mut values = Map::new();
                while values.len() < MAX_OBJECTS {
                    let Some(key) = access.next_key::<String>()? else {
                        return Ok(Unique(Value::Object(values)));
                    };
                    text(&key).map_err(de::Error::custom)?;
                    if values.contains_key(&key) {
                        return Err(de::Error::custom("duplicate JSON key"));
                    }
                    let Unique(value) = access.next_value()?;
                    values.insert(key, value);
                }

                access.next_key_seed(RejectExtraEntry("JSON object member limit"))?;
                Ok(Unique(Value::Object(values)))
            }
        }

        deserializer.deserialize_any(JsonValueVisitor)
    }
}

/// Parse bounded UTF-8 JSON, rejecting duplicate keys and oversized or deeply nested input.
pub fn parse(bytes: &[u8]) -> Result<Value> {
    if bytes.is_empty() || bytes.len() > MAX_INPUT_BYTES {
        return Err(SceneError::new("JSON input byte limit"));
    }

    scan_structure(bytes)?;

    let Unique(value) = serde_json::from_slice(bytes).map_err(normalize_json_error)?;
    Ok(value)
}

fn scan_structure(bytes: &[u8]) -> Result<()> {
    let mut delimiters = Vec::new();
    let mut in_string = false;
    let mut escaped = false;

    for &byte in bytes {
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
            continue;
        }

        match byte {
            b'"' => in_string = true,
            b'{' | b'[' => {
                if delimiters.len() >= MAX_DEPTH {
                    return Err(SceneError::new("JSON depth limit"));
                }
                delimiters.push(byte);
            }
            b'}' | b']' => {
                let expected = match byte {
                    b'}' => b'{',
                    b']' => b'[',
                    _ => unreachable!(),
                };
                if delimiters.pop() != Some(expected) {
                    return Err(SceneError::new("malformed JSON"));
                }
            }
            _ => (),
        }
    }

    if in_string || escaped || !delimiters.is_empty() {
        return Err(SceneError::new("malformed JSON"));
    }
    Ok(())
}

fn normalize_json_error(error: serde_json::Error) -> SceneError {
    let detail = error.to_string();
    for stable in [
        "duplicate JSON key",
        "JSON array item limit",
        "JSON object member limit",
        "oversized string or embedded NUL",
    ] {
        if detail.contains(stable) {
            return SceneError::new(stable);
        }
    }
    SceneError::new("malformed JSON")
}
