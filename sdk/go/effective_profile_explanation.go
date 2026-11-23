package pale

import (
	"bytes"
	"encoding/json"
	"fmt"
)

// EffectiveProfileExplanation is copied compiler evidence for an exact opaque
// handle. It remains readable after its profile closes and grants no authority.
type EffectiveProfileExplanation struct {
	Handle   string                              `json:"handle"`
	Evidence EffectiveProfileExplanationEvidence `json:"evidence"`
}

// UnmarshalJSON checks the complete SDK record without admitting its handle.
func (value *EffectiveProfileExplanation) UnmarshalJSON(data []byte) error {
	type plain EffectiveProfileExplanation
	var next plain
	if err := decodeTransportObject(data, &next, []string{"handle", "evidence"}); err != nil {
		return err
	}
	*value = EffectiveProfileExplanation(next)
	return nil
}

// EffectiveProfileExplanationEvidenceKind identifies the SDK-selected proof variant.
type EffectiveProfileExplanationEvidenceKind string

const (
	EffectiveProfileEvidenceDeclaredSelection        EffectiveProfileExplanationEvidenceKind = "declared-selection"
	EffectiveProfileEvidenceResponsibility           EffectiveProfileExplanationEvidenceKind = "responsibility"
	EffectiveProfileEvidenceEffectiveSelection       EffectiveProfileExplanationEvidenceKind = "effective-selection"
	EffectiveProfileEvidenceExecutionPath            EffectiveProfileExplanationEvidenceKind = "execution-path"
	EffectiveProfileEvidenceOperationClassification  EffectiveProfileExplanationEvidenceKind = "operation-classification"
	EffectiveProfileEvidenceCapabilityClassification EffectiveProfileExplanationEvidenceKind = "capability-classification"
	EffectiveProfileEvidenceFindingClassification    EffectiveProfileExplanationEvidenceKind = "finding-classification"
	EffectiveProfileEvidenceMachineRealization       EffectiveProfileExplanationEvidenceKind = "machine-realization"
	EffectiveProfileEvidenceConfiguration            EffectiveProfileExplanationEvidenceKind = "configuration"
)

// EffectiveProfileExplanationEvidence retains exactly one selected payload.
// Configuration has no payload. Decoding preserves the SDK's classifications;
// editing these copied values cannot change the retained profile.
type EffectiveProfileExplanationEvidence struct {
	Kind                     EffectiveProfileExplanationEvidenceKind
	DeclaredSelection        *EffectiveProfileExplanationDeclaredSelection
	Responsibility           *EffectiveProfileExplanationResponsibility
	EffectiveSelection       *EffectiveProfileExplanationEffectiveSelection
	ExecutionPath            *EffectiveProfileExplanationExecutionPath
	OperationClassification  *EffectiveProfileExplanationOperationClassification
	CapabilityClassification *EffectiveProfileExplanationCapabilityClassification
	FindingClassification    *EffectiveProfileExplanationFindingClassification
	MachineRealization       *EffectiveProfileExplanationMachineRealization
}

// EffectiveProfileExplanationDeclaredSelection copies the normalized declared selection.
type EffectiveProfileExplanationDeclaredSelection struct {
	Selection EffectiveProfileDeclaredSelection `json:"selection"`
}

// EffectiveProfileExplanationResponsibility copies a retained responsibility
// and its ordered composition reasons.
type EffectiveProfileExplanationResponsibility struct {
	ContractID      string                       `json:"contractId"`
	ExecutionPathID string                       `json:"executionPathId"`
	ReasonSteps     []EffectiveProfilePathReason `json:"reasonSteps"`
}

// EffectiveProfileExplanationEffectiveSelection copies the admitted effective selection.
type EffectiveProfileExplanationEffectiveSelection struct {
	Selection EffectiveProfileSelection `json:"selection"`
}

// EffectiveProfileExplanationExecutionPath copies a selected path and its
// ordered composition reasons.
type EffectiveProfileExplanationExecutionPath struct {
	SelectionID string                       `json:"selectionId"`
	ReasonSteps []EffectiveProfilePathReason `json:"reasonSteps"`
}

// EffectiveProfileExplanationOperationClassification copies independent
// operation exposure and native-evidence classifications.
type EffectiveProfileExplanationOperationClassification struct {
	Exposure          EffectiveProfileOperationExposure `json:"exposure"`
	NativeRealization EffectiveProfileNativeRealization `json:"nativeRealization"`
}

// EffectiveProfileExplanationCapabilityClassification copies independent
// requirement, provider and native-evidence classifications.
type EffectiveProfileExplanationCapabilityClassification struct {
	RequirementDisposition EffectiveProfileRequirementDisposition `json:"requirementDisposition"`
	ProviderDisposition    EffectiveProfileProviderDisposition    `json:"providerDisposition"`
	NativeRealization      EffectiveProfileNativeRealization      `json:"nativeRealization"`
}

