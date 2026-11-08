//! gRPC server implementation and service handlers.

use crate::state::DaemonState;
use orchestra_api::cluster::ClusterSpec as ProtoClusterSpec;
use orchestra_api::cluster::cluster_service_server::ClusterService;
use orchestra_api::cluster::{
    ClusterInfo, ClusterStatus as ProtoClusterStatus, CreateClusterRequest, CreateClusterResponse,
    DeleteClusterRequest, DeleteClusterResponse, GetClusterRequest, GetClusterResponse,
    ListClustersRequest, ListClustersResponse, StartClusterRequest, StartClusterResponse,
    StopClusterRequest, StopClusterResponse,
};
use orchestra_api::ebpf::ebpf_service_server::EbpfService;
use orchestra_api::ebpf::*;
use orchestra_api::network::network_service_server::NetworkService;
use orchestra_api::network::*;
use orchestra_api::node::node_service_server::NodeService;
use orchestra_api::node::*;
use orchestra_api::system::system_service_server::SystemService;
use orchestra_api::system::*;
use orchestra_common::identifiers::ClusterId;
use orchestra_core::cluster::{Cluster, ClusterSpec as CoreClusterSpec};
use orchestra_storage::repository::cluster::{
    delete_cluster, get_cluster, get_cluster_by_name, list_clusters, save_cluster,
};
use tonic::{Request, Response, Status};
use tracing::info;

#[derive(Clone)]
pub struct GrpcServer {
    state: DaemonState,
}

impl GrpcServer {
    pub fn new(state: DaemonState) -> Self {
        Self { state }
    }

    pub fn state(&self) -> &DaemonState {
        &self.state
    }
}

fn core_to_proto_spec(spec: &CoreClusterSpec) -> ProtoClusterSpec {
    ProtoClusterSpec {
        name: spec.name.clone(),
        kubernetes_version: spec.kubernetes_version.clone(),
        control_plane_nodes: spec.control_plane_nodes as u32,
        worker_nodes: spec.worker_nodes as u32,
        node_image: spec.node_image.clone(),
        cni_plugin: spec.cni_plugin.to_string(),
        service_proxy_mode: spec.service_proxy_mode.to_string(),
        pod_cidr: spec.pod_cidr.clone(),
        service_cidr: spec.service_cidr.clone(),
        api_server_port: spec.api_server_port as u32,
    }
}

fn cluster_to_proto_info(cluster: &Cluster) -> ClusterInfo {
    ClusterInfo {
        id: cluster.id.to_string(),
        spec: Some(core_to_proto_spec(&cluster.spec)),
        status: Some(ProtoClusterStatus {
            state: cluster.status.state.to_string(),
            control_plane_nodes: cluster.status.desired_control_planes as u32,
            worker_nodes: cluster.status.desired_workers as u32,
            api_endpoint: String::new(),
            created_at: None,
            updated_at: None,
        }),
    }
}

#[tonic::async_trait]
impl SystemService for GrpcServer {
    async fn ping(&self, _request: Request<PingRequest>) -> Result<Response<PingResponse>, Status> {
        Ok(Response::new(PingResponse {
            message: "pong".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
        }))
    }

    async fn get_system_info(
        &self,
        _request: Request<GetSystemInfoRequest>,
    ) -> Result<Response<GetSystemInfoResponse>, Status> {
        let isolation_audit = self.state.isolation.audit();
        info!(
            cgroup_v2 = isolation_audit.cgroup_v2_ready,
            controllers = ?isolation_audit.controllers,
            "Serving system info"
        );

        Ok(Response::new(GetSystemInfoResponse {
            daemon_version: env!("CARGO_PKG_VERSION").to_string(),
            git_commit: "HEAD".to_string(),
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            uptime_seconds: 0,
        }))
    }
}

#[tonic::async_trait]
impl ClusterService for GrpcServer {
    async fn create_cluster(
        &self,
        request: Request<CreateClusterRequest>,
    ) -> Result<Response<CreateClusterResponse>, Status> {
        let req = request.into_inner();
        let spec_proto = req
            .spec
            .ok_or_else(|| Status::invalid_argument("spec is required"))?;

        info!(cluster = %spec_proto.name, "Creating new cluster via API");

        // Validate security policy for node launch
        if let Err(e) = self.state.security_policy.validate_node_launch(
            orchestra_core::node::NodeRole::ControlPlane,
            self.state.security_policy.allow_privileged,
            &[],
            &[],
        ) {
            let _ = self.state.audit.log_action(
                "grpc-client",
                "cluster.create",
                &format!("cluster:{}", spec_proto.name),
                orchestra_security::AuditOutcome::Denied,
                serde_json::json!({"error": e.to_string()}),
            );
            return Err(Status::permission_denied(format!(
                "Security policy violation: {e}"
            )));
        }

        let mut spec = CoreClusterSpec::new(&spec_proto.name, &spec_proto.kubernetes_version);
        if spec_proto.control_plane_nodes > 0 {
            spec.control_plane_nodes = spec_proto.control_plane_nodes as usize;
        }
        if spec_proto.worker_nodes > 0 {
            spec.worker_nodes = spec_proto.worker_nodes as usize;
        }
        if !spec_proto.node_image.is_empty() {
            spec.node_image = spec_proto.node_image;
        }

        let cluster_id = ClusterId::generate();
        let cluster = Cluster::new(cluster_id.clone(), spec);

        save_cluster(&self.state.db, &cluster)
            .await
            .map_err(|e| Status::internal(format!("Database error: {e}")))?;

        let _ = self.state.audit.log_action(
            "grpc-client",
            "cluster.create",
            &format!("cluster:{}", cluster.id),
            orchestra_security::AuditOutcome::Allowed,
            serde_json::json!({
                "name": cluster.spec.name,
                "kubernetes_version": cluster.spec.kubernetes_version,
            }),
        );

        Ok(Response::new(CreateClusterResponse {
            operation_id: format!("op-{}", uuid::Uuid::new_v4()),
            cluster: Some(cluster_to_proto_info(&cluster)),
        }))
    }

