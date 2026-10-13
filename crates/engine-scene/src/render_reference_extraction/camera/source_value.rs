use crate::yaml::Node;
use std::collections::BTreeMap;

/// The parse state and parser-retained source value for one extracted field.
#[derive(Clone, Debug, PartialEq)]
pub enum CameraField<T> {
    /// No supported serialized key for this property occurs in the source mapping.
    Missing,
    /// The field was parsed into a supported value.
    Present {
        /// Parser-retained source value before typed conversion.
        source_value: CameraSourceValue,
        /// Typed value decoded from the source representation.
        value: T,
    },
    /// The field occurs, but could not be converted into its supported value.
    Invalid {
        /// Parser-retained source value that failed typed conversion.
        source_value: CameraSourceValue,
        /// Stable field-specific conversion diagnostic.
        diagnostic: String,
    },
}

/// An owned value from the bounded scene parser, retaining scalar kind and
/// collection shape for Camera fields.
#[derive(Clone, Debug, PartialEq)]
pub enum CameraSourceValue {
    /// An unquoted scalar token.
    Scalar(String),
    /// A quoted scalar payload as represented by the parser.
    Quoted(String),
    /// Mapping entries with their parsed keys and values.
    Map(BTreeMap<String, CameraSourceValue>),
    /// Sequence entries in parser order.
    Sequence(Vec<CameraSourceValue>),
}

impl CameraSourceValue {
    pub(super) fn from_node(node: &Node) -> Self {
        match node {
            Node::Scalar(value) => Self::Scalar(value.clone()),
            Node::Quoted(value) => Self::Quoted(value.clone()),
            Node::Map(entries) => Self::Map(
                entries
                    .iter()
                    .map(|(key, value)| (key.clone(), Self::from_node(value)))
                    .collect(),
            ),
            Node::Seq(values) => Self::Sequence(values.iter().map(Self::from_node).collect()),
        }
    }
}
