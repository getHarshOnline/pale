// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package resource_continuity_profile_v1 contains the generated resource-continuity-profile.v1 data model.
package resource_continuity_profile_v1

import (
	"encoding/json"
	"fmt"
	sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"
)

type AuthorityEvidenceV1 struct {
	// AuthorityRef corresponds to the JSON schema field "authority_ref".
	AuthorityRef ResidentRecordRefV1 `json:"authority_ref"`

	// AuthoritySha256 corresponds to the JSON schema field "authority_sha256".
	AuthoritySha256 sha256_digest_v1.Sha256DigestV1 `json:"authority_sha256"`
}

type ContinuityAuthorityBindingV1 struct {
	// Authority corresponds to the JSON schema field "authority".
	Authority AuthorityEvidenceV1 `json:"authority"`

	// OwnerSystemID corresponds to the JSON schema field "owner_system_id".
	OwnerSystemID OwnerSystemIDV1 `json:"owner_system_id"`
}

type ContinuityControllerBindingV1 struct {
	// Authority corresponds to the JSON schema field "authority".
	Authority ContinuityRecordCoordinateV1 `json:"authority"`

	// ControllerID corresponds to the JSON schema field "controller_id".
	ControllerID ResidentControllerIDV1 `json:"controller_id"`
}

type ContinuityHolderDeviceV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Scope corresponds to the JSON schema field "scope".
	Scope ResourceContinuityDeviceScopeIDV1 `json:"scope"`
}

type ContinuityHolderLocationV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Scope corresponds to the JSON schema field "scope".
	Scope ResidentRecordRefV1 `json:"scope"`
}

type ContinuityHolderMachineV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Scope corresponds to the JSON schema field "scope".
	Scope ResourceContinuityMachineScopeIDV1 `json:"scope"`
}