// EffectiveProfileExplanationFindingClassification copies the retained finding,
// without deriving presentation prose.
type EffectiveProfileExplanationFindingClassification struct {
	Finding EffectiveProfileFinding `json:"finding"`
}

// EffectiveProfileExplanationMachineRealization copies the SDK's native-evidence
// classification, not a readiness claim.
type EffectiveProfileExplanationMachineRealization struct {
	Realization EffectiveProfileNativeRealization `json:"realization"`
}

// UnmarshalJSON rejects unknown or incomplete variants and commits atomically.
func (value *EffectiveProfileExplanationEvidence) UnmarshalJSON(data []byte) error {
	kind, err := transportVariant(data, "kind", map[string][]string{
		"declared-selection":        {"selection"},
		"responsibility":            {"contractId", "executionPathId", "reasonSteps"},
		"effective-selection":       {"selection"},
		"execution-path":            {"selectionId", "reasonSteps"},
		"operation-classification":  {"exposure", "nativeRealization"},
		"capability-classification": {"requirementDisposition", "providerDisposition", "nativeRealization"},
		"finding-classification":    {"finding"},
		"machine-realization":       {"realization"},
		"configuration":             {},
	})
	if err != nil {
		return err
	}
	next := EffectiveProfileExplanationEvidence{Kind: EffectiveProfileExplanationEvidenceKind(kind)}
	switch kind {
	case "declared-selection":
		var wire struct {
			Kind string `json:"kind"`
			EffectiveProfileExplanationDeclaredSelection
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "selection"}); err != nil {
			return err
		}
		next.DeclaredSelection = &wire.EffectiveProfileExplanationDeclaredSelection
	case "responsibility":
		var wire struct {
			Kind string `json:"kind"`
			EffectiveProfileExplanationResponsibility
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "contractId", "executionPathId", "reasonSteps"}); err != nil {
			return err
		}
		next.Responsibility = &wire.EffectiveProfileExplanationResponsibility
	case "effective-selection":
		var wire struct {
			Kind string `json:"kind"`
			EffectiveProfileExplanationEffectiveSelection
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "selection"}); err != nil {
			return err
		}
		next.EffectiveSelection = &wire.EffectiveProfileExplanationEffectiveSelection
	case "execution-path":
		var wire struct {
			Kind string `json:"kind"`
			EffectiveProfileExplanationExecutionPath
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "selectionId", "reasonSteps"}); err != nil {
			return err
		}
		next.ExecutionPath = &wire.EffectiveProfileExplanationExecutionPath
	case "operation-classification":
		var wire struct {
			Kind string `json:"kind"`
			EffectiveProfileExplanationOperationClassification
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "exposure", "nativeRealization"}); err != nil {
			return err
		}
		next.OperationClassification = &wire.EffectiveProfileExplanationOperationClassification
	case "capability-classification":
		var wire struct {
			Kind string `json:"kind"`
			EffectiveProfileExplanationCapabilityClassification
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "requirementDisposition", "providerDisposition", "nativeRealization"}); err != nil {
			return err
		}
		next.CapabilityClassification = &wire.EffectiveProfileExplanationCapabilityClassification
	case "finding-classification":
		var wire struct {
			Kind string `json:"kind"`
			EffectiveProfileExplanationFindingClassification
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "finding"}); err != nil {
			return err
		}
		next.FindingClassification = &wire.EffectiveProfileExplanationFindingClassification
	case "machine-realization":
		var wire struct {
			Kind string `json:"kind"`
			EffectiveProfileExplanationMachineRealization
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "realization"}); err != nil {
			return err
		}
		next.MachineRealization = &wire.EffectiveProfileExplanationMachineRealization
	}
	*value = next
	return nil
}

// The scalar response contains no native allocation or canonical document.
// Whole null is the SDK's unmatched-handle result, not a missing response.
func decodeEffectiveProfileExplanation(result response, requested string) (*EffectiveProfileExplanation, error) {
	if result.object != 0 || len(result.payload) != 0 {
		return nil, fmt.Errorf("invalid explanation response envelope")
	}
	if bytes.Equal(bytes.TrimSpace(result.metadata), []byte("null")) {
		return nil, nil
	}
	var value EffectiveProfileExplanation
	if err := json.Unmarshal(result.metadata, &value); err != nil {
		return nil, fmt.Errorf("invalid explanation response metadata")
	}
	if value.Handle != requested {
		return nil, fmt.Errorf("explanation response does not match the requested handle")
	}
	return &value, nil
}
