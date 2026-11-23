package pale

import (
	"encoding/json"
	"fmt"
)

// ObservationPagePosition describes the snapshot, raw limits and next retained
// fact bound into the opaque page cursor. This readable value is not resumable
// and does not prove that later facts are absent.
type ObservationPagePosition struct {
	Snapshot                 ObservationCursor `json:"snapshot"`
	InspectedPosition        uint32            `json:"inspectedPosition"`
	MaximumInspected         uint16            `json:"maximumInspected"`
	MaximumReturned          uint16            `json:"maximumReturned"`
	MaximumCopiedStringBytes uint32            `json:"maximumCopiedStringBytes"`
}

// UnmarshalJSON requires the complete SDK page position.
func (value *ObservationPagePosition) UnmarshalJSON(data []byte) error {
	type plain ObservationPagePosition
	var next plain
	if err := decodeTransportObject(data, &next, []string{"snapshot", "inspectedPosition", "maximumInspected", "maximumReturned", "maximumCopiedStringBytes"}); err != nil {
		return err
	}
	*value = ObservationPagePosition(next)
	return nil
}

// ObservationStepLimitKind identifies the independent resource that bounded a step.
type ObservationStepLimitKind string

const (
	ObservationLimitInspected         ObservationStepLimitKind = "inspected"
	ObservationLimitReturned          ObservationStepLimitKind = "returned"
	ObservationLimitCopiedStringBytes ObservationStepLimitKind = "copied-string-bytes"
	ObservationLimitCausalPathLookups ObservationStepLimitKind = "causal-path-lookups"
	ObservationLimitCausalSteps       ObservationStepLimitKind = "causal-steps"
)

// ObservationStepLimit carries an optional minimum for an indivisible result.
type ObservationStepLimit struct {
	Kind            ObservationStepLimitKind `json:"kind"`
	MinimumRequired *uint32                  `json:"minimumRequired,omitempty"`
}

// UnmarshalJSON retains only the payload selected by the limit kind.
func (value *ObservationStepLimit) UnmarshalJSON(data []byte) error {
	variants := map[string][]string{
		"inspected": {}, "returned": {}, "copied-string-bytes": {"minimumRequired"},
		"causal-path-lookups": {}, "causal-steps": {"minimumRequired"},
	}
	kind, err := transportVariant(data, "kind", variants)
	if err != nil {
		return err
	}
	type plain ObservationStepLimit
	var next plain
	if err := decodeObservationObject(data, &next, variants[kind]...); err != nil {
		return err
	}
	if kind == "causal-steps" && next.MinimumRequired != nil && *next.MinimumRequired > uint32(^uint16(0)) {
		return fmt.Errorf("observation causal-step minimum exceeds uint16")
	}
	*value = ObservationStepLimit(next)
	return nil
}

// ObservationStepCompletionKind distinguishes selected-input exhaustion from
// a resumable resource stop and an indivisible result that cannot fit.
type ObservationStepCompletionKind string

const (
	ObservationStepComplete   ObservationStepCompletionKind = "complete"
	ObservationStepStopped    ObservationStepCompletionKind = "stopped"
	ObservationStepIncomplete ObservationStepCompletionKind = "incomplete"
)

// ObservationStepCompletion reports why the step ended. Stopped permits the
// returned cursor to continue. Incomplete has no cursor because the same limits
// cannot make progress; neither state proves that later input is absent.
type ObservationStepCompletion struct {
	Kind  ObservationStepCompletionKind `json:"kind"`
	Limit *ObservationStepLimit         `json:"limit,omitempty"`
}

// UnmarshalJSON requires a limit only for stopped and incomplete steps.
func (value *ObservationStepCompletion) UnmarshalJSON(data []byte) error {
	variants := map[string][]string{
		"complete": {}, "stopped": {"limit"}, "incomplete": {"limit"},
	}
	kind, err := transportVariant(data, "kind", variants)
	if err != nil {
		return err
	}
	type plain ObservationStepCompletion
	var next plain
	if err := decodeObservationObject(data, &next, variants[kind]...); err != nil {
		return err
	}
	*value = ObservationStepCompletion(next)
	return nil
}

