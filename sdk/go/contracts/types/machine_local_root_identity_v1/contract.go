// Code generated from the canonical Pale initial-v1 contract bundle. DO NOT EDIT.
// SPDX-License-Identifier: Apache-2.0

// Package machine_local_root_identity_v1 contains the generated machine-local-root-identity.v1 data model.
package machine_local_root_identity_v1

type FilesystemDeviceIDV1 string

type FilesystemInodeIDV1 string

type MachineLocalRootIdentityV1 struct {
	// CanonicalPath corresponds to the JSON schema field "canonical_path".
	CanonicalPath string `json:"canonical_path"`

	// Device corresponds to the JSON schema field "device".
	Device FilesystemDeviceIDV1 `json:"device"`

	// Inode corresponds to the JSON schema field "inode".
	Inode FilesystemInodeIDV1 `json:"inode"`
}
