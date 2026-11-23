package pale

// WorkspaceImpactState says whether a current semantic subject is reachable
// from an exact change root. It does not describe readiness or execution state.
type WorkspaceImpactState string

const (
	WorkspaceImpactAffected   WorkspaceImpactState = "affected"
	WorkspaceImpactUnaffected WorkspaceImpactState = "unaffected"
)

// UnmarshalJSON accepts only the compiler's two impact classifications.
func (value *WorkspaceImpactState) UnmarshalJSON(data []byte) error {
	return decodeTransportTag(data, value, WorkspaceImpactAffected, WorkspaceImpactUnaffected)
}

// WorkspaceImpact is one member of the current comparison's complete impact
// partition. Affected entries name at least one contributing change root;
// unaffected entries name none.
type WorkspaceImpact struct {
	Subject               WorkspaceSubjectValue `json:"subject"`
	State                 WorkspaceImpactState  `json:"state"`
	ContributingRootCount uint32                `json:"contributingRootCount"`
}

// UnmarshalJSON validates the complete impact before publishing any fields.
func (value *WorkspaceImpact) UnmarshalJSON(data []byte) error {
	var wire struct {
		Subject               portableWorkspaceSubjectValue `json:"subject"`
		State                 WorkspaceImpactState          `json:"state"`
		ContributingRootCount uint32                        `json:"contributingRootCount"`
	}
	if err := decodeTransportObject(data, &wire, []string{
		"subject", "state", "contributingRootCount",
	}); err != nil {
		return err
	}
	if (wire.State == WorkspaceImpactAffected) != (wire.ContributingRootCount > 0) {
		return invalidTransportProjection("impact state contradicts its contributing root count")
	}
	*value = WorkspaceImpact{
		Subject:               WorkspaceSubjectValue(wire.Subject),
		State:                 wire.State,
		ContributingRootCount: wire.ContributingRootCount,
	}
	return nil
}

// WorkspaceImpactPage is one bounded page of the current impact partition.
// NextOffset is absent only when the retained comparison has no later entries.
type WorkspaceImpactPage struct {
	Entries    []WorkspaceImpact `json:"entries"`
	NextOffset *uint32           `json:"nextOffset"`
}

// UnmarshalJSON rejects partial or extended page projections atomically.
func (value *WorkspaceImpactPage) UnmarshalJSON(data []byte) error {
	type plain WorkspaceImpactPage
	var next plain
	if err := decodeTransportObject(data, &next, []string{"entries"}, "nextOffset"); err != nil {
		return err
	}
	*value = WorkspaceImpactPage(next)
	return nil
}
