package pale

import (
	"encoding/json"
	"fmt"
)

// LocalControlStorageInspectionRequest privately retains one scope and root.
// Construction performs no I/O or semantic validation; the API owns both.
type LocalControlStorageInspectionRequest struct{ wire any }

// NewPrincipalControlStorageInspectionRequest selects one principal trust store.
func NewPrincipalControlStorageInspectionRequest(runtimeStateRoot, principalID, logicalStoreID, controlIncarnationID string) LocalControlStorageInspectionRequest {
	return LocalControlStorageInspectionRequest{struct {
		Scope                string `json:"scope"`
		RuntimeStateRoot     string `json:"runtimeStateRoot"`
		PrincipalID          string `json:"principalId"`
		LogicalStoreID       string `json:"logicalStoreId"`
		ControlIncarnationID string `json:"controlIncarnationId"`
	}{"principal", runtimeStateRoot, principalID, logicalStoreID, controlIncarnationID}}
}

// NewDeploymentControlStorageInspectionRequest selects one deployment control store.
func NewDeploymentControlStorageInspectionRequest(runtimeStateRoot, workspaceID, deploymentBindingRef, deploymentBindingSHA256, logicalStoreID, controlIncarnationID string) LocalControlStorageInspectionRequest {
	return LocalControlStorageInspectionRequest{struct {
		Scope                   string `json:"scope"`
		RuntimeStateRoot        string `json:"runtimeStateRoot"`
		WorkspaceID             string `json:"workspaceId"`
		DeploymentBindingRef    string `json:"deploymentBindingRef"`
		DeploymentBindingSHA256 string `json:"deploymentBindingSha256"`
		LogicalStoreID          string `json:"logicalStoreId"`
		ControlIncarnationID    string `json:"controlIncarnationId"`
	}{"deployment", runtimeStateRoot, workspaceID, deploymentBindingRef, deploymentBindingSHA256, logicalStoreID, controlIncarnationID}}
}

// String redacts all input coordinates from ordinary formatting.
func (LocalControlStorageInspectionRequest) String() string {
	return "LocalControlStorageInspectionRequest{<redacted>}"
}

// GoString also redacts input coordinates from diagnostic formatting.
func (request LocalControlStorageInspectionRequest) GoString() string { return request.String() }

// LocalControlStoreKind projects the API's closed store-kind vocabulary.
type LocalControlStoreKind string

const (
	LocalControlStoreKindPrincipalTrust    LocalControlStoreKind = "principal-trust"
	LocalControlStoreKindDeploymentControl LocalControlStoreKind = "deployment-control"
)

// UnmarshalJSON rejects vocabulary absent from the owning API projection.
func (value *LocalControlStoreKind) UnmarshalJSON(data []byte) error {
	return decodeTransportTag(data, value,
		LocalControlStoreKindPrincipalTrust,
		LocalControlStoreKindDeploymentControl,
	)
}

// LocalControlStorageProfileRequirement projects the API's closed unselected-profile requirement vocabulary.
// Ordering comes from the returned API sequence, not declaration order here.
type LocalControlStorageProfileRequirement string

