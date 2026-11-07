//! YAML and TOML configuration parsers and serializes.

use crate::schema::OrchestraConfig;
use crate::versioning::check_api_version;
use orchestra_core::error::ConfigError;

/// Parses a YAML configuration string into an `OrchestraConfig`.
pub fn parse_yaml(content: &str) -> Result<OrchestraConfig, ConfigError> {
    let config: OrchestraConfig = serde_yaml::from_str(content)
        .map_err(|e| ConfigError::ParseError(format!("Failed to parse YAML: {}", e)))?;
    check_api_version(&config.api_version)?;
    Ok(config)
}

/// Parses a TOML configuration string into an `OrchestraConfig`.
pub fn parse_toml(content: &str) -> Result<OrchestraConfig, ConfigError> {
    let config: OrchestraConfig = toml::from_str(content)
        .map_err(|e| ConfigError::ParseError(format!("Failed to parse TOML: {}", e)))?;
    check_api_version(&config.api_version)?;
    Ok(config)
}

/// Serializes an `OrchestraConfig` to a YAML string.
pub fn to_yaml(config: &OrchestraConfig) -> Result<String, ConfigError> {
    serde_yaml::to_string(config)
        .map_err(|e| ConfigError::ParseError(format!("Failed to serialize YAML: {}", e)))
}

/// Serializes an `OrchestraConfig` to a formatted TOML string.
pub fn to_toml(config: &OrchestraConfig) -> Result<String, ConfigError> {
    toml::to_string_pretty(config)
        .map_err(|e| ConfigError::ParseError(format!("Failed to serialize TOML: {}", e)))
}

/// Serializes an `OrchestraConfig` to pretty JSON.
pub fn to_json(config: &OrchestraConfig) -> Result<String, ConfigError> {
    serde_json::to_string_pretty(config)
        .map_err(|e| ConfigError::ParseError(format!("Failed to serialize JSON: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::defaults::{Profile, create_profile_config};

    #[test]
    fn test_yaml_roundtrip() {
        let original = create_profile_config("yaml-test", Profile::Dev);
        let yaml_str = to_yaml(&original).unwrap();
        assert!(yaml_str.contains("apiVersion: orchestra.io/v1alpha1"));
        assert!(yaml_str.contains("kind: Cluster"));

        let parsed = parse_yaml(&yaml_str).unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn test_toml_roundtrip() {
        let original = create_profile_config("toml-test", Profile::Dev);
        let toml_str = to_toml(&original).unwrap();
        assert!(toml_str.contains("apiVersion = \"orchestra.io/v1alpha1\""));

        let parsed = parse_toml(&toml_str).unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn test_malformed_yaml() {
        let malformed = "apiVersion: orchestra.io/v1alpha1\nkind: Cluster\nmetadata: [invalid";
        assert!(parse_yaml(malformed).is_err());
    }
}