// ObservationPage contains ordered typed facts and inspectable continuation data.
// ObservationPageResult.Continuation retains the corresponding native request handle.
type ObservationPage struct {
	Cursor               ObservationCursor         `json:"cursor"`
	Facts                []ObservationFact         `json:"facts"`
	Continuation         *ObservationPagePosition  `json:"continuation"`
	Completion           ObservationStepCompletion `json:"completion"`
	Inspected            uint16                    `json:"inspected"`
	CopiedStringBytes    uint32                    `json:"copiedStringBytes"`
	SnapshotOmittedCount uint32                    `json:"snapshotOmittedCount"`
}

// UnmarshalJSON rejects incomplete pages without publishing partial facts.
func (value *ObservationPage) UnmarshalJSON(data []byte) error {
	type plain ObservationPage
	var next plain
	if err := decodeTransportObject(data, &next, []string{"cursor", "facts", "completion", "inspected", "copiedStringBytes", "snapshotOmittedCount"}, "continuation"); err != nil {
		return err
	}
	*value = ObservationPage(next)
	return nil
}

// ObservationResynchronizationKind explains why a fresh snapshot is required.
type ObservationResynchronizationKind string

const (
	ObservationGenerationMismatch   ObservationResynchronizationKind = "generation-mismatch"
	ObservationRevisionGap          ObservationResynchronizationKind = "revision-gap"
	ObservationContinuationMismatch ObservationResynchronizationKind = "continuation-mismatch"
)

// ObservationResynchronizationReason keeps generation and revision mismatches distinct.
type ObservationResynchronizationReason struct {
	Kind                ObservationResynchronizationKind
	RequestedGeneration *ObservationGeneration
	CurrentGeneration   *ObservationGeneration
	RequestedRevision   *ObservationRevision
	CurrentRevision     *ObservationRevision
}

// UnmarshalJSON decodes only the fields selected by the reason tag.
func (value *ObservationResynchronizationReason) UnmarshalJSON(data []byte) error {
	kind, err := transportVariant(data, "kind", map[string][]string{
		"generation-mismatch": {"requested", "current"}, "revision-gap": {"requested", "current"}, "continuation-mismatch": {},
	})
	if err != nil {
		return err
	}
	var wire struct {
		Kind      string          `json:"kind"`
		Requested json.RawMessage `json:"requested"`
		Current   json.RawMessage `json:"current"`
	}
	if err := decodeObservationObject(data, &wire, "kind"); err != nil {
		return err
	}
	next := ObservationResynchronizationReason{Kind: ObservationResynchronizationKind(kind)}
	if kind == "generation-mismatch" {
		var requested, current ObservationGeneration
		if err := json.Unmarshal(wire.Requested, &requested); err != nil {
			return err
		}
		if err := json.Unmarshal(wire.Current, &current); err != nil {
			return err
		}
		next.RequestedGeneration, next.CurrentGeneration = &requested, &current
	} else if kind == "revision-gap" {
		var requested, current ObservationRevision
		if err := json.Unmarshal(wire.Requested, &requested); err != nil {
			return err
		}
		if err := json.Unmarshal(wire.Current, &current); err != nil {
			return err
		}
		next.RequestedRevision, next.CurrentRevision = &requested, &current
	}
	*value = next
	return nil
}

// ObservationResynchronization returns the current cursor and exact rejection reason.
type ObservationResynchronization struct {
	Reason  ObservationResynchronizationReason `json:"reason"`
	Current ObservationCursor                  `json:"current"`
}

// UnmarshalJSON requires both recovery coordinates and reason.
func (value *ObservationResynchronization) UnmarshalJSON(data []byte) error {
	type plain ObservationResynchronization
	var next plain
	if err := decodeTransportObject(data, &next, []string{"reason", "current"}); err != nil {
		return err
	}
	*value = ObservationResynchronization(next)
	return nil
}

// ObservationChangeKind selects replacement or removal of one fact.
type ObservationChangeKind string

const (
	ObservationUpsert ObservationChangeKind = "upsert"
	ObservationRemove ObservationChangeKind = "remove"
)

// ObservationChange contains a complete fact or a removed category/subject key.
type ObservationChange struct {
	Kind     ObservationChangeKind    `json:"kind"`
	Fact     *ObservationFact         `json:"fact,omitempty"`
	Category *ObservationCategory     `json:"category,omitempty"`
	Subject  *ObservationSubjectValue `json:"subject,omitempty"`
}

