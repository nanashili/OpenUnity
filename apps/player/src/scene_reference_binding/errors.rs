use std::fmt::{self, Display, Formatter};

/// Stable failure categories emitted while pairing extracted render components.
#[derive(Debug)]
pub(super) enum PairingError {
    InvalidDrawOwner {
        owner: i64,
    },
    DuplicateDrawOwner {
        owner: i64,
    },
    DuplicateComponent {
        owner: i64,
        component: ComponentKind,
        component_file_id: i64,
    },
    MissingComponent {
        owner: i64,
        component: ComponentKind,
    },
    UncoveredComponent {
        owner: i64,
        component: ComponentKind,
        component_file_id: i64,
    },
}

/// The extracted component class and its serialized reference field.
#[derive(Clone, Copy, Debug)]
pub(super) enum ComponentKind {
    MeshFilter,
    MeshRenderer,
}

impl ComponentKind {
    pub(super) const fn name(self) -> &'static str {
        match self {
            Self::MeshFilter => "MeshFilter",
            Self::MeshRenderer => "MeshRenderer",
        }
    }

    pub(super) const fn field(self) -> &'static str {
        match self {
            Self::MeshFilter => "m_Mesh",
            Self::MeshRenderer => "m_Materials",
        }
    }
}

impl Display for PairingError {
    fn fmt(&self, output: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDrawOwner { owner } => {
                write!(output, "render-owner: draw owner {owner} must be positive")
            }
            Self::DuplicateDrawOwner { owner } => {
                write!(output, "render-owner: duplicate draw owner {owner}")
            }
            Self::DuplicateComponent {
                owner,
                component,
                component_file_id,
            } => write!(
                output,
                "render-component: owner {owner} has duplicate {} component {component_file_id} for field {}",
                component.name(),
                component.field()
            ),
            Self::MissingComponent { owner, component } => write!(
                output,
                "render-component: owner {owner} is missing {} field {}",
                component.name(),
                component.field()
            ),
            Self::UncoveredComponent {
                owner,
                component,
                component_file_id,
            } => write!(
                output,
                "render-component: owner {owner} component {} ({component_file_id}) field {} is not covered by a draw",
                component.name(),
                component.field()
            ),
        }
    }
}

impl std::error::Error for PairingError {}
