//! Compose sidecar integration for cross-chain transaction coordination.
//!
//! This module provides a client for polling the compose sidecar at flashblock
//! boundaries to retrieve cross-chain transactions that must be included in blocks.

mod client;
mod config;
mod types;
mod overrides;

pub use client::SidecarClient;
pub use config::SidecarConfig;
pub use types::{ExternalTransaction, PollRequest, PollResponse, SidecarError};
pub use overrides::build_state_overrides;
