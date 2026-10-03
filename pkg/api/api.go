// Package api provides public helper functions and type aliases for Orchestra API resources.
package api

import "github.com/orchestra/orchestra/api/v1alpha1"

// Type aliases re-export all six list/resource types from v1alpha1, making them
// assignment-compatible with the underlying types in every context.

type OrchestraCluster = v1alpha1.OrchestraCluster
type OrchestraClusterList = v1alpha1.OrchestraClusterList
type OrchestraNode = v1alpha1.OrchestraNode
type OrchestraNodeList = v1alpha1.OrchestraNodeList
type OrchestraNetwork = v1alpha1.OrchestraNetwork
type OrchestraNetworkList = v1alpha1.OrchestraNetworkList

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

// GroupVersionKind returns the fully qualified group/version/kind string.
func GroupVersionKind(kind string) string {
	return v1alpha1.APIVersion + "/" + kind
}
