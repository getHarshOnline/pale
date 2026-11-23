// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package responsibility_derivation_v1 contains the generated responsibility-derivation.v1 data model.
package responsibility_derivation_v1

import (
	"encoding/json"
	"fmt"
)

type AttributableAnswerForBoundedResponsibilityOmittedV1 struct {
	// Attribution corresponds to the JSON schema field "attribution".
	Attribution ResponsibilityAttributionV1 `json:"attribution"`

	// Disposition corresponds to the JSON schema field "disposition".
	Disposition string `json:"disposition"`

	// Reason corresponds to the JSON schema field "reason".
	Reason ResponsibilityOmissionReasonID `json:"reason"`
}

type AttributableAnswerForBoundedResponsibilitySuppliedV1 struct {
	// Attribution corresponds to the JSON schema field "attribution".
	Attribution ResponsibilityAttributionV1 `json:"attribution"`

	// Disposition corresponds to the JSON schema field "disposition".
	Disposition string `json:"disposition"`

	// Value corresponds to the JSON schema field "value".
	Value BoundedResponsibilityV1 `json:"value"`
}

type AttributableAnswerForConsumerAcceptanceRequirementOmittedV1 struct {
	// Attribution corresponds to the JSON schema field "attribution".
	Attribution ResponsibilityAttributionV1 `json:"attribution"`

	// Disposition corresponds to the JSON schema field "disposition".
	Disposition string `json:"disposition"`

	// Reason corresponds to the JSON schema field "reason".
	Reason ResponsibilityOmissionReasonID `json:"reason"`
}

type AttributableAnswerForConsumerAcceptanceRequirementSuppliedV1 struct {
	// Attribution corresponds to the JSON schema field "attribution".
	Attribution ResponsibilityAttributionV1 `json:"attribution"`

	// Disposition corresponds to the JSON schema field "disposition".
	Disposition string `json:"disposition"`

	// Value corresponds to the JSON schema field "value".
	Value ConsumerAcceptanceRequirementV1 `json:"value"`
}

type AttributableAnswerForNativeReadBackRequirementOmittedV1 struct {
	// Attribution corresponds to the JSON schema field "attribution".
	Attribution ResponsibilityAttributionV1 `json:"attribution"`

	// Disposition corresponds to the JSON schema field "disposition".
	Disposition string `json:"disposition"`

	// Reason corresponds to the JSON schema field "reason".
	Reason ResponsibilityOmissionReasonID `json:"reason"`
}

type AttributableAnswerForNativeReadBackRequirementSuppliedV1 struct {
	// Attribution corresponds to the JSON schema field "attribution".
	Attribution ResponsibilityAttributionV1 `json:"attribution"`

	// Disposition corresponds to the JSON schema field "disposition".
	Disposition string `json:"disposition"`

	// Value corresponds to the JSON schema field "value".
	Value NativeReadBackRequirementV1 `json:"value"`
}

type AttributableAnswerForPaleSettlementRequirementOmittedV1 struct {
	// Attribution corresponds to the JSON schema field "attribution".
	Attribution ResponsibilityAttributionV1 `json:"attribution"`

	// Disposition corresponds to the JSON schema field "disposition".
	Disposition string `json:"disposition"`

	// Reason corresponds to the JSON schema field "reason".
	Reason ResponsibilityOmissionReasonID `json:"reason"`
}

type AttributableAnswerForPaleSettlementRequirementSuppliedV1 struct {
	// Attribution corresponds to the JSON schema field "attribution".
	Attribution ResponsibilityAttributionV1 `json:"attribution"`

	// Disposition corresponds to the JSON schema field "disposition".
	Disposition string `json:"disposition"`

	// Value corresponds to the JSON schema field "value".
	Value PaleSettlementRequirementV1 `json:"value"`
}

type AttributableAnswerForProviderAcceptanceRequirementOmittedV1 struct {
	// Attribution corresponds to the JSON schema field "attribution".
	Attribution ResponsibilityAttributionV1 `json:"attribution"`

	// Disposition corresponds to the JSON schema field "disposition".
	Disposition string `json:"disposition"`

	// Reason corresponds to the JSON schema field "reason".
	Reason ResponsibilityOmissionReasonID `json:"reason"`
}

type AttributableAnswerForProviderAcceptanceRequirementSuppliedV1 struct {
	// Attribution corresponds to the JSON schema field "attribution".
	Attribution ResponsibilityAttributionV1 `json:"attribution"`

	// Disposition corresponds to the JSON schema field "disposition".
	Disposition string `json:"disposition"`

	// Value corresponds to the JSON schema field "value".
	Value ProviderAcceptanceRequirementV1 `json:"value"`
}

type AttributableAnswerForProviderCompletionRequirementOmittedV1 struct {
	// Attribution corresponds to the JSON schema field "attribution".
	Attribution ResponsibilityAttributionV1 `json:"attribution"`

	// Disposition corresponds to the JSON schema field "disposition".
	Disposition string `json:"disposition"`

	// Reason corresponds to the JSON schema field "reason".
	Reason ResponsibilityOmissionReasonID `json:"reason"`
}

type AttributableAnswerForProviderCompletionRequirementSuppliedV1 struct {
	// Attribution corresponds to the JSON schema field "attribution".
	Attribution ResponsibilityAttributionV1 `json:"attribution"`

	// Disposition corresponds to the JSON schema field "disposition".
	Disposition string `json:"disposition"`

	// Value corresponds to the JSON schema field "value".
	Value ProviderCompletionRequirementV1 `json:"value"`
}

type AttributableAnswerForResponsibilityAdopterModuleOmittedV1 struct {
	// Attribution corresponds to the JSON schema field "attribution".
	Attribution ResponsibilityAttributionV1 `json:"attribution"`

	// Disposition corresponds to the JSON schema field "disposition".
	Disposition string `json:"disposition"`

	// Reason corresponds to the JSON schema field "reason".
	Reason ResponsibilityOmissionReasonID `json:"reason"`
}

type AttributableAnswerForResponsibilityAdopterModuleSuppliedV1 struct {
	// Attribution corresponds to the JSON schema field "attribution".
	Attribution ResponsibilityAttributionV1 `json:"attribution"`

	// Disposition corresponds to the JSON schema field "disposition".
	Disposition string `json:"disposition"`

	// Value corresponds to the JSON schema field "value".
	Value ResponsibilityAdopterModuleV1 `json:"value"`
}

type AttributableAnswerForResponsibilityCeilingsOmittedV1 struct {
	// Attribution corresponds to the JSON schema field "attribution".
	Attribution ResponsibilityAttributionV1 `json:"attribution"`

	// Disposition corresponds to the JSON schema field "disposition".
	Disposition string `json:"disposition"`

	// Reason corresponds to the JSON schema field "reason".
	Reason ResponsibilityOmissionReasonID `json:"reason"`
}

type AttributableAnswerForResponsibilityCeilingsSuppliedV1 struct {
	// Attribution corresponds to the JSON schema field "attribution".
	Attribution ResponsibilityAttributionV1 `json:"attribution"`

	// Disposition corresponds to the JSON schema field "disposition".
	Disposition string `json:"disposition"`

	// Value corresponds to the JSON schema field "value".
	Value ResponsibilityCeilingsV1 `json:"value"`
}

type AttributableAnswerForResponsibilityDirectPathOmittedV1 struct {
	// Attribution corresponds to the JSON schema field "attribution".
	Attribution ResponsibilityAttributionV1 `json:"attribution"`

	// Disposition corresponds to the JSON schema field "disposition".
	Disposition string `json:"disposition"`

	// Reason corresponds to the JSON schema field "reason".
	Reason ResponsibilityOmissionReasonID `json:"reason"`
}

type AttributableAnswerForResponsibilityDirectPathSuppliedV1 struct {
	// Attribution corresponds to the JSON schema field "attribution".
	Attribution ResponsibilityAttributionV1 `json:"attribution"`

	// Disposition corresponds to the JSON schema field "disposition".
	Disposition string `json:"disposition"`

	// Value corresponds to the JSON schema field "value".
	Value ResponsibilityDirectPathV1 `json:"value"`
}

type AttributableAnswerForResponsibilityIncumbentOmittedV1 struct {
	// Attribution corresponds to the JSON schema field "attribution".
	Attribution ResponsibilityAttributionV1 `json:"attribution"`

	// Disposition corresponds to the JSON schema field "disposition".
	Disposition string `json:"disposition"`

	// Reason corresponds to the JSON schema field "reason".
	Reason ResponsibilityOmissionReasonID `json:"reason"`
}

type AttributableAnswerForResponsibilityIncumbentSuppliedV1 struct {
	// Attribution corresponds to the JSON schema field "attribution".
	Attribution ResponsibilityAttributionV1 `json:"attribution"`

	// Disposition corresponds to the JSON schema field "disposition".
	Disposition string `json:"disposition"`

	// Value corresponds to the JSON schema field "value".
	Value ResponsibilityIncumbentV1 `json:"value"`
}

type AttributableAnswerForResponsibilityNativeOwnerOmittedV1 struct {
	// Attribution corresponds to the JSON schema field "attribution".
	Attribution ResponsibilityAttributionV1 `json:"attribution"`

	// Disposition corresponds to the JSON schema field "disposition".
	Disposition string `json:"disposition"`

	// Reason corresponds to the JSON schema field "reason".
	Reason ResponsibilityOmissionReasonID `json:"reason"`
}

