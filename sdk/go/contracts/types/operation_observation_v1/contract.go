// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package operation_observation_v1 contains the generated operation-observation.v1 data model.
package operation_observation_v1

import (
	"encoding/json"
	"fmt"
	sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"
)

type CanonicalSignatureV1 string

type CapabilityAttemptIDV1 string

type CapabilityResultReferenceV1 struct {
	// RecordRef corresponds to the JSON schema field "record_ref".
	RecordRef ResidentRecordRefV1 `json:"record_ref"`

	// RecordSha256 corresponds to the JSON schema field "record_sha256".
	RecordSha256 sha256_digest_v1.Sha256DigestV1 `json:"record_sha256"`
}

type EffectReceiptReferenceV1 struct {
	// RecordRef corresponds to the JSON schema field "record_ref".
	RecordRef ResidentRecordRefV1 `json:"record_ref"`

	// RecordSha256 corresponds to the JSON schema field "record_sha256".
	RecordSha256 sha256_digest_v1.Sha256DigestV1 `json:"record_sha256"`
}

type ImmutableRecordReferenceV1 struct {
	// RecordRef corresponds to the JSON schema field "record_ref".
	RecordRef ResidentRecordRefV1 `json:"record_ref"`

	// RecordSha256 corresponds to the JSON schema field "record_sha256".
	RecordSha256 sha256_digest_v1.Sha256DigestV1 `json:"record_sha256"`
}

type OperationActivePhaseCollectingConsultationV1 struct {
	// Evidence corresponds to the JSON schema field "evidence".
	Evidence ImmutableRecordReferenceV1 `json:"evidence"`

	// Phase corresponds to the JSON schema field "phase".
	Phase string `json:"phase"`
}

type OperationActivePhaseConsultationCollectedV1 struct {
	// Evidence corresponds to the JSON schema field "evidence".
	Evidence ImmutableRecordReferenceV1 `json:"evidence"`

	// Phase corresponds to the JSON schema field "phase".
	Phase string `json:"phase"`
}

type OperationActivePhaseContextBoundV1 struct {
	// Evidence corresponds to the JSON schema field "evidence".
	Evidence ImmutableRecordReferenceV1 `json:"evidence"`

	// Phase corresponds to the JSON schema field "phase".
	Phase string `json:"phase"`
}

type OperationActivePhaseDispatchPreparedV1 struct {
	// AttemptID corresponds to the JSON schema field "attempt_id".
	AttemptID CapabilityAttemptIDV1 `json:"attempt_id"`

	// Evidence corresponds to the JSON schema field "evidence".
	Evidence ImmutableRecordReferenceV1 `json:"evidence"`

	// Phase corresponds to the JSON schema field "phase".
	Phase string `json:"phase"`
}

type OperationActivePhaseDispatchedV1 struct {
	// AttemptID corresponds to the JSON schema field "attempt_id".
	AttemptID CapabilityAttemptIDV1 `json:"attempt_id"`

	// Evidence corresponds to the JSON schema field "evidence".
	Evidence ImmutableRecordReferenceV1 `json:"evidence"`

	// Phase corresponds to the JSON schema field "phase".
	Phase string `json:"phase"`
}

type OperationActivePhaseProviderResolvedV1 struct {
	// Phase corresponds to the JSON schema field "phase".
	Phase string `json:"phase"`

	// ProviderBinding corresponds to the JSON schema field "provider_binding".
	ProviderBinding ProviderBindingRefV1 `json:"provider_binding"`
}

type OperationActivePhaseResolvingV1 struct {
	// Phase corresponds to the JSON schema field "phase".
	Phase string `json:"phase"`
}

type OperationHandleIDV1 string

type OperationObservationRevisionV1 int

type OperationObservationStateActiveV1 struct {
	// Phase corresponds to the JSON schema field "phase".
	Phase OperationActivePhaseV1 `json:"phase"`

	// State corresponds to the JSON schema field "state".
	State string `json:"state"`
}

type OperationObservationStateCancellationRequestedV1 struct {
	// Request corresponds to the JSON schema field "request".
	Request ImmutableRecordReferenceV1 `json:"request"`

	// State corresponds to the JSON schema field "state".
	State string `json:"state"`
}

type OperationObservationStateCancelledV1 struct {
	// Diagnostic corresponds to the JSON schema field "diagnostic".
	Diagnostic ImmutableRecordReferenceV1 `json:"diagnostic"`

	// State corresponds to the JSON schema field "state".
	State string `json:"state"`
}

