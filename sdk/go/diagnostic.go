package pale

import (
	"encoding/json"
	"strings"
)

// DiagnosticStage identifies the Engine phase that owns a diagnostic.
type DiagnosticStage string

const (
	// DiagnosticInputAdmission covers external input admission.
	DiagnosticInputAdmission DiagnosticStage = "input_admission"
	// DiagnosticContractValidation covers generated contract validation.
	DiagnosticContractValidation DiagnosticStage = "contract_validation"
	// DiagnosticSemanticAdmission covers owner-specific semantic admission.
	DiagnosticSemanticAdmission DiagnosticStage = "semantic_admission"
	// DiagnosticPackageLoading covers Package source loading.
	DiagnosticPackageLoading DiagnosticStage = "package_loading"
	// DiagnosticWorkspaceLoading covers workspace source loading.
	DiagnosticWorkspaceLoading DiagnosticStage = "workspace_loading"
	// DiagnosticCompilation covers compiler resolution or sealing.
	DiagnosticCompilation DiagnosticStage = "compilation"
	// DiagnosticRendering covers generated-surface rendering.
	DiagnosticRendering DiagnosticStage = "rendering"
	// DiagnosticOperationAdmission covers operation admission.
	DiagnosticOperationAdmission DiagnosticStage = "operation_admission"
	// DiagnosticResidentAdmission covers resident admission.
	DiagnosticResidentAdmission DiagnosticStage = "resident_admission"
	// DiagnosticResidentRecovery covers resident safe-continuation proof.
	DiagnosticResidentRecovery DiagnosticStage = "resident_recovery"
	// DiagnosticPlanning covers effect-free planning.
	DiagnosticPlanning DiagnosticStage = "planning"
	// DiagnosticMaterialization covers materialization or reconciliation.
	DiagnosticMaterialization DiagnosticStage = "materialization"
	// DiagnosticNativeAction covers pure native-action computation.
	DiagnosticNativeAction DiagnosticStage = "native_action"
	// DiagnosticHostObservation covers external prerequisite observation.
	DiagnosticHostObservation DiagnosticStage = "host_observation"
	// DiagnosticInternal covers another Engine-owned invariant.
	DiagnosticInternal DiagnosticStage = "internal"
)

// MarshalJSON translates the canonical diagnostic spelling into the private
// kebab-case FFI request vocabulary.
func (value DiagnosticStage) MarshalJSON() ([]byte, error) {
	return json.Marshal(strings.ReplaceAll(string(value), "_", "-"))
}

// DiagnosticDisposition classifies how a failed operation ended.
type DiagnosticDisposition string

const (
	// DiagnosticOperationalFailure reports a runtime prerequisite failure.
	DiagnosticOperationalFailure DiagnosticDisposition = "operational_failure"
	// DiagnosticInvalidInput reports authored input requiring correction.
	DiagnosticInvalidInput DiagnosticDisposition = "invalid_input"
	// DiagnosticBlocked reports a request closed by current admitted state.
	DiagnosticBlocked DiagnosticDisposition = "blocked"
	// DiagnosticInternalFailure reports an Engine-owned invariant failure.
	DiagnosticInternalFailure DiagnosticDisposition = "internal_failure"
	// DiagnosticEffectOutcomeUnknown requires reconciliation before retry.
	DiagnosticEffectOutcomeUnknown DiagnosticDisposition = "effect_outcome_unknown"
)

// MarshalJSON translates the canonical diagnostic spelling into the private
// kebab-case FFI request vocabulary.
func (value DiagnosticDisposition) MarshalJSON() ([]byte, error) {
	return json.Marshal(strings.ReplaceAll(string(value), "_", "-"))
}

// DiagnosticRetry is the API-owned recovery instruction.
type DiagnosticRetry string

const (
	// DiagnosticRetryNotIndicated establishes no retry instruction.
	DiagnosticRetryNotIndicated DiagnosticRetry = "not_indicated"
	// DiagnosticRetryAfterCorrection allows retry after correcting input.
	DiagnosticRetryAfterCorrection DiagnosticRetry = "after_correction"
	// DiagnosticRetryAfterStateChange allows retry after state changes.
	DiagnosticRetryAfterStateChange DiagnosticRetry = "after_state_change"
	// DiagnosticReconcileFirst requires reconciliation before retry is decided.
	DiagnosticReconcileFirst DiagnosticRetry = "reconcile_first"
)

// MarshalJSON translates the canonical diagnostic spelling into the private
// kebab-case FFI request vocabulary.
func (value DiagnosticRetry) MarshalJSON() ([]byte, error) {
	return json.Marshal(strings.ReplaceAll(string(value), "_", "-"))
}

// DiagnosticEffectCertainty states what is known about an external effect.
type DiagnosticEffectCertainty string

