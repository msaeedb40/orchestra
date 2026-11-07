//! Kubernetes cluster bootstrap coordinator.

use crate::kubeadm::generate_init_config;
use crate::kubeconfig::rewrite_server_endpoint;
use orchestra_core::cluster::ClusterSpec;
use orchestra_core::error::KubernetesError;
use orchestra_core::traits::RuntimeProvider;
use tracing::info;

/// Coordinates the bootstrapping of a Kubernetes cluster via kubeadm.
pub struct ClusterBootstrapper<R: RuntimeProvider> {
    runtime: R,
}

impl<R: RuntimeProvider> ClusterBootstrapper<R> {
    pub fn new(runtime: R) -> Self {
        Self { runtime }
    }

    /// Bootstraps the primary control-plane node.
    pub async fn init_control_plane(
        &self,
        container_id: &str,
        spec: &ClusterSpec,
        node_ip: &str,
        host_port: u16,
    ) -> Result<String, KubernetesError> {
        info!(container_id, "Generating kubeadm configuration");
        let init_config = generate_init_config(spec, node_ip)?;

        // 1. Write kubeadm config inside container
        let write_cmd = format!("cat <<'EOF' > /tmp/kubeadm-config.yaml\n{init_config}\nEOF");
        let (exit_code, output) = self
            .runtime
            .exec_in_node(container_id, &["sh", "-c", &write_cmd])
            .await
            .map_err(|e| KubernetesError::KubeadmInitFailed(e.to_string()))?;

        if exit_code != 0 {
            return Err(KubernetesError::KubeadmInitFailed(format!(
                "Failed to write kubeadm config: {output}"
            )));
        }

        // 2. Execute kubeadm init
        info!(container_id, "Executing kubeadm init");
        let init_cmd =
            "kubeadm init --config=/tmp/kubeadm-config.yaml --ignore-preflight-errors=all --v=5";
        let (exit_code, output) = self
            .runtime
            .exec_in_node(container_id, &["sh", "-c", init_cmd])
            .await
            .map_err(|e| KubernetesError::KubeadmInitFailed(e.to_string()))?;

        if exit_code != 0 {
            return Err(KubernetesError::KubeadmInitFailed(format!(
                "kubeadm init failed with code {exit_code}: {output}"
            )));
        }

        // 3. Extract admin kubeconfig
        let (exit_code, raw_kubeconfig) = self
            .runtime
            .exec_in_node(container_id, &["cat", "/etc/kubernetes/admin.conf"])
            .await
            .map_err(|e| KubernetesError::KubeconfigError(e.to_string()))?;

        if exit_code != 0 {
            return Err(KubernetesError::KubeconfigError(format!(
                "Failed to read admin.conf: {raw_kubeconfig}"
            )));
        }

        // 4. Rewrite server endpoint for host access
        let host_kubeconfig = rewrite_server_endpoint(&raw_kubeconfig, host_port)?;
        info!(container_id, "Control plane initialized successfully");

        Ok(host_kubeconfig)
    }

    /// Joins a worker node to the cluster.
    pub async fn join_worker(
        &self,
        worker_container_id: &str,
        join_command: &str,
    ) -> Result<(), KubernetesError> {
        info!(worker_container_id, "Executing kubeadm join on worker node");
        let full_cmd = format!("{join_command} --ignore-preflight-errors=all");

        let (exit_code, output) = self
            .runtime
            .exec_in_node(worker_container_id, &["sh", "-c", &full_cmd])
            .await
            .map_err(|e| KubernetesError::KubeadmJoinFailed {
                node_id: worker_container_id.to_string(),
                message: e.to_string(),
            })?;

        if exit_code != 0 {
            return Err(KubernetesError::KubeadmJoinFailed {
                node_id: worker_container_id.to_string(),
                message: output,
            });
        }

        info!(worker_container_id, "Worker joined cluster successfully");
        Ok(())
    }
}
