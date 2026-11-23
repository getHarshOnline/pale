// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package genealogy_edge_v1 contains the generated genealogy-edge.v1 data model.
package genealogy_edge_v1

import (
	"encoding/json"
	"fmt"
	sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"
)

type CanonicalSignatureV1 string

type GenealogyConfidenceBasisPointsV1 int

type GenealogyEdgeIDV1 string

type GenealogyEdgeV1 struct {
	// AdmittedAt corresponds to the JSON schema field "admitted_at".
	AdmittedAt UnixTimeV1 `json:"admitted_at"`

	// CanonicalGenealogyEdgeSha256 corresponds to the JSON schema field
	// "canonical_genealogy_edge_sha256".
	CanonicalGenealogyEdgeSha256 sha256_digest_v1.Sha256DigestV1 `json:"canonical_genealogy_edge_sha256"`

	// DeploymentRuntimeAuthorityRef corresponds to the JSON schema field
	// "deployment_runtime_authority_ref".
	DeploymentRuntimeAuthorityRef ResidentRecordRefV1 `json:"deployment_runtime_authority_ref"`

	// DeploymentRuntimeAuthoritySha256 corresponds to the JSON schema field
	// "deployment_runtime_authority_sha256".
	DeploymentRuntimeAuthoritySha256 sha256_digest_v1.Sha256DigestV1 `json:"deployment_runtime_authority_sha256"`

	// DestinationOccurrenceRef corresponds to the JSON schema field
	// "destination_occurrence_ref".
	DestinationOccurrenceRef ResidentRecordRefV1 `json:"destination_occurrence_ref"`

	// DestinationOccurrenceSha256 corresponds to the JSON schema field
	// "destination_occurrence_sha256".
	DestinationOccurrenceSha256 sha256_digest_v1.Sha256DigestV1 `json:"destination_occurrence_sha256"`

	// EdgeID corresponds to the JSON schema field "edge_id".
	EdgeID GenealogyEdgeIDV1 `json:"edge_id"`

	// IssuerKeyEpoch corresponds to the JSON schema field "issuer_key_epoch".
	IssuerKeyEpoch SigningKeyEpochV1 `json:"issuer_key_epoch"`

	// IssuerKeyID corresponds to the JSON schema field "issuer_key_id".
	IssuerKeyID ResidentKeyIDV1 `json:"issuer_key_id"`

	// IssuerPrincipalID corresponds to the JSON schema field "issuer_principal_id".
	IssuerPrincipalID ResidentPrincipalIDV1 `json:"issuer_principal_id"`

	// Proof corresponds to the JSON schema field "proof".
	Proof GenealogyProofV1 `json:"proof"`

	// SchemaID corresponds to the JSON schema field "schema_id".
	SchemaID SchemaID `json:"schema_id"`

	// SchemaSha256 corresponds to the JSON schema field "schema_sha256".
	SchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"schema_sha256"`

	// Signature corresponds to the JSON schema field "signature".
	Signature CanonicalSignatureV1 `json:"signature"`

	// SourceOccurrenceRef corresponds to the JSON schema field
	// "source_occurrence_ref".
	SourceOccurrenceRef ResidentRecordRefV1 `json:"source_occurrence_ref"`

	// SourceOccurrenceSha256 corresponds to the JSON schema field
	// "source_occurrence_sha256".
	SourceOccurrenceSha256 sha256_digest_v1.Sha256DigestV1 `json:"source_occurrence_sha256"`
}

type GenealogyProofNativeStructuralV1 struct {
	// NativeOwnerEvidence corresponds to the JSON schema field
	// "native_owner_evidence".
	NativeOwnerEvidence OwnerRecordCoordinateV1 `json:"native_owner_evidence"`

	// Relation corresponds to the JSON schema field "relation".
	Relation GenealogyRelationV1 `json:"relation"`

	// TruthClass corresponds to the JSON schema field "truth_class".
	TruthClass string `json:"truth_class"`
}

type GenealogyProofPaleConsequenceLineageV1 struct {
	// AdmissionRef corresponds to the JSON schema field "admission_ref".
	AdmissionRef ResidentRecordRefV1 `json:"admission_ref"`

	// AdmissionSha256 corresponds to the JSON schema field "admission_sha256".
	AdmissionSha256 sha256_digest_v1.Sha256DigestV1 `json:"admission_sha256"`

	// DestinationOwnerReceiptRef corresponds to the JSON schema field
	// "destination_owner_receipt_ref".
	DestinationOwnerReceiptRef ResidentRecordRefV1 `json:"destination_owner_receipt_ref"`

	// DestinationOwnerReceiptSha256 corresponds to the JSON schema field
	// "destination_owner_receipt_sha256".
	DestinationOwnerReceiptSha256 sha256_digest_v1.Sha256DigestV1 `json:"destination_owner_receipt_sha256"`

	// Relation corresponds to the JSON schema field "relation".
	Relation GenealogyRelationV1 `json:"relation"`

	// TruthClass corresponds to the JSON schema field "truth_class".
	TruthClass string `json:"truth_class"`
}

type GenealogyProofSemanticInferredV1 struct {
	// ConfidenceBasisPoints corresponds to the JSON schema field
	// "confidence_basis_points".
	ConfidenceBasisPoints GenealogyConfidenceBasisPointsV1 `json:"confidence_basis_points"`

	// EvidenceRootSha256 corresponds to the JSON schema field "evidence_root_sha256".
	EvidenceRootSha256 sha256_digest_v1.Sha256DigestV1 `json:"evidence_root_sha256"`

	// OccupantRef corresponds to the JSON schema field "occupant_ref".
	OccupantRef ResidentRecordRefV1 `json:"occupant_ref"`

	// OccupantSha256 corresponds to the JSON schema field "occupant_sha256".
	OccupantSha256 sha256_digest_v1.Sha256DigestV1 `json:"occupant_sha256"`

	// ProfileRef corresponds to the JSON schema field "profile_ref".
	ProfileRef ResidentRecordRefV1 `json:"profile_ref"`

	// ProfileSha256 corresponds to the JSON schema field "profile_sha256".
	ProfileSha256 sha256_digest_v1.Sha256DigestV1 `json:"profile_sha256"`

	// Relation corresponds to the JSON schema field "relation".
	Relation GenealogyRelationV1 `json:"relation"`

	// ScopeTransactionRef corresponds to the JSON schema field
	// "scope_transaction_ref".
	ScopeTransactionRef ResidentRecordRefV1 `json:"scope_transaction_ref"`

	// ScopeTransactionSha256 corresponds to the JSON schema field
	// "scope_transaction_sha256".
	ScopeTransactionSha256 sha256_digest_v1.Sha256DigestV1 `json:"scope_transaction_sha256"`

	// TruthClass corresponds to the JSON schema field "truth_class".
	TruthClass string `json:"truth_class"`

	// Uncertainty corresponds to the JSON schema field "uncertainty".
	Uncertainty SemanticUncertaintyV1 `json:"uncertainty"`
}

type GenealogyRelationV1 string

type NativeRevisionOwnerV1 string

type NativeRevisionTokenV1 string

type NativeRevisionV1 struct {
	// NativeRevisionOwner corresponds to the JSON schema field
	// "native_revision_owner".
	NativeRevisionOwner NativeRevisionOwnerV1 `json:"native_revision_owner"`

	// NativeRevisionToken corresponds to the JSON schema field
	// "native_revision_token".
	NativeRevisionToken NativeRevisionTokenV1 `json:"native_revision_token"`
}

type OwnerRecordCoordinateV1 struct {
	// NativeRevision corresponds to the JSON schema field "native_revision".
	NativeRevision NativeRevisionV1 `json:"native_revision"`

	// OwnerSystemID corresponds to the JSON schema field "owner_system_id".
	OwnerSystemID OwnerSystemIDV1 `json:"owner_system_id"`

	// RecordContentSha256 corresponds to the JSON schema field
	// "record_content_sha256".
	RecordContentSha256 sha256_digest_v1.Sha256DigestV1 `json:"record_content_sha256"`

	// RecordRef corresponds to the JSON schema field "record_ref".
	RecordRef ResidentRecordRefV1 `json:"record_ref"`

	// RecordSchemaID corresponds to the JSON schema field "record_schema_id".
	RecordSchemaID SchemaID `json:"record_schema_id"`

	// RecordSchemaSha256 corresponds to the JSON schema field "record_schema_sha256".
	RecordSchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"record_schema_sha256"`
}

