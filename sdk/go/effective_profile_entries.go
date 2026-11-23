package pale

import (
	"bytes"
	"encoding/json"
	"fmt"
)

// Only these profile payloads contain nonnullable string arrays. encoding/json
// otherwise accepts null string elements as empty strings, losing transport shape.
func requireProfileStringArrayElements(data []byte, names ...string) error {
	fields, err := transportObjectFields(data)
	if err != nil {
		return err
	}
	for _, name := range names {
		var elements []json.RawMessage
		if err := json.Unmarshal(fields[name], &elements); err != nil {
			return fmt.Errorf("invalid profile string array")
		}
		for _, element := range elements {
			if bytes.Equal(bytes.TrimSpace(element), []byte("null")) {
				return fmt.Errorf("null profile string array element")
			}
		}
	}
	return nil
}

// EffectiveProfileEntryKind identifies the SDK-selected payload variant.
type EffectiveProfileEntryKind string

const (
	EffectiveProfileEntryDeclaredSelectionKind   EffectiveProfileEntryKind = "declared-selection"
	EffectiveProfileEntryResponsibilityKind      EffectiveProfileEntryKind = "responsibility"
	EffectiveProfileEntrySelectionKind           EffectiveProfileEntryKind = "selection"
	EffectiveProfileEntryCapabilityNarrowingKind EffectiveProfileEntryKind = "capability-narrowing"
	EffectiveProfileEntryGoverningCeilingKind    EffectiveProfileEntryKind = "governing-ceiling"
	EffectiveProfileEntryRepositoryKind          EffectiveProfileEntryKind = "repository"
	EffectiveProfileEntryProviderRepositoryKind  EffectiveProfileEntryKind = "provider-repository"
	EffectiveProfileEntryExecutionPathKind       EffectiveProfileEntryKind = "execution-path"
	EffectiveProfileEntryOperationKind           EffectiveProfileEntryKind = "operation"
	EffectiveProfileEntryCapabilityKind          EffectiveProfileEntryKind = "capability"
	EffectiveProfileEntryMachineRealizationKind  EffectiveProfileEntryKind = "machine-realization"
	EffectiveProfileEntryFindingKind             EffectiveProfileEntryKind = "finding"
)

// EffectiveProfileEntry retains one SDK-selected payload. Decoding establishes the
// kind/payload relationship; editing these copied fields grants no authority.
type EffectiveProfileEntry struct {
	Kind                EffectiveProfileEntryKind
	ExplanationHandle   string
	DeclaredSelection   *EffectiveProfileEntryDeclaredSelection
	Responsibility      *EffectiveProfileEntryResponsibility
	Selection           *EffectiveProfileEntrySelection
	CapabilityNarrowing *EffectiveProfileEntryCapabilityNarrowing
	GoverningCeiling    *EffectiveProfileEntryGoverningCeiling
	Repository          *EffectiveProfileEntryRepository
	ProviderRepository  *EffectiveProfileEntryProviderRepository
	ExecutionPath       *EffectiveProfileEntryExecutionPath
	Operation           *EffectiveProfileEntryOperation
	Capability          *EffectiveProfileEntryCapability
	MachineRealization  *EffectiveProfileEntryMachineRealization
	Finding             *EffectiveProfileEntryFinding
}

// EffectiveProfileEntryDeclaredSelection is the declared-selection payload.
type EffectiveProfileEntryDeclaredSelection struct {
	Selection EffectiveProfileDeclaredSelection `json:"selection"`
}

// EffectiveProfileEntryResponsibility is the responsibility payload.
type EffectiveProfileEntryResponsibility struct {
	ContractID       string                           `json:"contractId"`
	ExecutionPathID  string                           `json:"executionPathId"`
	Participant      EffectiveProfileParticipant      `json:"participant"`
	Responsibilities []EffectiveProfileResponsibility `json:"responsibilities"`
	AuthorityCeiling []EffectiveProfileEffectClass    `json:"authorityCeiling"`
	ReasonSteps      []EffectiveProfilePathReason     `json:"reasonSteps"`
	Subject          WorkspaceSubjectValue            `json:"subject"`
}

// EffectiveProfileEntrySelection is the selection payload.
type EffectiveProfileEntrySelection struct {
	Selection EffectiveProfileSelection `json:"selection"`
}