type ContinuityHolderServiceV1 struct {
	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`

	// Scope corresponds to the JSON schema field "scope".
	Scope ResidentRecordRefV1 `json:"scope"`
}

type ContinuityOptionalContractAbsentV1 struct {
	// State corresponds to the JSON schema field "state".
	State string `json:"state"`
}

type ContinuityOptionalContractBoundV1 struct {
	// Contract corresponds to the JSON schema field "contract".
	Contract ContinuityRecordCoordinateV1 `json:"contract"`

	// State corresponds to the JSON schema field "state".
	State string `json:"state"`
}

type ContinuityRecordCoordinateV1 struct {
	// RecordRef corresponds to the JSON schema field "record_ref".
	RecordRef ResidentRecordRefV1 `json:"record_ref"`

	// RecordSha256 corresponds to the JSON schema field "record_sha256".
	RecordSha256 sha256_digest_v1.Sha256DigestV1 `json:"record_sha256"`
}

type OwnerSystemIDV1 string

type RecoveryPointObjectiveSecondsV1 int

type RecoveryTimeObjectiveSecondsV1 int

type ResidentControllerIDV1 string

type ResidentPrincipalIDV1 string

type ResidentRecordRefV1 string

type ResourceContinuityAccessRoleV1 string

const ResourceContinuityAccessRoleV1EffectWriter ResourceContinuityAccessRoleV1 = "effect_writer"
const ResourceContinuityAccessRoleV1ProposalWriter ResourceContinuityAccessRoleV1 = "proposal_writer"
const ResourceContinuityAccessRoleV1Reader ResourceContinuityAccessRoleV1 = "reader"

type ResourceContinuityAccessV1 struct {
	// AuthorizedControllers corresponds to the JSON schema field
	// "authorized_controllers".
	AuthorizedControllers ResourceContinuityControllerSetV1 `json:"authorized_controllers"`

	// AuthorizedPrincipals corresponds to the JSON schema field
	// "authorized_principals".
	AuthorizedPrincipals ResourceContinuityPrincipalSetV1 `json:"authorized_principals"`

	// Grants corresponds to the JSON schema field "grants".
	Grants ResourceContinuityGrantSetV1 `json:"grants"`
}

type ResourceContinuityAuthorityV1 struct {
	// AuthoritativeSource corresponds to the JSON schema field
	// "authoritative_source".
	AuthoritativeSource ContinuityAuthorityBindingV1 `json:"authoritative_source"`

	// AuthorityModel corresponds to the JSON schema field "authority_model".
	AuthorityModel ContinuityRecordCoordinateV1 `json:"authority_model"`

	// Controller corresponds to the JSON schema field "controller".
	Controller ContinuityControllerBindingV1 `json:"controller"`

	// EffectAuthority corresponds to the JSON schema field "effect_authority".
	EffectAuthority ContinuityAuthorityBindingV1 `json:"effect_authority"`

	// NativeOwner corresponds to the JSON schema field "native_owner".
	NativeOwner OwnerSystemIDV1 `json:"native_owner"`
}

type ResourceContinuityControllerSetV1 []ResidentControllerIDV1

type ResourceContinuityCopyClassV1 string

const ResourceContinuityCopyClassV1Archive ResourceContinuityCopyClassV1 = "archive"
const ResourceContinuityCopyClassV1Cache ResourceContinuityCopyClassV1 = "cache"
const ResourceContinuityCopyClassV1LocalReplica ResourceContinuityCopyClassV1 = "local_replica"
const ResourceContinuityCopyClassV1RemoteReplica ResourceContinuityCopyClassV1 = "remote_replica"
const ResourceContinuityCopyClassV1View ResourceContinuityCopyClassV1 = "view"

type ResourceContinuityCopyIDV1 string

type ResourceContinuityCopyPolicyV1 struct {
	// FrontierContract corresponds to the JSON schema field "frontier_contract".
	FrontierContract ContinuityRecordCoordinateV1 `json:"frontier_contract"`

	// RetentionContract corresponds to the JSON schema field "retention_contract".
	RetentionContract ContinuityRecordCoordinateV1 `json:"retention_contract"`

	// RevocationContract corresponds to the JSON schema field "revocation_contract".
	RevocationContract ContinuityRecordCoordinateV1 `json:"revocation_contract"`

	// Scope corresponds to the JSON schema field "scope".
	Scope ContinuityRecordCoordinateV1 `json:"scope"`
}

type ResourceContinuityCopySetV1 []ResourceContinuityCopyV1

type ResourceContinuityCopyV1 struct {
	// Class corresponds to the JSON schema field "class".
	Class ResourceContinuityCopyClassV1 `json:"class"`

	// CopyID corresponds to the JSON schema field "copy_id".
	CopyID ResourceContinuityCopyIDV1 `json:"copy_id"`

	// Custodian corresponds to the JSON schema field "custodian".
	Custodian ContinuityAuthorityBindingV1 `json:"custodian"`

	// Holder corresponds to the JSON schema field "holder".
	Holder ContinuityHolderV1 `json:"holder"`

	// Policy corresponds to the JSON schema field "policy".
	Policy ResourceContinuityCopyPolicyV1 `json:"policy"`
}

type ResourceContinuityCutoverV1 struct {
	// CutoverState corresponds to the JSON schema field "cutover_state".
	CutoverState ContinuityRecordCoordinateV1 `json:"cutover_state"`

	// RollbackContract corresponds to the JSON schema field "rollback_contract".
	RollbackContract ContinuityRecordCoordinateV1 `json:"rollback_contract"`
}

type ResourceContinuityDeviceScopeIDV1 string

type ResourceContinuityDeviceScopeSetV1 []ResourceContinuityDeviceScopeIDV1

type ResourceContinuityEvidenceV1 struct {
	// EvidenceDestination corresponds to the JSON schema field
	// "evidence_destination".
	EvidenceDestination ContinuityRecordCoordinateV1 `json:"evidence_destination"`

	// EvidenceOwner corresponds to the JSON schema field "evidence_owner".
	EvidenceOwner ContinuityAuthorityBindingV1 `json:"evidence_owner"`

	// ReadBackContract corresponds to the JSON schema field "read_back_contract".
	ReadBackContract ContinuityRecordCoordinateV1 `json:"read_back_contract"`

	// ReadBackOwner corresponds to the JSON schema field "read_back_owner".
	ReadBackOwner ContinuityAuthorityBindingV1 `json:"read_back_owner"`
}

type ResourceContinuityGovernanceV1 struct {
	// DeletionPolicy corresponds to the JSON schema field "deletion_policy".
	DeletionPolicy ContinuityRecordCoordinateV1 `json:"deletion_policy"`

	// LegalHoldPolicy corresponds to the JSON schema field "legal_hold_policy".
	LegalHoldPolicy ContinuityRecordCoordinateV1 `json:"legal_hold_policy"`

	// PrivacyPolicy corresponds to the JSON schema field "privacy_policy".
	PrivacyPolicy ContinuityRecordCoordinateV1 `json:"privacy_policy"`

	// ResidencyPolicy corresponds to the JSON schema field "residency_policy".
	ResidencyPolicy ContinuityRecordCoordinateV1 `json:"residency_policy"`

	// RetentionPolicy corresponds to the JSON schema field "retention_policy".
	RetentionPolicy ContinuityRecordCoordinateV1 `json:"retention_policy"`

	// RevocationPolicy corresponds to the JSON schema field "revocation_policy".
	RevocationPolicy ContinuityRecordCoordinateV1 `json:"revocation_policy"`
}

type ResourceContinuityGrantSetV1 []ResourceContinuityGrantV1

type ResourceContinuityGrantV1 struct {
	// Authority corresponds to the JSON schema field "authority".
	Authority ContinuityRecordCoordinateV1 `json:"authority"`

	// Credential corresponds to the JSON schema field "credential".
	Credential ContinuityOptionalContractV1 `json:"credential"`

	// Role corresponds to the JSON schema field "role".
	Role ResourceContinuityAccessRoleV1 `json:"role"`

	// Subject corresponds to the JSON schema field "subject".
	Subject ResourceContinuitySubjectV1 `json:"subject"`

	// Transport corresponds to the JSON schema field "transport".
	Transport ContinuityOptionalContractV1 `json:"transport"`
}

type ResourceContinuityMachineScopeIDV1 string

type ResourceContinuityMachineScopeSetV1 []ResourceContinuityMachineScopeIDV1

type ResourceContinuityMechanismDispositionForbiddenV1 struct {
	// State corresponds to the JSON schema field "state".
	State string `json:"state"`
}

type ResourceContinuityMechanismDispositionGovernedV1 struct {
	// Contract corresponds to the JSON schema field "contract".
	Contract ContinuityRecordCoordinateV1 `json:"contract"`

	// State corresponds to the JSON schema field "state".
	State string `json:"state"`
}

type ResourceContinuityMechanismKindV1 string

const ResourceContinuityMechanismKindV1Archive ResourceContinuityMechanismKindV1 = "archive"
const ResourceContinuityMechanismKindV1BackupRestore ResourceContinuityMechanismKindV1 = "backup_restore"
const ResourceContinuityMechanismKindV1CheckpointReplay ResourceContinuityMechanismKindV1 = "checkpoint_replay"
const ResourceContinuityMechanismKindV1ExportImport ResourceContinuityMechanismKindV1 = "export_import"
const ResourceContinuityMechanismKindV1MigrationCutover ResourceContinuityMechanismKindV1 = "migration_cutover"
const ResourceContinuityMechanismKindV1ProjectionRegeneration ResourceContinuityMechanismKindV1 = "projection_regeneration"
const ResourceContinuityMechanismKindV1Recovery ResourceContinuityMechanismKindV1 = "recovery"
const ResourceContinuityMechanismKindV1Replication ResourceContinuityMechanismKindV1 = "replication"
const ResourceContinuityMechanismKindV1Sync ResourceContinuityMechanismKindV1 = "sync"
const ResourceContinuityMechanismKindV1Transport ResourceContinuityMechanismKindV1 = "transport"

type ResourceContinuityMechanismPolicyV1 struct {
	// Disposition corresponds to the JSON schema field "disposition".
	Disposition ResourceContinuityMechanismDispositionV1 `json:"disposition"`

	// Kind corresponds to the JSON schema field "kind".
	Kind ResourceContinuityMechanismKindV1 `json:"kind"`
}

type ResourceContinuityMechanismTableV1 []ResourceContinuityMechanismPolicyV1

type ResourceContinuityPrincipalSetV1 []ResidentPrincipalIDV1

type ResourceContinuityProfileIDV1 string

type ResourceContinuityProfileV1 struct {
	// Access corresponds to the JSON schema field "access".
	Access ResourceContinuityAccessV1 `json:"access"`

	// Authority corresponds to the JSON schema field "authority".
	Authority ResourceContinuityAuthorityV1 `json:"authority"`

	// Convergence corresponds to the JSON schema field "convergence".
	Convergence ResourceConvergenceV1 `json:"convergence"`

	// Copies corresponds to the JSON schema field "copies".
	Copies ResourceContinuityCopySetV1 `json:"copies"`

	// Cutover corresponds to the JSON schema field "cutover".
	Cutover ResourceContinuityCutoverV1 `json:"cutover"`

	// Evidence corresponds to the JSON schema field "evidence".
	Evidence ResourceContinuityEvidenceV1 `json:"evidence"`

	// Governance corresponds to the JSON schema field "governance".
	Governance ResourceContinuityGovernanceV1 `json:"governance"`

	// Mechanisms corresponds to the JSON schema field "mechanisms".
	Mechanisms ResourceContinuityMechanismTableV1 `json:"mechanisms"`

	// ProfileID corresponds to the JSON schema field "profile_id".
	ProfileID ResourceContinuityProfileIDV1 `json:"profile_id"`

	// Recovery corresponds to the JSON schema field "recovery".
	Recovery ResourceContinuityRecoveryV1 `json:"recovery"`

	// ResourceContinuityProfileDigest corresponds to the JSON schema field
	// "resource_continuity_profile_digest".
	ResourceContinuityProfileDigest sha256_digest_v1.Sha256DigestV1 `json:"resource_continuity_profile_digest"`

	// ResourceID corresponds to the JSON schema field "resource_id".
	ResourceID WorkspaceResourceID `json:"resource_id"`

	// Schema corresponds to the JSON schema field "schema".
	Schema SchemaID `json:"schema"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// SchemaVersion corresponds to the JSON schema field "schema_version".
	SchemaVersion int `json:"schema_version"`

	// Scope corresponds to the JSON schema field "scope".
	Scope ResourceContinuityScopeV1 `json:"scope"`

	// SessionRelation corresponds to the JSON schema field "session_relation".
	SessionRelation ResourceSessionContinuityV1 `json:"session_relation"`
}

