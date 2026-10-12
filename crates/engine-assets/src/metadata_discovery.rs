#[path = "metadata_discovery_error.rs"]
mod metadata_discovery_error;
#[path = "metadata_scanner.rs"]
mod metadata_scanner;

pub use metadata_discovery_error::{MetadataDiscoveryError, MetadataDiscoveryLimits};
pub use metadata_scanner::discover_metadata_index;