const (
	// DiagnosticEffectNotApplicable means no external effect boundary exists.
	DiagnosticEffectNotApplicable DiagnosticEffectCertainty = "not_applicable"
	// DiagnosticEffectNotStarted proves execution did not start.
	DiagnosticEffectNotStarted DiagnosticEffectCertainty = "not_started"
	// DiagnosticEffectConfirmed means owner evidence confirms the effect.
	DiagnosticEffectConfirmed DiagnosticEffectCertainty = "confirmed"
	// DiagnosticEffectUnknown means evidence cannot establish the outcome.
	DiagnosticEffectUnknown DiagnosticEffectCertainty = "outcome_unknown"
)

// MarshalJSON translates the canonical diagnostic spelling into the private
// kebab-case FFI request vocabulary.
func (value DiagnosticEffectCertainty) MarshalJSON() ([]byte, error) {
	return json.Marshal(strings.ReplaceAll(string(value), "_", "-"))
}

// DiagnosticProjection is an owned candidate for diagnostic.v1 projection.
type DiagnosticProjection struct {
	Code          string                    `json:"code"`
	Stage         DiagnosticStage           `json:"stage"`
	Disposition   DiagnosticDisposition     `json:"disposition"`
	Summary       string                    `json:"summary"`
	Retry         DiagnosticRetry           `json:"retry"`
	Effect        DiagnosticEffectCertainty `json:"effect"`
	Causes        []string                  `json:"causes,omitempty"`
	OmittedCauses uint32                    `json:"omittedCauses,omitempty"`
	Help          []string                  `json:"help,omitempty"`
}

// ProjectDiagnostic constructs canonical diagnostic.v1 bytes from a typed candidate.
func (client *Client) ProjectDiagnostic(candidate DiagnosticProjection) (CanonicalDocument, error) {
	result, err := client.call(candidate, opProjectDiagnostic)
	return CanonicalDocument{Bytes: result.payload}, err
}

// AdmitDiagnostic validates exact canonical diagnostic.v1 bytes.
func (client *Client) AdmitDiagnostic(bytes []byte) (CanonicalDocument, error) {
	result, err := client.call(struct {
		Bytes []byte `json:"bytes"`
	}{bytes}, opAdmitDiagnostic)
	return CanonicalDocument{Bytes: result.payload}, err
}

// DiagnosticError preserves exact canonical diagnostic.v1 bytes together with
// the small classification needed by ordinary Go error handling.
type DiagnosticError struct {
	// Identity is the SDK-owned opaque identity of Canonical, not recomputed by Go.
	Identity    string
	Code        string
	Stage       DiagnosticStage
	Disposition DiagnosticDisposition
	Summary     string
	Retry       DiagnosticRetry
	Effect      DiagnosticEffectCertainty
	Canonical   []byte
	// AuthoringCorrections carries copied source evidence, not permission to write.
	AuthoringCorrections []AuthoringCorrection
	// AuthoringLocation is nil when the compiler cannot prove an exact source path.
	AuthoringLocation *AuthoringSourceLocation
}

func (failure *DiagnosticError) Error() string {
	return failure.Code + ": " + failure.Summary
}

type diagnosticMetadata struct {
	Identity             string                   `json:"diagnosticIdentity"`
	AuthoringCorrections []AuthoringCorrection    `json:"authoringCorrections"`
	AuthoringLocation    *AuthoringSourceLocation `json:"authoringLocation"`
}

func (value *diagnosticMetadata) UnmarshalJSON(data []byte) error {
	type plain diagnosticMetadata
	var next plain
	if err := decodeTransportObject(data, &next,
		[]string{"diagnosticIdentity", "authoringCorrections"}, "authoringLocation"); err != nil {
		return err
	}
	*value = diagnosticMetadata(next)
	return nil
}

// Consumes the Go-owned buffers already copied from the native response.
// The metadata is private transport; Canonical remains the diagnostic owner.
func newDiagnosticError(canonical, metadata []byte) error {
	var projection struct {
		Code        string                    `json:"code"`
		Stage       DiagnosticStage           `json:"stage"`
		Disposition DiagnosticDisposition     `json:"disposition"`
		Summary     string                    `json:"summary"`
		Retry       DiagnosticRetry           `json:"retry"`
		Effect      DiagnosticEffectCertainty `json:"effect"`
	}
	if err := json.Unmarshal(canonical, &projection); err != nil {
		return &BridgeError{Code: "invalid_diagnostic", Message: err.Error()}
	}
	var sidecars diagnosticMetadata
	if err := json.Unmarshal(metadata, &sidecars); err != nil {
		return &BridgeError{Code: "invalid_response_metadata", Message: "invalid diagnostic metadata"}
	}
	return &DiagnosticError{
		Identity:             sidecars.Identity,
		Code:                 projection.Code,
		Stage:                projection.Stage,
		Disposition:          projection.Disposition,
		Summary:              projection.Summary,
		Retry:                projection.Retry,
		Effect:               projection.Effect,
		Canonical:            canonical,
		AuthoringCorrections: sidecars.AuthoringCorrections,
		AuthoringLocation:    sidecars.AuthoringLocation,
	}
}
