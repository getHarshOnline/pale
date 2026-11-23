package pale

import "fmt"

// EffectiveProfileCoordinates identify the exact retained compilation. The
// portable profile, private instance, and composition plan have separate identities.
type EffectiveProfileCoordinates struct {
	WorkspaceID          string `json:"workspaceId"`
	ProjectID            string `json:"projectId"`
	ExecutionWorkspaceID string `json:"executionWorkspaceId"`
	MachineID            string `json:"machineId"`
	ProfileDigest        string `json:"profileDigest"`
	// InstanceDigest identifies the compiled private configuration, not observed machine or native readiness.
	InstanceDigest                   string `json:"instanceDigest"`
	SelectedTopologyDigest           string `json:"selectedTopologyDigest"`
	PackageSemanticIRDigest          string `json:"packageSemanticIrDigest"`
	CompositionPlanDigest            string `json:"compositionPlanDigest"`
	WorkspaceSemanticIRDigest        string `json:"workspaceSemanticIrDigest"`
	WorkspaceAdmissionEvidenceDigest string `json:"workspaceAdmissionEvidenceDigest"`
}

// EffectiveProfile is an opaque admitted effective-profile compilation.
type EffectiveProfile struct{ handle }

// Close releases this profile and invalidates its copied handles. Independent
// observations, identities and review stages survive. No Engine effect occurs.
func (profile *EffectiveProfile) Close() {
	if profile != nil {
		closeNativeHandle(&profile.handle, dropEffectiveProfile)
	}
}

// EffectiveProfileRequest supplies the private and Package inputs selected
// after portable workspace admission.
type EffectiveProfileRequest struct {
	ProjectID            string
	ExecutionWorkspaceID string
	MachineID            string
	PrivateInputs        PrivateInputs
	PackageDocuments     []Document
}

// UnmarshalJSON decodes exact SDK fields without recomputing their identities.
func (coordinates *EffectiveProfileCoordinates) UnmarshalJSON(data []byte) error {
	type plain EffectiveProfileCoordinates
	var next plain
	if err := decodeTransportObject(data, &next, []string{
		"workspaceId", "projectId", "executionWorkspaceId", "machineId",
		"profileDigest", "instanceDigest", "selectedTopologyDigest",
		"packageSemanticIrDigest", "compositionPlanDigest", "workspaceSemanticIrDigest",
		"workspaceAdmissionEvidenceDigest",
	}); err != nil {
		return err
	}
	*coordinates = EffectiveProfileCoordinates(next)
	return nil
}

// EffectiveProfileSection selects one bounded profile-inspection partition.
type EffectiveProfileSection string

const (
	ProfileIntendedUseCase        EffectiveProfileSection = "intended-use-case"
	ProfileEffectiveConfiguration EffectiveProfileSection = "effective-configuration"
	ProfileCapabilitiesOperations EffectiveProfileSection = "capabilities-and-operations"
	ProfileMachineRealization     EffectiveProfileSection = "machine-realization"
	ProfileGapsAndExplanations    EffectiveProfileSection = "gaps-and-explanations"
)

// CompileEffectiveProfile derives one selected profile from an admitted workspace.
func (workspace *PortableWorkspace) CompileEffectiveProfile(request EffectiveProfileRequest) (*EffectiveProfile, EffectiveProfileCoordinates, error) {
	if workspace == nil {
		return nil, EffectiveProfileCoordinates{}, &BridgeError{Code: "invalid_handle", Message: "opaque Pale value is unavailable or consumed"}
	}
	if err := workspace.valid(); err != nil {
		return nil, EffectiveProfileCoordinates{}, err
	}
	var coordinates EffectiveProfileCoordinates
	result, err := workspace.client.callDecoded(struct {
		Workspace            uint64        `json:"workspace"`
		ProjectID            string        `json:"projectId"`
		ExecutionWorkspaceID string        `json:"executionWorkspaceId"`
		MachineID            string        `json:"machineId"`
		PrivateInputs        privateInputs `json:"privateInputs"`
		PackageDocuments     []Document    `json:"packageDocuments"`
	}{workspace.id(), request.ProjectID, request.ExecutionWorkspaceID, request.MachineID, request.PrivateInputs.wire(), request.PackageDocuments}, opCompileEffectiveProfile, func(result response) error {
		if result.object == 0 {
			return fmt.Errorf("effective profile handle is missing")
		}
		var decodeErr error
		coordinates, decodeErr = decodeMetadata[EffectiveProfileCoordinates](result)
		return decodeErr
	})
	if err != nil {
		return nil, EffectiveProfileCoordinates{}, err
	}
	return &EffectiveProfile{newHandle(workspace.client, result.object)}, coordinates, nil
}

// Inspect returns one canonical bounded effective-profile page.
func (profile *EffectiveProfile) Inspect(section EffectiveProfileSection, offset uint32, limit uint16) (CanonicalDocument, error) {
	if profile == nil {
		return CanonicalDocument{}, &BridgeError{Code: "invalid_handle", Message: "opaque Pale value is unavailable or consumed"}
	}
	if err := profile.valid(); err != nil {
		return CanonicalDocument{}, err
	}
	result, err := profile.client.call(struct {
		Handle  uint64                  `json:"handle"`
		Section EffectiveProfileSection `json:"section"`
		Offset  uint32                  `json:"offset"`
		Limit   uint16                  `json:"limit"`
	}{profile.id(), section, offset, limit}, opInspectEffectiveProfile)
	return CanonicalDocument{Bytes: result.payload}, err
}

// Explain resolves one profile-local lowercase SHA-256 explanation handle.
// A valid unmatched handle returns nil; malformed handles return an SDK error.
func (profile *EffectiveProfile) Explain(explanation string) (*EffectiveProfileExplanation, error) {
	if profile == nil {
		return nil, &BridgeError{Code: "invalid_handle", Message: "opaque Pale value is unavailable or consumed"}
	}
	if err := profile.valid(); err != nil {
		return nil, err
	}
	result, err := profile.client.call(struct {
		Handle      uint64 `json:"handle"`
		Explanation string `json:"explanation"`
	}{profile.id(), explanation}, opExplainEffectiveProfile)
	if err != nil {
		return nil, err
	}
	return decodeEffectiveProfileExplanation(result, explanation)
}

// InspectAuthoring returns grammar facts retained by this profile.
func (profile *EffectiveProfile) InspectAuthoring(sourceLabel string, path []StructuralPathSegment) (AuthoringInspection, error) {
	if profile == nil {
		return AuthoringInspection{}, &BridgeError{Code: "invalid_handle", Message: "opaque Pale value is unavailable or consumed"}
	}
	if err := profile.valid(); err != nil {
		return AuthoringInspection{}, err
	}
	result, err := profile.client.call(struct {
		Handle      uint64                  `json:"handle"`
		SourceLabel string                  `json:"sourceLabel"`
		Path        []StructuralPathSegment `json:"path"`
	}{profile.id(), sourceLabel, path}, opInspectEffectiveAuthoring)
	if err != nil {
		return AuthoringInspection{}, err
	}
	return decodeMetadata[AuthoringInspection](result)
}

// NavigateAuthoring resolves one exact source occurrence retained by the profile.
func (profile *EffectiveProfile) NavigateAuthoring(sourceLabel string, path []InstancePathSegment) (AuthoringNavigation, error) {
	if profile == nil {
		return AuthoringNavigation{}, &BridgeError{Code: "invalid_handle", Message: "opaque Pale value is unavailable or consumed"}
	}
	return navigateAuthoring(&profile.handle, sourceLabel, path, opNavigateEffectiveAuthoring)
}
