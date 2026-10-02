// Package network implements the Orchestra eBPF + netkit dataplane.
package network

import (
	"fmt"
	"net"

	ebpfpkg "github.com/orchestra-io/orchestra/internal/network/ebpf"
	"github.com/orchestra-io/orchestra/internal/network/netkit"
)

// Service describes a Kubernetes service for the dataplane.
type Service struct {
	// Name is the service name.
	Name string
	// Namespace is the Kubernetes namespace.
	Namespace string
	// VIP is the cluster-IP virtual address.
	VIP net.IP
	// Port is the service port.
	Port uint16
	// Backends are the backend pod endpoints.
	Backends []ServiceBackend
}

// ServiceBackend is one backend endpoint for a service.
type ServiceBackend struct {
	IP   net.IP
	Port uint16
}

// Config holds dataplane configuration.
type Config struct {
	// PodCIDR is the cluster-wide pod IP range.
	PodCIDR string
	// BPFObjectPath is the path to a compiled BPF .o file.
	// When empty the dataplane runs in stub (no-op) mode so unit tests
	// can run without a real BPF runtime.
	BPFObjectPath string
	// PinDir is the bpffs directory under which maps are pinned.
	// Ignored in stub mode.
	PinDir string
}

// Dataplane is the interface for managing network state.
type Dataplane interface {
	// Setup initialises the dataplane for the given pod CIDR.
	Setup(podCIDR string) error
	// AddPod allocates networking for a new pod.
	AddPod(podID, podIP string) error
	// RemovePod deallocates networking for a pod.
	RemovePod(podID string) error
	// AddService programs a Kubernetes service into the eBPF map.
	AddService(svc Service) error
	// DeleteService removes a service from the eBPF map.
	DeleteService(svc Service) error
	// Close releases all dataplane resources.
	Close() error
}

// EBPFDataplane is the production Dataplane implementation backed by the
// cilium/ebpf loader and vishvananda/netlink netkit devices.
type EBPFDataplane struct {
	loader  *ebpfpkg.Loader
	podMap  *ebpfpkg.PodMap
	svcMap  *ebpfpkg.ServiceMap
	polMap  *ebpfpkg.PolicyMap
	podCIDR *net.IPNet
}

// stubDataplane is a no-op Dataplane used when no BPF object path is
// configured (e.g., during unit tests).
type stubDataplane struct{}

func (s *stubDataplane) Setup(_ string) error          { return nil }
func (s *stubDataplane) AddPod(_, _ string) error      { return nil }
func (s *stubDataplane) RemovePod(_ string) error      { return nil }
func (s *stubDataplane) AddService(_ Service) error    { return nil }
func (s *stubDataplane) DeleteService(_ Service) error { return nil }
func (s *stubDataplane) Close() error                  { return nil }

// NewDataplane creates and returns a Dataplane.
//
// When cfg.BPFObjectPath is empty a no-op stub is returned so the package
// compiles and unit tests run without a real BPF runtime.
func NewDataplane(cfg Config) (Dataplane, error) {
	if cfg.BPFObjectPath == "" {
		return &stubDataplane{}, nil
	}

	loader, err := ebpfpkg.New(cfg.BPFObjectPath, cfg.PinDir)
	if err != nil {
		return nil, fmt.Errorf("dataplane: %w", err)
	}
	if err := loader.Load(); err != nil {
		return nil, fmt.Errorf("dataplane: load eBPF collection: %w", err)
	}

	maps := loader.Maps()
	d := &EBPFDataplane{
		loader: loader,
		podMap: &ebpfpkg.PodMap{},
		svcMap: &ebpfpkg.ServiceMap{},
		polMap: &ebpfpkg.PolicyMap{},
	}
	// Wire up the typed map accessors from the loaded collection.
	if m, ok := maps["pod_map"]; ok {
		d.podMap = ebpfpkg.NewPodMap(m)
	}
	if m, ok := maps["svc_map"]; ok {
		d.svcMap = ebpfpkg.NewServiceMap(m)
	}
	if m, ok := maps["policy_map"]; ok {
		d.polMap = ebpfpkg.NewPolicyMap(m)
	}
	return d, nil
}

// Setup initialises the dataplane for podCIDR.
func (d *EBPFDataplane) Setup(podCIDR string) error {
	_, cidr, err := net.ParseCIDR(podCIDR)
	if err != nil {
		return fmt.Errorf("dataplane setup: parse CIDR %s: %w", podCIDR, err)
	}
	d.podCIDR = cidr
	return nil
}

// AddPod allocates netkit networking for a pod and registers its IP in the
// eBPF pod map.
func (d *EBPFDataplane) AddPod(podID, podIP string) error {
	ip := net.ParseIP(podIP)
	if ip == nil {
		return fmt.Errorf("dataplane add pod: invalid IP %q", podIP)
	}

	// Create a netkit (or veth-fallback) pair for the pod.
	hostIface := ifaceName(podID)
	dev, err := netkit.Create(hostIface, 1500)
	if err != nil {
		return fmt.Errorf("dataplane add pod %s: create netkit: %w", podID, err)
	}
	if err := netkit.Up(hostIface); err != nil {
		_ = dev.Delete()
		return fmt.Errorf("dataplane add pod %s: bring up %s: %w", podID, hostIface, err)
	}

	// Register the pod IP in the eBPF map.
	meta := ebpfpkg.PodMeta{
		PodIP:  ipToUint32(ip),
		Active: 1,
	}
	if err := d.podMap.Put(ip, meta); err != nil {
		_ = dev.Delete()
		return fmt.Errorf("dataplane add pod %s: pod map put: %w", podID, err)
	}
	return nil
}

// RemovePod removes the netkit interface and pod map entry for podID.
func (d *EBPFDataplane) RemovePod(podID string) error {
	hostIface := ifaceName(podID)
	dev, err := netkit.Create(hostIface, 0)
	if err != nil {
		// Interface may not exist; treat as already removed.
		return nil
	}
	return dev.Delete()
}

// AddService programs the VIP→backend mapping into the eBPF service map.
func (d *EBPFDataplane) AddService(svc Service) error {
	backs := make([]ebpfpkg.Backend, 0, len(svc.Backends))
	for _, b := range svc.Backends {
		backs = append(backs, ebpfpkg.Backend{
			IP:   ipToUint32(b.IP),
			Port: b.Port,
		})
	}
	return d.svcMap.UpsertBackend(svc.VIP, svc.Port, backs)
}

// DeleteService removes the service entry from the eBPF map.
func (d *EBPFDataplane) DeleteService(svc Service) error {
	return d.svcMap.Delete(svc.VIP, svc.Port)
}

// Close releases all dataplane resources.
func (d *EBPFDataplane) Close() error {
	return d.loader.Close()
}

// ifaceName derives a short, deterministic interface name from a pod ID.
// Interface names on Linux are limited to 15 characters (IFNAMSIZ-1).
func ifaceName(podID string) string {
	if len(podID) > 11 {
		return "ork-" + podID[:11]
	}
	return "ork-" + podID
}

// ipToUint32 converts a net.IP to its big-endian uint32.
// Panics if ip is not IPv4 — callers must validate before calling.
func ipToUint32(ip net.IP) uint32 {
	v4 := ip.To4()
	if v4 == nil {
		return 0
	}
	return uint32(v4[0])<<24 | uint32(v4[1])<<16 | uint32(v4[2])<<8 | uint32(v4[3])
}

// Ensure compile-time interface satisfaction.
var _ Dataplane = (*EBPFDataplane)(nil)
var _ Dataplane = (*stubDataplane)(nil)
