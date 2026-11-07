//! Shared common utilities for Orchestra.
//!
//! Provides typed identifiers, global constants, time helpers, general validators,
//! and structured logging configuration used throughout the entire workspace.

pub mod constants;
pub mod identifiers;
pub mod logging;
pub mod time;
pub mod utilities;

pub use constants::*;
pub use identifiers::{ClusterId, NetworkId, NodeId, OperationId};
pub use logging::{LogConfig, LogFormat, init_tracing};
pub use time::{Stopwatch, format_age, format_duration, format_rfc3339, now_utc};
pub use utilities::{
    RetryConfig, add_ip_offset, cidr_contains_ip, cidrs_do_not_overlap, is_nodeport,
    is_valid_dns_subdomain, is_valid_label_key, is_valid_label_value, is_valid_port, parse_cidr,
    retry_with_backoff, sanitize_name,
};
