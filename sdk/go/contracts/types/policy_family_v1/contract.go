// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package policy_family_v1 contains the generated policy-family.v1 data model.
package policy_family_v1

type OwnedConfinedRelativePath string

type PackageResourceIDV1 string

type PolicyFamilyV1 struct {
	// Body corresponds to the JSON schema field "body".
	Body OwnedConfinedRelativePath `json:"body"`

	// ID corresponds to the JSON schema field "id".
	ID PackageResourceIDV1 `json:"id"`

	// Purpose corresponds to the JSON schema field "purpose".
	Purpose string `json:"purpose"`

	// Title corresponds to the JSON schema field "title".
	Title string `json:"title"`
}
