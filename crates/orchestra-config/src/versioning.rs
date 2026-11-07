//! API versioning and schema evolution.

use crate::schema::CURRENT_API_VERSION;
use orchestra_core::error::ConfigError;

/// List of currently accepted apiVersion values.
pub const SUPPORTED_API_VERSIONS: &[&str] =
    &["orchestra.io/v1alpha1", "orchestra.io/v1beta1", "v1alpha1"];

/// Validates that an apiVersion string is recognized.
pub fn check_api_version(version: &str) -> Result<(), ConfigError> {
    if SUPPORTED_API_VERSIONS.contains(&version) {
        Ok(())
    } else {
        Err(ConfigError::ValidationError(format!(
            "Unsupported apiVersion '{}'. Supported: {:?}",
            version, SUPPORTED_API_VERSIONS
        )))
    }
}

/// Normalizes legacy or shorthand apiVersion strings to current canonical version.
pub fn normalize_api_version(version: &str) -> String {
    if version == "v1alpha1" {
        CURRENT_API_VERSION.to_string()
    } else {
        version.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_validation() {
        assert!(check_api_version("orchestra.io/v1alpha1").is_ok());
        assert!(check_api_version("v1alpha1").is_ok());
        assert!(check_api_version("orchestra.io/v999").is_err());
    }

    #[test]
    fn test_version_normalization() {
        assert_eq!(normalize_api_version("v1alpha1"), CURRENT_API_VERSION);
        assert_eq!(
            normalize_api_version(CURRENT_API_VERSION),
            CURRENT_API_VERSION
        );
    }
}
