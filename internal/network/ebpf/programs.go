package ebpf

import (
	"fmt"

	ciliumebpf "github.com/cilium/ebpf"
	"github.com/vishvananda/netlink"
	"golang.org/x/sys/unix"
)

// AttachDirection controls whether a TC BPF program attaches to ingress or
// egress traffic.
type AttachDirection int

const (
	// DirectionIngress attaches the program on the ingress path.
	DirectionIngress AttachDirection = iota
	// DirectionEgress attaches the program on the egress path.
	DirectionEgress
)

// TCProgram holds the state of a single TC BPF program attached to an
// interface.
type TCProgram struct {
	prog    *ciliumebpf.Program
	ifindex int
	handle  uint32
	filter  netlink.Filter
}

// AttachTC attaches prog to the interface identified by ifindex on the
// direction indicated by direction.
//
// It creates a clsact qdisc on the interface (harmless if one already exists)
// and then installs a BPF filter referencing prog.
func AttachTC(prog *ciliumebpf.Program, ifindex int, direction AttachDirection) (*TCProgram, error) {
	link, err := netlink.LinkByIndex(ifindex)
	if err != nil {
		return nil, fmt.Errorf("ebpf tc: find interface index %d: %w", ifindex, err)
	}

	// Ensure a clsact qdisc exists on the interface; ignore EEXIST.
	qdisc := &netlink.GenericQdisc{
		QdiscAttrs: netlink.QdiscAttrs{
			LinkIndex: ifindex,
			Handle:    netlink.MakeHandle(0xffff, 0),
			Parent:    netlink.HANDLE_CLSACT,
		},
		QdiscType: "clsact",
	}
	if err := netlink.QdiscAdd(qdisc); err != nil && !isEEXIST(err) {
		return nil, fmt.Errorf("ebpf tc: add clsact qdisc on %s: %w", link.Attrs().Name, err)
	}

	// Build the filter parent handle for ingress or egress.
	var parent uint32
	if direction == DirectionIngress {
		parent = netlink.HANDLE_MIN_INGRESS
	} else {
		parent = netlink.HANDLE_MIN_EGRESS
	}

	// Choose a consistent handle so re-attaching is idempotent.
	handle := netlink.MakeHandle(0, 1)

	fd := prog.FD()
	filter := &netlink.BpfFilter{
		FilterAttrs: netlink.FilterAttrs{
			LinkIndex: ifindex,
			Parent:    parent,
			Handle:    handle,
			Protocol:  unix.ETH_P_ALL,
			Priority:  1,
		},
		Fd:           fd,
		Name:         prog.String(),
		DirectAction: true,
	}
	if err := netlink.FilterAdd(filter); err != nil {
		return nil, fmt.Errorf("ebpf tc: add filter on ifindex %d: %w", ifindex, err)
	}

	return &TCProgram{
		prog:    prog,
		ifindex: ifindex,
		handle:  handle,
		filter:  filter,
	}, nil
}

// Detach removes the BPF filter that was installed by AttachTC.
func (p *TCProgram) Detach() error {
	if err := netlink.FilterDel(p.filter); err != nil {
		return fmt.Errorf("ebpf tc: delete filter on ifindex %d: %w", p.ifindex, err)
	}
	return nil
}

// isEEXIST returns true when the error is unix.EEXIST.
func isEEXIST(err error) bool {
	return err == unix.EEXIST
}