type ResourceContinuityRecoveryV1 struct {
	// RecoveryPointObjectiveSeconds corresponds to the JSON schema field
	// "recovery_point_objective_seconds".
	RecoveryPointObjectiveSeconds RecoveryPointObjectiveSecondsV1 `json:"recovery_point_objective_seconds"`

	// RecoveryTimeObjectiveSeconds corresponds to the JSON schema field
	// "recovery_time_objective_seconds".
	RecoveryTimeObjectiveSeconds RecoveryTimeObjectiveSecondsV1 `json:"recovery_time_objective_seconds"`

	// VerificationContract corresponds to the JSON schema field
	// "verification_contract".
	VerificationContract ContinuityRecordCoordinateV1 `json:"verification_contract"`
}

type ResourceContinuityScopeV1 struct {
	// BootstrapVerification corresponds to the JSON schema field
	// "bootstrap_verification".
	BootstrapVerification ContinuityRecordCoordinateV1 `json:"bootstrap_verification"`

	// CrossMachineBootstrap corresponds to the JSON schema field
	// "cross_machine_bootstrap".
	CrossMachineBootstrap ContinuityRecordCoordinateV1 `json:"cross_machine_bootstrap"`

	// Devices corresponds to the JSON schema field "devices".
	Devices ResourceContinuityDeviceScopeSetV1 `json:"devices"`

	// Machines corresponds to the JSON schema field "machines".
	Machines ResourceContinuityMachineScopeSetV1 `json:"machines"`
}

