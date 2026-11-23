package pale

import "encoding/json"

// UnmarshalJSON accepts only SDK-owned observation axes.
func (value *ObservationCategory) UnmarshalJSON(data []byte) error {
	return decodeObservationTag(data, value, ObservationOperationExposure,
		ObservationCapabilityRequirementDisposition, ObservationHealth, ObservationReadiness,
		ObservationEffect, ObservationNativeOwner, ObservationReconciliation, ObservationProcessIntent)
}

// ObservationHealthState describes health evidence, not permission or readiness.
type ObservationHealthState string

const (
	ObservationHealthy           ObservationHealthState = "healthy"
	ObservationDegraded          ObservationHealthState = "degraded"
	ObservationUnhealthy         ObservationHealthState = "unhealthy"
	ObservationHealthUnknown     ObservationHealthState = "unknown"
	ObservationHealthUnavailable ObservationHealthState = "unavailable"
)

// UnmarshalJSON rejects unknown health classifications.
func (value *ObservationHealthState) UnmarshalJSON(data []byte) error {
	return decodeObservationTag(data, value, ObservationHealthy, ObservationDegraded,
		ObservationUnhealthy, ObservationHealthUnknown, ObservationHealthUnavailable)
}

// ObservationReadinessState preserves the SDK's separate readiness predicate.
type ObservationReadinessState string

const (
	ObservationReady                ObservationReadinessState = "ready"
	ObservationBlocked              ObservationReadinessState = "blocked"
	ObservationNarrowed             ObservationReadinessState = "narrowed"
	ObservationUnproven             ObservationReadinessState = "unproven"
	ObservationReadinessUnavailable ObservationReadinessState = "unavailable"
)

// UnmarshalJSON rejects unknown readiness classifications.
func (value *ObservationReadinessState) UnmarshalJSON(data []byte) error {
	return decodeObservationTag(data, value, ObservationReady, ObservationBlocked,
		ObservationNarrowed, ObservationUnproven, ObservationReadinessUnavailable)
}

// EffectiveProfileOperationExposure separates binding absence from semantic-only
// and capability-bearing operations; none of these states proves native execution.
type EffectiveProfileOperationExposure string

const (
	EffectiveProfileOperationUnbound           EffectiveProfileOperationExposure = "unbound"
	EffectiveProfileOperationSemanticOnly      EffectiveProfileOperationExposure = "semantic-only"
	EffectiveProfileOperationCapabilityBearing EffectiveProfileOperationExposure = "capability-bearing"
)

// UnmarshalJSON rejects unknown exposure states.
func (value *EffectiveProfileOperationExposure) UnmarshalJSON(data []byte) error {
	return decodeObservationTag(data, value, EffectiveProfileOperationUnbound,
		EffectiveProfileOperationSemanticOnly, EffectiveProfileOperationCapabilityBearing)
}

// EffectiveProfileRequirementKind reports the SDK's portable requirement result.
type EffectiveProfileRequirementKind string

const (
	EffectiveProfileRequirementSatisfied      EffectiveProfileRequirementKind = "satisfied"
	EffectiveProfileRequirementProviderAbsent EffectiveProfileRequirementKind = "provider-binding-absent"
	EffectiveProfileRequirementNarrowed       EffectiveProfileRequirementKind = "narrowed"
	EffectiveProfileRequirementBlocked        EffectiveProfileRequirementKind = "blocked"
)

// EffectiveProfileNarrowingReason identifies a missing positive-set membership.
type EffectiveProfileNarrowingReason string

const (
	EffectiveProfileCapabilityNotPermitted          EffectiveProfileNarrowingReason = "capability-not-permitted"
	EffectiveProfileEffectNotPermitted              EffectiveProfileNarrowingReason = "effect-not-permitted"
	EffectiveProfileCapabilityAndEffectNotPermitted EffectiveProfileNarrowingReason = "capability-and-effect-not-permitted"
)

// UnmarshalJSON rejects unknown narrowing dimensions.
func (value *EffectiveProfileNarrowingReason) UnmarshalJSON(data []byte) error {
	return decodeObservationTag(data, value, EffectiveProfileCapabilityNotPermitted,
		EffectiveProfileEffectNotPermitted, EffectiveProfileCapabilityAndEffectNotPermitted)
}

// EffectiveProfilePolicyBlock identifies an explicit denial or binding mismatch.
type EffectiveProfilePolicyBlock string

