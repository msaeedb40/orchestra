//! Structured logging, Prometheus metrics, event buffering, and diagnostics.

pub mod diagnostics;
pub mod events;
pub mod metrics;
pub mod tracing;

pub use diagnostics::DiagnosticReport;
pub use events::EventCollector;
pub use metrics::MetricsRegistry;
pub use tracing::{LogFormat, init_tracing};