// UnmarshalJSON rejects incomplete or mixed changes.
func (value *ObservationChange) UnmarshalJSON(data []byte) error {
	variants := map[string][]string{"upsert": {"fact"}, "remove": {"category", "subject"}}
	kind, err := transportVariant(data, "kind", variants)
	if err != nil {
		return err
	}
	type plain ObservationChange
	var next plain
	if err := decodeObservationObject(data, &next, variants[kind]...); err != nil {
		return err
	}
	*value = ObservationChange(next)
	return nil
}

// ObservationDelta is a complete ordered change set; apply all changes before To.
type ObservationDelta struct {
	From                      ObservationCursor   `json:"from"`
	To                        ObservationCursor   `json:"to"`
	Changes                   []ObservationChange `json:"changes"`
	SuccessorOmittedFactCount uint32              `json:"successorOmittedFactCount"`
}

// UnmarshalJSON rejects missing members and malformed nested changes atomically.
func (value *ObservationDelta) UnmarshalJSON(data []byte) error {
	type plain ObservationDelta
	var next plain
	if err := decodeTransportObject(data, &next, []string{"from", "to", "changes", "successorOmittedFactCount"}); err != nil {
		return err
	}
	*value = ObservationDelta(next)
	return nil
}

// ObservationDeltaKind preserves cancellation, backpressure and resynchronization.
type ObservationDeltaKind string

const (
	ObservationCaughtUp           ObservationDeltaKind = "caught-up"
	ObservationDeltaReady         ObservationDeltaKind = "delta"
	ObservationDeltaResynchronize ObservationDeltaKind = "resynchronize"
	ObservationDeltaCancelled     ObservationDeltaKind = "cancelled"
	ObservationBackpressured      ObservationDeltaKind = "backpressured"
)

// ObservationBackpressureRecovery is the SDK-selected slow-consumer recovery.
type ObservationBackpressureRecovery string

const ObservationRequestSnapshot ObservationBackpressureRecovery = "request-snapshot"

// UnmarshalJSON rejects unselected recovery actions.
func (value *ObservationBackpressureRecovery) UnmarshalJSON(data []byte) error {
	return decodeObservationTag(data, value, ObservationRequestSnapshot)
}

// ObservationDeltaResult carries only its selected payload; no partial delta is emitted.
type ObservationDeltaResult struct {
	Kind              ObservationDeltaKind             `json:"kind"`
	Cursor            *ObservationCursor               `json:"cursor,omitempty"`
	Delta             *ObservationDelta                `json:"delta,omitempty"`
	Resynchronization *ObservationResynchronization    `json:"resynchronization,omitempty"`
	Resume            *ObservationResumeCoordinate     `json:"resume,omitempty"`
	PendingChanges    *uint32                          `json:"pendingChanges,omitempty"`
	MaximumChanges    *uint16                          `json:"maximumChanges,omitempty"`
	Recovery          *ObservationBackpressureRecovery `json:"recovery,omitempty"`
	Current           *ObservationCursor               `json:"current,omitempty"`
}

// UnmarshalJSON preserves exact integer widths and rejects mixed result payloads.
func (value *ObservationDeltaResult) UnmarshalJSON(data []byte) error {
	variants := map[string][]string{
		"caught-up": {"cursor"}, "delta": {"delta"}, "resynchronize": {"resynchronization"},
		"cancelled": {"resume"}, "backpressured": {"pendingChanges", "maximumChanges", "recovery", "current"},
	}
	kind, err := transportVariant(data, "kind", variants)
	if err != nil {
		return err
	}
	type plain ObservationDeltaResult
	var next plain
	if err := decodeObservationObject(data, &next, variants[kind]...); err != nil {
		return err
	}
	*value = ObservationDeltaResult(next)
	return nil
}

// ObservationCausalStepKind names the exact retained compiler reason.
type ObservationCausalStepKind string

