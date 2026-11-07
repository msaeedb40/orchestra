//! Error types for the security subsystem.

use thiserror::Error;

/// Errors arising from security policy checks, authentication, authorization, or isolation.
#[derive(Debug, Error)]
pub enum SecurityError {
    /// Action violates the configured security policy.
    #[error("Security policy violation: {0}")]
    PolicyViolation(String),

    /// Client authentication failed.
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    /// Authenticated subject is not authorized for the requested action.
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    /// Seccomp filter profile generation or verification failed.
    #[error("Seccomp configuration error: {0}")]
    Seccomp(String),

    /// Linux namespace or cgroups isolation boundary check failed.
    #[error("Isolation boundary check failed: {0}")]
    Isolation(String),

    /// Audit logging failed.
    #[error("Audit log error: {0}")]
    Audit(String),

    /// Standard I/O error during file or socket operation.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization or deserialization failed.
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),
}
