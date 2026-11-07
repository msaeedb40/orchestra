//! Kubeconfig parsing, rewriting, and serialization.

use orchestra_core::error::KubernetesError;

/// Rewrites the `server:` URL in a kubeconfig YAML string to point to the host-mapped port.
pub fn rewrite_server_endpoint(
    kubeconfig_yaml: &str,
    host_port: u16,
) -> Result<String, KubernetesError> {
    let mut rewritten = Vec::new();
    let mut replaced = false;

    for line in kubeconfig_yaml.lines() {
        if line.trim_start().starts_with("server:") {
            rewritten.push(format!("    server: https://127.0.0.1:{host_port}"));
            replaced = true;
        } else {
            rewritten.push(line.to_string());
        }
    }

    if !replaced {
        return Err(KubernetesError::KubeconfigError(
            "Could not find 'server:' endpoint field in kubeconfig".to_string(),
        ));
    }

    Ok(rewritten.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rewrite_server_endpoint() {
        let sample = r#"apiVersion: v1
clusters:
- cluster:
    certificate-authority-data: LS0t...
    server: https://172.20.0.2:6443
  name: orchestra-dev
"#;
        let rewritten = rewrite_server_endpoint(sample, 45678).unwrap();
        assert!(rewritten.contains("server: https://127.0.0.1:45678"));
        assert!(!rewritten.contains("172.20.0.2:6443"));
    }
}
