package netkit

import (
	"fmt"
	"net"

	"github.com/vishvananda/netlink"
)

// Up brings the named interface up.
func Up(name string) error {
	link, err := netlink.LinkByName(name)
	if err != nil {
		return fmt.Errorf("netkit link: find %s: %w", name, err)
	}
	if err := netlink.LinkSetUp(link); err != nil {
		return fmt.Errorf("netkit link: set up %s: %w", name, err)
	}
	return nil
}

// SetAddr assigns an IPv4 address to the named interface.
func SetAddr(name string, addr net.IPNet) error {
	link, err := netlink.LinkByName(name)
	if err != nil {
		return fmt.Errorf("netkit link: find %s: %w", name, err)
	}
	nlAddr := &netlink.Addr{IPNet: &addr}
	if err := netlink.AddrAdd(link, nlAddr); err != nil {
		return fmt.Errorf("netkit link: add addr %s to %s: %w", addr.String(), name, err)
	}
	return nil
}

// SetMTU sets the MTU of the named interface.
func SetMTU(name string, mtu int) error {
	link, err := netlink.LinkByName(name)
	if err != nil {
		return fmt.Errorf("netkit link: find %s: %w", name, err)
	}
	if err := netlink.LinkSetMTU(link, mtu); err != nil {
		return fmt.Errorf("netkit link: set MTU %d on %s: %w", mtu, name, err)
	}
	return nil
}

// AddRoute adds a route for dst via gw on the interface named linkName.
// If gw is nil, the route is a link-scoped (on-link) route.
func AddRoute(dst net.IPNet, gw net.IP, linkName string) error {
	link, err := netlink.LinkByName(linkName)
	if err != nil {
		return fmt.Errorf("netkit link: find %s: %w", linkName, err)
	}
	route := &netlink.Route{
		LinkIndex: link.Attrs().Index,
		Dst:       &dst,
		Gw:        gw,
	}
	if err := netlink.RouteAdd(route); err != nil {
		return fmt.Errorf("netkit link: add route %s via %v on %s: %w", dst.String(), gw, linkName, err)
	}
	return nil
}
