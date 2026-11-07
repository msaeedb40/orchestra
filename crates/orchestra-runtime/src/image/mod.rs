//! Image management: pull, inspect, and cache container images for Orchestra nodes.

use crate::client::DockerClient;
use bollard::image::{CreateImageOptions, ListImagesOptions};
use bollard::models::ImageSummary;
use futures::StreamExt;
use orchestra_core::error::RuntimeError;
use tracing::{debug, info};

/// Manages Docker image availability for Orchestra node containers.
pub struct ImageManager {
    client: DockerClient,
}

impl ImageManager {
    /// Creates a new ImageManager with the given Docker client.
    pub fn new(client: DockerClient) -> Self {
        Self { client }
    }

    /// Checks whether the given image (name:tag) is already available locally.
    pub async fn is_image_available(&self, image: &str) -> Result<bool, RuntimeError> {
        let filters: std::collections::HashMap<&str, Vec<&str>> =
            [("reference", vec![image])].into_iter().collect();

        let opts = ListImagesOptions {
            all: false,
            filters,
            ..Default::default()
        };

        let images: Vec<ImageSummary> = self
            .client
            .inner()
            .list_images(Some(opts))
            .await
            .map_err(|e| RuntimeError::ImagePull(format!("Failed to list images: {e}")))?;

        Ok(!images.is_empty())
    }

    /// Pulls the given image from the registry if not already present locally.
    /// Streams pull progress through the Docker events API.
    pub async fn ensure_image(&self, image: &str) -> Result<(), RuntimeError> {
        if self.is_image_available(image).await? {
            debug!(image, "Image already present locally, skipping pull");
            return Ok(());
        }

        info!(image, "Pulling node image");

        let (image_name, tag) = parse_image_ref(image);
        let opts = CreateImageOptions {
            from_image: image_name,
            tag,
            ..Default::default()
        };

        let mut stream = self.client.inner().create_image(Some(opts), None, None);

        while let Some(event) = stream.next().await {
            match event {
                Ok(info) => {
                    if let Some(status) = &info.status {
                        debug!(status, "Pull progress");
                    }
                }
                Err(e) => {
                    return Err(RuntimeError::ImagePull(format!(
                        "Failed to pull image '{image}': {e}"
                    )));
                }
            }
        }

        info!(image, "Image pulled successfully");
        Ok(())
    }
}

/// Splits an image reference into (name, tag) parts.
/// "kindest/node:v1.31.2" → ("kindest/node", "v1.31.2")
/// "ubuntu" → ("ubuntu", "latest")
fn parse_image_ref(image: &str) -> (&str, &str) {
    match image.rsplit_once(':') {
        Some((name, tag)) => (name, tag),
        None => (image, "latest"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_image_ref_with_tag() {
        let (name, tag) = parse_image_ref("kindest/node:v1.36.4");
        assert_eq!(name, "kindest/node");
        assert_eq!(tag, "v1.36.4");
    }

    #[test]
    fn test_parse_image_ref_without_tag() {
        let (name, tag) = parse_image_ref("ubuntu");
        assert_eq!(name, "ubuntu");
        assert_eq!(tag, "latest");
    }
}