type OperationObservationStateExpiredV1 struct {
	// Diagnostic corresponds to the JSON schema field "diagnostic".
	Diagnostic ImmutableRecordReferenceV1 `json:"diagnostic"`

	// State corresponds to the JSON schema field "state".
	State string `json:"state"`
}

type OperationObservationStateFailedV1 struct {
	// Diagnostic corresponds to the JSON schema field "diagnostic".
	Diagnostic ImmutableRecordReferenceV1 `json:"diagnostic"`

	// Result corresponds to the JSON schema field "result".
	Result *CapabilityResultReferenceV1 `json:"result,omitempty,omitzero"`

	// State corresponds to the JSON schema field "state".
	State string `json:"state"`
}

type OperationObservationStateOutcomeUnknownV1 struct {
	// Diagnostic corresponds to the JSON schema field "diagnostic".
	Diagnostic ImmutableRecordReferenceV1 `json:"diagnostic"`

	// EffectReceipt corresponds to the JSON schema field "effect_receipt".
	EffectReceipt *EffectReceiptReferenceV1 `json:"effect_receipt,omitempty,omitzero"`

	// Reconciliation corresponds to the JSON schema field "reconciliation".
	Reconciliation ReconciliationFrontierReferenceV1 `json:"reconciliation"`

	// State corresponds to the JSON schema field "state".
	State string `json:"state"`
}

type OperationObservationStateRecoveringV1 struct {
	// Reconciliation corresponds to the JSON schema field "reconciliation".
	Reconciliation ReconciliationFrontierReferenceV1 `json:"reconciliation"`

	// State corresponds to the JSON schema field "state".
	State string `json:"state"`
}

type OperationObservationStateSucceededV1 struct {
	// EffectReceipt corresponds to the JSON schema field "effect_receipt".
	EffectReceipt *EffectReceiptReferenceV1 `json:"effect_receipt,omitempty,omitzero"`

	// Result corresponds to the JSON schema field "result".
	Result CapabilityResultReferenceV1 `json:"result"`

	// State corresponds to the JSON schema field "state".
	State string `json:"state"`
}