    async fn get_cluster(
        &self,
        request: Request<GetClusterRequest>,
    ) -> Result<Response<GetClusterResponse>, Status> {
        let name_or_id = request.into_inner().name_or_id;

        let cluster = get_cluster_by_name(&self.state.db, &name_or_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let cluster = match cluster {
            Some(c) => c,
            None => get_cluster(&self.state.db, &ClusterId::new(&name_or_id))
                .await
                .map_err(|e| Status::internal(e.to_string()))?
                .ok_or_else(|| Status::not_found(format!("Cluster '{name_or_id}' not found")))?,
        };

        Ok(Response::new(GetClusterResponse {
            cluster: Some(cluster_to_proto_info(&cluster)),
        }))
    }

    async fn list_clusters(
        &self,
        _request: Request<ListClustersRequest>,
    ) -> Result<Response<ListClustersResponse>, Status> {
        let clusters = list_clusters(&self.state.db)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let cluster_infos = clusters.iter().map(cluster_to_proto_info).collect();

        Ok(Response::new(ListClustersResponse {
            clusters: cluster_infos,
        }))
    }

    async fn delete_cluster(
        &self,
        request: Request<DeleteClusterRequest>,
    ) -> Result<Response<DeleteClusterResponse>, Status> {
        let name_or_id = request.into_inner().name_or_id;

        let cluster = get_cluster_by_name(&self.state.db, &name_or_id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let cluster = match cluster {
            Some(c) => c,
            None => get_cluster(&self.state.db, &ClusterId::new(&name_or_id))
                .await
                .map_err(|e| Status::internal(e.to_string()))?
                .ok_or_else(|| Status::not_found(format!("Cluster '{name_or_id}' not found")))?,
        };

        delete_cluster(&self.state.db, &cluster.id)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let _ = self.state.audit.log_action(
            "grpc-client",
            "cluster.delete",
            &format!("cluster:{}", cluster.id),
            orchestra_security::AuditOutcome::Allowed,
            serde_json::json!({"name": cluster.spec.name}),
        );

        Ok(Response::new(DeleteClusterResponse {
            operation_id: format!("op-del-{}", uuid::Uuid::new_v4()),
            success: true,
        }))
    }

    async fn start_cluster(
        &self,
        _request: Request<StartClusterRequest>,
    ) -> Result<Response<StartClusterResponse>, Status> {
        Ok(Response::new(StartClusterResponse {
            operation_id: format!("op-{}", uuid::Uuid::new_v4()),
            success: true,
        }))
    }

    async fn stop_cluster(
        &self,
        _request: Request<StopClusterRequest>,
    ) -> Result<Response<StopClusterResponse>, Status> {
        Ok(Response::new(StopClusterResponse {
            operation_id: format!("op-{}", uuid::Uuid::new_v4()),
            success: true,
        }))
    }
}

#[tonic::async_trait]
impl NodeService for GrpcServer {
    async fn add_node(
        &self,
        _request: Request<AddNodeRequest>,
    ) -> Result<Response<AddNodeResponse>, Status> {
        Err(Status::unimplemented("add_node not implemented yet"))
    }

    async fn remove_node(
        &self,
        _request: Request<RemoveNodeRequest>,
    ) -> Result<Response<RemoveNodeResponse>, Status> {
        Err(Status::unimplemented("remove_node not implemented yet"))
    }

    async fn list_nodes(
        &self,
        _request: Request<ListNodesRequest>,
    ) -> Result<Response<ListNodesResponse>, Status> {
        Ok(Response::new(ListNodesResponse { nodes: Vec::new() }))
    }
}

#[tonic::async_trait]
impl NetworkService for GrpcServer {
    async fn get_network_status(
        &self,
        _request: Request<GetNetworkStatusRequest>,
    ) -> Result<Response<GetNetworkStatusResponse>, Status> {
        Ok(Response::new(GetNetworkStatusResponse {
            network_mode: "bridge".to_string(),
            cni_plugin: "orchestranet".to_string(),
            service_proxy_mode: "orchestra".to_string(),
            pod_cidr: "10.244.0.0/16".to_string(),
            service_cidr: "10.96.0.0/12".to_string(),
            interfaces: Vec::new(),
        }))
    }
}

#[tonic::async_trait]
impl EbpfService for GrpcServer {
    async fn get_ebpf_status(
        &self,
        _request: Request<GetEbpfStatusRequest>,
    ) -> Result<Response<GetEbpfStatusResponse>, Status> {
        Ok(Response::new(GetEbpfStatusResponse {
            is_active: true,
            programs: Vec::new(),
            maps: Vec::new(),
        }))
    }
}
