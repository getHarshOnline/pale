package pale

import (
	"encoding/json"
	"fmt"
)

// WorkspaceSubjectComparison returns selected values from the retained pair.
// ReceiptDigest links to that pair's comparison receipt. Subject preserves the
// full semantic identity. Neither these copies nor their edits grant authority.
type WorkspaceSubjectComparison struct {
	ReceiptDigest string                 `json:"receiptDigest"`
	Subject       WorkspaceSubjectValue  `json:"subject"`
	Values        WorkspaceSubjectValues `json:"values"`
}

// WorkspaceSubjectValues distinguishes absence from a one-sided or shared value.
type WorkspaceSubjectValues struct {
	Value WorkspaceSubjectMembership
}

// WorkspaceSubjectMembership is a closed set of four membership alternatives.
type WorkspaceSubjectMembership interface {
	isWorkspaceSubjectMembership()
}

// AbsentWorkspaceSubject has no selected output in either compilation.
type AbsentWorkspaceSubject struct{}

// PriorOnlyWorkspaceSubject has an output only in the prior compilation.
type PriorOnlyWorkspaceSubject struct {
	PriorFact WorkspaceFact `json:"priorFact"`
}

// CurrentOnlyWorkspaceSubject has an output only in the current compilation.
type CurrentOnlyWorkspaceSubject struct {
	CurrentFact WorkspaceFact `json:"currentFact"`
}

// SharedWorkspaceSubject retains both outputs and the compiler's value comparison.
// Equal values can still be affected by a change to their proof inputs.
type SharedWorkspaceSubject struct {
	PriorFact   WorkspaceFact            `json:"priorFact"`
	CurrentFact WorkspaceFact            `json:"currentFact"`
	Comparison  WorkspaceValueComparison `json:"comparison"`
}

func (AbsentWorkspaceSubject) isWorkspaceSubjectMembership()      {}
func (PriorOnlyWorkspaceSubject) isWorkspaceSubjectMembership()   {}
func (CurrentOnlyWorkspaceSubject) isWorkspaceSubjectMembership() {}
func (SharedWorkspaceSubject) isWorkspaceSubjectMembership()      {}

// WorkspaceValueComparison describes typed output equality, not impact or readiness.
type WorkspaceValueComparison string

const (
	WorkspaceValuesEqual   WorkspaceValueComparison = "equal"
	WorkspaceValuesChanged WorkspaceValueComparison = "changed"
)

// UnmarshalJSON accepts only the SDK's two selected-value comparison states.
func (value *WorkspaceValueComparison) UnmarshalJSON(data []byte) error {
	return decodeTransportTag(data, value, WorkspaceValuesEqual, WorkspaceValuesChanged)
}

// UnmarshalJSON replaces the result only after its complete shape is validated.
func (value *WorkspaceSubjectComparison) UnmarshalJSON(data []byte) error {
	var next struct {
		ReceiptDigest string                        `json:"receiptDigest"`
		Subject       portableWorkspaceSubjectValue `json:"subject"`
		Values        WorkspaceSubjectValues        `json:"values"`
	}
	if err := decodeTransportObject(data, &next, []string{"receiptDigest", "subject", "values"}); err != nil {
		return err
	}
	*value = WorkspaceSubjectComparison{ReceiptDigest: next.ReceiptDigest, Subject: WorkspaceSubjectValue(next.Subject), Values: next.Values}
	return nil
}

// Portable derivation and effective-profile responses have distinct tag
// spellings. Keep each wire vocabulary exact while reusing the subject payload
// types; these five payload-free variants must not become transport aliases.
type portableWorkspaceSubjectValue WorkspaceSubjectValue

func (value *portableWorkspaceSubjectValue) UnmarshalJSON(data []byte) error {
	fields, err := transportObjectFields(data)
	if err != nil {
		return err
	}
	var tag string
	if err := json.Unmarshal(fields["kind"], &tag); err != nil {
		return fmt.Errorf("invalid portable subject kind")
	}
	var next WorkspaceSubjectValue
	switch tag {
	case "responsibility_closure":
		next.Kind = WorkspaceSubjectValueResponsibilityClosureKind
	case "organizational_translation_closure":
		next.Kind = WorkspaceSubjectValueOrganizationalTranslationClosureKind
	case "experience_composition":
		next.Kind = WorkspaceSubjectValueExperienceCompositionKind
	case "resolution_lock":
		next.Kind = WorkspaceSubjectValueResolutionLockKind
	case "universe_lock":
		next.Kind = WorkspaceSubjectValueUniverseLockKind
	case "source", "component", "module", "declaration", "import", "extension", "policy", "resource", "workspace":
		if err := next.UnmarshalJSON(data); err != nil {
			return err
		}
		*value = portableWorkspaceSubjectValue(next)
		return nil
	default:
		return fmt.Errorf("unknown portable subject kind")
	}
	var unit struct {
		Kind string `json:"kind"`
	}
	if err := decodeTransportObject(data, &unit, []string{"kind"}); err != nil {
		return err
	}
	*value = portableWorkspaceSubjectValue(next)
	return nil
}

// UnmarshalJSON rejects missing, null, or cross-variant payloads atomically.
func (value *WorkspaceSubjectValues) UnmarshalJSON(data []byte) error {
	variants := map[string][]string{
		"absent":       {},
		"prior_only":   {"priorFact"},
		"current_only": {"currentFact"},
		"shared":       {"priorFact", "currentFact", "comparison"},
	}
	kind, err := transportVariant(data, "membership", variants)
	if err != nil {
		return err
	}
	var wire struct {
		Membership  string                   `json:"membership"`
		PriorFact   WorkspaceFact            `json:"priorFact"`
		CurrentFact WorkspaceFact            `json:"currentFact"`
		Comparison  WorkspaceValueComparison `json:"comparison"`
	}
	if err := decodeTransportObject(data, &wire, append([]string{"membership"}, variants[kind]...)); err != nil {
		return err
	}
	var next WorkspaceSubjectMembership
	switch kind {
	case "absent":
		next = AbsentWorkspaceSubject{}
	case "prior_only":
		next = PriorOnlyWorkspaceSubject{PriorFact: wire.PriorFact}
	case "current_only":
		next = CurrentOnlyWorkspaceSubject{CurrentFact: wire.CurrentFact}
	case "shared":
		next = SharedWorkspaceSubject{PriorFact: wire.PriorFact, CurrentFact: wire.CurrentFact, Comparison: wire.Comparison}
	}
	*value = WorkspaceSubjectValues{Value: next}
	return nil
}