type AttributableAnswerForResponsibilityNativeOwnerSuppliedV1 struct {
	// Attribution corresponds to the JSON schema field "attribution".
	Attribution ResponsibilityAttributionV1 `json:"attribution"`

	// Disposition corresponds to the JSON schema field "disposition".
	Disposition string `json:"disposition"`

	// Value corresponds to the JSON schema field "value".
	Value ResponsibilityNativeOwnerV1 `json:"value"`
}

type AttributableAnswerForResponsibilityPlacementOmittedV1 struct {
	// Attribution corresponds to the JSON schema field "attribution".
	Attribution ResponsibilityAttributionV1 `json:"attribution"`

	// Disposition corresponds to the JSON schema field "disposition".
	Disposition string `json:"disposition"`

	// Reason corresponds to the JSON schema field "reason".
	Reason ResponsibilityOmissionReasonID `json:"reason"`
}

type AttributableAnswerForResponsibilityPlacementSuppliedV1 struct {
	// Attribution corresponds to the JSON schema field "attribution".
	Attribution ResponsibilityAttributionV1 `json:"attribution"`

	// Disposition corresponds to the JSON schema field "disposition".
	Disposition string `json:"disposition"`

	// Value corresponds to the JSON schema field "value".
	Value ResponsibilityPlacementV1 `json:"value"`
}

type AttributableAnswerForResponsibilityPublisherModuleOmittedV1 struct {
	// Attribution corresponds to the JSON schema field "attribution".
	Attribution ResponsibilityAttributionV1 `json:"attribution"`

	// Disposition corresponds to the JSON schema field "disposition".
	Disposition string `json:"disposition"`

	// Reason corresponds to the JSON schema field "reason".
	Reason ResponsibilityOmissionReasonID `json:"reason"`
}

type AttributableAnswerForResponsibilityPublisherModuleSuppliedV1 struct {
	// Attribution corresponds to the JSON schema field "attribution".
	Attribution ResponsibilityAttributionV1 `json:"attribution"`

	// Disposition corresponds to the JSON schema field "disposition".
	Disposition string `json:"disposition"`

	// Value corresponds to the JSON schema field "value".
	Value ResponsibilityPublisherModuleV1 `json:"value"`
}

type AttributableAnswerForResponsibilityReadinessOmittedV1 struct {
	// Attribution corresponds to the JSON schema field "attribution".
	Attribution ResponsibilityAttributionV1 `json:"attribution"`

	// Disposition corresponds to the JSON schema field "disposition".
	Disposition string `json:"disposition"`

	// Reason corresponds to the JSON schema field "reason".
	Reason ResponsibilityOmissionReasonID `json:"reason"`
}

type AttributableAnswerForResponsibilityReadinessSuppliedV1 struct {
	// Attribution corresponds to the JSON schema field "attribution".
	Attribution ResponsibilityAttributionV1 `json:"attribution"`

	// Disposition corresponds to the JSON schema field "disposition".
	Disposition string `json:"disposition"`

	// Value corresponds to the JSON schema field "value".
	Value ResponsibilityReadinessV1 `json:"value"`
}

type AttributableAnswerForResponsibilityRequestFramerOmittedV1 struct {
	// Attribution corresponds to the JSON schema field "attribution".
	Attribution ResponsibilityAttributionV1 `json:"attribution"`

	// Disposition corresponds to the JSON schema field "disposition".
	Disposition string `json:"disposition"`

	// Reason corresponds to the JSON schema field "reason".
	Reason ResponsibilityOmissionReasonID `json:"reason"`
}

type AttributableAnswerForResponsibilityRequestFramerSuppliedV1 struct {
	// Attribution corresponds to the JSON schema field "attribution".
	Attribution ResponsibilityAttributionV1 `json:"attribution"`

	// Disposition corresponds to the JSON schema field "disposition".
	Disposition string `json:"disposition"`

	// Value corresponds to the JSON schema field "value".
	Value ResponsibilityRequestFramerV1 `json:"value"`
}

type AttributableAnswerForResponsibilitySafeFailureOmittedV1 struct {
	// Attribution corresponds to the JSON schema field "attribution".
	Attribution ResponsibilityAttributionV1 `json:"attribution"`

	// Disposition corresponds to the JSON schema field "disposition".
	Disposition string `json:"disposition"`

	// Reason corresponds to the JSON schema field "reason".
	Reason ResponsibilityOmissionReasonID `json:"reason"`
}

type AttributableAnswerForResponsibilitySafeFailureSuppliedV1 struct {
	// Attribution corresponds to the JSON schema field "attribution".
	Attribution ResponsibilityAttributionV1 `json:"attribution"`

	// Disposition corresponds to the JSON schema field "disposition".
	Disposition string `json:"disposition"`

	// Value corresponds to the JSON schema field "value".
	Value ResponsibilitySafeFailureV1 `json:"value"`
}

type AttributableAnswerForResponsibilitySilenceBoundaryOmittedV1 struct {
	// Attribution corresponds to the JSON schema field "attribution".
	Attribution ResponsibilityAttributionV1 `json:"attribution"`

	// Disposition corresponds to the JSON schema field "disposition".
	Disposition string `json:"disposition"`

	// Reason corresponds to the JSON schema field "reason".
	Reason ResponsibilityOmissionReasonID `json:"reason"`
}

type AttributableAnswerForResponsibilitySilenceBoundarySuppliedV1 struct {
	// Attribution corresponds to the JSON schema field "attribution".
	Attribution ResponsibilityAttributionV1 `json:"attribution"`

	// Disposition corresponds to the JSON schema field "disposition".
	Disposition string `json:"disposition"`

	// Value corresponds to the JSON schema field "value".
	Value ResponsibilitySilenceBoundaryV1 `json:"value"`
}

type AttributableAnswerForResponsibilityTriggerOmittedV1 struct {
	// Attribution corresponds to the JSON schema field "attribution".
	Attribution ResponsibilityAttributionV1 `json:"attribution"`

	// Disposition corresponds to the JSON schema field "disposition".
	Disposition string `json:"disposition"`

	// Reason corresponds to the JSON schema field "reason".
	Reason ResponsibilityOmissionReasonID `json:"reason"`
}

type AttributableAnswerForResponsibilityTriggerSuppliedV1 struct {
	// Attribution corresponds to the JSON schema field "attribution".
	Attribution ResponsibilityAttributionV1 `json:"attribution"`

	// Disposition corresponds to the JSON schema field "disposition".
	Disposition string `json:"disposition"`

	// Value corresponds to the JSON schema field "value".
	Value ResponsibilityTriggerV1 `json:"value"`
}

type AttributableAnswerForWorkspaceResourceIDOmitted struct {
	// Attribution corresponds to the JSON schema field "attribution".
	Attribution ResponsibilityAttributionV1 `json:"attribution"`

	// Disposition corresponds to the JSON schema field "disposition".
	Disposition string `json:"disposition"`

	// Reason corresponds to the JSON schema field "reason".
	Reason ResponsibilityOmissionReasonID `json:"reason"`
}

type AttributableAnswerForWorkspaceResourceIDSupplied struct {
	// Attribution corresponds to the JSON schema field "attribution".
	Attribution ResponsibilityAttributionV1 `json:"attribution"`

	// Disposition corresponds to the JSON schema field "disposition".
	Disposition string `json:"disposition"`

	// Value corresponds to the JSON schema field "value".
	Value WorkspaceResourceID `json:"value"`
}

type BoundedResponsibilityV1 struct {
	// Object corresponds to the JSON schema field "object".
	Object ResponsibilityObjectID `json:"object"`

	// Outcome corresponds to the JSON schema field "outcome".
	Outcome ResponsibilityOutcomeID `json:"outcome"`

	// Verb corresponds to the JSON schema field "verb".
	Verb ResponsibilityVerbID `json:"verb"`
}

type ConsumerAcceptanceRequirementV1 struct {
	// Requirement corresponds to the JSON schema field "requirement".
	Requirement ResponsibilityEvidenceRequirementV1 `json:"requirement"`
}

type DirectionalServiceContractID string

type DirectionalServiceContractRefV1 struct {
	// ConsumerScope corresponds to the JSON schema field "consumer_scope".
	ConsumerScope LogicalScopeRefV1 `json:"consumer_scope"`

	// ContractID corresponds to the JSON schema field "contract_id".
	ContractID DirectionalServiceContractID `json:"contract_id"`

	// Namespace corresponds to the JSON schema field "namespace".
	Namespace WorkspaceNamespaceID `json:"namespace"`

	// ProviderScope corresponds to the JSON schema field "provider_scope".
	ProviderScope LogicalScopeRefV1 `json:"provider_scope"`
}

type DisclosureCeilingV1 string

const DisclosureCeilingV1Private DisclosureCeilingV1 = "private"
const DisclosureCeilingV1Protected DisclosureCeilingV1 = "protected"
const DisclosureCeilingV1Public DisclosureCeilingV1 = "public"

type EffectiveFactStateV1 string

const EffectiveFactStateV1Absent EffectiveFactStateV1 = "absent"
const EffectiveFactStateV1Denied EffectiveFactStateV1 = "denied"
const EffectiveFactStateV1Present EffectiveFactStateV1 = "present"
const EffectiveFactStateV1Revoked EffectiveFactStateV1 = "revoked"
const EffectiveFactStateV1Stale EffectiveFactStateV1 = "stale"
const EffectiveFactStateV1Superseded EffectiveFactStateV1 = "superseded"
const EffectiveFactStateV1Unknown EffectiveFactStateV1 = "unknown"