const (
	LocalControlStorageProfileRequirementSqliteCompileOptions                 LocalControlStorageProfileRequirement = "sqlite-compile-options"
	LocalControlStorageProfileRequirementSqliteSynchronous                    LocalControlStorageProfileRequirement = "sqlite-synchronous"
	LocalControlStorageProfileRequirementSqliteFullFsync                      LocalControlStorageProfileRequirement = "sqlite-full-fsync"
	LocalControlStorageProfileRequirementSqliteCheckpointFullFsync            LocalControlStorageProfileRequirement = "sqlite-checkpoint-full-fsync"
	LocalControlStorageProfileRequirementSqliteCheckpointMode                 LocalControlStorageProfileRequirement = "sqlite-checkpoint-mode"
	LocalControlStorageProfileRequirementSqliteWalAutocheckpointFrames        LocalControlStorageProfileRequirement = "sqlite-wal-autocheckpoint-frames"
	LocalControlStorageProfileRequirementSqliteDurabilityGrouping             LocalControlStorageProfileRequirement = "sqlite-durability-grouping"
	LocalControlStorageProfileRequirementAuthorityFrameEncoding               LocalControlStorageProfileRequirement = "authority-frame-encoding"
	LocalControlStorageProfileRequirementAuthorityCorruptionChecksum          LocalControlStorageProfileRequirement = "authority-corruption-checksum"
	LocalControlStorageProfileRequirementAuthorityCorruptionChecksumBytes     LocalControlStorageProfileRequirement = "authority-corruption-checksum-bytes"
	LocalControlStorageProfileRequirementAuthorityMaximumEntryBytes           LocalControlStorageProfileRequirement = "authority-maximum-entry-bytes"
	LocalControlStorageProfileRequirementAuthorityMaximumCheckpointBytes      LocalControlStorageProfileRequirement = "authority-maximum-checkpoint-bytes"
	LocalControlStorageProfileRequirementAuthorityMaximumCheckpointCandidates LocalControlStorageProfileRequirement = "authority-maximum-checkpoint-candidates"
	LocalControlStorageProfileRequirementAuthorityMaximumTailEntries          LocalControlStorageProfileRequirement = "authority-maximum-tail-entries"
	LocalControlStorageProfileRequirementAuthorityCheckpointCadence           LocalControlStorageProfileRequirement = "authority-checkpoint-cadence"
	LocalControlStorageProfileRequirementAuthorityRetentionPolicy             LocalControlStorageProfileRequirement = "authority-retention-policy"
	LocalControlStorageProfileRequirementAuthorityCompactionPolicy            LocalControlStorageProfileRequirement = "authority-compaction-policy"
	LocalControlStorageProfileRequirementBackupRestorePolicy                  LocalControlStorageProfileRequirement = "backup-restore-policy"
)

// UnmarshalJSON rejects vocabulary absent from the owning API projection.
func (value *LocalControlStorageProfileRequirement) UnmarshalJSON(data []byte) error {
	return decodeTransportTag(data, value,
		LocalControlStorageProfileRequirementSqliteCompileOptions,
		LocalControlStorageProfileRequirementSqliteSynchronous,
		LocalControlStorageProfileRequirementSqliteFullFsync,
		LocalControlStorageProfileRequirementSqliteCheckpointFullFsync,
		LocalControlStorageProfileRequirementSqliteCheckpointMode,
		LocalControlStorageProfileRequirementSqliteWalAutocheckpointFrames,
		LocalControlStorageProfileRequirementSqliteDurabilityGrouping,
		LocalControlStorageProfileRequirementAuthorityFrameEncoding,
		LocalControlStorageProfileRequirementAuthorityCorruptionChecksum,
		LocalControlStorageProfileRequirementAuthorityCorruptionChecksumBytes,
		LocalControlStorageProfileRequirementAuthorityMaximumEntryBytes,
		LocalControlStorageProfileRequirementAuthorityMaximumCheckpointBytes,
		LocalControlStorageProfileRequirementAuthorityMaximumCheckpointCandidates,
		LocalControlStorageProfileRequirementAuthorityMaximumTailEntries,
		LocalControlStorageProfileRequirementAuthorityCheckpointCadence,
		LocalControlStorageProfileRequirementAuthorityRetentionPolicy,
		LocalControlStorageProfileRequirementAuthorityCompactionPolicy,
		LocalControlStorageProfileRequirementBackupRestorePolicy,
	)
}

// LocalControlStorageInspectionRejectionKind projects the API's closed physical-rejection vocabulary.
type LocalControlStorageInspectionRejectionKind string