const (
	EffectiveProfileCapabilityForbidden          EffectiveProfilePolicyBlock = "capability-forbidden"
	EffectiveProfileEffectForbidden              EffectiveProfilePolicyBlock = "effect-forbidden"
	EffectiveProfileCapabilityAndEffectForbidden EffectiveProfilePolicyBlock = "capability-and-effect-forbidden"
	EffectiveProfileProviderEffectMismatch       EffectiveProfilePolicyBlock = "provider-effect-mismatch"
	EffectiveProfileRecipeContractMismatch       EffectiveProfilePolicyBlock = "recipe-contract-mismatch"
)

// UnmarshalJSON rejects unknown policy dimensions.
func (value *EffectiveProfilePolicyBlock) UnmarshalJSON(data []byte) error {
	return decodeObservationTag(data, value, EffectiveProfileCapabilityForbidden,
		EffectiveProfileEffectForbidden, EffectiveProfileCapabilityAndEffectForbidden,
		EffectiveProfileProviderEffectMismatch, EffectiveProfileRecipeContractMismatch)
}

// EffectiveProfileRequirementDisposition retains only the reason belonging to Kind.
// It copies the SDK result and performs no provider or policy classification.
type EffectiveProfileRequirementDisposition struct {
	Kind      EffectiveProfileRequirementKind
	Narrowing *EffectiveProfileNarrowingReason
	Block     *EffectiveProfilePolicyBlock
}

// UnmarshalJSON checks the selected requirement payload atomically.
func (value *EffectiveProfileRequirementDisposition) UnmarshalJSON(data []byte) error {
	kind, err := transportVariant(data, "kind", map[string][]string{
		"satisfied": {}, "provider-binding-absent": {}, "narrowed": {"reason"}, "blocked": {"reason"},
	})
	if err != nil {
		return err
	}
	var wire struct {
		Kind   string          `json:"kind"`
		Reason json.RawMessage `json:"reason"`
	}
	if err := decodeObservationObject(data, &wire, "kind"); err != nil {
		return err
	}
	next := EffectiveProfileRequirementDisposition{Kind: EffectiveProfileRequirementKind(kind)}
	if kind == "narrowed" {
		var reason EffectiveProfileNarrowingReason
		if err := json.Unmarshal(wire.Reason, &reason); err != nil {
			return err
		}
		next.Narrowing = &reason
	} else if kind == "blocked" {
		var reason EffectiveProfilePolicyBlock
		if err := json.Unmarshal(wire.Reason, &reason); err != nil {
			return err
		}
		next.Block = &reason
	}
	*value = next
	return nil
}

// ObservationNativeKind distinguishes unobserved state from exact owner evidence.
type ObservationNativeKind string

const (
	ObservationNativeUnavailable ObservationNativeKind = "unavailable"
	ObservationNativeUnknown     ObservationNativeKind = "unknown"
	ObservationNativeObserved    ObservationNativeKind = "observed"
)

// ObservationNativeState retains references only, never the native owner's payload.
type ObservationNativeState struct {
	Kind            ObservationNativeKind `json:"kind"`
	OwnerSystemID   string                `json:"ownerSystemId,omitempty"`
	ObservationKind string                `json:"observationKind,omitempty"`
}

// UnmarshalJSON requires both owner-evidence fields only for observed state.
func (value *ObservationNativeState) UnmarshalJSON(data []byte) error {
	variants := map[string][]string{"unknown": {}, "unavailable": {}, "observed": {"ownerSystemId", "observationKind"}}
	kind, err := transportVariant(data, "kind", variants)
	if err != nil {
		return err
	}
	type plain ObservationNativeState
	var next plain
	if err := decodeObservationObject(data, &next, variants[kind]...); err != nil {
		return err
	}
	*value = ObservationNativeState(next)
	return nil
}

// ObservationReconciliationKind reports custody without asserting effect success.
type ObservationReconciliationKind string

const (
	ObservationReconciliationNotRequired ObservationReconciliationKind = "not-required"
	ObservationReconciliationRequired    ObservationReconciliationKind = "required"
	ObservationReconciliationInProgress  ObservationReconciliationKind = "in-progress"
	ObservationReconciliationSettled     ObservationReconciliationKind = "settled"
	ObservationReconciliationUnknown     ObservationReconciliationKind = "unknown"
	ObservationReconciliationUnavailable ObservationReconciliationKind = "unavailable"
)

// ObservationReconciliationState carries the case or result selected by Kind.
type ObservationReconciliationState struct {
	Kind         ObservationReconciliationKind `json:"kind"`
	CaseDigest   string                        `json:"caseDigest,omitempty"`
	ResultDigest string                        `json:"resultDigest,omitempty"`
}