type LogicalScopeID string

type LogicalScopeRefV1 struct {
	// Namespace corresponds to the JSON schema field "namespace".
	Namespace WorkspaceNamespaceID `json:"namespace"`

	// ScopeID corresponds to the JSON schema field "scope_id".
	ScopeID LogicalScopeID `json:"scope_id"`
}

type NativeReadBackRequirementV1 struct {
	// Requirement corresponds to the JSON schema field "requirement".
	Requirement ResponsibilityEvidenceRequirementV1 `json:"requirement"`
}

type PaleSettlementRequirementV1 struct {
	// Requirement corresponds to the JSON schema field "requirement".
	Requirement ResponsibilityEvidenceRequirementV1 `json:"requirement"`
}

type ProviderAcceptanceRequirementV1 struct {
	// Requirement corresponds to the JSON schema field "requirement".
	Requirement ResponsibilityEvidenceRequirementV1 `json:"requirement"`
}

type ProviderCompletionRequirementV1 struct {
	// Requirement corresponds to the JSON schema field "requirement".
	Requirement ResponsibilityEvidenceRequirementV1 `json:"requirement"`
}

type ResponsibilityAdopterModuleV1 struct {
	// AdoptionOwner corresponds to the JSON schema field "adoption_owner".
	AdoptionOwner WorkspaceResourceID `json:"adoption_owner"`

	// ImportedExportID corresponds to the JSON schema field "imported_export_id".
	ImportedExportID WorkspaceExportID `json:"imported_export_id"`

	// ImportedModuleID corresponds to the JSON schema field "imported_module_id".
	ImportedModuleID WorkspaceModuleID `json:"imported_module_id"`

	// ImportedReleaseID corresponds to the JSON schema field "imported_release_id".
	ImportedReleaseID WorkspaceReleaseID `json:"imported_release_id"`

	// LocalNamespace corresponds to the JSON schema field "local_namespace".
	LocalNamespace WorkspaceNamespaceID `json:"local_namespace"`

	// ModuleID corresponds to the JSON schema field "module_id".
	ModuleID WorkspaceModuleID `json:"module_id"`
}

type ResponsibilityAttributionV1 struct {
	// EvidenceRef corresponds to the JSON schema field "evidence_ref".
	EvidenceRef WorkspaceResourceID `json:"evidence_ref"`

	// Owner corresponds to the JSON schema field "owner".
	Owner WorkspaceResourceID `json:"owner"`

	// SourceID corresponds to the JSON schema field "source_id".
	SourceID WorkspaceDerivationSourceID `json:"source_id"`
}

type ResponsibilityCeilingsV1 struct {
	// Policy corresponds to the JSON schema field "policy".
	Policy WorkspacePolicyCeilingV1 `json:"policy"`

	// Resources corresponds to the JSON schema field "resources".
	Resources []WorkspaceResourceID `json:"resources"`
}

type ResponsibilityDerivationID string

type ResponsibilityDerivationLifecycleV1 string

const ResponsibilityDerivationLifecycleV1AcceptedTarget ResponsibilityDerivationLifecycleV1 = "accepted_target"
const ResponsibilityDerivationLifecycleV1Candidate ResponsibilityDerivationLifecycleV1 = "candidate"
const ResponsibilityDerivationLifecycleV1ConfigurationOpen ResponsibilityDerivationLifecycleV1 = "configuration_open"
const ResponsibilityDerivationLifecycleV1EvidenceGated ResponsibilityDerivationLifecycleV1 = "evidence_gated"
const ResponsibilityDerivationLifecycleV1Historical ResponsibilityDerivationLifecycleV1 = "historical"
const ResponsibilityDerivationLifecycleV1ImplementationOpen ResponsibilityDerivationLifecycleV1 = "implementation_open"
const ResponsibilityDerivationLifecycleV1OwnerDesiredConfiguration ResponsibilityDerivationLifecycleV1 = "owner_desired_configuration"
const ResponsibilityDerivationLifecycleV1Rejected ResponsibilityDerivationLifecycleV1 = "rejected"
const ResponsibilityDerivationLifecycleV1VerifiedCurrent ResponsibilityDerivationLifecycleV1 = "verified_current"

type ResponsibilityDerivationV1 struct {
	// AdopterModule corresponds to the JSON schema field "adopter_module".
	AdopterModule AttributableAnswerForResponsibilityAdopterModuleV1 `json:"adopter_module"`

	// Ceilings corresponds to the JSON schema field "ceilings".
	Ceilings AttributableAnswerForResponsibilityCeilingsV1 `json:"ceilings"`

	// ConsumerAcceptance corresponds to the JSON schema field "consumer_acceptance".
	ConsumerAcceptance AttributableAnswerForConsumerAcceptanceRequirementV1 `json:"consumer_acceptance"`

	// DerivationID corresponds to the JSON schema field "derivation_id".
	DerivationID ResponsibilityDerivationID `json:"derivation_id"`

	// DirectPath corresponds to the JSON schema field "direct_path".
	DirectPath AttributableAnswerForResponsibilityDirectPathV1 `json:"direct_path"`

	// GenealogyLinks corresponds to the JSON schema field "genealogy_links".
	GenealogyLinks []ResponsibilityGenealogyLinkV1 `json:"genealogy_links"`

	// Incumbent corresponds to the JSON schema field "incumbent".
	Incumbent AttributableAnswerForResponsibilityIncumbentV1 `json:"incumbent"`

	// LawfulSilence corresponds to the JSON schema field "lawful_silence".
	LawfulSilence AttributableAnswerForResponsibilitySilenceBoundaryV1 `json:"lawful_silence"`

	// Lifecycle corresponds to the JSON schema field "lifecycle".
	Lifecycle ResponsibilityDerivationLifecycleV1 `json:"lifecycle"`

	// NativeOwner corresponds to the JSON schema field "native_owner".
	NativeOwner AttributableAnswerForResponsibilityNativeOwnerV1 `json:"native_owner"`

	// NativeProjections corresponds to the JSON schema field "native_projections".
	NativeProjections []ResponsibilityNativeProjectionV1 `json:"native_projections"`

	// NativeResult corresponds to the JSON schema field "native_result".
	NativeResult AttributableAnswerForNativeReadBackRequirementV1 `json:"native_result"`

	// PaleSettlement corresponds to the JSON schema field "pale_settlement".
	PaleSettlement AttributableAnswerForPaleSettlementRequirementV1 `json:"pale_settlement"`

	// Placement corresponds to the JSON schema field "placement".
	Placement AttributableAnswerForResponsibilityPlacementV1 `json:"placement"`

	// Provenance corresponds to the JSON schema field "provenance".
	Provenance ResponsibilityAttributionV1 `json:"provenance"`

	// ProviderAcceptance corresponds to the JSON schema field "provider_acceptance".
	ProviderAcceptance AttributableAnswerForProviderAcceptanceRequirementV1 `json:"provider_acceptance"`

	// ProviderCompletion corresponds to the JSON schema field "provider_completion".
	ProviderCompletion AttributableAnswerForProviderCompletionRequirementV1 `json:"provider_completion"`

	// PublisherModule corresponds to the JSON schema field "publisher_module".
	PublisherModule AttributableAnswerForResponsibilityPublisherModuleV1 `json:"publisher_module"`

	// Readiness corresponds to the JSON schema field "readiness".
	Readiness AttributableAnswerForResponsibilityReadinessV1 `json:"readiness"`

	// RequestFramer corresponds to the JSON schema field "request_framer".
	RequestFramer AttributableAnswerForResponsibilityRequestFramerV1 `json:"request_framer"`

	// Residence corresponds to the JSON schema field "residence".
	Residence []ResponsibilityResidenceV1 `json:"residence"`

	// Responsibility corresponds to the JSON schema field "responsibility".
	Responsibility AttributableAnswerForBoundedResponsibilityV1 `json:"responsibility"`

	// SafeFailure corresponds to the JSON schema field "safe_failure".
	SafeFailure AttributableAnswerForResponsibilitySafeFailureV1 `json:"safe_failure"`

	// Schema corresponds to the JSON schema field "schema".
	Schema string `json:"schema"`

	// SchemaVersion corresponds to the JSON schema field "schema_version".
	SchemaVersion int `json:"schema_version"`

	// StructuralObjects corresponds to the JSON schema field "structural_objects".
	StructuralObjects []ResponsibilityStructuralSelectionV1 `json:"structural_objects"`

	// Trigger corresponds to the JSON schema field "trigger".
	Trigger AttributableAnswerForResponsibilityTriggerV1 `json:"trigger"`
}

type ResponsibilityDirectPathV1 struct {
	// AvailabilityPolicy corresponds to the JSON schema field "availability_policy".
	AvailabilityPolicy WorkspaceResourceID `json:"availability_policy"`

	// NativeOwner corresponds to the JSON schema field "native_owner".
	NativeOwner WorkspaceResourceID `json:"native_owner"`

	// Path corresponds to the JSON schema field "path".
	Path WorkspaceResourceID `json:"path"`
}

type ResponsibilityEvidenceRequirementV1 struct {
	// EvidenceRef corresponds to the JSON schema field "evidence_ref".
	EvidenceRef WorkspaceResourceID `json:"evidence_ref"`

	// Owner corresponds to the JSON schema field "owner".
	Owner WorkspaceResourceID `json:"owner"`
}