const (
	LocalControlStorageInspectionRejectionKindUnsafeEntryKind           LocalControlStorageInspectionRejectionKind = "unsafe-entry-kind"
	LocalControlStorageInspectionRejectionKindCorruptOrPartial          LocalControlStorageInspectionRejectionKind = "corrupt-or-partial"
	LocalControlStorageInspectionRejectionKindWrongStoreKind            LocalControlStorageInspectionRejectionKind = "wrong-store-kind"
	LocalControlStorageInspectionRejectionKindWrongLogicalStoreId       LocalControlStorageInspectionRejectionKind = "wrong-logical-store-id"
	LocalControlStorageInspectionRejectionKindWrongControlIncarnation   LocalControlStorageInspectionRejectionKind = "wrong-control-incarnation"
	LocalControlStorageInspectionRejectionKindWrongPrincipalScope       LocalControlStorageInspectionRejectionKind = "wrong-principal-scope"
	LocalControlStorageInspectionRejectionKindWrongDeploymentScope      LocalControlStorageInspectionRejectionKind = "wrong-deployment-scope"
	LocalControlStorageInspectionRejectionKindSourceIdentityMismatch    LocalControlStorageInspectionRejectionKind = "source-identity-mismatch"
	LocalControlStorageInspectionRejectionKindNewerVersion              LocalControlStorageInspectionRejectionKind = "newer-version"
	LocalControlStorageInspectionRejectionKindSqliteBuildMismatch       LocalControlStorageInspectionRejectionKind = "sqlite-build-mismatch"
	LocalControlStorageInspectionRejectionKindStructuralProfileMismatch LocalControlStorageInspectionRejectionKind = "structural-profile-mismatch"
)

// UnmarshalJSON rejects vocabulary absent from the owning API projection.
func (value *LocalControlStorageInspectionRejectionKind) UnmarshalJSON(data []byte) error {
	return decodeTransportTag(data, value,
		LocalControlStorageInspectionRejectionKindUnsafeEntryKind,
		LocalControlStorageInspectionRejectionKindCorruptOrPartial,
		LocalControlStorageInspectionRejectionKindWrongStoreKind,
		LocalControlStorageInspectionRejectionKindWrongLogicalStoreId,
		LocalControlStorageInspectionRejectionKindWrongControlIncarnation,
		LocalControlStorageInspectionRejectionKindWrongPrincipalScope,
		LocalControlStorageInspectionRejectionKindWrongDeploymentScope,
		LocalControlStorageInspectionRejectionKindSourceIdentityMismatch,
		LocalControlStorageInspectionRejectionKindNewerVersion,
		LocalControlStorageInspectionRejectionKindSqliteBuildMismatch,
		LocalControlStorageInspectionRejectionKindStructuralProfileMismatch,
	)
}

// LocalControlStorageInspectionStateKind projects the API's closed physical-state vocabulary.
type LocalControlStorageInspectionStateKind string

const (
	LocalControlStorageInspectionStateKindMissing           LocalControlStorageInspectionStateKind = "missing"
	LocalControlStorageInspectionStateKindEmpty             LocalControlStorageInspectionStateKind = "empty"
	LocalControlStorageInspectionStateKindProfileUnselected LocalControlStorageInspectionStateKind = "profile-unselected"
	// Physical checks match; writer, resident and native readiness remain unproven.
	LocalControlStorageInspectionStateKindReady    LocalControlStorageInspectionStateKind = "ready"
	LocalControlStorageInspectionStateKindRejected LocalControlStorageInspectionStateKind = "rejected"
)

// UnmarshalJSON rejects vocabulary absent from the owning API projection.
func (value *LocalControlStorageInspectionStateKind) UnmarshalJSON(data []byte) error {
	return decodeTransportTag(data, value,
		LocalControlStorageInspectionStateKindMissing,
		LocalControlStorageInspectionStateKindEmpty,
		LocalControlStorageInspectionStateKindProfileUnselected,
		LocalControlStorageInspectionStateKindReady,
		LocalControlStorageInspectionStateKindRejected,
	)
}

