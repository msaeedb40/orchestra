//! Container lifecycle management for Orchestra node containers.

use crate::client::DockerClient;
use bollard::container::{
    Config, CreateContainerOptions, InspectContainerOptions, LogsOptions, RemoveContainerOptions,
    StartContainerOptions, StopContainerOptions,
};
use bollard::exec::{CreateExecOptions, StartExecResults};
use bollard::models::{HostConfig, Mount, MountTypeEnum};
use futures::StreamExt;
use orchestra_core::error::RuntimeError;
use orchestra_core::node::{Node, NodeRole};
use std::collections::HashMap;
use tracing::{debug, info, warn};

/// Manages the full lifecycle of individual node Docker containers.
pub struct ContainerManager {
    client: DockerClient,
}

impl ContainerManager {
    pub fn new(client: DockerClient) -> Self {
        Self { client }
    }

    /// Creates (but does not start) a node container with appropriate configuration.
    /// Returns the Docker container ID.
    pub async fn create(
        &self,
        node: &Node,
        cluster_name: &str,
        network_name: &str,
        image: &str,
    ) -> Result<String, RuntimeError> {
        let container_name = node.name.clone();

        // Idempotency: return existing container if already created.
        if let Some(id) = self.find_container(&container_name).await? {
            debug!(container = %container_name, "Container already exists, reusing");
            return Ok(id);
        }

        info!(container = %container_name, role = %node.role, image, "Creating node container");

        let mut labels = HashMap::new();
        labels.insert("orchestra.io/cluster".to_string(), cluster_name.to_string());
        labels.insert("orchestra.io/role".to_string(), node.role.to_string());
        labels.insert("orchestra.io/node-id".to_string(), node.id.to_string());
        labels.insert("orchestra.io/managed".to_string(), "true".to_string());

        // Mounts required for kubelet and container runtime inside the node container.
        let mounts = node_mounts();

        // Linux capabilities required for kubelet + container runtime.
        let cap_add = node_capabilities(&node.role);

        // Port mappings for control-plane API server.
        let port_bindings = if node.role.is_control_plane() {
            Some(control_plane_port_bindings())
        } else {
            None
        };

        let host_config = HostConfig {
            privileged: Some(true), // Required for nested container runtime (containerd)
            mounts: Some(mounts),
            cap_add: Some(cap_add),
            port_bindings,
            network_mode: Some(network_name.to_string()),
            tmpfs: Some({
                let mut t = HashMap::new();
                t.insert("/tmp".to_string(), String::new());
                t.insert("/run".to_string(), String::new());
                t
            }),
            ..Default::default()
        };

        let config: Config<String> = Config {
            image: Some(image.to_string()),
            hostname: Some(node.name.clone()),
            labels: Some(labels),
            host_config: Some(host_config),
            ..Default::default()
        };

        let opts = CreateContainerOptions {
            name: container_name.as_str(),
            platform: None,
        };

        let response = self
            .client
            .inner()
            .create_container(Some(opts), config)
            .await
            .map_err(|e| {
                RuntimeError::Container(format!(
                    "Failed to create container '{container_name}': {e}"
                ))
            })?;

        info!(container = %container_name, id = %response.id, "Container created");
        Ok(response.id)
    }

    /// Starts a created or stopped container.
    pub async fn start(&self, container_id: &str) -> Result<(), RuntimeError> {
        debug!(container_id, "Starting container");
        self.client
            .inner()
            .start_container(container_id, None::<StartContainerOptions<String>>)
            .await
            .map_err(|e| {
                RuntimeError::Container(format!("Failed to start '{container_id}': {e}"))
            })?;
        info!(container_id, "Container started");
        Ok(())
    }

    /// Gracefully stops a running container.
    pub async fn stop(&self, container_id: &str, timeout_secs: u32) -> Result<(), RuntimeError> {
        debug!(container_id, timeout_secs, "Stopping container");
        let opts = StopContainerOptions {
            t: timeout_secs as i64,
        };
        self.client
            .inner()
            .stop_container(container_id, Some(opts))
            .await
            .map_err(|e| {
                RuntimeError::Container(format!("Failed to stop '{container_id}': {e}"))
            })?;
        info!(container_id, "Container stopped");
        Ok(())
    }

    /// Removes a container and its ephemeral volumes.
    pub async fn remove(&self, container_id: &str) -> Result<(), RuntimeError> {
        debug!(container_id, "Removing container");
        let opts = RemoveContainerOptions {
            force: true,
            v: true,
            ..Default::default()
        };
        self.client
            .inner()
            .remove_container(container_id, Some(opts))
            .await
            .map_err(|e| {
                RuntimeError::Container(format!("Failed to remove '{container_id}': {e}"))
            })?;
        info!(container_id, "Container removed");
        Ok(())
    }