const (
	ObservationOriginReceiptStep          ObservationCausalStepKind = "origin-receipt"
	ObservationEntryContractStep          ObservationCausalStepKind = "entry-contract"
	ObservationResponsibilityContractStep ObservationCausalStepKind = "responsibility-contract"
	ObservationProviderBindingStep        ObservationCausalStepKind = "provider-binding"
	ObservationDelegationEdgeStep         ObservationCausalStepKind = "delegation-edge"
	ObservationTerminationContractStep    ObservationCausalStepKind = "termination-contract"
	ObservationSettlementContractStep     ObservationCausalStepKind = "settlement-contract"
	ObservationContinuityContractStep     ObservationCausalStepKind = "continuity-contract"
)

// ObservationCausalStep carries the reference selected by Kind, not native proof bodies.
type ObservationCausalStep struct {
	Kind       ObservationCausalStepKind `json:"kind"`
	ContractID string                    `json:"contractId,omitempty"`
	BindingID  string                    `json:"bindingId,omitempty"`
	EdgeID     string                    `json:"edgeId,omitempty"`
}

// UnmarshalJSON retains reason order and validates only transport membership.
func (value *ObservationCausalStep) UnmarshalJSON(data []byte) error {
	variants := map[string][]string{
		"origin-receipt": {"contractId"}, "entry-contract": {"contractId"},
		"responsibility-contract": {"contractId"}, "provider-binding": {"bindingId"},
		"delegation-edge": {"edgeId"}, "termination-contract": {"contractId"},
		"settlement-contract": {"contractId"}, "continuity-contract": {"contractId"},
	}
	kind, err := transportVariant(data, "kind", variants)
	if err != nil {
		return err
	}
	type plain ObservationCausalStep
	var next plain
	if err := decodeObservationObject(data, &next, variants[kind]...); err != nil {
		return err
	}
	*value = ObservationCausalStep(next)
	return nil
}

// ObservationCausalPath retains ordered reasons under their exact generation and subject.
type ObservationCausalPath struct {
	Generation ObservationGeneration   `json:"generation"`
	Subject    ObservationSubjectValue `json:"subject"`
	Steps      []ObservationCausalStep `json:"steps"`
}

// UnmarshalJSON rejects incomplete causal paths atomically.
func (value *ObservationCausalPath) UnmarshalJSON(data []byte) error {
	type plain ObservationCausalPath
	var next plain
	if err := decodeTransportObject(data, &next, []string{"generation", "subject", "steps"}); err != nil {
		return err
	}
	*value = ObservationCausalPath(next)
	return nil
}

// ObservationCausalPathState distinguishes absence from a stale-coordinate rejection.
type ObservationCausalPathState string

const (
	ObservationCausalPathReady         ObservationCausalPathState = "ready"
	ObservationCausalPathAbsent        ObservationCausalPathState = "absent"
	ObservationCausalPathResynchronize ObservationCausalPathState = "resynchronize"
)

// ObservationCausalPathResult carries the exact SDK result without inventing a diagnosis.
type ObservationCausalPathResult struct {
	State             ObservationCausalPathState    `json:"state"`
	Path              *ObservationCausalPath        `json:"path,omitempty"`
	Resynchronization *ObservationResynchronization `json:"resynchronization,omitempty"`
}

// UnmarshalJSON rejects mixed path/absence/resynchronization payloads.
func (value *ObservationCausalPathResult) UnmarshalJSON(data []byte) error {
	variants := map[string][]string{"ready": {"path"}, "absent": {}, "resynchronize": {"resynchronization"}}
	kind, err := transportVariant(data, "state", variants)
	if err != nil {
		return err
	}
	type plain ObservationCausalPathResult
	var next plain
	if err := decodeObservationObject(data, &next, variants[kind]...); err != nil {
		return err
	}
	*value = ObservationCausalPathResult(next)
	return nil
}

// ObservationLensNode contains one subject and its complete selected facts.
type ObservationLensNode struct {
	Subject ObservationSubjectValue `json:"subject"`
	Facts   []ObservationFact       `json:"facts"`
}

// UnmarshalJSON requires the complete node without adding layout meaning.
func (value *ObservationLensNode) UnmarshalJSON(data []byte) error {
	type plain ObservationLensNode
	var next plain
	if err := decodeTransportObject(data, &next, []string{"subject", "facts"}); err != nil {
		return err
	}
	*value = ObservationLensNode(next)
	return nil
}