// LocalControlStorageInspectionRejection retains a physical contradiction.
// Version evidence exists only for NewerVersion; it is not runtime readiness.
type LocalControlStorageInspectionRejection struct {
	kind                LocalControlStorageInspectionRejectionKind
	observed, supported uint32
}

// Kind returns the exact physical rejection.
func (rejection LocalControlStorageInspectionRejection) Kind() LocalControlStorageInspectionRejectionKind {
	return rejection.kind
}

// Versions returns observed and supported versions only for NewerVersion.
func (rejection LocalControlStorageInspectionRejection) Versions() (uint32, uint32, bool) {
	return rejection.observed, rejection.supported, rejection.kind == LocalControlStorageInspectionRejectionKindNewerVersion
}

// UnmarshalJSON decodes the tagged wire evidence without inspecting a store.
func (rejection *LocalControlStorageInspectionRejection) UnmarshalJSON(data []byte) error {
	var wire struct {
		Kind      LocalControlStorageInspectionRejectionKind `json:"kind"`
		Observed  json.RawMessage                            `json:"observed"`
		Supported json.RawMessage                            `json:"supported"`
	}
	if err := decodeTransportJSON(data, &wire); err != nil {
		return err
	}
	if wire.Kind == "" {
		return fmt.Errorf("missing control-store rejection kind")
	}
	next := LocalControlStorageInspectionRejection{kind: wire.Kind}
	if wire.Kind == LocalControlStorageInspectionRejectionKindNewerVersion {
		var observed, supported *uint32
		if err := json.Unmarshal(wire.Observed, &observed); err != nil {
			return err
		}
		if err := json.Unmarshal(wire.Supported, &supported); err != nil {
			return err
		}
		if observed == nil || supported == nil {
			return fmt.Errorf("missing control-store version evidence")
		}
		next.observed, next.supported = *observed, *supported
	} else if len(wire.Observed) != 0 || len(wire.Supported) != 0 {
		return fmt.Errorf("unexpected control-store version evidence")
	}
	*rejection = next
	return nil
}

// MarshalJSON preserves the API's tagged physical-evidence shape.
func (rejection LocalControlStorageInspectionRejection) MarshalJSON() ([]byte, error) {
	if rejection.kind == "" {
		return nil, fmt.Errorf("missing control-store rejection kind")
	}
	if rejection.kind == LocalControlStorageInspectionRejectionKindNewerVersion {
		return json.Marshal(struct {
			Kind      LocalControlStorageInspectionRejectionKind `json:"kind"`
			Observed  uint32                                     `json:"observed"`
			Supported uint32                                     `json:"supported"`
		}{rejection.kind, rejection.observed, rejection.supported})
	}
	return json.Marshal(struct {
		Kind LocalControlStorageInspectionRejectionKind `json:"kind"`
	}{rejection.kind})
}

// LocalControlStorageInspectionState is a physical observation, not admission.
// Its variant payload cannot be independently combined with another state.
type LocalControlStorageInspectionState struct {
	kind       LocalControlStorageInspectionStateKind
	unresolved []LocalControlStorageProfileRequirement
	rejection  *LocalControlStorageInspectionRejection
}

// Kind returns the exact physical state, without inferring runtime readiness.
func (state LocalControlStorageInspectionState) Kind() LocalControlStorageInspectionStateKind {
	return state.kind
}

// Unresolved returns a copy in source-owned order, or nil outside ProfileUnselected.
func (state LocalControlStorageInspectionState) Unresolved() []LocalControlStorageProfileRequirement {
	if state.kind != LocalControlStorageInspectionStateKindProfileUnselected {
		return nil
	}
	return append([]LocalControlStorageProfileRequirement{}, state.unresolved...)
}

// Rejection returns the retained physical contradiction, if present.
func (state LocalControlStorageInspectionState) Rejection() *LocalControlStorageInspectionRejection {
	if state.rejection == nil {
		return nil
	}
	copy := *state.rejection
	return &copy
}

