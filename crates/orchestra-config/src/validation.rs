//! Semantic configuration validation engine.

use crate::schema::OrchestraConfig;
use orchestra_common::constants::is_k8s_version_supported;
use orchestra_common::utilities::{
    cidrs_do_not_overlap, is_valid_dns_subdomain, is_valid_label_key, is_valid_label_value,
    is_valid_port, parse_cidr,
};
use orchestra_core::error::ConfigError;
use orchestra_core::network::{CniPlugin, ServiceProxyMode};
use std::collections::HashSet;

/// Validates an `OrchestraConfig` against all structural and semantic rules.
pub fn validate_config(config: &OrchestraConfig) -> Result<(), ConfigError> {
    // 1. Metadata validation
    let name = &config.metadata.name;
    if !is_valid_dns_subdomain(name) {
        return Err(ConfigError::ValidationError(format!(
            "Cluster name '{}' is invalid: must be an RFC 1123 DNS subdomain (lowercase alphanumeric, '-', max 63 chars)",
            name
        )));
    }

    for (k, v) in &config.metadata.labels {
        if !is_valid_label_key(k) {
            return Err(ConfigError::ValidationError(format!(
                "Invalid metadata label key '{}'",
                k
            )));
        }
        if !is_valid_label_value(v) {
            return Err(ConfigError::ValidationError(format!(
                "Invalid metadata label value '{}' for key '{}'",
                v, k
            )));
        }
    }

    // 2. Kubernetes version validation
    if !is_k8s_version_supported(&config.spec.kubernetes_version) {
        return Err(ConfigError::ValidationError(format!(
            "Unsupported Kubernetes version '{}'. Actively supported versions: 1.28 - 1.32",
            config.spec.kubernetes_version
        )));
    }

    // 3. Node count validation
    let cp_count = config.spec.nodes.control_plane.count;
    if cp_count == 0 {
        return Err(ConfigError::ValidationError(
            "Control plane count must be at least 1".to_string(),
        ));
    }
    if cp_count > 1 && cp_count % 2 == 0 {
        return Err(ConfigError::ValidationError(format!(
            "High-availability control plane count should be an odd number (3, 5, etc.) for etcd quorum, got {}",
            cp_count
        )));
    }

    // 4. CIDR validations & overlap checks
    let pod_net = parse_cidr(&config.spec.network.pod_cidr)
        .map_err(|e| ConfigError::ValidationError(format!("Invalid pod_cidr: {}", e)))?;
    let svc_net = parse_cidr(&config.spec.network.service_cidr)
        .map_err(|e| ConfigError::ValidationError(format!("Invalid service_cidr: {}", e)))?;
    let br_net = parse_cidr(&config.spec.network.bridge_subnet)
        .map_err(|e| ConfigError::ValidationError(format!("Invalid bridge_subnet: {}", e)))?;

    if !cidrs_do_not_overlap(&pod_net, &svc_net) {
        return Err(ConfigError::ValidationError(format!(
            "pod_cidr ({}) and service_cidr ({}) overlap",
            pod_net, svc_net
        )));
    }
    if !cidrs_do_not_overlap(&pod_net, &br_net) {
        return Err(ConfigError::ValidationError(format!(
            "pod_cidr ({}) and bridge_subnet ({}) overlap",
            pod_net, br_net
        )));
    }
    if !cidrs_do_not_overlap(&svc_net, &br_net) {
        return Err(ConfigError::ValidationError(format!(
            "service_cidr ({}) and bridge_subnet ({}) overlap",
            svc_net, br_net
        )));
    }

    // 5. Port validations
    if !is_valid_port(config.spec.network.api_server_port as u32) {
        return Err(ConfigError::ValidationError(format!(
            "Invalid api_server_port: {}",
            config.spec.network.api_server_port
        )));
    }

    let mut mapped_ports = HashSet::new();
    mapped_ports.insert(config.spec.network.api_server_port);

    for mapping in &config.spec.nodes.control_plane.port_mappings {
        if !is_valid_port(mapping.container_port as u32) || !is_valid_port(mapping.host_port as u32)
        {
            return Err(ConfigError::ValidationError(format!(
                "Invalid port mapping: container_port {}, host_port {}",
                mapping.container_port, mapping.host_port
            )));
        }
        if !mapped_ports.insert(mapping.host_port) {
            return Err(ConfigError::ValidationError(format!(
                "Host port collision on port {}",
                mapping.host_port
            )));
        }
    }

    // 6. Component compatibility rules
    if config.spec.network.service_proxy == ServiceProxyMode::Orchestra && !config.spec.ebpf.enabled
    {
        return Err(ConfigError::ValidationError(
            "Service proxy mode 'orchestra' requires ebpf.enabled = true".to_string(),
        ));
    }

    if config.spec.network.service_proxy == ServiceProxyMode::Cni
        && matches!(
            config.spec.network.cni,
            CniPlugin::None | CniPlugin::OrchestraNet
        )
    {
        return Err(ConfigError::ValidationError(format!(
            "Service proxy mode 'cni' is incompatible with CNI plugin '{}' (requires Cilium or Calico)",
            config.spec.network.cni
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::MetadataConfig;

    #[test]
    fn test_valid_config() {
        let mut cfg = OrchestraConfig {
            api_version: "orchestra.io/v1alpha1".into(),
            kind: "Cluster".into(),
            metadata: MetadataConfig::new("valid-cluster"),
            spec: Default::default(),
        };
        assert!(validate_config(&cfg).is_ok());

        cfg.metadata.name = "INVALID_UPPERCASE".into();
        assert!(validate_config(&cfg).is_err());
    }

    #[test]
    fn test_overlapping_cidrs_rejected() {
        let mut cfg = OrchestraConfig {
            api_version: "orchestra.io/v1alpha1".into(),
            kind: "Cluster".into(),
            metadata: MetadataConfig::new("cidr-overlap"),
            spec: Default::default(),
        };
        cfg.spec.network.pod_cidr = "10.0.0.0/16".into();
        cfg.spec.network.service_cidr = "10.0.1.0/24".into(); // Inside 10.0.0.0/16!
        let err = validate_config(&cfg).unwrap_err();
        assert!(err.to_string().contains("overlap"));
    }

    #[test]
    fn test_proxy_compatibility_rejected() {
        let mut cfg = OrchestraConfig {
            api_version: "orchestra.io/v1alpha1".into(),
            kind: "Cluster".into(),
            metadata: MetadataConfig::new("proxy-test"),
            spec: Default::default(),
        };
        cfg.spec.network.service_proxy = ServiceProxyMode::Orchestra;
        cfg.spec.ebpf.enabled = false;
        let err = validate_config(&cfg).unwrap_err();
        assert!(err.to_string().contains("requires ebpf.enabled = true"));
    }
}