// EffectiveProfileEntryCapabilityNarrowing is the capability-narrowing payload.
type EffectiveProfileEntryCapabilityNarrowing struct {
	PermittedCapabilities []string                      `json:"permittedCapabilities"`
	ForbiddenCapabilities []string                      `json:"forbiddenCapabilities"`
	PermittedEffects      []EffectiveProfileEffectClass `json:"permittedEffects"`
	ForbiddenEffects      []EffectiveProfileEffectClass `json:"forbiddenEffects"`
}

// EffectiveProfileEntryGoverningCeiling is the governing-ceiling payload.
type EffectiveProfileEntryGoverningCeiling struct {
	Scope                 string                        `json:"scope"`
	SubjectResource       string                        `json:"subjectResource"`
	Contributors          []string                      `json:"contributors"`
	Policies              []string                      `json:"policies"`
	PermittedCapabilities []string                      `json:"permittedCapabilities"`
	ForbiddenCapabilities []string                      `json:"forbiddenCapabilities"`
	PermittedEffects      []EffectiveProfileEffectClass `json:"permittedEffects"`
	ForbiddenEffects      []EffectiveProfileEffectClass `json:"forbiddenEffects"`
}

// EffectiveProfileEntryRepository is the repository payload.
type EffectiveProfileEntryRepository struct {
	RepositoryID         string                           `json:"repositoryId"`
	MaterializationOrder uint32                           `json:"materializationOrder"`
	AccessCeiling        EffectiveProfileRepositoryAccess `json:"accessCeiling"`
}

// EffectiveProfileEntryProviderRepository is the provider-repository payload.
type EffectiveProfileEntryProviderRepository struct {
	RepositoryID               string                            `json:"repositoryId"`
	ProviderBindingID          string                            `json:"providerBindingId"`
	NativeRepositoryCoordinate string                            `json:"nativeRepositoryCoordinate"`
	NativeRealization          EffectiveProfileNativeRealization `json:"nativeRealization"`
}

// EffectiveProfileEntryExecutionPath is the execution-path payload.
type EffectiveProfileEntryExecutionPath struct {
	SelectionID                string                              `json:"selectionId"`
	ExperiencePathID           string                              `json:"experiencePathId"`
	ResponsibilityPathIdentity string                              `json:"responsibilityPathIdentity"`
	ResponsibilityContractIDs  []string                            `json:"responsibilityContractIds"`
	WorkDelegationEdgeIDs      []string                            `json:"workDelegationEdgeIds"`
	ProviderBindingIDs         []string                            `json:"providerBindingIds"`
	CapabilityCeiling          []string                            `json:"capabilityCeiling"`
	EffectCeiling              []EffectiveProfileEffectClass       `json:"effectCeiling"`
	TerminalProviderBinding    *EffectiveProfileProviderBindingRef `json:"terminalProviderBinding"`
	NativeRealization          EffectiveProfileNativeRealization   `json:"nativeRealization"`
	ReasonSteps                []EffectiveProfilePathReason        `json:"reasonSteps"`
	Subject                    WorkspaceSubjectValue               `json:"subject"`
}

// EffectiveProfileEntryOperation is the operation payload.
type EffectiveProfileEntryOperation struct {
	RepositoryID      string                            `json:"repositoryId"`
	OperationID       string                            `json:"operationId"`
	Exposure          EffectiveProfileOperationExposure `json:"exposure"`
	NativeRealization EffectiveProfileNativeRealization `json:"nativeRealization"`
}

// EffectiveProfileEntryCapability is the capability payload.
type EffectiveProfileEntryCapability struct {
	CapabilityID           string                                 `json:"capabilityId"`
	RepositoryID           string                                 `json:"repositoryId"`
	OperationID            string                                 `json:"operationId"`
	RequirementID          string                                 `json:"requirementId"`
	EffectClass            EffectiveProfileEffectClass            `json:"effectClass"`
	RequirementDisposition EffectiveProfileRequirementDisposition `json:"requirementDisposition"`
	ProviderDisposition    EffectiveProfileProviderDisposition    `json:"providerDisposition"`
	NativeRealization      EffectiveProfileNativeRealization      `json:"nativeRealization"`
}

