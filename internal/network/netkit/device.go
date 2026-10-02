// Package netkit provides netkit virtual network device management.
package netkit

import (
	"errors"
	"fmt"
	"log"

	"github.com/vishvananda/netlink"
	"golang.org/x/sys/unix"
)

// Device manages a netkit (or veth-fallback) interface.
type Device struct{ link netlink.Link }

// Create creates a netkit interface named name with the given mtu.
//
// It first attempts to create a kernel netkit link (kernel ≥ 6.7).  If the
// kernel does not support netkit (EOPNOTSUPP or EINVAL is returned), it falls
// back to creating a veth pair where the host side is name and the peer side
// is name+"-peer", and logs a warning.
func Create(name string, mtu int) (*Device, error) {
	la := netlink.NewLinkAttrs()
	la.Name = name
	if mtu > 0 {
		la.MTU = mtu
	}

	// Attempt netkit first (requires kernel ≥ 6.7).
	nk := &netlink.GenericLink{
		LinkAttrs: la,
		LinkType:  "netkit",
	}
	if err := netlink.LinkAdd(nk); err == nil {
		// Netkit created successfully; fetch the canonical link handle.
		created, lookupErr := netlink.LinkByName(name)
		if lookupErr != nil {
			return nil, fmt.Errorf("netkit: find %s after create: %w", name, lookupErr)
		}
		return &Device{link: created}, nil
	} else if !isUnsupported(err) {
		return nil, fmt.Errorf("netkit: create %s: %w", name, err)
	}

	// Kernel does not support netkit; fall back to a veth pair.
	log.Printf("WARNING: netkit not supported by kernel; falling back to veth pair %s <-> %s-peer", name, name)
	peerName := name + "-peer"
	veth := &netlink.Veth{
		LinkAttrs: la,
		PeerName:  peerName,
	}
	if err := netlink.LinkAdd(veth); err != nil {
		return nil, fmt.Errorf("netkit: create veth fallback %s<->%s: %w", name, peerName, err)
	}
	created, err := netlink.LinkByName(name)
	if err != nil {
		return nil, fmt.Errorf("netkit: find %s after veth create: %w", name, err)
	}
	return &Device{link: created}, nil
}

// Delete removes the interface (and its peer when it is a veth pair).
func (d *Device) Delete() error {
	if err := netlink.LinkDel(d.link); err != nil {
		return fmt.Errorf("netkit: delete %s: %w", d.link.Attrs().Name, err)
	}
	return nil
}

// Index returns the interface index of the host-side link.
func (d *Device) Index() int {
	return d.link.Attrs().Index
}

// SetNS moves the peer link into the network namespace of pid.
// For a veth pair the peer is identified by the name convention "name-peer".
func (d *Device) SetNS(pid int) error {
	attrs := d.link.Attrs()
	peerName := attrs.Name + "-peer"

	peer, err := netlink.LinkByName(peerName)
	if err != nil {
		return fmt.Errorf("netkit: find peer %s: %w", peerName, err)
	}
	if err := netlink.LinkSetNsPid(peer, pid); err != nil {
		return fmt.Errorf("netkit: set ns pid %d for %s: %w", pid, peerName, err)
	}
	return nil
}

// isUnsupported returns true when the error indicates the link type is not
// supported by the kernel.
func isUnsupported(err error) bool {
	return errors.Is(err, unix.EOPNOTSUPP) || errors.Is(err, unix.EINVAL)
}