type OperationObservationV1 struct {
	// CanonicalOperationObservationSha256 corresponds to the JSON schema field
	// "canonical_operation_observation_sha256".
	CanonicalOperationObservationSha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_operation_observation_sha256"`

	// DeploymentRuntimeAuthorityRef corresponds to the JSON schema field
	// "deployment_runtime_authority_ref".
	DeploymentRuntimeAuthorityRef ResidentRecordRefV1 `json:"deployment_runtime_authority_ref"`

	// DeploymentRuntimeAuthoritySha256 corresponds to the JSON schema field
	// "deployment_runtime_authority_sha256".
	DeploymentRuntimeAuthoritySha256 sha256_digest_v1.Sha256DigestV1 `json:"deployment_runtime_authority_sha256"`

	// IssuerKeyEpoch corresponds to the JSON schema field "issuer_key_epoch".
	IssuerKeyEpoch SigningKeyEpochV1 `json:"issuer_key_epoch"`

	// IssuerKeyID corresponds to the JSON schema field "issuer_key_id".
	IssuerKeyID ResidentKeyIDV1 `json:"issuer_key_id"`

	// IssuerPrincipalID corresponds to the JSON schema field "issuer_principal_id".
	IssuerPrincipalID ResidentPrincipalIDV1 `json:"issuer_principal_id"`

	// ObservedAt corresponds to the JSON schema field "observed_at".
	ObservedAt UnixTimeV1 `json:"observed_at"`

	// OperationHandleID corresponds to the JSON schema field "operation_handle_id".
	OperationHandleID OperationHandleIDV1 `json:"operation_handle_id"`

	// OperationHandleRef corresponds to the JSON schema field "operation_handle_ref".
	OperationHandleRef ResidentRecordRefV1 `json:"operation_handle_ref"`

	// OperationHandleSha256 corresponds to the JSON schema field
	// "operation_handle_sha256".
	OperationHandleSha256 sha256_digest_v1.Sha256DigestV1 `json:"operation_handle_sha256"`

	// PredecessorRef corresponds to the JSON schema field "predecessor_ref".
	PredecessorRef *ResidentRecordRefV1 `json:"predecessor_ref,omitempty,omitzero"`

	// PredecessorSha256 corresponds to the JSON schema field "predecessor_sha256".
	PredecessorSha256 *sha256_digest_v1.Sha256DigestV1 `json:"predecessor_sha256,omitempty,omitzero"`

	// Revision corresponds to the JSON schema field "revision".
	Revision OperationObservationRevisionV1 `json:"revision"`

	// SchemaID corresponds to the JSON schema field "schema_id".
	SchemaID SchemaID `json:"schema_id"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// Signature corresponds to the JSON schema field "signature".
	Signature CanonicalSignatureV1 `json:"signature"`

	// State corresponds to the JSON schema field "state".
	State OperationObservationStateV1 `json:"state"`
}

type ProviderBindingIdentityV1 struct {
	// ProviderBindingSha256 corresponds to the JSON schema field
	// "provider_binding_sha256".
	ProviderBindingSha256 sha256_digest_v1.Sha256DigestV1 `json:"provider_binding_sha256"`
}

type ProviderBindingRefV1 struct {
	// ProviderBindingID corresponds to the JSON schema field "provider_binding_id".
	ProviderBindingID string `json:"provider_binding_id"`

	// ProviderBindingIdentity corresponds to the JSON schema field
	// "provider_binding_identity".
	ProviderBindingIdentity ProviderBindingIdentityV1 `json:"provider_binding_identity"`
}

type ReconciliationFrontierReferenceV1 struct {
	// RecordRef corresponds to the JSON schema field "record_ref".
	RecordRef ResidentRecordRefV1 `json:"record_ref"`

	// RecordSha256 corresponds to the JSON schema field "record_sha256".
	RecordSha256 sha256_digest_v1.Sha256DigestV1 `json:"record_sha256"`
}

type ResidentKeyIDV1 string

type ResidentPrincipalIDV1 string

type ResidentRecordRefV1 string

type SchemaID string

type SigningKeyEpochV1 int

type UnixTimeV1 int

type OperationActivePhaseV1 struct {
	Resolving              *OperationActivePhaseResolvingV1              `json:"-"`
	ContextBound           *OperationActivePhaseContextBoundV1           `json:"-"`
	ProviderResolved       *OperationActivePhaseProviderResolvedV1       `json:"-"`
	DispatchPrepared       *OperationActivePhaseDispatchPreparedV1       `json:"-"`
	Dispatched             *OperationActivePhaseDispatchedV1             `json:"-"`
	CollectingConsultation *OperationActivePhaseCollectingConsultationV1 `json:"-"`
	ConsultationCollected  *OperationActivePhaseConsultationCollectedV1  `json:"-"`
}

func (value OperationActivePhaseV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Resolving != nil {
		if value.Resolving.Phase != "resolving" {
			return nil, fmt.Errorf("OperationActivePhaseV1: selected variant has discriminator %q", value.Resolving.Phase)
		}
		candidate, err := json.Marshal(value.Resolving)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.ContextBound != nil {
		if value.ContextBound.Phase != "context_bound" {
			return nil, fmt.Errorf("OperationActivePhaseV1: selected variant has discriminator %q", value.ContextBound.Phase)
		}
		candidate, err := json.Marshal(value.ContextBound)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.ProviderResolved != nil {
		if value.ProviderResolved.Phase != "provider_resolved" {
			return nil, fmt.Errorf("OperationActivePhaseV1: selected variant has discriminator %q", value.ProviderResolved.Phase)
		}
		candidate, err := json.Marshal(value.ProviderResolved)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.DispatchPrepared != nil {
		if value.DispatchPrepared.Phase != "dispatch_prepared" {
			return nil, fmt.Errorf("OperationActivePhaseV1: selected variant has discriminator %q", value.DispatchPrepared.Phase)
		}
		candidate, err := json.Marshal(value.DispatchPrepared)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Dispatched != nil {
		if value.Dispatched.Phase != "dispatched" {
			return nil, fmt.Errorf("OperationActivePhaseV1: selected variant has discriminator %q", value.Dispatched.Phase)
		}
		candidate, err := json.Marshal(value.Dispatched)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.CollectingConsultation != nil {
		if value.CollectingConsultation.Phase != "collecting_consultation" {
			return nil, fmt.Errorf("OperationActivePhaseV1: selected variant has discriminator %q", value.CollectingConsultation.Phase)
		}
		candidate, err := json.Marshal(value.CollectingConsultation)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.ConsultationCollected != nil {
		if value.ConsultationCollected.Phase != "consultation_collected" {
			return nil, fmt.Errorf("OperationActivePhaseV1: selected variant has discriminator %q", value.ConsultationCollected.Phase)
		}
		candidate, err := json.Marshal(value.ConsultationCollected)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("OperationActivePhaseV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *OperationActivePhaseV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Phase string `json:"phase"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = OperationActivePhaseV1{}
	switch discriminator.Phase {
	case "resolving":
		var variant OperationActivePhaseResolvingV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Resolving = &variant
	case "context_bound":
		var variant OperationActivePhaseContextBoundV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.ContextBound = &variant
	case "provider_resolved":
		var variant OperationActivePhaseProviderResolvedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.ProviderResolved = &variant
	case "dispatch_prepared":
		var variant OperationActivePhaseDispatchPreparedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.DispatchPrepared = &variant
	case "dispatched":
		var variant OperationActivePhaseDispatchedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Dispatched = &variant
	case "collecting_consultation":
		var variant OperationActivePhaseCollectingConsultationV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.CollectingConsultation = &variant
	case "consultation_collected":
		var variant OperationActivePhaseConsultationCollectedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.ConsultationCollected = &variant
	default:
		return fmt.Errorf("OperationActivePhaseV1: unknown discriminator %q", discriminator.Phase)
	}
	return nil
}

