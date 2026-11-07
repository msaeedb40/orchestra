//! Authentication and RBAC authorization tokens.

use crate::error::SecurityError;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::sync::{Arc, RwLock};

/// RBAC roles recognized across the Orchestra platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// Full administrative control over all clusters, nodes, eBPF, and system operations.
    Admin,
    /// Lifecycle operator capable of creating, modifying, and destroying clusters/nodes.
    Operator,
    /// Read-only observer able to query cluster state, topology, and metrics.
    Viewer,
    /// Specialized node agent identity for heartbeat and telemetry reporting.
    NodeAgent,
}

/// Granular permissions governing operations on Orchestra resources.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    ClusterCreate,
    ClusterRead,
    ClusterUpdate,
    ClusterDelete,
    NodeCreate,
    NodeRead,
    NodeUpdate,
    NodeDelete,
    EbpfManage,
    NetworkManage,
    SystemDiagnostics,
}

/// Authenticated client identity and assigned roles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthContext {
    /// Subject identifier (e.g. "user:alice", "system:node-worker-1").
    pub subject: String,
    /// Assigned RBAC roles.
    pub roles: Vec<Role>,
    /// Timestamp when this token or session was issued.
    pub issued_at: DateTime<Utc>,
    /// Optional expiration timestamp.
    pub expires_at: Option<DateTime<Utc>>,
}

impl AuthContext {
    /// Constructs a new authentication context.
    pub fn new(subject: impl Into<String>, roles: Vec<Role>) -> Self {
        Self {
            subject: subject.into(),
            roles,
            issued_at: Utc::now(),
            expires_at: None,
        }
    }

    /// Constructs an authentication context with a validity duration.
    pub fn with_duration(subject: impl Into<String>, roles: Vec<Role>, validity: Duration) -> Self {
        let now = Utc::now();
        Self {
            subject: subject.into(),
            roles,
            issued_at: now,
            expires_at: Some(now + validity),
        }
    }

    /// Checks if this authentication context has expired.
    pub fn is_expired(&self) -> bool {
        if let Some(exp) = self.expires_at {
            Utc::now() > exp
        } else {
            false
        }
    }
}

/// Role-Based Access Control (RBAC) policy enforcer.
#[derive(Debug, Clone)]
pub struct RbacEnforcer {
    role_permissions: HashMap<Role, HashSet<Permission>>,
}

impl Default for RbacEnforcer {
    fn default() -> Self {
        let mut role_permissions = HashMap::new();

        // Admin has all permissions
        let mut admin_perms = HashSet::new();
        admin_perms.insert(Permission::ClusterCreate);
        admin_perms.insert(Permission::ClusterRead);
        admin_perms.insert(Permission::ClusterUpdate);
        admin_perms.insert(Permission::ClusterDelete);
        admin_perms.insert(Permission::NodeCreate);
        admin_perms.insert(Permission::NodeRead);
        admin_perms.insert(Permission::NodeUpdate);
        admin_perms.insert(Permission::NodeDelete);
        admin_perms.insert(Permission::EbpfManage);
        admin_perms.insert(Permission::NetworkManage);
        admin_perms.insert(Permission::SystemDiagnostics);
        role_permissions.insert(Role::Admin, admin_perms);

        // Operator has cluster & node operations
        let mut operator_perms = HashSet::new();
        operator_perms.insert(Permission::ClusterCreate);
        operator_perms.insert(Permission::ClusterRead);
        operator_perms.insert(Permission::ClusterUpdate);
        operator_perms.insert(Permission::ClusterDelete);
        operator_perms.insert(Permission::NodeCreate);
        operator_perms.insert(Permission::NodeRead);
        operator_perms.insert(Permission::NodeUpdate);
        operator_perms.insert(Permission::NodeDelete);
        operator_perms.insert(Permission::NetworkManage);
        operator_perms.insert(Permission::SystemDiagnostics);
        role_permissions.insert(Role::Operator, operator_perms);

        // Viewer has read-only operations
        let mut viewer_perms = HashSet::new();
        viewer_perms.insert(Permission::ClusterRead);
        viewer_perms.insert(Permission::NodeRead);
        viewer_perms.insert(Permission::SystemDiagnostics);
        role_permissions.insert(Role::Viewer, viewer_perms);

        // NodeAgent has node read/update
        let mut agent_perms = HashSet::new();
        agent_perms.insert(Permission::NodeRead);
        agent_perms.insert(Permission::NodeUpdate);
        role_permissions.insert(Role::NodeAgent, agent_perms);

        Self { role_permissions }
    }
}