// ObservationLens is a bounded disposable view, never authoritative geometry.
type ObservationLens struct {
	Cursor            ObservationCursor         `json:"cursor"`
	Nodes             []ObservationLensNode     `json:"nodes"`
	CausalPaths       []ObservationCausalPath   `json:"causalPaths"`
	Continuation      *ObservationLensPosition  `json:"continuation"`
	Completion        ObservationStepCompletion `json:"completion"`
	InspectedSubjects uint16                    `json:"inspectedSubjects"`
	CopiedStringBytes uint32                    `json:"copiedStringBytes"`
	// CausalPathLookups includes lookups attempted before later limits stopped output.
	CausalPathLookups    uint16 `json:"causalPathLookups"`
	CausalSteps          uint16 `json:"causalSteps"`
	SnapshotOmittedCount uint32 `json:"snapshotOmittedCount"`
}

// UnmarshalJSON requires all SDK lens members and exact integer cardinality.
func (value *ObservationLens) UnmarshalJSON(data []byte) error {
	type plain ObservationLens
	var next plain
	if err := decodeTransportObject(data, &next, []string{"cursor", "nodes", "causalPaths", "completion", "inspectedSubjects", "copiedStringBytes", "causalPathLookups", "causalSteps", "snapshotOmittedCount"}, "continuation"); err != nil {
		return err
	}
	*value = ObservationLens(next)
	return nil
}

// ObservationLensPosition describes the snapshot, filter options, raw limits
// and next subject bound into the opaque lens cursor. This readable value is
// not resumable and does not prove that later subjects are absent.
type ObservationLensPosition struct {
	Snapshot                 ObservationCursor `json:"snapshot"`
	InspectedPosition        uint32            `json:"inspectedPosition"`
	MaximumInspected         uint16            `json:"maximumInspected"`
	MaximumNodes             uint16            `json:"maximumNodes"`
	MaximumCopiedStringBytes uint32            `json:"maximumCopiedStringBytes"`
	IncludeCausalPaths       bool              `json:"includeCausalPaths"`
	MaximumCausalPathLookups uint16            `json:"maximumCausalPathLookups"`
	MaximumCausalSteps       uint16            `json:"maximumCausalSteps"`
}

// UnmarshalJSON requires the full raw-limit set used to issue this position.
func (value *ObservationLensPosition) UnmarshalJSON(data []byte) error {
	type plain ObservationLensPosition
	var next plain
	if err := decodeTransportObject(data, &next, []string{
		"snapshot", "inspectedPosition", "maximumInspected", "maximumNodes",
		"maximumCopiedStringBytes", "includeCausalPaths", "maximumCausalPathLookups",
		"maximumCausalSteps",
	}); err != nil {
		return err
	}
	*value = ObservationLensPosition(next)
	return nil
}

// ObservationLensState distinguishes a valid bounded result, cancellation and
// a request that must restart from a fresh snapshot.
type ObservationLensState string

const (
	ObservationLensReady         ObservationLensState = "ready"
	ObservationLensResynchronize ObservationLensState = "resynchronize"
	ObservationLensCancelled     ObservationLensState = "cancelled"
)

// ObservationLensResult carries one valid bounded lens step, cancellation, or
// a resynchronization instruction. Ready does not mean the selected input was
// exhausted; inspect Lens.Completion.
type ObservationLensResult struct {
	Kind              ObservationLensState          `json:"kind"`
	Lens              *ObservationLens              `json:"lens,omitempty"`
	Continuation      *ObservationLensCursor        `json:"-"`
	Resynchronization *ObservationResynchronization `json:"resynchronization,omitempty"`
	ownedContinuation handle
}

// UnmarshalJSON rejects payloads on cancelled outcomes.
func (value *ObservationLensResult) UnmarshalJSON(data []byte) error {
	variants := map[string][]string{"ready": {"lens"}, "resynchronize": {"resynchronization"}, "cancelled": {}}
	kind, err := transportVariant(data, "kind", variants)
	if err != nil {
		return err
	}
	type plain ObservationLensResult
	var next plain
	if err := decodeObservationObject(data, &next, variants[kind]...); err != nil {
		return err
	}
	*value = ObservationLensResult(next)
	return nil
}
