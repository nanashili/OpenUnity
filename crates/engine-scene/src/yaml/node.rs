use crate::{Result, SceneError};
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub(crate) enum Node {
    Scalar(String),
    Quoted(String),
    Map(BTreeMap<String, Node>),
    Seq(Vec<Node>),
}
impl Node {
    pub(crate) fn map(&self) -> Result<&BTreeMap<String, Node>> {
        if let Self::Map(m) = self {
            Ok(m)
        } else {
            Err(SceneError::new("expected mapping"))
        }
    }
    pub(crate) fn seq(&self) -> Result<&[Node]> {
        if let Self::Seq(s) = self {
            Ok(s)
        } else {
            Err(SceneError::new("expected sequence"))
        }
    }
    pub(crate) fn scalar(&self) -> Result<&str> {
        if let Self::Scalar(s) | Self::Quoted(s) = self {
            Ok(s)
        } else {
            Err(SceneError::new("expected scalar"))
        }
    }
}