// EffectiveProfileEntryMachineRealization is the machine-realization payload.
type EffectiveProfileEntryMachineRealization struct {
	MachineID   string                            `json:"machineId"`
	Realization EffectiveProfileNativeRealization `json:"realization"`
}

// EffectiveProfileEntryFinding is the finding payload.
type EffectiveProfileEntryFinding struct {
	Finding EffectiveProfileFinding `json:"finding"`
}

// UnmarshalJSON preserves order, exact field spelling, and atomic failure.
func (value *EffectiveProfileEntry) UnmarshalJSON(data []byte) error {
	kind, err := transportVariant(data, "kind", map[string][]string{
		"declared-selection":   {"explanationHandle", "selection"},
		"responsibility":       {"explanationHandle", "contractId", "executionPathId", "participant", "responsibilities", "authorityCeiling", "reasonSteps", "subject"},
		"selection":            {"explanationHandle", "selection"},
		"capability-narrowing": {"explanationHandle", "permittedCapabilities", "forbiddenCapabilities", "permittedEffects", "forbiddenEffects"},
		"governing-ceiling":    {"explanationHandle", "scope", "subjectResource", "contributors", "policies", "permittedCapabilities", "forbiddenCapabilities", "permittedEffects", "forbiddenEffects"},
		"repository":           {"explanationHandle", "repositoryId", "materializationOrder", "accessCeiling"},
		"provider-repository":  {"explanationHandle", "repositoryId", "providerBindingId", "nativeRepositoryCoordinate", "nativeRealization"},
		"execution-path":       {"explanationHandle", "selectionId", "experiencePathId", "responsibilityPathIdentity", "responsibilityContractIds", "workDelegationEdgeIds", "providerBindingIds", "capabilityCeiling", "effectCeiling", "terminalProviderBinding", "nativeRealization", "reasonSteps", "subject"},
		"operation":            {"explanationHandle", "repositoryId", "operationId", "exposure", "nativeRealization"},
		"capability":           {"explanationHandle", "capabilityId", "repositoryId", "operationId", "requirementId", "effectClass", "requirementDisposition", "providerDisposition", "nativeRealization"},
		"machine-realization":  {"explanationHandle", "machineId", "realization"},
		"finding":              {"explanationHandle", "finding"},
	})
	if err != nil {
		return err
	}
	next := EffectiveProfileEntry{Kind: EffectiveProfileEntryKind(kind)}
	switch kind {
	case "declared-selection":
		var wire struct {
			Kind              string `json:"kind"`
			ExplanationHandle string `json:"explanationHandle"`
			EffectiveProfileEntryDeclaredSelection
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "explanationHandle", "selection"}); err != nil {
			return err
		}
		next.ExplanationHandle = wire.ExplanationHandle
		next.DeclaredSelection = &wire.EffectiveProfileEntryDeclaredSelection
	case "responsibility":
		var wire struct {
			Kind              string `json:"kind"`
			ExplanationHandle string `json:"explanationHandle"`
			EffectiveProfileEntryResponsibility
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "explanationHandle", "contractId", "executionPathId", "participant", "responsibilities", "authorityCeiling", "reasonSteps", "subject"}); err != nil {
			return err
		}
		next.ExplanationHandle = wire.ExplanationHandle
		next.Responsibility = &wire.EffectiveProfileEntryResponsibility
	case "selection":
		var wire struct {
			Kind              string `json:"kind"`
			ExplanationHandle string `json:"explanationHandle"`
			EffectiveProfileEntrySelection
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "explanationHandle", "selection"}); err != nil {
			return err
		}
		next.ExplanationHandle = wire.ExplanationHandle
		next.Selection = &wire.EffectiveProfileEntrySelection
	case "capability-narrowing":
		if err := requireProfileStringArrayElements(data, "permittedCapabilities", "forbiddenCapabilities"); err != nil {
			return err
		}
		var wire struct {
			Kind              string `json:"kind"`
			ExplanationHandle string `json:"explanationHandle"`
			EffectiveProfileEntryCapabilityNarrowing
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "explanationHandle", "permittedCapabilities", "forbiddenCapabilities", "permittedEffects", "forbiddenEffects"}); err != nil {
			return err
		}
		next.ExplanationHandle = wire.ExplanationHandle
		next.CapabilityNarrowing = &wire.EffectiveProfileEntryCapabilityNarrowing
	case "governing-ceiling":
		if err := requireProfileStringArrayElements(data, "contributors", "policies", "permittedCapabilities", "forbiddenCapabilities"); err != nil {
			return err
		}
		var wire struct {
			Kind              string `json:"kind"`
			ExplanationHandle string `json:"explanationHandle"`
			EffectiveProfileEntryGoverningCeiling
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "explanationHandle", "scope", "subjectResource", "contributors", "policies", "permittedCapabilities", "forbiddenCapabilities", "permittedEffects", "forbiddenEffects"}); err != nil {
			return err
		}
		next.ExplanationHandle = wire.ExplanationHandle
		next.GoverningCeiling = &wire.EffectiveProfileEntryGoverningCeiling
	case "repository":
		var wire struct {
			Kind              string `json:"kind"`
			ExplanationHandle string `json:"explanationHandle"`
			EffectiveProfileEntryRepository
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "explanationHandle", "repositoryId", "materializationOrder", "accessCeiling"}); err != nil {
			return err
		}
		next.ExplanationHandle = wire.ExplanationHandle
		next.Repository = &wire.EffectiveProfileEntryRepository
	case "provider-repository":
		var wire struct {
			Kind              string `json:"kind"`
			ExplanationHandle string `json:"explanationHandle"`
			EffectiveProfileEntryProviderRepository
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "explanationHandle", "repositoryId", "providerBindingId", "nativeRepositoryCoordinate", "nativeRealization"}); err != nil {
			return err
		}
		next.ExplanationHandle = wire.ExplanationHandle
		next.ProviderRepository = &wire.EffectiveProfileEntryProviderRepository
	case "execution-path":
		if err := requireProfileStringArrayElements(data, "responsibilityContractIds", "workDelegationEdgeIds", "providerBindingIds", "capabilityCeiling"); err != nil {
			return err
		}
		var wire struct {
			Kind              string `json:"kind"`
			ExplanationHandle string `json:"explanationHandle"`
			EffectiveProfileEntryExecutionPath
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "explanationHandle", "selectionId", "experiencePathId", "responsibilityPathIdentity", "responsibilityContractIds", "workDelegationEdgeIds", "providerBindingIds", "capabilityCeiling", "effectCeiling", "nativeRealization", "reasonSteps", "subject"}, "terminalProviderBinding"); err != nil {
			return err
		}
		next.ExplanationHandle = wire.ExplanationHandle
		next.ExecutionPath = &wire.EffectiveProfileEntryExecutionPath
	case "operation":
		var wire struct {
			Kind              string `json:"kind"`
			ExplanationHandle string `json:"explanationHandle"`
			EffectiveProfileEntryOperation
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "explanationHandle", "repositoryId", "operationId", "exposure", "nativeRealization"}); err != nil {
			return err
		}
		next.ExplanationHandle = wire.ExplanationHandle
		next.Operation = &wire.EffectiveProfileEntryOperation
	case "capability":
		var wire struct {
			Kind              string `json:"kind"`
			ExplanationHandle string `json:"explanationHandle"`
			EffectiveProfileEntryCapability
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "explanationHandle", "capabilityId", "repositoryId", "operationId", "requirementId", "effectClass", "requirementDisposition", "providerDisposition", "nativeRealization"}); err != nil {
			return err
		}
		next.ExplanationHandle = wire.ExplanationHandle
		next.Capability = &wire.EffectiveProfileEntryCapability
	case "machine-realization":
		var wire struct {
			Kind              string `json:"kind"`
			ExplanationHandle string `json:"explanationHandle"`
			EffectiveProfileEntryMachineRealization
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "explanationHandle", "machineId", "realization"}); err != nil {
			return err
		}
		next.ExplanationHandle = wire.ExplanationHandle
		next.MachineRealization = &wire.EffectiveProfileEntryMachineRealization
	case "finding":
		var wire struct {
			Kind              string `json:"kind"`
			ExplanationHandle string `json:"explanationHandle"`
			EffectiveProfileEntryFinding
		}
		if err := decodeTransportObject(data, &wire, []string{"kind", "explanationHandle", "finding"}); err != nil {
			return err
		}
		next.ExplanationHandle = wire.ExplanationHandle
		next.Finding = &wire.EffectiveProfileEntryFinding
	}
	*value = next
	return nil
}