type ResponsibilityGenealogyLinkID string

type ResponsibilityGenealogyLinkV1 struct {
	// LinkID corresponds to the JSON schema field "link_id".
	LinkID ResponsibilityGenealogyLinkID `json:"link_id"`

	// Occurrences corresponds to the JSON schema field "occurrences".
	Occurrences []ResponsibilityOccurrenceID `json:"occurrences"`

	// Projections corresponds to the JSON schema field "projections".
	Projections []ResponsibilityProjectionID `json:"projections"`

	// Responsibilities corresponds to the JSON schema field "responsibilities".
	Responsibilities []ResponsibilityDerivationID `json:"responsibilities"`
}

type ResponsibilityIncumbentV1 struct {
	// Mechanism corresponds to the JSON schema field "mechanism".
	Mechanism WorkspaceResourceID `json:"mechanism"`

	// RetainedEvidence corresponds to the JSON schema field "retained_evidence".
	RetainedEvidence WorkspaceResourceID `json:"retained_evidence"`

	// Writer corresponds to the JSON schema field "writer".
	Writer WorkspaceResourceID `json:"writer"`
}

type ResponsibilityNativeOwnerV1 struct {
	// AuthorityResolver corresponds to the JSON schema field "authority_resolver".
	AuthorityResolver WorkspaceResourceID `json:"authority_resolver"`

	// Owner corresponds to the JSON schema field "owner".
	Owner WorkspaceResourceID `json:"owner"`

	// OwnerClass corresponds to the JSON schema field "owner_class".
	OwnerClass ResponsibilityOwnerClassV1 `json:"owner_class"`
}

type ResponsibilityNativeProjectionV1 struct {
	// Class corresponds to the JSON schema field "class".
	Class ResponsibilityProjectionClassV1 `json:"class"`

	// NativeOwner corresponds to the JSON schema field "native_owner".
	NativeOwner WorkspaceResourceID `json:"native_owner"`

	// NativeRefs corresponds to the JSON schema field "native_refs".
	NativeRefs []WorkspaceResourceID `json:"native_refs"`

	// ProjectionID corresponds to the JSON schema field "projection_id".
	ProjectionID ResponsibilityProjectionID `json:"projection_id"`

	// State corresponds to the JSON schema field "state".
	State EffectiveFactStateV1 `json:"state"`
}

type ResponsibilityObjectID string

type ResponsibilityOccurrenceID string

type ResponsibilityOmissionReasonID string

type ResponsibilityOutcomeID string

type ResponsibilityOwnerClassV1 string

const ResponsibilityOwnerClassV1Module ResponsibilityOwnerClassV1 = "module"
const ResponsibilityOwnerClassV1NativeApplication ResponsibilityOwnerClassV1 = "native_application"
const ResponsibilityOwnerClassV1Principal ResponsibilityOwnerClassV1 = "principal"
const ResponsibilityOwnerClassV1RelationshipContract ResponsibilityOwnerClassV1 = "relationship_contract"

type ResponsibilityPlacementDirectionalServiceV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// ServiceContract corresponds to the JSON schema field "service_contract".
	ServiceContract DirectionalServiceContractRefV1 `json:"service_contract"`
}

type ResponsibilityPlacementVerticalLocalV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Scope corresponds to the JSON schema field "scope".
	Scope WorkspaceResourceID `json:"scope"`
}

type ResponsibilityProjectionClassV1 string

const ResponsibilityProjectionClassV1AccountableWork ResponsibilityProjectionClassV1 = "accountable_work"
const ResponsibilityProjectionClassV1Communication ResponsibilityProjectionClassV1 = "communication"
const ResponsibilityProjectionClassV1Context ResponsibilityProjectionClassV1 = "context"
const ResponsibilityProjectionClassV1DerivedView ResponsibilityProjectionClassV1 = "derived_view"
const ResponsibilityProjectionClassV1Genealogy ResponsibilityProjectionClassV1 = "genealogy"
const ResponsibilityProjectionClassV1NativeApplication ResponsibilityProjectionClassV1 = "native_application"
const ResponsibilityProjectionClassV1Relationship ResponsibilityProjectionClassV1 = "relationship"

type ResponsibilityProjectionID string

type ResponsibilityPublisherModuleV1 struct {
	// ExportID corresponds to the JSON schema field "export_id".
	ExportID WorkspaceExportID `json:"export_id"`

	// ModuleID corresponds to the JSON schema field "module_id".
	ModuleID WorkspaceModuleID `json:"module_id"`

	// ReleaseID corresponds to the JSON schema field "release_id".
	ReleaseID WorkspaceReleaseID `json:"release_id"`

	// SemanticOwner corresponds to the JSON schema field "semantic_owner".
	SemanticOwner WorkspaceResourceID `json:"semantic_owner"`
}

type ResponsibilityReadinessV1 struct {
	// FailureProof corresponds to the JSON schema field "failure_proof".
	FailureProof WorkspaceResourceID `json:"failure_proof"`

	// NativeReadBackProof corresponds to the JSON schema field
	// "native_read_back_proof".
	NativeReadBackProof WorkspaceResourceID `json:"native_read_back_proof"`

	// OperationContract corresponds to the JSON schema field "operation_contract".
	OperationContract WorkspaceResourceID `json:"operation_contract"`

	// RecoveryProof corresponds to the JSON schema field "recovery_proof".
	RecoveryProof WorkspaceResourceID `json:"recovery_proof"`

	// ReplayProof corresponds to the JSON schema field "replay_proof".
	ReplayProof WorkspaceResourceID `json:"replay_proof"`

	// RollbackProof corresponds to the JSON schema field "rollback_proof".
	RollbackProof WorkspaceResourceID `json:"rollback_proof"`
}

type ResponsibilityRequestFramerV1 struct {
	// Actor corresponds to the JSON schema field "actor".
	Actor WorkspaceResourceID `json:"actor"`

	// AuthorityScope corresponds to the JSON schema field "authority_scope".
	AuthorityScope WorkspaceAuthorityScopeID `json:"authority_scope"`

	// Owner corresponds to the JSON schema field "owner".
	Owner WorkspaceResourceID `json:"owner"`
}

type ResponsibilityResidenceClassV1 string

const ResponsibilityResidenceClassV1Configuration ResponsibilityResidenceClassV1 = "configuration"
const ResponsibilityResidenceClassV1NativeApplication ResponsibilityResidenceClassV1 = "native_application"
const ResponsibilityResidenceClassV1NormativePack ResponsibilityResidenceClassV1 = "normative_pack"
const ResponsibilityResidenceClassV1Package ResponsibilityResidenceClassV1 = "package"
const ResponsibilityResidenceClassV1Reference ResponsibilityResidenceClassV1 = "reference"

type ResponsibilityResidenceV1 struct {
	// Class corresponds to the JSON schema field "class".
	Class ResponsibilityResidenceClassV1 `json:"class"`

	// Owner corresponds to the JSON schema field "owner".
	Owner WorkspaceResourceID `json:"owner"`

	// Resource corresponds to the JSON schema field "resource".
	Resource WorkspaceResourceID `json:"resource"`
}

type ResponsibilitySafeFailureV1 string

const ResponsibilitySafeFailureV1Compensate ResponsibilitySafeFailureV1 = "compensate"
const ResponsibilitySafeFailureV1Degrade ResponsibilitySafeFailureV1 = "degrade"
const ResponsibilitySafeFailureV1Deny ResponsibilitySafeFailureV1 = "deny"
const ResponsibilitySafeFailureV1OwnerEscalation ResponsibilitySafeFailureV1 = "owner_escalation"
const ResponsibilitySafeFailureV1RemainNative ResponsibilitySafeFailureV1 = "remain_native"
const ResponsibilitySafeFailureV1Wait ResponsibilitySafeFailureV1 = "wait"

type ResponsibilitySilenceBoundaryV1 struct {
	// SilentSurfaces corresponds to the JSON schema field "silent_surfaces".
	SilentSurfaces []ResponsibilitySilenceClassV1 `json:"silent_surfaces"`
}

type ResponsibilitySilenceClassV1 string

const ResponsibilitySilenceClassV1AccountableWork ResponsibilitySilenceClassV1 = "accountable_work"
const ResponsibilitySilenceClassV1Communication ResponsibilitySilenceClassV1 = "communication"
const ResponsibilitySilenceClassV1RelationshipState ResponsibilitySilenceClassV1 = "relationship_state"

type ResponsibilityStructuralSelectionV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind StructuralObjectKindV1 `json:"kind"`

	// Selection corresponds to the JSON schema field "selection".
	Selection AttributableAnswerForWorkspaceResourceID `json:"selection"`
}

type ResponsibilityTriggerBoundedRequestV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// RequestContract corresponds to the JSON schema field "request_contract".
	RequestContract WorkspaceResourceID `json:"request_contract"`
}

