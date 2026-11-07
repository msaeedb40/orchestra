//! Core domain provider traits and abstractions.

pub mod controller;
pub mod network;
pub mod runtime;
pub mod storage;

pub use controller::{Controller, ReconcileResult};
pub use network::NetworkProvider;
pub use runtime::{NodeRuntimeInfo, RuntimeProvider};
pub use storage::StorageProvider;