type ResourceContinuitySubjectControllerV1 struct {
	// Identity corresponds to the JSON schema field "identity".
	Identity ResidentControllerIDV1 `json:"identity"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type ResourceContinuitySubjectPrincipalV1 struct {
	// Identity corresponds to the JSON schema field "identity".
	Identity ResidentPrincipalIDV1 `json:"identity"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type ResourceContinuityWriterV1 struct {
	// Authority corresponds to the JSON schema field "authority".
	Authority ContinuityRecordCoordinateV1 `json:"authority"`

	// Subject corresponds to the JSON schema field "subject".
	Subject ResourceContinuitySubjectV1 `json:"subject"`
}

type ResourceConvergenceExclusiveWriterV1 struct {
	// Mode corresponds to the JSON schema field "mode".
	Mode string `json:"mode"`

	// Writer corresponds to the JSON schema field "writer".
	Writer ResourceContinuityWriterV1 `json:"writer"`
}

type ResourceConvergenceMergeProtocolV1 struct {
	// ConflictOwner corresponds to the JSON schema field "conflict_owner".
	ConflictOwner ContinuityAuthorityBindingV1 `json:"conflict_owner"`

	// DurableMetadata corresponds to the JSON schema field "durable_metadata".
	DurableMetadata ResourceConvergenceMetadataSetV1 `json:"durable_metadata"`

	// MergeOwner corresponds to the JSON schema field "merge_owner".
	MergeOwner ContinuityAuthorityBindingV1 `json:"merge_owner"`

	// Mode corresponds to the JSON schema field "mode".
	Mode string `json:"mode"`

	// Protocol corresponds to the JSON schema field "protocol".
	Protocol ContinuityRecordCoordinateV1 `json:"protocol"`
}

type ResourceConvergenceMetadataSetV1 []ResourceConvergenceMetadataV1

type ResourceConvergenceMetadataV1 string

const ResourceConvergenceMetadataV1CausalClock ResourceConvergenceMetadataV1 = "causal_clock"
const ResourceConvergenceMetadataV1ConflictMarker ResourceConvergenceMetadataV1 = "conflict_marker"
const ResourceConvergenceMetadataV1MergeAncestry ResourceConvergenceMetadataV1 = "merge_ancestry"
const ResourceConvergenceMetadataV1PerFieldOrigin ResourceConvergenceMetadataV1 = "per_field_origin"
const ResourceConvergenceMetadataV1Tombstone ResourceConvergenceMetadataV1 = "tombstone"
const ResourceConvergenceMetadataV1VersionVector ResourceConvergenceMetadataV1 = "version_vector"
const ResourceConvergenceMetadataV1WriterClock ResourceConvergenceMetadataV1 = "writer_clock"

type ResourceSessionContinuityBoundV1 struct {
	// RelationContract corresponds to the JSON schema field "relation_contract".
	RelationContract ContinuityRecordCoordinateV1 `json:"relation_contract"`

	// State corresponds to the JSON schema field "state".
	State string `json:"state"`
}

type ResourceSessionContinuityUnrelatedV1 struct {
	// State corresponds to the JSON schema field "state".
	State string `json:"state"`
}

type SchemaID string

type WorkspaceResourceID string

type ContinuityHolderV1 struct {
	Device   *ContinuityHolderDeviceV1   `json:"-"`
	Machine  *ContinuityHolderMachineV1  `json:"-"`
	Service  *ContinuityHolderServiceV1  `json:"-"`
	Location *ContinuityHolderLocationV1 `json:"-"`
}

func (value ContinuityHolderV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Device != nil {
		if value.Device.Kind != "device" {
			return nil, fmt.Errorf("ContinuityHolderV1: selected variant has discriminator %q", value.Device.Kind)
		}
		candidate, err := json.Marshal(value.Device)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Machine != nil {
		if value.Machine.Kind != "machine" {
			return nil, fmt.Errorf("ContinuityHolderV1: selected variant has discriminator %q", value.Machine.Kind)
		}
		candidate, err := json.Marshal(value.Machine)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Service != nil {
		if value.Service.Kind != "service" {
			return nil, fmt.Errorf("ContinuityHolderV1: selected variant has discriminator %q", value.Service.Kind)
		}
		candidate, err := json.Marshal(value.Service)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Location != nil {
		if value.Location.Kind != "location" {
			return nil, fmt.Errorf("ContinuityHolderV1: selected variant has discriminator %q", value.Location.Kind)
		}
		candidate, err := json.Marshal(value.Location)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("ContinuityHolderV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *ContinuityHolderV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Kind string `json:"kind"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = ContinuityHolderV1{}
	switch discriminator.Kind {
	case "device":
		var variant ContinuityHolderDeviceV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Device = &variant
	case "machine":
		var variant ContinuityHolderMachineV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Machine = &variant
	case "service":
		var variant ContinuityHolderServiceV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Service = &variant
	case "location":
		var variant ContinuityHolderLocationV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Location = &variant
	default:
		return fmt.Errorf("ContinuityHolderV1: unknown discriminator %q", discriminator.Kind)
	}
	return nil
}

type ContinuityOptionalContractV1 struct {
	Absent *ContinuityOptionalContractAbsentV1 `json:"-"`
	Bound  *ContinuityOptionalContractBoundV1  `json:"-"`
}

func (value ContinuityOptionalContractV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Absent != nil {
		if value.Absent.State != "absent" {
			return nil, fmt.Errorf("ContinuityOptionalContractV1: selected variant has discriminator %q", value.Absent.State)
		}
		candidate, err := json.Marshal(value.Absent)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Bound != nil {
		if value.Bound.State != "bound" {
			return nil, fmt.Errorf("ContinuityOptionalContractV1: selected variant has discriminator %q", value.Bound.State)
		}
		candidate, err := json.Marshal(value.Bound)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("ContinuityOptionalContractV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *ContinuityOptionalContractV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		State string `json:"state"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = ContinuityOptionalContractV1{}
	switch discriminator.State {
	case "absent":
		var variant ContinuityOptionalContractAbsentV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Absent = &variant
	case "bound":
		var variant ContinuityOptionalContractBoundV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Bound = &variant
	default:
		return fmt.Errorf("ContinuityOptionalContractV1: unknown discriminator %q", discriminator.State)
	}
	return nil
}

type ResourceContinuityMechanismDispositionV1 struct {
	Forbidden *ResourceContinuityMechanismDispositionForbiddenV1 `json:"-"`
	Governed  *ResourceContinuityMechanismDispositionGovernedV1  `json:"-"`
}

func (value ResourceContinuityMechanismDispositionV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Forbidden != nil {
		if value.Forbidden.State != "forbidden" {
			return nil, fmt.Errorf("ResourceContinuityMechanismDispositionV1: selected variant has discriminator %q", value.Forbidden.State)
		}
		candidate, err := json.Marshal(value.Forbidden)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Governed != nil {
		if value.Governed.State != "governed" {
			return nil, fmt.Errorf("ResourceContinuityMechanismDispositionV1: selected variant has discriminator %q", value.Governed.State)
		}
		candidate, err := json.Marshal(value.Governed)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("ResourceContinuityMechanismDispositionV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *ResourceContinuityMechanismDispositionV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		State string `json:"state"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = ResourceContinuityMechanismDispositionV1{}
	switch discriminator.State {
	case "forbidden":
		var variant ResourceContinuityMechanismDispositionForbiddenV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Forbidden = &variant
	case "governed":
		var variant ResourceContinuityMechanismDispositionGovernedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Governed = &variant
	default:
		return fmt.Errorf("ResourceContinuityMechanismDispositionV1: unknown discriminator %q", discriminator.State)
	}
	return nil
}

type ResourceContinuitySubjectV1 struct {
	Principal  *ResourceContinuitySubjectPrincipalV1  `json:"-"`
	Controller *ResourceContinuitySubjectControllerV1 `json:"-"`
}

func (value ResourceContinuitySubjectV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Principal != nil {
		if value.Principal.Kind != "principal" {
			return nil, fmt.Errorf("ResourceContinuitySubjectV1: selected variant has discriminator %q", value.Principal.Kind)
		}
		candidate, err := json.Marshal(value.Principal)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Controller != nil {
		if value.Controller.Kind != "controller" {
			return nil, fmt.Errorf("ResourceContinuitySubjectV1: selected variant has discriminator %q", value.Controller.Kind)
		}
		candidate, err := json.Marshal(value.Controller)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("ResourceContinuitySubjectV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *ResourceContinuitySubjectV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Kind string `json:"kind"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = ResourceContinuitySubjectV1{}
	switch discriminator.Kind {
	case "principal":
		var variant ResourceContinuitySubjectPrincipalV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Principal = &variant
	case "controller":
		var variant ResourceContinuitySubjectControllerV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Controller = &variant
	default:
		return fmt.Errorf("ResourceContinuitySubjectV1: unknown discriminator %q", discriminator.Kind)
	}
	return nil
}

type ResourceConvergenceV1 struct {
	ExclusiveWriter *ResourceConvergenceExclusiveWriterV1 `json:"-"`
	MergeProtocol   *ResourceConvergenceMergeProtocolV1   `json:"-"`
}

func (value ResourceConvergenceV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.ExclusiveWriter != nil {
		if value.ExclusiveWriter.Mode != "exclusive_writer" {
			return nil, fmt.Errorf("ResourceConvergenceV1: selected variant has discriminator %q", value.ExclusiveWriter.Mode)
		}
		candidate, err := json.Marshal(value.ExclusiveWriter)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.MergeProtocol != nil {
		if value.MergeProtocol.Mode != "merge_protocol" {
			return nil, fmt.Errorf("ResourceConvergenceV1: selected variant has discriminator %q", value.MergeProtocol.Mode)
		}
		candidate, err := json.Marshal(value.MergeProtocol)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("ResourceConvergenceV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *ResourceConvergenceV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Mode string `json:"mode"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = ResourceConvergenceV1{}
	switch discriminator.Mode {
	case "exclusive_writer":
		var variant ResourceConvergenceExclusiveWriterV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.ExclusiveWriter = &variant
	case "merge_protocol":
		var variant ResourceConvergenceMergeProtocolV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.MergeProtocol = &variant
	default:
		return fmt.Errorf("ResourceConvergenceV1: unknown discriminator %q", discriminator.Mode)
	}
	return nil
}

type ResourceSessionContinuityV1 struct {
	Unrelated *ResourceSessionContinuityUnrelatedV1 `json:"-"`
	Bound     *ResourceSessionContinuityBoundV1     `json:"-"`
}

func (value ResourceSessionContinuityV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.Unrelated != nil {
		if value.Unrelated.State != "unrelated" {
			return nil, fmt.Errorf("ResourceSessionContinuityV1: selected variant has discriminator %q", value.Unrelated.State)
		}
		candidate, err := json.Marshal(value.Unrelated)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.Bound != nil {
		if value.Bound.State != "bound" {
			return nil, fmt.Errorf("ResourceSessionContinuityV1: selected variant has discriminator %q", value.Bound.State)
		}
		candidate, err := json.Marshal(value.Bound)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("ResourceSessionContinuityV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *ResourceSessionContinuityV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		State string `json:"state"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = ResourceSessionContinuityV1{}
	switch discriminator.State {
	case "unrelated":
		var variant ResourceSessionContinuityUnrelatedV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Unrelated = &variant
	case "bound":
		var variant ResourceSessionContinuityBoundV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.Bound = &variant
	default:
		return fmt.Errorf("ResourceSessionContinuityV1: unknown discriminator %q", discriminator.State)
	}
	return nil
}