type OwnerSystemIDV1 string

type ResidentKeyIDV1 string

type ResidentPrincipalIDV1 string

type ResidentRecordRefV1 string

type SchemaID string

type SemanticUncertaintyV1 string

type SigningKeyEpochV1 int

type UnixTimeV1 int

type GenealogyProofV1 struct {
	NativeStructural       *GenealogyProofNativeStructuralV1       `json:"-"`
	PaleConsequenceLineage *GenealogyProofPaleConsequenceLineageV1 `json:"-"`
	SemanticInferred       *GenealogyProofSemanticInferredV1       `json:"-"`
}

func (value GenealogyProofV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.NativeStructural != nil {
		if value.NativeStructural.TruthClass != "native_structural" {
			return nil, fmt.Errorf("GenealogyProofV1: selected variant has discriminator %q", value.NativeStructural.TruthClass)
		}
		candidate, err := json.Marshal(value.NativeStructural)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.PaleConsequenceLineage != nil {
		if value.PaleConsequenceLineage.TruthClass != "pale_consequence_lineage" {
			return nil, fmt.Errorf("GenealogyProofV1: selected variant has discriminator %q", value.PaleConsequenceLineage.TruthClass)
		}
		candidate, err := json.Marshal(value.PaleConsequenceLineage)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.SemanticInferred != nil {
		if value.SemanticInferred.TruthClass != "semantic_inferred" {
			return nil, fmt.Errorf("GenealogyProofV1: selected variant has discriminator %q", value.SemanticInferred.TruthClass)
		}
		candidate, err := json.Marshal(value.SemanticInferred)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("GenealogyProofV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *GenealogyProofV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		TruthClass string `json:"truth_class"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = GenealogyProofV1{}
	switch discriminator.TruthClass {
	case "native_structural":
		var variant GenealogyProofNativeStructuralV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.NativeStructural = &variant
	case "pale_consequence_lineage":
		var variant GenealogyProofPaleConsequenceLineageV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.PaleConsequenceLineage = &variant
	case "semantic_inferred":
		var variant GenealogyProofSemanticInferredV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.SemanticInferred = &variant
	default:
		return fmt.Errorf("GenealogyProofV1: unknown discriminator %q", discriminator.TruthClass)
	}
	return nil
}
