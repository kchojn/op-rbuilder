//! Sidecar client configuration.

use std::time::Duration;

/// Configuration for the compose sidecar client.
#[derive(Debug, Clone)]
pub struct SidecarConfig {
    /// HTTP endpoint of the sidecar (e.g., "http://localhost:8082").
    /// If empty, sidecar integration is disabled.
    pub endpoint: String,

    /// Timeout for individual HTTP poll requests.
    pub poll_timeout: Duration,

    /// Maximum number of retries when sidecar returns hold response.
    pub max_retries: u32,
}

impl Default for SidecarConfig {
    fn default() -> Self {
        Self {
            endpoint: String::new(),
            poll_timeout: Duration::from_millis(200),
            max_retries: 5,
        }
    }
}

impl SidecarConfig {
    /// Returns true if the sidecar integration is enabled.
    pub fn is_enabled(&self) -> bool {
        !self.endpoint.is_empty()
    }
}
