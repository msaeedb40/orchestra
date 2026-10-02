package ebpf

import (
	"encoding/binary"
	"fmt"
	"net"

	ciliumebpf "github.com/cilium/ebpf"
)

// PodMeta is the BPF map value for pod entries.
// Layout must match struct pod_meta in bpf/common/common.h.
type PodMeta struct {
	RxPackets uint64
	TxPackets uint64
	PodIP     uint32
	Active    uint8
}

// Backend is a service backend entry.
// Layout must match struct svc_backend in bpf/common/common.h.
type Backend struct {
	IP   uint32
	Port uint16
}

// svcMapKey is the composite key for the service BPF map.
// Layout must match struct svc_key in bpf/common/common.h.
type svcMapKey struct {
	VIP   uint32
	Port  uint16
	Proto uint8
}

// policyMapKey is the composite key for the policy BPF map.
type policyMapKey struct {
	SrcIP uint32
	DstIP uint32
}

// PodMap wraps a BPF hash map keyed by pod IPv4 address (big-endian uint32)
// with values of type PodMeta.
type PodMap struct{ m *ciliumebpf.Map }

// NewPodMap creates a PodMap from a raw cilium/ebpf Map handle.
func NewPodMap(m *ciliumebpf.Map) *PodMap { return &PodMap{m: m} }

// Put inserts or updates the PodMeta entry for ip.
// ip must be an IPv4 address; its big-endian uint32 is used as the map key.
func (m *PodMap) Put(ip net.IP, meta PodMeta) error {
	if m.m == nil {
		return nil // stub mode
	}
	key, err := ip4ToUint32(ip)
	if err != nil {
		return fmt.Errorf("pod map put: %w", err)
	}
	if err := m.m.Put(key, meta); err != nil {
		return fmt.Errorf("pod map put %s: %w", ip, err)
	}
	return nil
}

// Delete removes the entry for ip from the pod map.
func (m *PodMap) Delete(ip net.IP) error {
	if m.m == nil {
		return nil
	}
	key, err := ip4ToUint32(ip)
	if err != nil {
		return fmt.Errorf("pod map delete: %w", err)
	}
	if err := m.m.Delete(key); err != nil {
		return fmt.Errorf("pod map delete %s: %w", ip, err)
	}
	return nil
}

// Lookup retrieves the PodMeta entry for ip.
// Returns nil, nil when the key is not present.
func (m *PodMap) Lookup(ip net.IP) (*PodMeta, error) {
	if m.m == nil {
		return nil, nil
	}
	key, err := ip4ToUint32(ip)
	if err != nil {
		return nil, fmt.Errorf("pod map lookup: %w", err)
	}
	var meta PodMeta
	if err := m.m.Lookup(key, &meta); err != nil {
		return nil, fmt.Errorf("pod map lookup %s: %w", ip, err)
	}
	return &meta, nil
}

// ServiceMap wraps a BPF hash map keyed by (vip, port, proto) with values of
// type Backend.
type ServiceMap struct{ m *ciliumebpf.Map }

// NewServiceMap creates a ServiceMap from a raw cilium/ebpf Map handle.
func NewServiceMap(m *ciliumebpf.Map) *ServiceMap { return &ServiceMap{m: m} }

// UpsertBackend inserts or replaces all backends for the given VIP and port.
// The current implementation programmes only the first backend per VIP:port
// pair; a production dataplane would fan-out across a backend array.
func (m *ServiceMap) UpsertBackend(vip net.IP, port uint16, backends []Backend) error {
	if m.m == nil {
		return nil
	}
	if len(backends) == 0 {
		return fmt.Errorf("service map upsert: no backends for %s:%d", vip, port)
	}
	vipU32, err := ip4ToUint32(vip)
	if err != nil {
		return fmt.Errorf("service map upsert: %w", err)
	}
	key := svcMapKey{VIP: vipU32, Port: port}
	b := backends[0]
	if err := m.m.Put(key, b); err != nil {
		return fmt.Errorf("service map upsert %s:%d: %w", vip, port, err)
	}
	return nil
}

// Delete removes the service entry for the given VIP and port.
func (m *ServiceMap) Delete(vip net.IP, port uint16) error {
	if m.m == nil {
		return nil
	}
	vipU32, err := ip4ToUint32(vip)
	if err != nil {
		return fmt.Errorf("service map delete: %w", err)
	}
	key := svcMapKey{VIP: vipU32, Port: port}
	if err := m.m.Delete(key); err != nil {
		return fmt.Errorf("service map delete %s:%d: %w", vip, port, err)
	}
	return nil
}

// PolicyMap wraps a BPF hash map keyed by (srcIP, dstIP) with a uint32
// verdict: 1 = allow, 0 = deny.
type PolicyMap struct{ m *ciliumebpf.Map }

// NewPolicyMap creates a PolicyMap from a raw cilium/ebpf Map handle.
func NewPolicyMap(m *ciliumebpf.Map) *PolicyMap { return &PolicyMap{m: m} }

// Allow inserts or overwrites a policy entry permitting traffic from src to dst.
func (m *PolicyMap) Allow(src, dst net.IP) error {
	return m.setPolicy(src, dst, 1)
}

// Deny inserts or overwrites a policy entry blocking traffic from src to dst.
func (m *PolicyMap) Deny(src, dst net.IP) error {
	return m.setPolicy(src, dst, 0)
}

func (m *PolicyMap) setPolicy(src, dst net.IP, verdict uint32) error {
	if m.m == nil {
		return nil
	}
	srcU32, err := ip4ToUint32(src)
	if err != nil {
		return fmt.Errorf("policy map: src %w", err)
	}
	dstU32, err := ip4ToUint32(dst)
	if err != nil {
		return fmt.Errorf("policy map: dst %w", err)
	}
	key := policyMapKey{SrcIP: srcU32, DstIP: dstU32}
	if err := m.m.Put(key, verdict); err != nil {
		return fmt.Errorf("policy map set %s->%s verdict %d: %w", src, dst, verdict, err)
	}
	return nil
}

// ip4ToUint32 converts an IPv4 net.IP to its big-endian uint32 representation.
func ip4ToUint32(ip net.IP) (uint32, error) {
	v4 := ip.To4()
	if v4 == nil {
		return 0, fmt.Errorf("not an IPv4 address: %v", ip)
	}
	return binary.BigEndian.Uint32(v4), nil
}
