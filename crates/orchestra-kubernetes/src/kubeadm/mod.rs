//! Kubeadm configuration generation and command wrappers.

use orchestra_core::cluster::ClusterSpec;
use orchestra_core::error::KubernetesError;
use orchestra_core::network::ServiceProxyMode;

/// Generates the kubeadm init configuration YAML for a control plane node.
pub fn generate_init_config(
    spec: &ClusterSpec,
    control_plane_endpoint: &str,
) -> Result<String, KubernetesError> {
    let skip_kube_proxy = matches!(spec.service_proxy_mode, ServiceProxyMode::Orchestra);

    let mut config = format!(
        r#"apiVersion: kubeadm.k8s.io/v1beta3
kind: InitConfiguration
localAPIEndpoint:
  advertiseAddress: "0.0.0.0"
  bindPort: 6443
nodeRegistration:
  criSocket: "unix:///run/containerd/containerd.sock"
  imagePullPolicy: IfNotPresent
---
apiVersion: kubeadm.k8s.io/v1beta3
kind: ClusterConfiguration
kubernetesVersion: "{k8s_version}"
controlPlaneEndpoint: "{endpoint}:6443"
networking:
  podSubnet: "{pod_cidr}"
  serviceSubnet: "{service_cidr}"
apiServer:
  certSANs:
    - "127.0.0.1"
    - "localhost"
    - "{endpoint}"
"#,
        k8s_version = spec.kubernetes_version,
        endpoint = control_plane_endpoint,
        pod_cidr = spec.pod_cidr,
        service_cidr = spec.service_cidr,
    );

    if skip_kube_proxy {
        // Feature gate or phase skip can be configured, or kube-proxy daemonset removed post-init
        config.push_str(
            r#"---
apiVersion: kubeproxy.config.k8s.io/v1alpha1
kind: KubeProxyConfiguration
# Disabled when Orchestra eBPF proxy mode is active
"#,
        );
    }

    Ok(config)
}

/// Generates the kubeadm join configuration YAML for a worker node.
pub fn generate_join_config(
    control_plane_endpoint: &str,
    token: &str,
    ca_cert_hash: &str,
) -> String {
    format!(
        r#"apiVersion: kubeadm.k8s.io/v1beta3
kind: JoinConfiguration
discovery:
  bootstrapToken:
    apiServerEndpoint: "{endpoint}:6443"
    token: "{token}"
    caCertHashes:
      - "{hash}"
nodeRegistration:
  criSocket: "unix:///run/containerd/containerd.sock"
"#,
        endpoint = control_plane_endpoint,
        token = token,
        hash = ca_cert_hash,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_init_config() {
        let spec = ClusterSpec::new("my-cluster", "v1.36.6");
        let yaml = generate_init_config(&spec, "172.20.0.2").unwrap();

        assert!(yaml.contains("kubernetesVersion: \"v1.36.6\""));
        assert!(yaml.contains("controlPlaneEndpoint: \"172.20.0.2:6443\""));
        assert!(yaml.contains("podSubnet: \"10.244.0.0/16\""));
        assert!(yaml.contains("serviceSubnet: \"10.96.0.0/12\""));
    }

    #[test]
    fn test_generate_join_config() {
        let yaml = generate_join_config("172.20.0.2", "abcdef.0123456789abcdef", "sha256:123456");
        assert!(yaml.contains("apiServerEndpoint: \"172.20.0.2:6443\""));
        assert!(yaml.contains("token: \"abcdef.0123456789abcdef\""));
        assert!(yaml.contains("sha256:123456"));
    }
}
