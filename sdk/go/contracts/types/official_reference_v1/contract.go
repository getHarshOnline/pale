// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package official_reference_v1 contains the generated official-reference.v1 data model.
package official_reference_v1

type OfficialReferenceV1 struct {
	// Checked corresponds to the JSON schema field "checked".
	Checked OfficialReferenceV1Checked `json:"checked,omitempty,omitzero"`

	// Jurisdiction corresponds to the JSON schema field "jurisdiction".
	Jurisdiction OfficialReferenceV1Jurisdiction `json:"jurisdiction,omitempty,omitzero"`

	// Label corresponds to the JSON schema field "label".
	Label string `json:"label"`

	// URL corresponds to the JSON schema field "url".
	URL OfficialSourceURL `json:"url"`
}

type OfficialReferenceV1Checked *string

type OfficialReferenceV1Jurisdiction *string

type OfficialSourceURL string
