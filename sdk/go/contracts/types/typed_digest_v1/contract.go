// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package typed_digest_v1 contains the generated typed-digest.v1 data model.
package typed_digest_v1

import (
	"encoding/json"
	"fmt"
	sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"
)

type BoundedResidentTextV1 string

type ResidentRecordRefV1 string

type SchemaID string

type TypedDigestAlgorithmV1 string

const TypedDigestAlgorithmV1Sha256 TypedDigestAlgorithmV1 = "sha256"

type TypedDigestCanonicalizationV1 string

const TypedDigestCanonicalizationV1RawBytes TypedDigestCanonicalizationV1 = "raw_bytes"
const TypedDigestCanonicalizationV1Rfc8785JSON TypedDigestCanonicalizationV1 = "rfc8785_json"

type TypedDigestContentBindingImmutableContentV1 struct {
	// ContentRef corresponds to the JSON schema field "content_ref".
	ContentRef ResidentRecordRefV1 `json:"content_ref"`

	// ContentRefSha256 corresponds to the JSON schema field "content_ref_sha256".
	ContentRefSha256 sha256_digest_v1.Sha256DigestV1 `json:"content_ref_sha256"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type TypedDigestContentBindingInlineBytesBase64V1 struct {
	// InlineBytesBase64 corresponds to the JSON schema field "inline_bytes_base64".
	InlineBytesBase64 string `json:"inline_bytes_base64"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type TypedDigestContentBindingInlineJSONV1 struct {
	// InlineJSON corresponds to the JSON schema field "inline_json".
	InlineJSON any `json:"inline_json"`

	// Kind corresponds to the JSON schema field "kind".
	Kind string `json:"kind"`
}

type TypedDigestV1 struct {
	// Canonicalization corresponds to the JSON schema field "canonicalization".
	Canonicalization TypedDigestCanonicalizationV1 `json:"canonicalization"`

	// ContentBinding corresponds to the JSON schema field "content_binding".
	ContentBinding TypedDigestContentBindingV1 `json:"content_binding"`

	// ContentSchemaID corresponds to the JSON schema field "content_schema_id".
	ContentSchemaID SchemaID `json:"content_schema_id"`

	// ContentSchemaSha256 corresponds to the JSON schema field
	// "content_schema_sha256".
	ContentSchemaSha256 sha256_digest_v1.Sha256DigestV1 `json:"content_schema_sha256"`

	// Digest corresponds to the JSON schema field "digest".
	Digest sha256_digest_v1.Sha256DigestV1 `json:"digest"`

	// DigestAlgorithm corresponds to the JSON schema field "digest_algorithm".
	DigestAlgorithm TypedDigestAlgorithmV1 `json:"digest_algorithm"`

	// Kind corresponds to the JSON schema field "kind".
	Kind BoundedResidentTextV1 `json:"kind"`
}

type TypedDigestContentBindingV1 struct {
	InlineJson        *TypedDigestContentBindingInlineJSONV1        `json:"-"`
	InlineBytesBase64 *TypedDigestContentBindingInlineBytesBase64V1 `json:"-"`
	ImmutableContent  *TypedDigestContentBindingImmutableContentV1  `json:"-"`
}

func (value TypedDigestContentBindingV1) MarshalJSON() ([]byte, error) {
	var encoded []byte
	selected := 0
	if value.InlineJson != nil {
		if value.InlineJson.Kind != "inline_json" {
			return nil, fmt.Errorf("TypedDigestContentBindingV1: selected variant has discriminator %q", value.InlineJson.Kind)
		}
		candidate, err := json.Marshal(value.InlineJson)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.InlineBytesBase64 != nil {
		if value.InlineBytesBase64.Kind != "inline_bytes_base64" {
			return nil, fmt.Errorf("TypedDigestContentBindingV1: selected variant has discriminator %q", value.InlineBytesBase64.Kind)
		}
		candidate, err := json.Marshal(value.InlineBytesBase64)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if value.ImmutableContent != nil {
		if value.ImmutableContent.Kind != "immutable_content" {
			return nil, fmt.Errorf("TypedDigestContentBindingV1: selected variant has discriminator %q", value.ImmutableContent.Kind)
		}
		candidate, err := json.Marshal(value.ImmutableContent)
		if err != nil {
			return nil, err
		}
		encoded = candidate
		selected++
	}
	if selected != 1 {
		return nil, fmt.Errorf("TypedDigestContentBindingV1: expected exactly one variant, got %d", selected)
	}
	return encoded, nil
}

func (value *TypedDigestContentBindingV1) UnmarshalJSON(data []byte) error {
	var discriminator struct {
		Kind string `json:"kind"`
	}
	if err := json.Unmarshal(data, &discriminator); err != nil {
		return err
	}
	*value = TypedDigestContentBindingV1{}
	switch discriminator.Kind {
	case "inline_json":
		var variant TypedDigestContentBindingInlineJSONV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.InlineJson = &variant
	case "inline_bytes_base64":
		var variant TypedDigestContentBindingInlineBytesBase64V1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.InlineBytesBase64 = &variant
	case "immutable_content":
		var variant TypedDigestContentBindingImmutableContentV1
		if err := json.Unmarshal(data, &variant); err != nil {
			return err
		}
		value.ImmutableContent = &variant
	default:
		return fmt.Errorf("TypedDigestContentBindingV1: unknown discriminator %q", discriminator.Kind)
	}
	return nil
}