// UnmarshalJSON decodes only the API-selected variant payload.
func (state *LocalControlStorageInspectionState) UnmarshalJSON(data []byte) error {
	var wire struct {
		Kind    LocalControlStorageInspectionStateKind `json:"kind"`
		Details json.RawMessage                        `json:"details"`
	}
	if err := decodeTransportJSON(data, &wire); err != nil {
		return err
	}
	next := LocalControlStorageInspectionState{kind: wire.Kind}
	switch wire.Kind {
	case LocalControlStorageInspectionStateKindMissing, LocalControlStorageInspectionStateKindEmpty, LocalControlStorageInspectionStateKindReady:
		if len(wire.Details) != 0 {
			return fmt.Errorf("unexpected control-store state details")
		}
	case LocalControlStorageInspectionStateKindProfileUnselected:
		var details struct {
			Unresolved *[]LocalControlStorageProfileRequirement `json:"unresolved"`
		}
		if err := decodeTransportJSON(wire.Details, &details); err != nil {
			return err
		}
		if details.Unresolved == nil {
			return fmt.Errorf("missing control-store requirements")
		}
		next.unresolved = *details.Unresolved
	case LocalControlStorageInspectionStateKindRejected:
		var rejection LocalControlStorageInspectionRejection
		if err := json.Unmarshal(wire.Details, &rejection); err != nil {
			return err
		}
		next.rejection = &rejection
	default:
		return fmt.Errorf("missing control-store state kind")
	}
	*state = next
	return nil
}

// MarshalJSON preserves the API's physical-state shape and requirement order.
func (state LocalControlStorageInspectionState) MarshalJSON() ([]byte, error) {
	var details any
	switch state.kind {
	case LocalControlStorageInspectionStateKindMissing, LocalControlStorageInspectionStateKindEmpty, LocalControlStorageInspectionStateKindReady:
	case LocalControlStorageInspectionStateKindProfileUnselected:
		details = struct {
			Unresolved []LocalControlStorageProfileRequirement `json:"unresolved"`
		}{state.Unresolved()}
	case LocalControlStorageInspectionStateKindRejected:
		details = state.rejection
	default:
		return nil, fmt.Errorf("missing control-store state kind")
	}
	return json.Marshal(struct {
		Kind    LocalControlStorageInspectionStateKind `json:"kind"`
		Details any                                    `json:"details,omitempty"`
	}{state.kind, details})
}

// LocalControlStorageInspection is a path-free physical result.
// Decoding its JSON is shape projection, not store, writer or resident admission.
type LocalControlStorageInspection struct {
	Kind  LocalControlStoreKind              `json:"kind"`
	State LocalControlStorageInspectionState `json:"state"`
}

// UnmarshalJSON rejects incomplete or mismatched private response shapes.
func (inspection *LocalControlStorageInspection) UnmarshalJSON(data []byte) error {
	var wire struct {
		Kind  *LocalControlStoreKind              `json:"kind"`
		State *LocalControlStorageInspectionState `json:"state"`
	}
	if err := decodeTransportJSON(data, &wire); err != nil {
		return err
	}
	if wire.Kind == nil || wire.State == nil {
		return fmt.Errorf("missing control-store inspection fields")
	}
	*inspection = LocalControlStorageInspection{Kind: *wire.Kind, State: *wire.State}
	return nil
}

// InspectLocalControlStore performs one API-owned read-only physical inspection.
// Missing and rejected stores are typed results. Invalid coordinates or host
// failures retain canonical DiagnosticError evidence. No store is initialized,
// repaired, admitted or activated; even Ready supplies no runtime authority.
func (client *Client) InspectLocalControlStore(request LocalControlStorageInspectionRequest) (LocalControlStorageInspection, error) {
	result, err := client.call(request.wire, opInspectLocalControlStore)
	if err != nil {
		return LocalControlStorageInspection{}, err
	}
	return decodeMetadata[LocalControlStorageInspection](result)
}
