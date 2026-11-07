//! Async Docker Engine client wrapper using Bollard.

use bollard::Docker;
use orchestra_core::error::RuntimeError;
use std::sync::Arc;

/// A shared, cheaply-cloneable handle to the Docker Engine API.
#[derive(Clone)]
pub struct DockerClient {
    inner: Arc<Docker>,
}

impl DockerClient {
    /// Connects to the local Docker Engine using the platform-default socket.
    pub fn connect() -> Result<Self, RuntimeError> {
        let docker = Docker::connect_with_local_defaults()
            .map_err(|e| RuntimeError::Connection(e.to_string()))?;
        Ok(Self {
            inner: Arc::new(docker),
        })
    }

    /// Returns a reference to the underlying Bollard client.
    pub fn inner(&self) -> &Docker {
        &self.inner
    }

    /// Verifies that the daemon is reachable by calling Docker version API.
    pub async fn ping(&self) -> Result<(), RuntimeError> {
        self.inner
            .ping()
            .await
            .map_err(|e| RuntimeError::Connection(format!("Docker ping failed: {e}")))?;
        Ok(())
    }
}

impl std::fmt::Debug for DockerClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DockerClient").finish()
    }
}
