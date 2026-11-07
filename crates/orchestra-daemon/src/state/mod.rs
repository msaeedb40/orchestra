//! Shared daemon application state and dependencies.

use orchestra_runtime::{DockerClient, DockerRuntimeProvider};
use orchestra_security::{
    AuditLogger, IsolationChecker, RbacEnforcer, SecurityPolicy, TokenManager,
};
use orchestra_storage::database::Database;

/// Central shared state passed to gRPC service handlers.
#[derive(Clone)]
pub struct DaemonState {
    pub db: Database,
    pub runtime: DockerRuntimeProvider,
    pub security_policy: SecurityPolicy,
    pub rbac: RbacEnforcer,
    pub tokens: TokenManager,
    pub audit: AuditLogger,
    pub isolation: IsolationChecker,
}

impl DaemonState {
    /// Constructs daemon state with default security policies and audit logging.
    pub fn new(db: Database, runtime: DockerRuntimeProvider) -> Self {
        Self {
            db,
            runtime,
            security_policy: SecurityPolicy::default(),
            rbac: RbacEnforcer::default(),
            tokens: TokenManager::new(),
            audit: AuditLogger::default(),
            isolation: IsolationChecker::new(),
        }
    }

    /// Constructs daemon state with explicit security configuration.
    pub fn with_security(
        db: Database,
        runtime: DockerRuntimeProvider,
        security_policy: SecurityPolicy,
        rbac: RbacEnforcer,
        tokens: TokenManager,
        audit: AuditLogger,
        isolation: IsolationChecker,
    ) -> Self {
        Self {
            db,
            runtime,
            security_policy,
            rbac,
            tokens,
            audit,
            isolation,
        }
    }

    /// Creates an in-memory test state.
    pub async fn in_memory() -> Result<Self, Box<dyn std::error::Error>> {
        let db = Database::open_in_memory()?;
        orchestra_storage::run_migrations(&db).await?;
        let client = DockerClient::connect()?;
        let runtime = DockerRuntimeProvider::new(client);
        Ok(Self::new(db, runtime))
    }
}
