//! Configuration loading, parsing, validation, and profile management for Orchestra.

pub mod defaults;
pub mod loader;
pub mod parser;
pub mod schema;
pub mod validation;
pub mod versioning;

pub use defaults::{Profile, create_profile_config};
pub use loader::{ConfigOverrides, find_default_config_path, load_config, load_from_file};
pub use parser::{parse_toml, parse_yaml, to_json, to_toml, to_yaml};
pub use schema::{
    CLUSTER_KIND, CURRENT_API_VERSION, ClusterConfigSpec, ControlPlaneNodeConfig, EbpfConfigSpec,
    MetadataConfig, MountConfig, NetworkConfigSpec, NodeConfigSpec, ObservabilityConfigSpec,
    OrchestraConfig, PortMapping, RuntimeConfigSpec, SecurityConfigSpec, WorkerNodeConfig,
};
pub use validation::validate_config;
pub use versioning::{SUPPORTED_API_VERSIONS, check_api_version, normalize_api_version};