type OperationObservationStateV1 struct {
	Active                *OperationObservationStateActiveV1                `json:"-"`
	CancellationRequested *OperationObservationStateCancellationRequestedV1 `json:"-"`
	Recovering            *OperationObservationStateRecoveringV1            `json:"-"`
	Succeeded             *OperationObservationStateSucceededV1             `json:"-"`
	Failed                *OperationObservationStateFailedV1                `json:"-"`
	Cancelled             *OperationObservationStateCancelledV1             `json:"-"`
	OutcomeUnknown        *OperationObservationStateOutcomeUnknownV1        `json:"-"`
	Expired               *OperationObservationStateExpiredV1               `json:"-"`
}

func (value OperationObservationStateV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Active != nil {
		if value.Active.State != "active" {
			return nil, fmt.Errorf("OperationObservationStateV1: selected variant has discriminator %q", value.Active.State)
		}
		candidate, err := json.Marshal(value.Active)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.CancellationRequested != nil {
		if value.CancellationRequested.State != "cancellation_requested" {
			return nil, fmt.Errorf("OperationObservationStateV1: selected variant has discriminator %q", value.CancellationRequested.State)
		}
		candidate, err := json.Marshal(value.CancellationRequested)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Recovering != nil {
		if value.Recovering.State != "recovering" {
			return nil, fmt.Errorf("OperationObservationStateV1: selected variant has discriminator %q", value.Recovering.State)
		}
		candidate, err := json.Marshal(value.Recovering)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Succeeded != nil {
		if value.Succeeded.State != "succeeded" {
			return nil, fmt.Errorf("OperationObservationStateV1: selected variant has discriminator %q", value.Succeeded.State)
		}
		candidate, err := json.Marshal(value.Succeeded)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Failed != nil {
		if value.Failed.State != "failed" {
			return nil, fmt.Errorf("OperationObservationStateV1: selected variant has discriminator %q", value.Failed.State)
		}
		candidate, err := json.Marshal(value.Failed)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Cancelled != nil {
		if value.Cancelled.State != "cancelled" {
			return nil, fmt.Errorf("OperationObservationStateV1: selected variant has discriminator %q", value.Cancelled.State)
		}
		candidate, err := json.Marshal(value.Cancelled)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.OutcomeUnknown != nil {
		if value.OutcomeUnknown.State != "outcome_unknown" {
			return nil, fmt.Errorf("OperationObservationStateV1: selected variant has discriminator %q", value.OutcomeUnknown.State)
		}
		candidate, err := json.Marshal(value.OutcomeUnknown)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Expired != nil {
		if value.Expired.State != "expired" {
			return nil, fmt.Errorf("OperationObservationStateV1: selected variant has discriminator %q", value.Expired.State)
		}
		candidate, err := json.Marshal(value.Expired)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("OperationObservationStateV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *OperationObservationStateV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		State string `json:"state"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = OperationObservationStateV1{}
	switch discriminator.State {
	case "active":
		var variant OperationObservationStateActiveV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Active = &variant
	case "cancellation_requested":
		var variant OperationObservationStateCancellationRequestedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.CancellationRequested = &variant
	case "recovering":
		var variant OperationObservationStateRecoveringV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Recovering = &variant
	case "succeeded":
		var variant OperationObservationStateSucceededV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Succeeded = &variant
	case "failed":
		var variant OperationObservationStateFailedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Failed = &variant
	case "cancelled":
		var variant OperationObservationStateCancelledV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Cancelled = &variant
	case "outcome_unknown":
		var variant OperationObservationStateOutcomeUnknownV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.OutcomeUnknown = &variant
	case "expired":
		var variant OperationObservationStateExpiredV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Expired = &variant
	default:
		return fmt.Errorf("OperationObservationStateV1: unknown discriminator %q", discriminator.State)
	}
	return nil
}