// UnmarshalJSON rejects mixed case/result payloads atomically.
func (value *ObservationReconciliationState) UnmarshalJSON(data []byte) error {
	variants := map[string][]string{
		"not-required": {}, "required": {"caseDigest"}, "in-progress": {"caseDigest"},
		"settled": {"resultDigest"}, "unknown": {}, "unavailable": {},
	}
	kind, err := transportVariant(data, "kind", variants)
	if err != nil {
		return err
	}
	type plain ObservationReconciliationState
	var next plain
	if err := decodeObservationObject(data, &next, variants[kind]...); err != nil {
		return err
	}
	*value = ObservationReconciliationState(next)
	return nil
}

// ObservationProcessState is desired intent, never a liveness observation.
type ObservationProcessState string

const (
	ObservationProcessRunning ObservationProcessState = "running"
	ObservationProcessStopped ObservationProcessState = "stopped"
)

// UnmarshalJSON rejects unknown process intents.
func (value *ObservationProcessState) UnmarshalJSON(data []byte) error {
	return decodeObservationTag(data, value, ObservationProcessRunning, ObservationProcessStopped)
}

// ObservationFact has exactly one state payload selected by Category.
// Process intent alone has no Subject. Separate pointers preserve zero/absent
// distinctions without combining independent evidence into one status.
type ObservationFact struct {
	Category               ObservationCategory
	Subject                *ObservationSubjectValue
	Freshness              ObservationFreshness
	OperationExposure      *EffectiveProfileOperationExposure
	RequirementDisposition *EffectiveProfileRequirementDisposition
	Health                 *ObservationHealthState
	Readiness              *ObservationReadinessState
	Effect                 *DiagnosticEffectCertainty
	NativeOwner            *ObservationNativeState
	Reconciliation         *ObservationReconciliationState
	ProcessIntent          *ObservationProcessState
}

// UnmarshalJSON checks the selected fact payload before publishing any state.
func (value *ObservationFact) UnmarshalJSON(data []byte) error {
	variants := map[string][]string{
		"operation-exposure":                 {"subject", "state", "freshness"},
		"capability-requirement-disposition": {"subject", "state", "freshness"},
		"health":                             {"subject", "state", "freshness"}, "readiness": {"subject", "state", "freshness"},
		"effect": {"subject", "certainty", "freshness"}, "native-owner": {"subject", "state", "freshness"},
		"reconciliation": {"subject", "state", "freshness"}, "process-intent": {"state", "freshness"},
	}
	kind, err := transportVariant(data, "category", variants)
	if err != nil {
		return err
	}
	var wire struct {
		Category  ObservationCategory      `json:"category"`
		Subject   *ObservationSubjectValue `json:"subject"`
		Freshness ObservationFreshness     `json:"freshness"`
		State     json.RawMessage          `json:"state"`
		Certainty json.RawMessage          `json:"certainty"`
	}
	if err := decodeObservationObject(data, &wire, variants[kind]...); err != nil {
		return err
	}
	next := ObservationFact{Category: wire.Category, Subject: wire.Subject, Freshness: wire.Freshness}
	switch wire.Category {
	case ObservationOperationExposure:
		var state EffectiveProfileOperationExposure
		err = json.Unmarshal(wire.State, &state)
		next.OperationExposure = &state
	case ObservationCapabilityRequirementDisposition:
		var state EffectiveProfileRequirementDisposition
		err = json.Unmarshal(wire.State, &state)
		next.RequirementDisposition = &state
	case ObservationHealth:
		var state ObservationHealthState
		err = json.Unmarshal(wire.State, &state)
		next.Health = &state
	case ObservationReadiness:
		var state ObservationReadinessState
		err = json.Unmarshal(wire.State, &state)
		next.Readiness = &state
	case ObservationEffect:
		var state DiagnosticEffectCertainty
		err = decodeObservationTag(wire.Certainty, &state,
			DiagnosticEffectNotApplicable, DiagnosticEffectNotStarted, DiagnosticEffectConfirmed,
			DiagnosticEffectUnknown)
		next.Effect = &state
	case ObservationNativeOwner:
		var state ObservationNativeState
		err = json.Unmarshal(wire.State, &state)
		next.NativeOwner = &state
	case ObservationReconciliation:
		var state ObservationReconciliationState
		err = json.Unmarshal(wire.State, &state)
		next.Reconciliation = &state
	case ObservationProcessIntent:
		var state ObservationProcessState
		err = json.Unmarshal(wire.State, &state)
		next.ProcessIntent = &state
	}
	if err != nil {
		return err
	}
	*value = next
	return nil
}
