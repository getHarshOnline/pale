// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package native_tool_request_v1 contains the generated native-tool-request.v1 data model.
package native_tool_request_v1

import lifecycle_event_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/lifecycle_event_v1"
import sha256_digest_v1 "github.com/getHarshOnline/pale/sdk/go/contracts/types/sha256_digest_v1"

type LogicalRepositoryIDV1 string

type NativeToolRequestV1 struct {
	// Event corresponds to the JSON schema field "event".
	Event lifecycle_event_v1.LifecycleEventV1 `json:"event"`

	// NativeEvent corresponds to the JSON schema field "native_event".
	NativeEvent string `json:"native_event"`

	// ProviderBinding corresponds to the JSON schema field "provider_binding".
	ProviderBinding ProviderBindingRefV1 `json:"provider_binding"`

	// RepositoryID corresponds to the JSON schema field "repository_id".
	RepositoryID LogicalRepositoryIDV1 `json:"repository_id"`

	// WorkspaceID corresponds to the JSON schema field "workspace_id".
	WorkspaceID WorkspaceID `json:"workspace_id"`
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

type WorkspaceID string