type ResponsibilityTriggerNativeEventV1 struct {
	// Event corresponds to the JSON schema field "event".
	Event WorkspaceResourceID `json:"event"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type ResponsibilityTriggerOwnerEventV1 struct {
	// Event corresponds to the JSON schema field "event".
	Event WorkspaceResourceID `json:"event"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type ResponsibilityTriggerRecurringCadenceV1 struct {
	// Cadence corresponds to the JSON schema field "cadence".
	Cadence WorkspaceResourceID `json:"cadence"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type ResponsibilityVerbID string

type StructuralObjectKindV1 string

const StructuralObjectKindV1AccountableWorkProjection StructuralObjectKindV1 = "accountable_work_projection"
const StructuralObjectKindV1CommunicationProjection StructuralObjectKindV1 = "communication_projection"
const StructuralObjectKindV1DirectionalServiceContract StructuralObjectKindV1 = "directional_service_contract"
const StructuralObjectKindV1LogicalOrganization StructuralObjectKindV1 = "logical_organization"
const StructuralObjectKindV1NativeApplication StructuralObjectKindV1 = "native_application"
const StructuralObjectKindV1NormativePack StructuralObjectKindV1 = "normative_pack"
const StructuralObjectKindV1OccupantProvider StructuralObjectKindV1 = "occupant_provider"
const StructuralObjectKindV1Package StructuralObjectKindV1 = "package"
const StructuralObjectKindV1ReferenceOnly StructuralObjectKindV1 = "reference_only"
const StructuralObjectKindV1RelationshipProjection StructuralObjectKindV1 = "relationship_projection"
const StructuralObjectKindV1StableRole StructuralObjectKindV1 = "stable_role"

type WorkspaceAuthorityCeilingV1 struct {
	// Permitted corresponds to the JSON schema field "permitted".
	Permitted []WorkspaceAuthorityScopeID `json:"permitted"`
}

type WorkspaceAuthorityScopeID string

type WorkspaceControllerCeilingV1 struct {
	// Permitted corresponds to the JSON schema field "permitted".
	Permitted []WorkspaceControllerScopeID `json:"permitted"`
}

type WorkspaceControllerScopeID string

type WorkspaceDerivationSourceID string

type WorkspaceEffectCeilingV1 string

const WorkspaceEffectCeilingV1Mutate WorkspaceEffectCeilingV1 = "mutate"
const WorkspaceEffectCeilingV1None WorkspaceEffectCeilingV1 = "none"
const WorkspaceEffectCeilingV1Read WorkspaceEffectCeilingV1 = "read"

type WorkspaceExportID string

type WorkspaceModuleID string

type WorkspaceNamespaceID string

type WorkspacePolicyCeilingV1 struct {
	// Authorities corresponds to the JSON schema field "authorities".
	Authorities WorkspaceAuthorityCeilingV1 `json:"authorities"`

	// Controllers corresponds to the JSON schema field "controllers".
	Controllers WorkspaceControllerCeilingV1 `json:"controllers"`

	// Disclosure corresponds to the JSON schema field "disclosure".
	Disclosure DisclosureCeilingV1 `json:"disclosure"`

	// Effect corresponds to the JSON schema field "effect".
	Effect WorkspaceEffectCeilingV1 `json:"effect"`

	// ModelUsePermitted corresponds to the JSON schema field "model_use_permitted".
	ModelUsePermitted bool `json:"model_use_permitted"`

	// RetentionSeconds corresponds to the JSON schema field "retention_seconds".
	RetentionSeconds int `json:"retention_seconds"`

	// Rights corresponds to the JSON schema field "rights".
	Rights WorkspaceRightsCeilingV1 `json:"rights"`
}

type WorkspaceReleaseID string

type WorkspaceResourceID string

type WorkspaceRightID string

type WorkspaceRightsCeilingV1 struct {
	// Permitted corresponds to the JSON schema field "permitted".
	Permitted []WorkspaceRightID `json:"permitted"`
}

type AttributableAnswerForBoundedResponsibilityV1 struct {
	Supplied *AttributableAnswerForBoundedResponsibilitySuppliedV1 `json:"-"`
	Omitted  *AttributableAnswerForBoundedResponsibilityOmittedV1  `json:"-"`
}

func (value AttributableAnswerForBoundedResponsibilityV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Supplied != nil {
		if value.Supplied.Disposition != "supplied" {
			return nil, fmt.Errorf("AttributableAnswerForBoundedResponsibilityV1: selected variant has discriminator %q", value.Supplied.Disposition)
		}
		candidate, err := json.Marshal(value.Supplied)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Omitted != nil {
		if value.Omitted.Disposition != "omitted" {
			return nil, fmt.Errorf("AttributableAnswerForBoundedResponsibilityV1: selected variant has discriminator %q", value.Omitted.Disposition)
		}
		candidate, err := json.Marshal(value.Omitted)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("AttributableAnswerForBoundedResponsibilityV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *AttributableAnswerForBoundedResponsibilityV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Disposition string `json:"disposition"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = AttributableAnswerForBoundedResponsibilityV1{}
	switch discriminator.Disposition {
	case "supplied":
		var variant AttributableAnswerForBoundedResponsibilitySuppliedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Supplied = &variant
	case "omitted":
		var variant AttributableAnswerForBoundedResponsibilityOmittedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Omitted = &variant
	default:
		return fmt.Errorf("AttributableAnswerForBoundedResponsibilityV1: unknown discriminator %q", discriminator.Disposition)
	}
	return nil
}

type AttributableAnswerForConsumerAcceptanceRequirementV1 struct {
	Supplied *AttributableAnswerForConsumerAcceptanceRequirementSuppliedV1 `json:"-"`
	Omitted  *AttributableAnswerForConsumerAcceptanceRequirementOmittedV1  `json:"-"`
}

func (value AttributableAnswerForConsumerAcceptanceRequirementV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Supplied != nil {
		if value.Supplied.Disposition != "supplied" {
			return nil, fmt.Errorf("AttributableAnswerForConsumerAcceptanceRequirementV1: selected variant has discriminator %q", value.Supplied.Disposition)
		}
		candidate, err := json.Marshal(value.Supplied)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Omitted != nil {
		if value.Omitted.Disposition != "omitted" {
			return nil, fmt.Errorf("AttributableAnswerForConsumerAcceptanceRequirementV1: selected variant has discriminator %q", value.Omitted.Disposition)
		}
		candidate, err := json.Marshal(value.Omitted)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("AttributableAnswerForConsumerAcceptanceRequirementV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *AttributableAnswerForConsumerAcceptanceRequirementV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Disposition string `json:"disposition"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = AttributableAnswerForConsumerAcceptanceRequirementV1{}
	switch discriminator.Disposition {
	case "supplied":
		var variant AttributableAnswerForConsumerAcceptanceRequirementSuppliedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Supplied = &variant
	case "omitted":
		var variant AttributableAnswerForConsumerAcceptanceRequirementOmittedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Omitted = &variant
	default:
		return fmt.Errorf("AttributableAnswerForConsumerAcceptanceRequirementV1: unknown discriminator %q", discriminator.Disposition)
	}
	return nil
}

type AttributableAnswerForNativeReadBackRequirementV1 struct {
	Supplied *AttributableAnswerForNativeReadBackRequirementSuppliedV1 `json:"-"`
	Omitted  *AttributableAnswerForNativeReadBackRequirementOmittedV1  `json:"-"`
}

func (value AttributableAnswerForNativeReadBackRequirementV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Supplied != nil {
		if value.Supplied.Disposition != "supplied" {
			return nil, fmt.Errorf("AttributableAnswerForNativeReadBackRequirementV1: selected variant has discriminator %q", value.Supplied.Disposition)
		}
		candidate, err := json.Marshal(value.Supplied)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Omitted != nil {
		if value.Omitted.Disposition != "omitted" {
			return nil, fmt.Errorf("AttributableAnswerForNativeReadBackRequirementV1: selected variant has discriminator %q", value.Omitted.Disposition)
		}
		candidate, err := json.Marshal(value.Omitted)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("AttributableAnswerForNativeReadBackRequirementV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *AttributableAnswerForNativeReadBackRequirementV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Disposition string `json:"disposition"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = AttributableAnswerForNativeReadBackRequirementV1{}
	switch discriminator.Disposition {
	case "supplied":
		var variant AttributableAnswerForNativeReadBackRequirementSuppliedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Supplied = &variant
	case "omitted":
		var variant AttributableAnswerForNativeReadBackRequirementOmittedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Omitted = &variant
	default:
		return fmt.Errorf("AttributableAnswerForNativeReadBackRequirementV1: unknown discriminator %q", discriminator.Disposition)
	}
	return nil
}

type AttributableAnswerForPaleSettlementRequirementV1 struct {
	Supplied *AttributableAnswerForPaleSettlementRequirementSuppliedV1 `json:"-"`
	Omitted  *AttributableAnswerForPaleSettlementRequirementOmittedV1  `json:"-"`
}

func (value AttributableAnswerForPaleSettlementRequirementV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Supplied != nil {
		if value.Supplied.Disposition != "supplied" {
			return nil, fmt.Errorf("AttributableAnswerForPaleSettlementRequirementV1: selected variant has discriminator %q", value.Supplied.Disposition)
		}
		candidate, err := json.Marshal(value.Supplied)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Omitted != nil {
		if value.Omitted.Disposition != "omitted" {
			return nil, fmt.Errorf("AttributableAnswerForPaleSettlementRequirementV1: selected variant has discriminator %q", value.Omitted.Disposition)
		}
		candidate, err := json.Marshal(value.Omitted)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("AttributableAnswerForPaleSettlementRequirementV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *AttributableAnswerForPaleSettlementRequirementV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Disposition string `json:"disposition"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = AttributableAnswerForPaleSettlementRequirementV1{}
	switch discriminator.Disposition {
	case "supplied":
		var variant AttributableAnswerForPaleSettlementRequirementSuppliedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Supplied = &variant
	case "omitted":
		var variant AttributableAnswerForPaleSettlementRequirementOmittedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Omitted = &variant
	default:
		return fmt.Errorf("AttributableAnswerForPaleSettlementRequirementV1: unknown discriminator %q", discriminator.Disposition)
	}
	return nil
}

type AttributableAnswerForProviderAcceptanceRequirementV1 struct {
	Supplied *AttributableAnswerForProviderAcceptanceRequirementSuppliedV1 `json:"-"`
	Omitted  *AttributableAnswerForProviderAcceptanceRequirementOmittedV1  `json:"-"`
}

func (value AttributableAnswerForProviderAcceptanceRequirementV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Supplied != nil {
		if value.Supplied.Disposition != "supplied" {
			return nil, fmt.Errorf("AttributableAnswerForProviderAcceptanceRequirementV1: selected variant has discriminator %q", value.Supplied.Disposition)
		}
		candidate, err := json.Marshal(value.Supplied)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Omitted != nil {
		if value.Omitted.Disposition != "omitted" {
			return nil, fmt.Errorf("AttributableAnswerForProviderAcceptanceRequirementV1: selected variant has discriminator %q", value.Omitted.Disposition)
		}
		candidate, err := json.Marshal(value.Omitted)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("AttributableAnswerForProviderAcceptanceRequirementV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *AttributableAnswerForProviderAcceptanceRequirementV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Disposition string `json:"disposition"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = AttributableAnswerForProviderAcceptanceRequirementV1{}
	switch discriminator.Disposition {
	case "supplied":
		var variant AttributableAnswerForProviderAcceptanceRequirementSuppliedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Supplied = &variant
	case "omitted":
		var variant AttributableAnswerForProviderAcceptanceRequirementOmittedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Omitted = &variant
	default:
		return fmt.Errorf("AttributableAnswerForProviderAcceptanceRequirementV1: unknown discriminator %q", discriminator.Disposition)
	}
	return nil
}

type AttributableAnswerForProviderCompletionRequirementV1 struct {
	Supplied *AttributableAnswerForProviderCompletionRequirementSuppliedV1 `json:"-"`
	Omitted  *AttributableAnswerForProviderCompletionRequirementOmittedV1  `json:"-"`
}

func (value AttributableAnswerForProviderCompletionRequirementV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Supplied != nil {
		if value.Supplied.Disposition != "supplied" {
			return nil, fmt.Errorf("AttributableAnswerForProviderCompletionRequirementV1: selected variant has discriminator %q", value.Supplied.Disposition)
		}
		candidate, err := json.Marshal(value.Supplied)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Omitted != nil {
		if value.Omitted.Disposition != "omitted" {
			return nil, fmt.Errorf("AttributableAnswerForProviderCompletionRequirementV1: selected variant has discriminator %q", value.Omitted.Disposition)
		}
		candidate, err := json.Marshal(value.Omitted)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("AttributableAnswerForProviderCompletionRequirementV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *AttributableAnswerForProviderCompletionRequirementV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Disposition string `json:"disposition"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = AttributableAnswerForProviderCompletionRequirementV1{}
	switch discriminator.Disposition {
	case "supplied":
		var variant AttributableAnswerForProviderCompletionRequirementSuppliedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Supplied = &variant
	case "omitted":
		var variant AttributableAnswerForProviderCompletionRequirementOmittedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Omitted = &variant
	default:
		return fmt.Errorf("AttributableAnswerForProviderCompletionRequirementV1: unknown discriminator %q", discriminator.Disposition)
	}
	return nil
}

type AttributableAnswerForResponsibilityAdopterModuleV1 struct {
	Supplied *AttributableAnswerForResponsibilityAdopterModuleSuppliedV1 `json:"-"`
	Omitted  *AttributableAnswerForResponsibilityAdopterModuleOmittedV1  `json:"-"`
}

func (value AttributableAnswerForResponsibilityAdopterModuleV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Supplied != nil {
		if value.Supplied.Disposition != "supplied" {
			return nil, fmt.Errorf("AttributableAnswerForResponsibilityAdopterModuleV1: selected variant has discriminator %q", value.Supplied.Disposition)
		}
		candidate, err := json.Marshal(value.Supplied)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Omitted != nil {
		if value.Omitted.Disposition != "omitted" {
			return nil, fmt.Errorf("AttributableAnswerForResponsibilityAdopterModuleV1: selected variant has discriminator %q", value.Omitted.Disposition)
		}
		candidate, err := json.Marshal(value.Omitted)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("AttributableAnswerForResponsibilityAdopterModuleV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *AttributableAnswerForResponsibilityAdopterModuleV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Disposition string `json:"disposition"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = AttributableAnswerForResponsibilityAdopterModuleV1{}
	switch discriminator.Disposition {
	case "supplied":
		var variant AttributableAnswerForResponsibilityAdopterModuleSuppliedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Supplied = &variant
	case "omitted":
		var variant AttributableAnswerForResponsibilityAdopterModuleOmittedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Omitted = &variant
	default:
		return fmt.Errorf("AttributableAnswerForResponsibilityAdopterModuleV1: unknown discriminator %q", discriminator.Disposition)
	}
	return nil
}

type AttributableAnswerForResponsibilityCeilingsV1 struct {
	Supplied *AttributableAnswerForResponsibilityCeilingsSuppliedV1 `json:"-"`
	Omitted  *AttributableAnswerForResponsibilityCeilingsOmittedV1  `json:"-"`
}

func (value AttributableAnswerForResponsibilityCeilingsV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Supplied != nil {
		if value.Supplied.Disposition != "supplied" {
			return nil, fmt.Errorf("AttributableAnswerForResponsibilityCeilingsV1: selected variant has discriminator %q", value.Supplied.Disposition)
		}
		candidate, err := json.Marshal(value.Supplied)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Omitted != nil {
		if value.Omitted.Disposition != "omitted" {
			return nil, fmt.Errorf("AttributableAnswerForResponsibilityCeilingsV1: selected variant has discriminator %q", value.Omitted.Disposition)
		}
		candidate, err := json.Marshal(value.Omitted)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("AttributableAnswerForResponsibilityCeilingsV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *AttributableAnswerForResponsibilityCeilingsV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Disposition string `json:"disposition"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = AttributableAnswerForResponsibilityCeilingsV1{}
	switch discriminator.Disposition {
	case "supplied":
		var variant AttributableAnswerForResponsibilityCeilingsSuppliedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Supplied = &variant
	case "omitted":
		var variant AttributableAnswerForResponsibilityCeilingsOmittedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Omitted = &variant
	default:
		return fmt.Errorf("AttributableAnswerForResponsibilityCeilingsV1: unknown discriminator %q", discriminator.Disposition)
	}
	return nil
}

type AttributableAnswerForResponsibilityDirectPathV1 struct {
	Supplied *AttributableAnswerForResponsibilityDirectPathSuppliedV1 `json:"-"`
	Omitted  *AttributableAnswerForResponsibilityDirectPathOmittedV1  `json:"-"`
}

func (value AttributableAnswerForResponsibilityDirectPathV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Supplied != nil {
		if value.Supplied.Disposition != "supplied" {
			return nil, fmt.Errorf("AttributableAnswerForResponsibilityDirectPathV1: selected variant has discriminator %q", value.Supplied.Disposition)
		}
		candidate, err := json.Marshal(value.Supplied)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Omitted != nil {
		if value.Omitted.Disposition != "omitted" {
			return nil, fmt.Errorf("AttributableAnswerForResponsibilityDirectPathV1: selected variant has discriminator %q", value.Omitted.Disposition)
		}
		candidate, err := json.Marshal(value.Omitted)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("AttributableAnswerForResponsibilityDirectPathV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *AttributableAnswerForResponsibilityDirectPathV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Disposition string `json:"disposition"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = AttributableAnswerForResponsibilityDirectPathV1{}
	switch discriminator.Disposition {
	case "supplied":
		var variant AttributableAnswerForResponsibilityDirectPathSuppliedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Supplied = &variant
	case "omitted":
		var variant AttributableAnswerForResponsibilityDirectPathOmittedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Omitted = &variant
	default:
		return fmt.Errorf("AttributableAnswerForResponsibilityDirectPathV1: unknown discriminator %q", discriminator.Disposition)
	}
	return nil
}

type AttributableAnswerForResponsibilityIncumbentV1 struct {
	Supplied *AttributableAnswerForResponsibilityIncumbentSuppliedV1 `json:"-"`
	Omitted  *AttributableAnswerForResponsibilityIncumbentOmittedV1  `json:"-"`
}

func (value AttributableAnswerForResponsibilityIncumbentV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Supplied != nil {
		if value.Supplied.Disposition != "supplied" {
			return nil, fmt.Errorf("AttributableAnswerForResponsibilityIncumbentV1: selected variant has discriminator %q", value.Supplied.Disposition)
		}
		candidate, err := json.Marshal(value.Supplied)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Omitted != nil {
		if value.Omitted.Disposition != "omitted" {
			return nil, fmt.Errorf("AttributableAnswerForResponsibilityIncumbentV1: selected variant has discriminator %q", value.Omitted.Disposition)
		}
		candidate, err := json.Marshal(value.Omitted)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("AttributableAnswerForResponsibilityIncumbentV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *AttributableAnswerForResponsibilityIncumbentV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Disposition string `json:"disposition"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = AttributableAnswerForResponsibilityIncumbentV1{}
	switch discriminator.Disposition {
	case "supplied":
		var variant AttributableAnswerForResponsibilityIncumbentSuppliedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Supplied = &variant
	case "omitted":
		var variant AttributableAnswerForResponsibilityIncumbentOmittedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Omitted = &variant
	default:
		return fmt.Errorf("AttributableAnswerForResponsibilityIncumbentV1: unknown discriminator %q", discriminator.Disposition)
	}
	return nil
}

type AttributableAnswerForResponsibilityNativeOwnerV1 struct {
	Supplied *AttributableAnswerForResponsibilityNativeOwnerSuppliedV1 `json:"-"`
	Omitted  *AttributableAnswerForResponsibilityNativeOwnerOmittedV1  `json:"-"`
}

func (value AttributableAnswerForResponsibilityNativeOwnerV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Supplied != nil {
		if value.Supplied.Disposition != "supplied" {
			return nil, fmt.Errorf("AttributableAnswerForResponsibilityNativeOwnerV1: selected variant has discriminator %q", value.Supplied.Disposition)
		}
		candidate, err := json.Marshal(value.Supplied)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Omitted != nil {
		if value.Omitted.Disposition != "omitted" {
			return nil, fmt.Errorf("AttributableAnswerForResponsibilityNativeOwnerV1: selected variant has discriminator %q", value.Omitted.Disposition)
		}
		candidate, err := json.Marshal(value.Omitted)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("AttributableAnswerForResponsibilityNativeOwnerV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *AttributableAnswerForResponsibilityNativeOwnerV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Disposition string `json:"disposition"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = AttributableAnswerForResponsibilityNativeOwnerV1{}
	switch discriminator.Disposition {
	case "supplied":
		var variant AttributableAnswerForResponsibilityNativeOwnerSuppliedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Supplied = &variant
	case "omitted":
		var variant AttributableAnswerForResponsibilityNativeOwnerOmittedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Omitted = &variant
	default:
		return fmt.Errorf("AttributableAnswerForResponsibilityNativeOwnerV1: unknown discriminator %q", discriminator.Disposition)
	}
	return nil
}

type AttributableAnswerForResponsibilityPlacementV1 struct {
	Supplied *AttributableAnswerForResponsibilityPlacementSuppliedV1 `json:"-"`
	Omitted  *AttributableAnswerForResponsibilityPlacementOmittedV1  `json:"-"`
}

func (value AttributableAnswerForResponsibilityPlacementV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Supplied != nil {
		if value.Supplied.Disposition != "supplied" {
			return nil, fmt.Errorf("AttributableAnswerForResponsibilityPlacementV1: selected variant has discriminator %q", value.Supplied.Disposition)
		}
		candidate, err := json.Marshal(value.Supplied)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Omitted != nil {
		if value.Omitted.Disposition != "omitted" {
			return nil, fmt.Errorf("AttributableAnswerForResponsibilityPlacementV1: selected variant has discriminator %q", value.Omitted.Disposition)
		}
		candidate, err := json.Marshal(value.Omitted)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("AttributableAnswerForResponsibilityPlacementV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *AttributableAnswerForResponsibilityPlacementV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Disposition string `json:"disposition"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = AttributableAnswerForResponsibilityPlacementV1{}
	switch discriminator.Disposition {
	case "supplied":
		var variant AttributableAnswerForResponsibilityPlacementSuppliedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Supplied = &variant
	case "omitted":
		var variant AttributableAnswerForResponsibilityPlacementOmittedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Omitted = &variant
	default:
		return fmt.Errorf("AttributableAnswerForResponsibilityPlacementV1: unknown discriminator %q", discriminator.Disposition)
	}
	return nil
}

type AttributableAnswerForResponsibilityPublisherModuleV1 struct {
	Supplied *AttributableAnswerForResponsibilityPublisherModuleSuppliedV1 `json:"-"`
	Omitted  *AttributableAnswerForResponsibilityPublisherModuleOmittedV1  `json:"-"`
}

func (value AttributableAnswerForResponsibilityPublisherModuleV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Supplied != nil {
		if value.Supplied.Disposition != "supplied" {
			return nil, fmt.Errorf("AttributableAnswerForResponsibilityPublisherModuleV1: selected variant has discriminator %q", value.Supplied.Disposition)
		}
		candidate, err := json.Marshal(value.Supplied)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Omitted != nil {
		if value.Omitted.Disposition != "omitted" {
			return nil, fmt.Errorf("AttributableAnswerForResponsibilityPublisherModuleV1: selected variant has discriminator %q", value.Omitted.Disposition)
		}
		candidate, err := json.Marshal(value.Omitted)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("AttributableAnswerForResponsibilityPublisherModuleV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *AttributableAnswerForResponsibilityPublisherModuleV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Disposition string `json:"disposition"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = AttributableAnswerForResponsibilityPublisherModuleV1{}
	switch discriminator.Disposition {
	case "supplied":
		var variant AttributableAnswerForResponsibilityPublisherModuleSuppliedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Supplied = &variant
	case "omitted":
		var variant AttributableAnswerForResponsibilityPublisherModuleOmittedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Omitted = &variant
	default:
		return fmt.Errorf("AttributableAnswerForResponsibilityPublisherModuleV1: unknown discriminator %q", discriminator.Disposition)
	}
	return nil
}

type AttributableAnswerForResponsibilityReadinessV1 struct {
	Supplied *AttributableAnswerForResponsibilityReadinessSuppliedV1 `json:"-"`
	Omitted  *AttributableAnswerForResponsibilityReadinessOmittedV1  `json:"-"`
}

func (value AttributableAnswerForResponsibilityReadinessV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Supplied != nil {
		if value.Supplied.Disposition != "supplied" {
			return nil, fmt.Errorf("AttributableAnswerForResponsibilityReadinessV1: selected variant has discriminator %q", value.Supplied.Disposition)
		}
		candidate, err := json.Marshal(value.Supplied)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Omitted != nil {
		if value.Omitted.Disposition != "omitted" {
			return nil, fmt.Errorf("AttributableAnswerForResponsibilityReadinessV1: selected variant has discriminator %q", value.Omitted.Disposition)
		}
		candidate, err := json.Marshal(value.Omitted)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("AttributableAnswerForResponsibilityReadinessV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *AttributableAnswerForResponsibilityReadinessV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Disposition string `json:"disposition"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = AttributableAnswerForResponsibilityReadinessV1{}
	switch discriminator.Disposition {
	case "supplied":
		var variant AttributableAnswerForResponsibilityReadinessSuppliedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Supplied = &variant
	case "omitted":
		var variant AttributableAnswerForResponsibilityReadinessOmittedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Omitted = &variant
	default:
		return fmt.Errorf("AttributableAnswerForResponsibilityReadinessV1: unknown discriminator %q", discriminator.Disposition)
	}
	return nil
}

type AttributableAnswerForResponsibilityRequestFramerV1 struct {
	Supplied *AttributableAnswerForResponsibilityRequestFramerSuppliedV1 `json:"-"`
	Omitted  *AttributableAnswerForResponsibilityRequestFramerOmittedV1  `json:"-"`
}

func (value AttributableAnswerForResponsibilityRequestFramerV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Supplied != nil {
		if value.Supplied.Disposition != "supplied" {
			return nil, fmt.Errorf("AttributableAnswerForResponsibilityRequestFramerV1: selected variant has discriminator %q", value.Supplied.Disposition)
		}
		candidate, err := json.Marshal(value.Supplied)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Omitted != nil {
		if value.Omitted.Disposition != "omitted" {
			return nil, fmt.Errorf("AttributableAnswerForResponsibilityRequestFramerV1: selected variant has discriminator %q", value.Omitted.Disposition)
		}
		candidate, err := json.Marshal(value.Omitted)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("AttributableAnswerForResponsibilityRequestFramerV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *AttributableAnswerForResponsibilityRequestFramerV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Disposition string `json:"disposition"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = AttributableAnswerForResponsibilityRequestFramerV1{}
	switch discriminator.Disposition {
	case "supplied":
		var variant AttributableAnswerForResponsibilityRequestFramerSuppliedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Supplied = &variant
	case "omitted":
		var variant AttributableAnswerForResponsibilityRequestFramerOmittedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Omitted = &variant
	default:
		return fmt.Errorf("AttributableAnswerForResponsibilityRequestFramerV1: unknown discriminator %q", discriminator.Disposition)
	}
	return nil
}

type AttributableAnswerForResponsibilitySafeFailureV1 struct {
	Supplied *AttributableAnswerForResponsibilitySafeFailureSuppliedV1 `json:"-"`
	Omitted  *AttributableAnswerForResponsibilitySafeFailureOmittedV1  `json:"-"`
}

func (value AttributableAnswerForResponsibilitySafeFailureV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Supplied != nil {
		if value.Supplied.Disposition != "supplied" {
			return nil, fmt.Errorf("AttributableAnswerForResponsibilitySafeFailureV1: selected variant has discriminator %q", value.Supplied.Disposition)
		}
		candidate, err := json.Marshal(value.Supplied)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Omitted != nil {
		if value.Omitted.Disposition != "omitted" {
			return nil, fmt.Errorf("AttributableAnswerForResponsibilitySafeFailureV1: selected variant has discriminator %q", value.Omitted.Disposition)
		}
		candidate, err := json.Marshal(value.Omitted)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("AttributableAnswerForResponsibilitySafeFailureV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *AttributableAnswerForResponsibilitySafeFailureV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Disposition string `json:"disposition"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = AttributableAnswerForResponsibilitySafeFailureV1{}
	switch discriminator.Disposition {
	case "supplied":
		var variant AttributableAnswerForResponsibilitySafeFailureSuppliedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Supplied = &variant
	case "omitted":
		var variant AttributableAnswerForResponsibilitySafeFailureOmittedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Omitted = &variant
	default:
		return fmt.Errorf("AttributableAnswerForResponsibilitySafeFailureV1: unknown discriminator %q", discriminator.Disposition)
	}
	return nil
}

type AttributableAnswerForResponsibilitySilenceBoundaryV1 struct {
	Supplied *AttributableAnswerForResponsibilitySilenceBoundarySuppliedV1 `json:"-"`
	Omitted  *AttributableAnswerForResponsibilitySilenceBoundaryOmittedV1  `json:"-"`
}

func (value AttributableAnswerForResponsibilitySilenceBoundaryV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Supplied != nil {
		if value.Supplied.Disposition != "supplied" {
			return nil, fmt.Errorf("AttributableAnswerForResponsibilitySilenceBoundaryV1: selected variant has discriminator %q", value.Supplied.Disposition)
		}
		candidate, err := json.Marshal(value.Supplied)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Omitted != nil {
		if value.Omitted.Disposition != "omitted" {
			return nil, fmt.Errorf("AttributableAnswerForResponsibilitySilenceBoundaryV1: selected variant has discriminator %q", value.Omitted.Disposition)
		}
		candidate, err := json.Marshal(value.Omitted)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("AttributableAnswerForResponsibilitySilenceBoundaryV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *AttributableAnswerForResponsibilitySilenceBoundaryV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Disposition string `json:"disposition"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = AttributableAnswerForResponsibilitySilenceBoundaryV1{}
	switch discriminator.Disposition {
	case "supplied":
		var variant AttributableAnswerForResponsibilitySilenceBoundarySuppliedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Supplied = &variant
	case "omitted":
		var variant AttributableAnswerForResponsibilitySilenceBoundaryOmittedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Omitted = &variant
	default:
		return fmt.Errorf("AttributableAnswerForResponsibilitySilenceBoundaryV1: unknown discriminator %q", discriminator.Disposition)
	}
	return nil
}

type AttributableAnswerForResponsibilityTriggerV1 struct {
	Supplied *AttributableAnswerForResponsibilityTriggerSuppliedV1 `json:"-"`
	Omitted  *AttributableAnswerForResponsibilityTriggerOmittedV1  `json:"-"`
}

func (value AttributableAnswerForResponsibilityTriggerV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Supplied != nil {
		if value.Supplied.Disposition != "supplied" {
			return nil, fmt.Errorf("AttributableAnswerForResponsibilityTriggerV1: selected variant has discriminator %q", value.Supplied.Disposition)
		}
		candidate, err := json.Marshal(value.Supplied)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Omitted != nil {
		if value.Omitted.Disposition != "omitted" {
			return nil, fmt.Errorf("AttributableAnswerForResponsibilityTriggerV1: selected variant has discriminator %q", value.Omitted.Disposition)
		}
		candidate, err := json.Marshal(value.Omitted)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("AttributableAnswerForResponsibilityTriggerV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *AttributableAnswerForResponsibilityTriggerV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Disposition string `json:"disposition"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = AttributableAnswerForResponsibilityTriggerV1{}
	switch discriminator.Disposition {
	case "supplied":
		var variant AttributableAnswerForResponsibilityTriggerSuppliedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Supplied = &variant
	case "omitted":
		var variant AttributableAnswerForResponsibilityTriggerOmittedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Omitted = &variant
	default:
		return fmt.Errorf("AttributableAnswerForResponsibilityTriggerV1: unknown discriminator %q", discriminator.Disposition)
	}
	return nil
}

type AttributableAnswerForWorkspaceResourceID struct {
	Supplied *AttributableAnswerForWorkspaceResourceIDSupplied `json:"-"`
	Omitted  *AttributableAnswerForWorkspaceResourceIDOmitted  `json:"-"`
}

func (value AttributableAnswerForWorkspaceResourceID) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Supplied != nil {
		if value.Supplied.Disposition != "supplied" {
			return nil, fmt.Errorf("AttributableAnswerForWorkspaceResourceID: selected variant has discriminator %q", value.Supplied.Disposition)
		}
		candidate, err := json.Marshal(value.Supplied)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Omitted != nil {
		if value.Omitted.Disposition != "omitted" {
			return nil, fmt.Errorf("AttributableAnswerForWorkspaceResourceID: selected variant has discriminator %q", value.Omitted.Disposition)
		}
		candidate, err := json.Marshal(value.Omitted)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("AttributableAnswerForWorkspaceResourceID: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *AttributableAnswerForWorkspaceResourceID) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Disposition string `json:"disposition"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = AttributableAnswerForWorkspaceResourceID{}
	switch discriminator.Disposition {
	case "supplied":
		var variant AttributableAnswerForWorkspaceResourceIDSupplied
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Supplied = &variant
	case "omitted":
		var variant AttributableAnswerForWorkspaceResourceIDOmitted
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Omitted = &variant
	default:
		return fmt.Errorf("AttributableAnswerForWorkspaceResourceID: unknown discriminator %q", discriminator.Disposition)
	}
	return nil
}

type ResponsibilityPlacementV1 struct {
	VerticalLocal      *ResponsibilityPlacementVerticalLocalV1      `json:"-"`
	DirectionalService *ResponsibilityPlacementDirectionalServiceV1 `json:"-"`
}

func (value ResponsibilityPlacementV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.VerticalLocal != nil {
		if value.VerticalLocal.Kind != "vertical_local" {
			return nil, fmt.Errorf("ResponsibilityPlacementV1: selected variant has discriminator %q", value.VerticalLocal.Kind)
		}
		candidate, err := json.Marshal(value.VerticalLocal)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.DirectionalService != nil {
		if value.DirectionalService.Kind != "directional_service" {
			return nil, fmt.Errorf("ResponsibilityPlacementV1: selected variant has discriminator %q", value.DirectionalService.Kind)
		}
		candidate, err := json.Marshal(value.DirectionalService)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("ResponsibilityPlacementV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *ResponsibilityPlacementV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Kind string `json:"kind"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = ResponsibilityPlacementV1{}
	switch discriminator.Kind {
	case "vertical_local":
		var variant ResponsibilityPlacementVerticalLocalV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.VerticalLocal = &variant
	case "directional_service":
		var variant ResponsibilityPlacementDirectionalServiceV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.DirectionalService = &variant
	default:
		return fmt.Errorf("ResponsibilityPlacementV1: unknown discriminator %q", discriminator.Kind)
	}
	return nil
}

type ResponsibilityTriggerV1 struct {
	RecurringCadence *ResponsibilityTriggerRecurringCadenceV1 `json:"-"`
	OwnerEvent       *ResponsibilityTriggerOwnerEventV1       `json:"-"`
	NativeEvent      *ResponsibilityTriggerNativeEventV1      `json:"-"`
	BoundedRequest   *ResponsibilityTriggerBoundedRequestV1   `json:"-"`
}

func (value ResponsibilityTriggerV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.RecurringCadence != nil {
		if value.RecurringCadence.Kind != "recurring_cadence" {
			return nil, fmt.Errorf("ResponsibilityTriggerV1: selected variant has discriminator %q", value.RecurringCadence.Kind)
		}
		candidate, err := json.Marshal(value.RecurringCadence)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.OwnerEvent != nil {
		if value.OwnerEvent.Kind != "owner_event" {
			return nil, fmt.Errorf("ResponsibilityTriggerV1: selected variant has discriminator %q", value.OwnerEvent.Kind)
		}
		candidate, err := json.Marshal(value.OwnerEvent)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.NativeEvent != nil {
		if value.NativeEvent.Kind != "native_event" {
			return nil, fmt.Errorf("ResponsibilityTriggerV1: selected variant has discriminator %q", value.NativeEvent.Kind)
		}
		candidate, err := json.Marshal(value.NativeEvent)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.BoundedRequest != nil {
		if value.BoundedRequest.Kind != "bounded_request" {
			return nil, fmt.Errorf("ResponsibilityTriggerV1: selected variant has discriminator %q", value.BoundedRequest.Kind)
		}
		candidate, err := json.Marshal(value.BoundedRequest)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("ResponsibilityTriggerV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *ResponsibilityTriggerV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Kind string `json:"kind"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = ResponsibilityTriggerV1{}
	switch discriminator.Kind {
	case "recurring_cadence":
		var variant ResponsibilityTriggerRecurringCadenceV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.RecurringCadence = &variant
	case "owner_event":
		var variant ResponsibilityTriggerOwnerEventV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.OwnerEvent = &variant
	case "native_event":
		var variant ResponsibilityTriggerNativeEventV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.NativeEvent = &variant
	case "bounded_request":
		var variant ResponsibilityTriggerBoundedRequestV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.BoundedRequest = &variant
	default:
		return fmt.Errorf("ResponsibilityTriggerV1: unknown discriminator %q", discriminator.Kind)
	}
	return nil
}
