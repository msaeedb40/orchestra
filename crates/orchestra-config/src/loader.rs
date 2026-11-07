//! Layered configuration loading and merging engine.

use crate::defaults::{Profile, create_profile_config};
use crate::parser::{parse_toml, parse_yaml};
use crate::schema::OrchestraConfig;
use crate::validation::validate_config;
use orchestra_core::error::ConfigError;
use std::path::{Path, PathBuf};

/// Programmatic overrides supplied via CLI flags.
#[derive(Debug, Clone, Default)]
pub struct ConfigOverrides {
    pub name: Option<String>,
    pub kubernetes_version: Option<String>,
    pub control_plane_count: Option<usize>,
    pub worker_count: Option<usize>,
    pub cni_plugin: Option<String>,
    pub log_level: Option<String>,
    pub ebpf_enabled: Option<bool>,
}

/// Standard candidate paths for locating an Orchestra config file.
pub fn find_default_config_path() -> Option<PathBuf> {
    let candidates = [
        PathBuf::from("orchestra.yaml"),
        PathBuf::from("orchestra.yml"),
        PathBuf::from("orchestra.toml"),
        PathBuf::from(".orchestra.yaml"),
    ];

    for path in &candidates {
        if path.is_file() {
            return Some(path.clone());
        }
    }

    if let Ok(home) = std::env::var("HOME") {
        let user_path = Path::new(&home).join(".orchestra/config.yaml");
        if user_path.is_file() {
            return Some(user_path);
        }
    }

    None
}

/// Loads an `OrchestraConfig` from a specified file path.
pub fn load_from_file(path: impl AsRef<Path>) -> Result<OrchestraConfig, ConfigError> {
    let path = path.as_ref();
    if !path.exists() {
        return Err(ConfigError::FileNotFound(path.display().to_string()));
    }

    let content = std::fs::read_to_string(path).map_err(|e| {
        ConfigError::ParseError(format!("Failed to read '{}': {}", path.display(), e))
    })?;

    let is_toml = path.extension().and_then(|ext| ext.to_str()) == Some("toml");
    if is_toml {
        parse_toml(&content)
    } else {
        parse_yaml(&content)
    }
}

/// Loads a fully merged and validated `OrchestraConfig` following the hierarchy:
/// Defaults (Profile) -> Config File -> Environment Variables -> CLI Overrides.
pub fn load_config(
    file_path: Option<&Path>,
    profile: Option<Profile>,
    overrides: Option<ConfigOverrides>,
) -> Result<OrchestraConfig, ConfigError> {
    // 1. Base layer from profile defaults
    let selected_profile = profile.unwrap_or(Profile::Dev);
    let mut config = create_profile_config("cluster", selected_profile);

    // 2. Layer from config file if provided or discovered
    let target_file = file_path
        .map(PathBuf::from)
        .or_else(find_default_config_path);

    if let Some(path) = target_file {
        if path.exists() {
            let file_config = load_from_file(&path)?;
            merge_config(&mut config, file_config);
        }
    }

    // 3. Layer from environment variables
    apply_env_overrides(&mut config);

    // 4. Layer from CLI flag overrides
    if let Some(ov) = overrides {
        apply_cli_overrides(&mut config, ov);
    }

    // 5. Final semantic validation
    validate_config(&config)?;

    Ok(config)
}

fn merge_config(base: &mut OrchestraConfig, incoming: OrchestraConfig) {
    if !incoming.metadata.name.is_empty() {
        base.metadata.name = incoming.metadata.name;
    }
    base.metadata.labels.extend(incoming.metadata.labels);
    base.metadata
        .annotations
        .extend(incoming.metadata.annotations);

    base.spec.kubernetes_version = incoming.spec.kubernetes_version;
    base.spec.nodes = incoming.spec.nodes;
    base.spec.network = incoming.spec.network;
    base.spec.ebpf = incoming.spec.ebpf;
    base.spec.runtime = incoming.spec.runtime;
    base.spec.observability = incoming.spec.observability;
    base.spec.security = incoming.spec.security;
}

fn apply_env_overrides(config: &mut OrchestraConfig) {
    if let Ok(name) = std::env::var("ORCHESTRA_CLUSTER_NAME") {
        config.metadata.name = name;
    }
    if let Ok(k8s) = std::env::var("ORCHESTRA_K8S_VERSION") {
        config.spec.kubernetes_version = k8s;
    }
    if let Ok(cp) = std::env::var("ORCHESTRA_CP_COUNT") {
        if let Ok(n) = cp.parse() {
            config.spec.nodes.control_plane.count = n;
        }
    }
    if let Ok(workers) = std::env::var("ORCHESTRA_WORKER_COUNT") {
        if let Ok(n) = workers.parse() {
            config.spec.nodes.workers.count = n;
        }
    }
    if let Ok(log_level) = std::env::var("ORCHESTRA_LOG_LEVEL") {
        config.spec.observability.log_level = log_level;
    }
    if let Ok(ebpf) = std::env::var("ORCHESTRA_EBPF_ENABLED") {
        config.spec.ebpf.enabled = ebpf == "true" || ebpf == "1";
    }
}

fn apply_cli_overrides(config: &mut OrchestraConfig, overrides: ConfigOverrides) {
    if let Some(name) = overrides.name {
        config.metadata.name = name;
    }
    if let Some(k8s) = overrides.kubernetes_version {
        config.spec.kubernetes_version = k8s;
    }
    if let Some(cp) = overrides.control_plane_count {
        config.spec.nodes.control_plane.count = cp;
    }
    if let Some(workers) = overrides.worker_count {
        config.spec.nodes.workers.count = workers;
    }
    if let Some(log_level) = overrides.log_level {
        config.spec.observability.log_level = log_level;
    }
    if let Some(ebpf) = overrides.ebpf_enabled {
        config.spec.ebpf.enabled = ebpf;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_config_with_overrides() {
        let overrides = ConfigOverrides {
            name: Some("overridden-name".into()),
            kubernetes_version: Some("v1.31.2".into()),
            control_plane_count: Some(1),
            worker_count: Some(4),
            ..Default::default()
        };

        let cfg = load_config(None, Some(Profile::Dev), Some(overrides)).unwrap();
        assert_eq!(cfg.metadata.name, "overridden-name");
        assert_eq!(cfg.spec.nodes.workers.count, 4);
    }
}
