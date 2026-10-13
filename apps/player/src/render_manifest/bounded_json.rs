//! Streaming JSON bounds and duplicate-member validation for v2 fragments.

use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use std::fmt;

const MAX_BYTES: usize = 16 * 1024 * 1024;
const MAX_DEPTH: usize = 64;
const MAX_NODES: usize = 2_000_000;
const MAX_MEMBERS: usize = 20_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BoundedJsonError {
    InvalidUtf8,
    InvalidJson,
    DuplicateKey,
    Limit(&'static str),
}

pub(super) fn within_document_limit(bytes: &[u8]) -> bool {
    bytes.len() <= MAX_BYTES && std::str::from_utf8(bytes).is_ok()
}

pub(crate) fn validate_unique_fragment(bytes: &[u8]) -> Result<(), BoundedJsonError> {
    if bytes.len() > MAX_BYTES {
        return Err(BoundedJsonError::Limit("manifest bytes"));
    }
    if std::str::from_utf8(bytes).is_err() {
        return Err(BoundedJsonError::InvalidUtf8);
    }

    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let mut budget = ParseBudget::default();
    let result = AnySeed {
        budget: &mut budget,
        reject_duplicates: true,
    }
    .deserialize(&mut deserializer);

    if result.is_err() {
        return Err(budget.failure.unwrap_or(BoundedJsonError::InvalidJson));
    }
    if deserializer.end().is_err() {
        return Err(BoundedJsonError::InvalidJson);
    }

    Ok(())
}

#[derive(Default)]
pub(super) struct ParseBudget {
    depth: usize,
    nodes: usize,
    members: usize,
    failure: Option<BoundedJsonError>,
}

impl ParseBudget {
    pub(super) fn node(&mut self) -> Result<(), &'static str> {
        self.nodes += 1;
        if self.nodes > MAX_NODES {
            self.failure = Some(BoundedJsonError::Limit("manifest value tree"));
            return Err("node limit");
        }
        Ok(())
    }

    pub(super) fn enter(&mut self) -> Result<(), &'static str> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            self.failure = Some(BoundedJsonError::Limit("manifest nesting depth"));
            return Err("depth limit");
        }
        Ok(())
    }

    pub(super) fn leave(&mut self) {
        self.depth -= 1;
    }

    pub(super) fn member(&mut self) -> Result<(), &'static str> {
        self.members += 1;
        if self.members > MAX_MEMBERS {
            self.failure = Some(BoundedJsonError::Limit("manifest object members"));
            return Err("object member limit");
        }
        Ok(())
    }
}

pub(super) struct AnySeed<'a> {
    pub(super) budget: &'a mut ParseBudget,
    pub(super) reject_duplicates: bool,
}

impl<'de, 'a> DeserializeSeed<'de> for AnySeed<'a> {
    type Value = ();

    fn deserialize<D: serde::Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        deserializer.deserialize_any(AnyVisitor {
            budget: self.budget,
            reject_duplicates: self.reject_duplicates,
        })
    }
}

struct AnyVisitor<'a> {
    budget: &'a mut ParseBudget,
    reject_duplicates: bool,
}

impl<'de, 'a> Visitor<'de> for AnyVisitor<'a> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a bounded JSON value")
    }

    fn visit_bool<E>(self, _: bool) -> Result<(), E>
    where
        E: de::Error,
    {
        self.budget.node().map_err(E::custom)
    }

    fn visit_i64<E>(self, _: i64) -> Result<(), E>
    where
        E: de::Error,
    {
        self.budget.node().map_err(E::custom)
    }

    fn visit_u64<E>(self, _: u64) -> Result<(), E>
    where
        E: de::Error,
    {
        self.budget.node().map_err(E::custom)
    }

    fn visit_f64<E>(self, _: f64) -> Result<(), E>
    where
        E: de::Error,
    {
        self.budget.node().map_err(E::custom)
    }

    fn visit_str<E>(self, _: &str) -> Result<(), E>
    where
        E: de::Error,
    {
        self.budget.node().map_err(E::custom)
    }

    fn visit_none<E>(self) -> Result<(), E>
    where
        E: de::Error,
    {
        self.budget.node().map_err(E::custom)
    }

    fn visit_seq<A>(self, mut access: A) -> Result<(), A::Error>
    where
        A: SeqAccess<'de>,
    {
        self.budget.node().map_err(de::Error::custom)?;
        self.budget.enter().map_err(de::Error::custom)?;
        while access
            .next_element_seed(AnySeed {
                budget: self.budget,
                reject_duplicates: self.reject_duplicates,
            })?
            .is_some()
        {}
        self.budget.leave();
        Ok(())
    }

    fn visit_map<A>(self, mut access: A) -> Result<(), A::Error>
    where
        A: MapAccess<'de>,
    {
        self.budget.node().map_err(de::Error::custom)?;
        self.budget.enter().map_err(de::Error::custom)?;
        let mut keys = std::collections::BTreeSet::new();
        while let Some(key) = access.next_key::<String>()? {
            self.budget.member().map_err(de::Error::custom)?;
            if self.reject_duplicates && !keys.insert(key) {
                self.budget.failure = Some(BoundedJsonError::DuplicateKey);
                return Err(de::Error::custom("duplicate object key"));
            }
            access.next_value_seed(AnySeed {
                budget: self.budget,
                reject_duplicates: self.reject_duplicates,
            })?;
        }
        self.budget.leave();
        Ok(())
    }

    fn visit_unit<E>(self) -> Result<(), E>
    where
        E: de::Error,
    {
        self.budget.node().map_err(E::custom)
    }
}
