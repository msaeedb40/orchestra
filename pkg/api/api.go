// Package api provides public helper functions and type aliases for Orchestra API resources.
//
// Callers can import this package as a single façade and avoid importing
// api/v1alpha1 directly for most use cases.
package api

import "github.com/orchestra/orchestra/api/v1alpha1"

// ---- API group constants ----

const (
	GroupName    = v1alpha1.GroupName
	GroupVersion = v1alpha1.GroupVersion
	APIVersion   = v1alpha1.APIVersion
)

// ---- Resource name constants ----

const (
	ResourceOrchestraCluster = v1alpha1.ResourceOrchestraCluster
	ResourceOrchestraNode    = v1alpha1.ResourceOrchestraNode
	ResourceOrchestraNetwork = v1alpha1.ResourceOrchestraNetwork
)

// ---- Kind constants ----

const (
	KindOrchestraCluster = v1alpha1.KindOrchestraCluster
	KindOrchestraNode    = v1alpha1.KindOrchestraNode
	KindOrchestraNetwork = v1alpha1.KindOrchestraNetwork
)

// ---- Kubernetes version constants ----

const (
	KubernetesV135           = v1alpha1.KubernetesV135
	KubernetesV136           = v1alpha1.KubernetesV136
	KubernetesV137           = v1alpha1.KubernetesV137
	DefaultKubernetesVersion = v1alpha1.DefaultKubernetesVersion
	KubernetesVersion136     = v1alpha1.KubernetesVersion136
)

// SupportedKubernetesVersions lists all Kubernetes versions Orchestra can provision.
var SupportedKubernetesVersions = v1alpha1.SupportedKubernetesVersions

// ---- Networking defaults ----

const (
	DefaultPodCIDR     = v1alpha1.DefaultPodCIDR
	DefaultServiceCIDR = v1alpha1.DefaultServiceCIDR
	DefaultCNIPlugin   = v1alpha1.DefaultCNIPlugin
)

// ---- ClusterPhase constants ----

const (
	ClusterPhasePending      = v1alpha1.ClusterPhasePending
	ClusterPhaseProvisioning = v1alpha1.ClusterPhaseProvisioning
	ClusterPhaseRunning      = v1alpha1.ClusterPhaseRunning
	ClusterPhaseUpgrading    = v1alpha1.ClusterPhaseUpgrading
	ClusterPhaseDeleting     = v1alpha1.ClusterPhaseDeleting
	ClusterPhaseFailed       = v1alpha1.ClusterPhaseFailed
)

// ---- NodeRole constants ----

const (
	NodeRoleControlPlane = v1alpha1.NodeRoleControlPlane
	NodeRoleWorker       = v1alpha1.NodeRoleWorker
)

// ---- NodePhase constants ----

const (
	NodePhasePending      = v1alpha1.NodePhasePending
	NodePhaseProvisioning = v1alpha1.NodePhaseProvisioning
	NodePhaseJoining      = v1alpha1.NodePhaseJoining
	NodePhaseReady        = v1alpha1.NodePhaseReady
	NodePhaseDraining     = v1alpha1.NodePhaseDraining
	NodePhaseRemoving     = v1alpha1.NodePhaseRemoving
	NodePhaseFailed       = v1alpha1.NodePhaseFailed
)

// ---- ConditionStatus constants ----

const (
	ConditionTrue    = v1alpha1.ConditionTrue
	ConditionFalse   = v1alpha1.ConditionFalse
	ConditionUnknown = v1alpha1.ConditionUnknown
)

// ---- Type aliases ----
// Type aliases are assignment-compatible with their v1alpha1 counterparts —
// a pkg/api.OrchestraCluster can be passed directly to any function that
// accepts a v1alpha1.OrchestraCluster.

type OrchestraCluster = v1alpha1.OrchestraCluster
type OrchestraClusterList = v1alpha1.OrchestraClusterList
type OrchestraNode = v1alpha1.OrchestraNode
type OrchestraNodeList = v1alpha1.OrchestraNodeList
type OrchestraNetwork = v1alpha1.OrchestraNetwork
type OrchestraNetworkList = v1alpha1.OrchestraNetworkList

// Common meta type aliases.
type TypeMeta = v1alpha1.TypeMeta
type ObjectMeta = v1alpha1.ObjectMeta
type Condition = v1alpha1.Condition
type ConditionStatus = v1alpha1.ConditionStatus

// Kubernetes/cluster sub-type aliases.
type ClusterPhase = v1alpha1.ClusterPhase
type KubernetesVersion = v1alpha1.KubernetesVersion
type NodeRole = v1alpha1.NodeRole
type NodePhase = v1alpha1.NodePhase
type ClusterSpec = v1alpha1.ClusterSpec
type ClusterStatus = v1alpha1.ClusterStatus
type ControlPlaneSpec = v1alpha1.ControlPlaneSpec
type WorkerSpec = v1alpha1.WorkerSpec
type NetworkingSpec = v1alpha1.NetworkingSpec
type RuntimeSpec = v1alpha1.RuntimeSpec
type NodeSpec = v1alpha1.NodeSpec
type NodeStatus = v1alpha1.NodeStatus
type VMNodeSpec = v1alpha1.VMNodeSpec
type NetworkSpec = v1alpha1.NetworkSpec
type NetworkStatus = v1alpha1.NetworkStatus

// ---- Constructor helpers ----

// NewCluster returns an OrchestraCluster with TypeMeta pre-filled.
func NewCluster(name, namespace string) *v1alpha1.OrchestraCluster {
	return &v1alpha1.OrchestraCluster{
		TypeMeta: v1alpha1.TypeMeta{
			Kind:       v1alpha1.KindOrchestraCluster,
			APIVersion: v1alpha1.APIVersion,
		},
		ObjectMeta: v1alpha1.ObjectMeta{
			Name:      name,
			Namespace: namespace,
		},
	}
}

// NewNode returns an OrchestraNode with TypeMeta pre-filled.
func NewNode(name, namespace string) *v1alpha1.OrchestraNode {
	return &v1alpha1.OrchestraNode{
		TypeMeta: v1alpha1.TypeMeta{
			Kind:       v1alpha1.KindOrchestraNode,
			APIVersion: v1alpha1.APIVersion,
		},
		ObjectMeta: v1alpha1.ObjectMeta{
			Name:      name,
			Namespace: namespace,
		},
	}
}

// NewNetwork returns an OrchestraNetwork with TypeMeta pre-filled.
func NewNetwork(name, namespace string) *v1alpha1.OrchestraNetwork {
	return &v1alpha1.OrchestraNetwork{
		TypeMeta: v1alpha1.TypeMeta{
			Kind:       v1alpha1.KindOrchestraNetwork,
			APIVersion: v1alpha1.APIVersion,
		},
		ObjectMeta: v1alpha1.ObjectMeta{
			Name:      name,
			Namespace: namespace,
		},
	}
}

// ---- Utility functions ----

// GroupVersionKind returns a slash-delimited label string of the form
// "orchestra.io/v1alpha1/Kind" (e.g. "orchestra.io/v1alpha1/OrchestraCluster").
//
// NOTE: This is NOT a Kubernetes apiVersion string (which is "orchestra.io/v1alpha1"
// without the kind segment) and must not be passed to Kubernetes API machinery
// that expects apiVersion. It is intended only as a human-readable logging or
// display label. For TypeMeta fields use the APIVersion constant and
// KindOrchestraCluster/KindOrchestraNode/KindOrchestraNetwork constants directly.
func GroupVersionKind(kind string) string {
	return v1alpha1.APIVersion + "/" + kind
}
