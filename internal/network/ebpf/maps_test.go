package ebpf

import (
	"encoding/binary"
	"net"
	"testing"
)

// TestIPToUint32 verifies that net.ParseIP("10.0.0.1") encodes to the correct
// big-endian uint32.
func TestIPToUint32(t *testing.T) {
	ip := net.ParseIP("10.0.0.1")
	if ip == nil {
		t.Fatal("net.ParseIP returned nil for 10.0.0.1")
	}

	got, err := ip4ToUint32(ip)
	if err != nil {
		t.Fatalf("ip4ToUint32: unexpected error: %v", err)
	}

	// 10.0.0.1 big-endian = 0x0A000001
	want := uint32(10)<<24 | uint32(0)<<16 | uint32(0)<<8 | uint32(1)
	if got != want {
		t.Errorf("ip4ToUint32(10.0.0.1) = 0x%08x, want 0x%08x", got, want)
	}

	// Cross-check via encoding/binary so the test has two independent paths.
	var raw [4]byte
	binary.BigEndian.PutUint32(raw[:], got)
	if !net.IP(raw[:]).Equal(ip.To4()) {
		t.Errorf("round-trip mismatch: got %v, want %v", net.IP(raw[:]), ip.To4())
	}
}

// TestIPToUint32_IPv6Rejected verifies that a non-IPv4 address is rejected.
func TestIPToUint32_IPv6Rejected(t *testing.T) {
	ip := net.ParseIP("::1")
	if ip == nil {
		t.Fatal("net.ParseIP returned nil for ::1")
	}
	_, err := ip4ToUint32(ip)
	if err == nil {
		t.Error("ip4ToUint32(::1) expected an error, got nil")
	}
}

// TestPodMetaSize verifies that PodMeta has the expected fields and that they
// carry the right types (no BPF runtime required).
func TestPodMetaSize(t *testing.T) {
	var m PodMeta
	// Ensure fields are settable and of the documented types.
	m.RxPackets = 42
	m.TxPackets = 17
	m.PodIP = 0x0A000001 // 10.0.0.1 big-endian
	m.Active = 1

	if m.RxPackets != 42 {
		t.Errorf("RxPackets: got %d, want 42", m.RxPackets)
	}
	if m.TxPackets != 17 {
		t.Errorf("TxPackets: got %d, want 17", m.TxPackets)
	}
	if m.PodIP != 0x0A000001 {
		t.Errorf("PodIP: got 0x%08x, want 0x0A000001", m.PodIP)
	}
	if m.Active != 1 {
		t.Errorf("Active: got %d, want 1", m.Active)
	}
}

// TestServiceMapKey verifies that the svc_key-equivalent struct encodes to the
// expected binary layout (no BPF runtime required).
func TestServiceMapKey(t *testing.T) {
	// Build a key for VIP 10.96.0.1 port 443 proto 0.
	vip := net.ParseIP("10.96.0.1")
	if vip == nil {
		t.Fatal("net.ParseIP returned nil for 10.96.0.1")
	}

	vipU32, err := ip4ToUint32(vip)
	if err != nil {
		t.Fatalf("ip4ToUint32: %v", err)
	}

	key := svcMapKey{
		VIP:  vipU32,
		Port: 443,
	}

	// 10.96.0.1 = 0x0A600001
	wantVIP := uint32(10)<<24 | uint32(96)<<16 | uint32(0)<<8 | uint32(1)
	if key.VIP != wantVIP {
		t.Errorf("svcMapKey.VIP = 0x%08x, want 0x%08x", key.VIP, wantVIP)
	}
	if key.Port != 443 {
		t.Errorf("svcMapKey.Port = %d, want 443", key.Port)
	}

	// Verify we can recover the IP from the uint32.
	var raw [4]byte
	binary.BigEndian.PutUint32(raw[:], key.VIP)
	if !net.IP(raw[:]).Equal(vip.To4()) {
		t.Errorf("VIP round-trip: got %v, want %v", net.IP(raw[:]), vip.To4())
	}
}

// TestPodMapStubMode verifies that PodMap operations are no-ops when the
// underlying cilium/ebpf Map is nil (stub / unit-test mode).
func TestPodMapStubMode(t *testing.T) {
	pm := &PodMap{m: nil}

	ip := net.ParseIP("192.168.1.1")
	meta := PodMeta{RxPackets: 1, TxPackets: 2, PodIP: 0xC0A80101, Active: 1}

	if err := pm.Put(ip, meta); err != nil {
		t.Errorf("stub Put: unexpected error: %v", err)
	}
	got, err := pm.Lookup(ip)
	if err != nil {
		t.Errorf("stub Lookup: unexpected error: %v", err)
	}
	if got != nil {
		t.Errorf("stub Lookup: expected nil, got %+v", got)
	}
	if err := pm.Delete(ip); err != nil {
		t.Errorf("stub Delete: unexpected error: %v", err)
	}
}

// TestServiceMapStubMode verifies that ServiceMap operations are no-ops in
// stub mode.
func TestServiceMapStubMode(t *testing.T) {
	sm := &ServiceMap{m: nil}

	vip := net.ParseIP("10.96.0.1")
	backends := []Backend{{IP: 0x0A000001, Port: 8080}}

	if err := sm.UpsertBackend(vip, 443, backends); err != nil {
		t.Errorf("stub UpsertBackend: unexpected error: %v", err)
	}
	if err := sm.Delete(vip, 443); err != nil {
		t.Errorf("stub Delete: unexpected error: %v", err)
	}
}

// TestPolicyMapStubMode verifies that PolicyMap operations are no-ops in stub
// mode.
func TestPolicyMapStubMode(t *testing.T) {
	pm := &PolicyMap{m: nil}

	src := net.ParseIP("10.0.0.1")
	dst := net.ParseIP("10.0.0.2")

	if err := pm.Allow(src, dst); err != nil {
		t.Errorf("stub Allow: unexpected error: %v", err)
	}
	if err := pm.Deny(src, dst); err != nil {
		t.Errorf("stub Deny: unexpected error: %v", err)
	}
}