impl RbacEnforcer {
    /// Verifies whether the provided auth context holds the required permission.
    pub fn authorize(&self, auth: &AuthContext, required: Permission) -> Result<(), SecurityError> {
        if auth.is_expired() {
            return Err(SecurityError::AuthenticationFailed(format!(
                "Token for subject '{}' has expired",
                auth.subject
            )));
        }

        for role in &auth.roles {
            if let Some(perms) = self.role_permissions.get(role) {
                if perms.contains(&required) {
                    return Ok(());
                }
            }
        }

        Err(SecurityError::Unauthorized(format!(
            "Subject '{}' with roles {:?} lacks permission '{:?}'",
            auth.subject, auth.roles, required
        )))
    }
}

/// Token store and authenticator for gRPC/IPC sessions.
#[derive(Debug, Default, Clone)]
pub struct TokenManager {
    tokens: Arc<RwLock<HashMap<String, AuthContext>>>,
}

impl TokenManager {
    /// Creates a new empty token manager.
    pub fn new() -> Self {
        Self {
            tokens: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Issues and stores a new bearer token for an identity.
    pub fn issue_token(&self, token: impl Into<String>, auth_ctx: AuthContext) {
        let mut lock = self.tokens.write().expect("TokenManager lock poisoned");
        lock.insert(token.into(), auth_ctx);
    }

    /// Authenticates a bearer token and returns the associated `AuthContext`.
    pub fn authenticate(&self, token: &str) -> Result<AuthContext, SecurityError> {
        let lock = self.tokens.read().expect("TokenManager lock poisoned");
        if let Some(ctx) = lock.get(token) {
            if ctx.is_expired() {
                return Err(SecurityError::AuthenticationFailed(
                    "Bearer token has expired".to_string(),
                ));
            }
            Ok(ctx.clone())
        } else {
            Err(SecurityError::AuthenticationFailed(
                "Invalid or unknown bearer token".to_string(),
            ))
        }
    }

    /// Revokes an existing token.
    pub fn revoke_token(&self, token: &str) -> bool {
        let mut lock = self.tokens.write().expect("TokenManager lock poisoned");
        lock.remove(token).is_some()
    }
}

/// Access control verifier for Unix domain sockets (e.g. `/var/run/orchestra.sock`).
pub struct SocketAccessControl;

impl SocketAccessControl {
    /// Validates that the socket file exists and has secure file permissions (e.g. not world-writable).
    pub fn validate_socket_permissions(socket_path: &Path) -> Result<(), SecurityError> {
        if !socket_path.exists() {
            return Err(SecurityError::Isolation(format!(
                "Socket file '{}' does not exist",
                socket_path.display()
            )));
        }

        let metadata = fs::metadata(socket_path)?;
        let mode = metadata.permissions().mode();

        // Check if world writable (0o002)
        if mode & 0o002 != 0 {
            return Err(SecurityError::PolicyViolation(format!(
                "Socket '{}' is insecurely world-writable (mode: {:o})",
                socket_path.display(),
                mode & 0o777
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rbac_admin_vs_viewer() {
        let enforcer = RbacEnforcer::default();

        let admin = AuthContext::new("admin-user", vec![Role::Admin]);
        assert!(
            enforcer
                .authorize(&admin, Permission::ClusterDelete)
                .is_ok()
        );
        assert!(enforcer.authorize(&admin, Permission::EbpfManage).is_ok());

        let viewer = AuthContext::new("viewer-user", vec![Role::Viewer]);
        assert!(enforcer.authorize(&viewer, Permission::ClusterRead).is_ok());
        assert!(
            enforcer
                .authorize(&viewer, Permission::ClusterDelete)
                .is_err()
        );
    }

    #[test]
    fn test_token_expiration() {
        let manager = TokenManager::new();
        let enforcer = RbacEnforcer::default();

        let expired_ctx = AuthContext::with_duration(
            "expired-user",
            vec![Role::Admin],
            Duration::milliseconds(-100),
        );
        manager.issue_token("token-123", expired_ctx);

        let auth_res = manager.authenticate("token-123");
        assert!(auth_res.is_err());

        let direct_res = enforcer.authorize(
            &AuthContext::with_duration("expired", vec![Role::Admin], Duration::milliseconds(-10)),
            Permission::ClusterRead,
        );
        assert!(direct_res.is_err());
    }
}
