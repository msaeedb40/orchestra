// Package ebpf provides eBPF program loading and management for Orchestra.
package ebpf

import (
	"bytes"
	"fmt"
	"os"
	"path/filepath"

	ciliumebpf "github.com/cilium/ebpf"
)

// Loader manages the lifecycle of a set of eBPF objects loaded from a
// compiled BPF .o file or from embedded bytes.
type Loader struct {
	coll   *ciliumebpf.Collection
	spec   *ciliumebpf.CollectionSpec
	pinDir string
}

// New creates a Loader that will load a BPF object from bpfObjPath and pin
// its maps under pinDir.
func New(bpfObjPath, pinDir string) (*Loader, error) {
	spec, err := ciliumebpf.LoadCollectionSpec(bpfObjPath)
	if err != nil {
		return nil, fmt.Errorf("ebpf loader: load spec from %s: %w", bpfObjPath, err)
	}
	return &Loader{spec: spec, pinDir: pinDir}, nil
}

// LoadFromBytes creates a Loader from embedded BPF object bytes (for use with
// go:embed). Maps will be pinned under pinDir.
func LoadFromBytes(data []byte, pinDir string) (*Loader, error) {
	reader := bytes.NewReader(data)
	spec, err := ciliumebpf.LoadCollectionSpecFromReader(reader)
	if err != nil {
		return nil, fmt.Errorf("ebpf loader: load spec from bytes: %w", err)
	}
	return &Loader{spec: spec, pinDir: pinDir}, nil
}

// Load loads the eBPF collection from the previously loaded spec and pins
// each map under pinDir.
func (l *Loader) Load() error {
	if l.spec == nil {
		return fmt.Errorf("ebpf loader: no spec loaded; call New or LoadFromBytes first")
	}

	col, err := ciliumebpf.NewCollection(l.spec)
	if err != nil {
		return fmt.Errorf("ebpf loader: create collection: %w", err)
	}
	l.coll = col

	// Pin each map under pinDir.
	if l.pinDir != "" {
		if err := os.MkdirAll(l.pinDir, 0o700); err != nil {
			return fmt.Errorf("ebpf loader: mkdir pin dir %s: %w", l.pinDir, err)
		}
		for name, m := range col.Maps {
			pinPath := filepath.Join(l.pinDir, name)
			if err := m.Pin(pinPath); err != nil {
				return fmt.Errorf("ebpf loader: pin map %s to %s: %w", name, pinPath, err)
			}
		}
	}
	return nil
}

// Programs returns a snapshot of the loaded eBPF programs keyed by name.
// Returns an empty map if no collection has been loaded.
func (l *Loader) Programs() map[string]*ciliumebpf.Program {
	if l.coll == nil {
		return map[string]*ciliumebpf.Program{}
	}
	out := make(map[string]*ciliumebpf.Program, len(l.coll.Programs))
	for k, v := range l.coll.Programs {
		out[k] = v
	}
	return out
}

// Maps returns a snapshot of the loaded eBPF maps keyed by name.
// Returns an empty map if no collection has been loaded.
func (l *Loader) Maps() map[string]*ciliumebpf.Map {
	if l.coll == nil {
		return map[string]*ciliumebpf.Map{}
	}
	out := make(map[string]*ciliumebpf.Map, len(l.coll.Maps))
	for k, v := range l.coll.Maps {
		out[k] = v
	}
	return out
}

// Close unloads all eBPF programs and maps.
func (l *Loader) Close() error {
	if l.coll != nil {
		l.coll.Close()
		l.coll = nil
	}
	return nil
}
