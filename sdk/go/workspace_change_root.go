package pale

// WorkspaceChangeKind identifies one exact source transition.
type WorkspaceChangeKind string

const (
	WorkspaceChangeAdded    WorkspaceChangeKind = "added"
	WorkspaceChangeModified WorkspaceChangeKind = "modified"
	WorkspaceChangeRemoved  WorkspaceChangeKind = "removed"
)

// UnmarshalJSON accepts only the compiler's three source transitions.
func (value *WorkspaceChangeKind) UnmarshalJSON(data []byte) error {
	return decodeTransportTag(data, value, WorkspaceChangeAdded, WorkspaceChangeModified, WorkspaceChangeRemoved)
}

// WorkspaceChangeRoot preserves one content-addressed source transition.
// PriorFact is absent only for an addition. CurrentFact is absent only for a
// removal. These copied facts grant no authority over either compilation.
type WorkspaceChangeRoot struct {
	RootDigest  string                `json:"rootDigest"`
	Subject     WorkspaceSubjectValue `json:"subject"`
	Kind        WorkspaceChangeKind   `json:"kind"`
	PriorFact   *WorkspaceFact        `json:"priorFact"`
	CurrentFact *WorkspaceFact        `json:"currentFact"`
}

// UnmarshalJSON validates the complete transition before publishing it.
func (value *WorkspaceChangeRoot) UnmarshalJSON(data []byte) error {
	var wire struct {
		RootDigest  string                        `json:"rootDigest"`
		Subject     portableWorkspaceSubjectValue `json:"subject"`
		Kind        WorkspaceChangeKind           `json:"kind"`
		PriorFact   *WorkspaceFact                `json:"priorFact"`
		CurrentFact *WorkspaceFact                `json:"currentFact"`
	}
	if err := decodeTransportObject(data, &wire,
		[]string{"rootDigest", "subject", "kind"}, "priorFact", "currentFact"); err != nil {
		return err
	}
	valid := wire.Kind == WorkspaceChangeAdded && wire.PriorFact == nil && wire.CurrentFact != nil ||
		wire.Kind == WorkspaceChangeModified && wire.PriorFact != nil && wire.CurrentFact != nil ||
		wire.Kind == WorkspaceChangeRemoved && wire.PriorFact != nil && wire.CurrentFact == nil
	if !valid {
		return invalidTransportProjection("change kind contradicts its predecessor and current facts")
	}
	*value = WorkspaceChangeRoot{
		RootDigest:  wire.RootDigest,
		Subject:     WorkspaceSubjectValue(wire.Subject),
		Kind:        wire.Kind,
		PriorFact:   wire.PriorFact,
		CurrentFact: wire.CurrentFact,
	}
	return nil
}

// WorkspaceChangeRootPage is one bounded page in canonical receipt order.
type WorkspaceChangeRootPage struct {
	Entries    []WorkspaceChangeRoot `json:"entries"`
	NextOffset *uint32               `json:"nextOffset"`
}

// UnmarshalJSON rejects partial or extended page projections atomically.
func (value *WorkspaceChangeRootPage) UnmarshalJSON(data []byte) error {
	type plain WorkspaceChangeRootPage
	var next plain
	if err := decodeTransportObject(data, &next, []string{"entries"}, "nextOffset"); err != nil {
		return err
	}
	*value = WorkspaceChangeRootPage(next)
	return nil
}