    /// Executes a command inside a container and returns (exit_code, combined output).
    pub async fn exec(
        &self,
        container_id: &str,
        cmd: &[&str],
    ) -> Result<(i64, String), RuntimeError> {
        debug!(container_id, ?cmd, "Executing command in container");

        let exec_opts = CreateExecOptions {
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            cmd: Some(cmd.iter().map(|s| s.to_string()).collect()),
            ..Default::default()
        };

        let exec_id = self
            .client
            .inner()
            .create_exec(container_id, exec_opts)
            .await
            .map_err(|e| RuntimeError::Exec(format!("Failed to create exec: {e}")))?;

        let start_result = self
            .client
            .inner()
            .start_exec(&exec_id.id, None)
            .await
            .map_err(|e| RuntimeError::Exec(format!("Failed to start exec: {e}")))?;

        let mut output = String::new();
        if let StartExecResults::Attached {
            output: mut stream, ..
        } = start_result
        {
            while let Some(chunk) = stream.next().await {
                match chunk {
                    Ok(log) => output.push_str(&log.to_string()),
                    Err(e) => warn!("Exec output error: {e}"),
                }
            }
        }

        // Inspect exit code.
        let exec_inspect = self
            .client
            .inner()
            .inspect_exec(&exec_id.id)
            .await
            .map_err(|e| RuntimeError::Exec(format!("Failed to inspect exec: {e}")))?;

        let exit_code = exec_inspect.exit_code.unwrap_or(-1);
        Ok((exit_code, output))
    }

    /// Inspects a container and returns whether it is running and its IP address.
    pub async fn inspect(
        &self,
        container_id: &str,
    ) -> Result<(bool, Option<String>), RuntimeError> {
        let info = self
            .client
            .inner()
            .inspect_container(container_id, None::<InspectContainerOptions>)
            .await
            .map_err(|e| {
                RuntimeError::Container(format!("Failed to inspect '{container_id}': {e}"))
            })?;

        let is_running = info.state.as_ref().and_then(|s| s.running).unwrap_or(false);

        let ip = info
            .network_settings
            .as_ref()
            .and_then(|ns| ns.ip_address.clone())
            .filter(|ip| !ip.is_empty());

        Ok((is_running, ip))
    }

    /// Finds a container by name and returns its ID if present.
    pub async fn find_container(&self, name: &str) -> Result<Option<String>, RuntimeError> {
        let mut filters = HashMap::new();
        filters.insert("name".to_string(), vec![format!("^/{}$", name)]);

        let opts = bollard::container::ListContainersOptions::<String> {
            all: true,
            filters,
            ..Default::default()
        };

        let containers = self
            .client
            .inner()
            .list_containers(Some(opts))
            .await
            .map_err(|e| RuntimeError::Container(format!("Failed to list containers: {e}")))?;

        Ok(containers.into_iter().find_map(|c| c.id))
    }

    /// Streams the last N log lines from a container as a String.
    pub async fn logs(&self, container_id: &str, tail: usize) -> Result<String, RuntimeError> {
        let opts = LogsOptions::<String> {
            stdout: true,
            stderr: true,
            tail: tail.to_string(),
            ..Default::default()
        };

        let mut output = String::new();
        let mut stream = self.client.inner().logs(container_id, Some(opts));

        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(log) => output.push_str(&log.to_string()),
                Err(e) => warn!("Log streaming error: {e}"),
            }
        }
        Ok(output)
    }
}

/// Returns the list of Linux capabilities required for a Kubernetes node container.
fn node_capabilities(role: &NodeRole) -> Vec<String> {
    let mut caps = vec![
        "NET_ADMIN".to_string(),
        "NET_RAW".to_string(),
        "SYS_ADMIN".to_string(),
        "SYS_MODULE".to_string(),
        "SYS_PTRACE".to_string(),
        "AUDIT_WRITE".to_string(),
        "KILL".to_string(),
        "MKNOD".to_string(),
        "SETFCAP".to_string(),
    ];
    if role.is_control_plane() {
        caps.push("DAC_OVERRIDE".to_string());
    }
    caps
}

/// Returns the mounts required inside a node container for kubelet + containerd.
fn node_mounts() -> Vec<Mount> {
    vec![
        Mount {
            target: Some("/lib/modules".to_string()),
            source: Some("/lib/modules".to_string()),
            typ: Some(MountTypeEnum::BIND),
            read_only: Some(true),
            bind_options: Some(bollard::models::MountBindOptions {
                propagation: Some(bollard::models::MountBindOptionsPropagationEnum::RPRIVATE),
                ..Default::default()
            }),
            ..Default::default()
        },
        Mount {
            target: Some("/sys/fs/cgroup".to_string()),
            typ: Some(MountTypeEnum::TMPFS),
            read_only: Some(false),
            tmpfs_options: Some(bollard::models::MountTmpfsOptions {
                // 256 MB is plenty for the cgroup hierarchy
                size_bytes: Some(256 * 1024 * 1024),
                ..Default::default()
            }),
            ..Default::default()
        },
    ]
}

/// Returns the port bindings for a control-plane container (expose API server).
fn control_plane_port_bindings() -> HashMap<String, Option<Vec<bollard::models::PortBinding>>> {
    let mut bindings = HashMap::new();
    bindings.insert(
        "6443/tcp".to_string(),
        Some(vec![bollard::models::PortBinding {
            host_ip: Some("127.0.0.1".to_string()),
            host_port: Some("0".to_string()), // ephemeral port
        }]),
    );
    bindings
}
