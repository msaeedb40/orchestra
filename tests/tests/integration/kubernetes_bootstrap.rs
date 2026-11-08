//! Integration tests for Kubernetes bootstrap engine.

use orchestra_core::cluster::{ClusterSpec, CniPlugin, ServiceProxyMode};
use orchestra_kubernetes::{
    generate_cni_manifest, generate_init_config, generate_join_config, rewrite_server_endpoint,
};

#[test]
fn test_kubeadm_multi_version_matrix() {
    let versions = ["v1.34.0", "v1.35.0", "v1.36.0", "v1.37.0"];
    for version in versions {
        let mut spec = ClusterSpec::new(format!("cluster-{}", &version[1..]), version);
        spec.service_proxy_mode = ServiceProxyMode::Orchestra;

        let init_yaml =
            generate_init_config(&spec, "172.18.0.2").expect("Init config generation failed");

        assert!(init_yaml.contains(&format!("kubernetesVersion: \"{}\"", version)));
        assert!(init_yaml.contains("ClusterConfiguration"));
        assert!(init_yaml.contains("InitConfiguration"));
        // Orchestra mode should skip kube-proxy addon phase
        assert!(init_yaml.contains("KubeProxyConfiguration"));
    }
}

#[test]
fn test_kubeadm_join_config_generation() {
    let join_yaml =
        generate_join_config("172.18.0.2:6443", "abcdef.0123456789abcdef", "sha256:1234");
    assert!(join_yaml.contains("JoinConfiguration"));
    assert!(join_yaml.contains("172.18.0.2:6443"));
    assert!(join_yaml.contains("abcdef.0123456789abcdef"));
}

#[test]
fn test_kubeconfig_rewriter_integration() {
    let raw_kubeconfig = r#"
apiVersion: v1
clusters:
- cluster:
    certificate-authority-data: dGVzdC1jYQ==
    server: https://172.18.0.2:6443
  name: test-cluster
contexts:
- context:
    cluster: test-cluster
    user: kubernetes-admin
  name: kubernetes-admin@test-cluster
current-context: kubernetes-admin@test-cluster
kind: Config
users:
- name: kubernetes-admin
  user:
    client-certificate-data: dGVzdC1jZXJ0
    client-key-data: dGVzdC1rZXk=
"#;

    let rewritten =
        rewrite_server_endpoint(raw_kubeconfig, 32800).expect("Kubeconfig rewriting failed");

    assert!(rewritten.contains("https://127.0.0.1:32800") || rewritten.contains(":32800"));
    assert!(!rewritten.contains("https://172.18.0.2:6443"));
    assert!(rewritten.contains("certificate-authority-data"));
}

#[test]
fn test_cni_manifest_generation() {
    let mut orchestranet_spec = ClusterSpec::new("orchestranet-cluster", "v1.36.6");
    orchestranet_spec.cni_plugin = CniPlugin::OrchestraNet;
    let orchestranet_manifest =
        generate_cni_manifest(&orchestranet_spec).expect("OrchestraNet manifest generation failed");
    assert!(orchestranet_manifest.contains("orchestranet"));
    assert!(orchestranet_manifest.contains("10.244.0.0/16"));

    let mut flannel_spec = ClusterSpec::new("flannel-cluster", "v1.36.6");
    flannel_spec.cni_plugin = CniPlugin::Flannel;
    let flannel_manifest =
        generate_cni_manifest(&flannel_spec).expect("Flannel manifest generation failed");
    assert!(flannel_manifest.contains("kube-flannel"));
}
