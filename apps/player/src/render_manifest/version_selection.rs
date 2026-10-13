//! Non-diagnostic, bounded dispatch for the versioned player manifest.

#[path = "bounded_json.rs"]
mod bounded_json;

use bounded_json::{AnySeed, ParseBudget, within_document_limit};
pub(crate) use bounded_json::{BoundedJsonError, validate_unique_fragment};
use serde::de::{self, DeserializeSeed, MapAccess, Visitor};
use serde_json::value::RawValue;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ManifestRoute {
    Version2,
    Legacy,
}

/// Selects v2 only for one complete object with one exact integer `2` field.
pub(crate) fn select(bytes: &[u8]) -> ManifestRoute {
    if !within_document_limit(bytes) {
        return ManifestRoute::Legacy;
    }
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let mut budget = ParseBudget::default();
    let version = VersionSeed {
        budget: &mut budget,
    }
    .deserialize(&mut deserializer);
    let Ok(version) = version else {
        return ManifestRoute::Legacy;
    };
    if deserializer.end().is_err() {
        return ManifestRoute::Legacy;
    }
    if version {
        ManifestRoute::Version2
    } else {
        ManifestRoute::Legacy
    }
}

struct VersionSeed<'a> {
    budget: &'a mut ParseBudget,
}

impl<'de, 'a> DeserializeSeed<'de> for VersionSeed<'a> {
    type Value = bool;

    fn deserialize<D: serde::Deserializer<'de>>(self, deserializer: D) -> Result<bool, D::Error> {
        deserializer.deserialize_map(VersionVisitor {
            budget: self.budget,
        })
    }
}

struct VersionVisitor<'a> {
    budget: &'a mut ParseBudget,
}

impl<'de, 'a> Visitor<'de> for VersionVisitor<'a> {
    type Value = bool;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON object")
    }

    fn visit_map<A>(self, mut access: A) -> Result<bool, A::Error>
    where
        A: MapAccess<'de>,
    {
        self.budget.node().map_err(de::Error::custom)?;
        self.budget.enter().map_err(de::Error::custom)?;
        let mut found = None;
        while let Some(key) = access.next_key::<String>()? {
            self.budget.member().map_err(de::Error::custom)?;
            if key == "schema_version" {
                if found.is_some() {
                    return Err(de::Error::custom("duplicate schema_version key"));
                }
                let raw = access.next_value::<&RawValue>()?;
                consume_raw(raw, self.budget).map_err(de::Error::custom)?;
                found = Some(raw.get() == "2");
            } else {
                access.next_value_seed(AnySeed {
                    budget: self.budget,
                    reject_duplicates: false,
                })?;
            }
        }
        self.budget.leave();
        Ok(found == Some(true))
    }
}

fn consume_raw(raw: &RawValue, budget: &mut ParseBudget) -> Result<(), &'static str> {
    let mut deserializer = serde_json::Deserializer::from_str(raw.get());
    AnySeed {
        budget,
        reject_duplicates: false,
    }
    .deserialize(&mut deserializer)
    .map_err(|_| "invalid JSON value")?;
    deserializer.end().map_err(|_| "trailing JSON data")
}
