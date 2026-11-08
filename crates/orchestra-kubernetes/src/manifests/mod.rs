//! Standard CNI and cluster addon manifest templates.

use orchestra_core::cluster::ClusterSpec;
use orchestra_core::network::CniPlugin;

/// Generates standard CNI plugin manifest based on cluster specification.
pub fn generate_cni_manifest(spec: &ClusterSpec) -> Option<String> {
    match spec.cni_plugin {
        CniPlugin::OrchestraNet => Some(orchestranet_manifest(&spec.pod_cidr)),
        CniPlugin::Flannel => Some(flannel_manifest(&spec.pod_cidr)),
        CniPlugin::Calico | CniPlugin::Cilium | CniPlugin::None => None,
    }
}

/// Generates Orchestra's native lightweight CNI manifest.
fn orchestranet_manifest(pod_cidr: &str) -> String {
    format!(
        r#"apiVersion: v1
kind: ConfigMap
metadata:
  name: orchestranet-config
  namespace: kube-system
data:
  pod-cidr: "{pod_cidr}"
"#
    )
}

/// Generates flannel CNI network manifest.
fn flannel_manifest(pod_cidr: &str) -> String {
    format!(
        r#"apiVersion: v1
kind: ConfigMap
metadata:
  name: kube-flannel-cfg
  namespace: kube-system
data:
  net-conf.json: |
    {{
      "Network": "{pod_cidr}",
      "Backend": {{
        "Type": "vxlan"
      }}
    }}
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cni_manifest_generation() {
        let mut spec = ClusterSpec::new("dev", "v1.36.6");
        spec.cni_plugin = CniPlugin::OrchestraNet;
        let manifest = generate_cni_manifest(&spec).unwrap();
        assert!(manifest.contains("orchestranet-config"));
        assert!(manifest.contains("10.244.0.0/16"));
    }
}
