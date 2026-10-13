use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};
use std::cell::Cell;
use std::fmt;
use std::rc::Rc;

const MAX_JSON_NODES: usize = 2_000_000;
const MAX_OBJECT_MEMBERS: usize = 20_000;
pub(super) const NODE_LIMIT: &str = "JSON node limit exceeded";
pub(super) const MEMBER_LIMIT: &str = "JSON object member limit exceeded";
pub(super) const ARRAY_LIMIT: &str = "JSON array item limit exceeded";

pub(super) struct Unique(pub(super) Value);

#[derive(Clone)]
pub(super) struct ParseBudget {
    nodes: Rc<Cell<usize>>,
    object_members: Rc<Cell<usize>>,
}

impl ParseBudget {
    pub(super) fn new() -> Self {
        Self {
            nodes: Rc::new(Cell::new(0)),
            object_members: Rc::new(Cell::new(0)),
        }
    }

    fn consume_node<E: de::Error>(&self) -> Result<(), E> {
        consume_bounded_counter(&self.nodes, MAX_JSON_NODES, NODE_LIMIT)
    }

    fn consume_member<E: de::Error>(&self) -> Result<(), E> {
        consume_bounded_counter(&self.object_members, MAX_OBJECT_MEMBERS, MEMBER_LIMIT)
    }
}

fn consume_bounded_counter<E: de::Error>(
    counter: &Cell<usize>,
    maximum: usize,
    diagnostic: &'static str,
) -> Result<(), E> {
    let current = counter.get();
    if current == maximum {
        return Err(E::custom(diagnostic));
    }
    counter.set(current + 1);
    Ok(())
}

#[derive(Clone, Copy)]
struct RootArrayLimit {
    maximum_items: usize,
}

pub(super) struct UniqueSeed {
    budget: ParseBudget,
    root_array_limit: Option<RootArrayLimit>,
}

impl UniqueSeed {
    pub(super) fn new(budget: ParseBudget) -> Self {
        Self {
            budget,
            root_array_limit: None,
        }
    }

    pub(super) fn bounded_root_array(budget: ParseBudget, maximum_items: usize) -> Self {
        Self {
            budget,
            root_array_limit: Some(RootArrayLimit { maximum_items }),
        }
    }
}

impl<'de> DeserializeSeed<'de> for UniqueSeed {
    type Value = Unique;

    fn deserialize<D: serde::Deserializer<'de>>(self, deserializer: D) -> Result<Unique, D::Error> {
        self.budget.consume_node()?;
        let visitor = JsonVisitor {
            budget: self.budget,
            root_array_limit: self.root_array_limit,
        };

        if self.root_array_limit.is_some() {
            deserializer.deserialize_seq(visitor)
        } else {
            deserializer.deserialize_any(visitor)
        }
    }
}

struct JsonVisitor {
    budget: ParseBudget,
    root_array_limit: Option<RootArrayLimit>,
}

impl<'de> Visitor<'de> for JsonVisitor {
    type Value = Unique;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("bounded unique-key JSON")
    }

    fn visit_bool<E: de::Error>(self, value: bool) -> Result<Unique, E> {
        Ok(Unique(Value::Bool(value)))
    }

    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Unique, E> {
        Ok(Unique(value.into()))
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Unique, E> {
        Ok(Unique(value.into()))
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Unique, E> {
        Number::from_f64(value)
            .map(|number| Unique(Value::Number(number)))
            .ok_or_else(|| E::custom("nonfinite number"))
    }

    fn visit_unit<E: de::Error>(self) -> Result<Unique, E> {
        Ok(Unique(Value::Null))
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Unique, E> {
        Ok(Unique(Value::String(value.into())))
    }

    fn visit_string<E: de::Error>(self, value: String) -> Result<Unique, E> {
        Ok(Unique(Value::String(value)))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut access: A) -> Result<Unique, A::Error> {
        if let Some(limit) = self.root_array_limit {
            return self.visit_bounded_root_array(access, limit);
        }

        let mut values = Vec::new();
        while let Some(Unique(value)) =
            access.next_element_seed(UniqueSeed::new(self.budget.clone()))?
        {
            values.push(value);
        }
        Ok(Unique(Value::Array(values)))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Unique, A::Error> {
        let mut values = Map::new();
        while let Some(key) = access.next_key::<String>()? {
            self.budget.consume_member()?;
            if values.contains_key(&key) {
                return Err(de::Error::custom("duplicate JSON key"));
            }
            let Unique(value) = access.next_value_seed(UniqueSeed::new(self.budget.clone()))?;
            values.insert(key, value);
        }
        Ok(Unique(Value::Object(values)))
    }
}

impl JsonVisitor {
    fn visit_bounded_root_array<'de, A: SeqAccess<'de>>(
        self,
        mut access: A,
        limit: RootArrayLimit,
    ) -> Result<Unique, A::Error> {
        let capacity = access.size_hint().unwrap_or(0).min(limit.maximum_items);
        let mut values = Vec::with_capacity(capacity);

        while values.len() < limit.maximum_items {
            let Some(Unique(value)) =
                access.next_element_seed(UniqueSeed::new(self.budget.clone()))?
            else {
                return Ok(Unique(Value::Array(values)));
            };
            values.push(value);
        }

        if access.next_element_seed(RejectExtraEntry)?.is_some() {
            return Err(de::Error::custom(ARRAY_LIMIT));
        }
        Ok(Unique(Value::Array(values)))
    }
}

struct RejectExtraEntry;

impl<'de> DeserializeSeed<'de> for RejectExtraEntry {
    type Value = ();

    fn deserialize<D: serde::Deserializer<'de>>(self, _: D) -> Result<(), D::Error> {
        Err(de::Error::custom(ARRAY_LIMIT))
    }
}
