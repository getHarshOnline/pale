package pale

import (
	"encoding/json"
	"fmt"
)

// PortableWorkspaceRequest is one path-free workspace source family.
type PortableWorkspaceRequest struct {
	RootManifest string     `json:"rootManifest"`
	Documents    []Document `json:"documents"`
}

// PortableWorkspace is an opaque admitted portable-workspace compilation.
type PortableWorkspace struct{ handle }

// WorkspaceComparison is an opaque comparison between admitted workspaces.
type WorkspaceComparison struct{ handle }

// Close releases this workspace and invalidates its copied handles. Independent
// comparisons, profiles and review stages survive. No Engine effect occurs.
func (workspace *PortableWorkspace) Close() {
	if workspace != nil {
		closeNativeHandle(&workspace.handle, dropPortableWorkspace)
	}
}

// Close releases this comparison and invalidates its copied handles. Original
// workspace handles and copied documents survive. No Engine effect occurs.
func (comparison *WorkspaceComparison) Close() {
	if comparison != nil {
		closeNativeHandle(&comparison.handle, dropWorkspaceComparison)
	}
}

// CompilePortableWorkspace admits a path-free workspace source family.
func (client *Client) CompilePortableWorkspace(request PortableWorkspaceRequest) (*PortableWorkspace, CanonicalDocument, error) {
	result, err := client.callDecoded(request, opCompilePortableWorkspace, decodeCanonicalHandleResponse)
	if err != nil {
		return nil, CanonicalDocument{}, err
	}
	return &PortableWorkspace{newHandle(client, result.object)}, CanonicalDocument{Bytes: result.metadata}, nil
}

// InspectAuthoring returns grammar facts retained by this workspace.
func (workspace *PortableWorkspace) InspectAuthoring(sourceLabel string, path []StructuralPathSegment) (AuthoringInspection, error) {
	if workspace == nil {
		return AuthoringInspection{}, &BridgeError{Code: "invalid_handle", Message: "opaque Pale value is unavailable or consumed"}
	}
	if err := workspace.valid(); err != nil {
		return AuthoringInspection{}, err
	}
	result, err := workspace.client.call(struct {
		Handle      uint64                  `json:"handle"`
		SourceLabel string                  `json:"sourceLabel"`
		Path        []StructuralPathSegment `json:"path"`
	}{workspace.id(), sourceLabel, path}, opInspectPortableAuthoring)
	if err != nil {
		return AuthoringInspection{}, err
	}
	return decodeMetadata[AuthoringInspection](result)
}

// NavigateAuthoring resolves one exact source occurrence.
func (workspace *PortableWorkspace) NavigateAuthoring(sourceLabel string, path []InstancePathSegment) (AuthoringNavigation, error) {
	if workspace == nil {
		return AuthoringNavigation{}, &BridgeError{Code: "invalid_handle", Message: "opaque Pale value is unavailable or consumed"}
	}
	return navigateAuthoring(&workspace.handle, sourceLabel, path, opNavigatePortableAuthoring)
}

// ComparePortableWorkspaces derives one retained semantic comparison.
func (client *Client) ComparePortableWorkspaces(prior, current *PortableWorkspace) (*WorkspaceComparison, CanonicalDocument, error) {
	if prior == nil || current == nil || prior.client != client || current.client != client {
		return nil, CanonicalDocument{}, &BridgeError{Code: "invalid_handle", Message: "workspaces must belong to this client"}
	}
	if err := prior.valid(); err != nil {
		return nil, CanonicalDocument{}, err
	}
	if err := current.valid(); err != nil {
		return nil, CanonicalDocument{}, err
	}
	result, err := client.callDecoded(struct {
		Prior   uint64 `json:"prior"`
		Current uint64 `json:"current"`
	}{prior.id(), current.id()}, opComparePortableWorkspaces, decodeCanonicalHandleResponse)
	if err != nil {
		return nil, CanonicalDocument{}, err
	}
	return &WorkspaceComparison{newHandle(client, result.object)}, CanonicalDocument{Bytes: result.metadata}, nil
}

// ChangeRoots returns one typed bounded page and its exact canonical projection.
func (comparison *WorkspaceComparison) ChangeRoots(offset uint32, limit uint16) (WorkspaceChangeRootPage, CanonicalDocument, error) {
	var page WorkspaceChangeRootPage
	document, err := comparison.decodePage(offset, limit, opPortableChangeRoots, &page)
	return page, document, err
}

// Impacts returns one typed bounded page and its exact canonical projection.
func (comparison *WorkspaceComparison) Impacts(offset uint32, limit uint16) (WorkspaceImpactPage, CanonicalDocument, error) {
	var page WorkspaceImpactPage
	document, err := comparison.decodePage(offset, limit, opPortableImpacts, &page)
	return page, document, err
}

func (comparison *WorkspaceComparison) decodePage(offset uint32, limit uint16, operation operation, value any) (CanonicalDocument, error) {
	if comparison == nil {
		return CanonicalDocument{}, &BridgeError{Code: "invalid_handle", Message: "opaque Pale value is unavailable or consumed"}
	}
	if err := comparison.valid(); err != nil {
		return CanonicalDocument{}, err
	}
	result, err := comparison.client.callDecoded(struct {
		Handle uint64 `json:"handle"`
		Offset uint32 `json:"offset"`
		Limit  uint16 `json:"limit"`
	}{comparison.id(), offset, limit}, operation, func(response response) error {
		if response.object != 0 || len(response.metadata) != 0 || len(response.payload) == 0 {
			return fmt.Errorf("unexpected comparison-page response envelope")
		}
		return decodeTransportJSON(response.payload, value)
	})
	return CanonicalDocument{Bytes: result.payload}, err
}

// WorkspaceSubject is one closed causal comparison coordinate. Its unexported
// marker prevents callers from inventing an invalid subject variant.
type WorkspaceSubject interface {
	json.Marshaler
	isWorkspaceSubject()
}

type workspaceSubject struct {
	kind                 string
	sourceID             string
	componentID          string
	moduleID             string
	ownerModuleID        string
	namespaceID          string
	declarationID        string
	importerModuleID     string
	upstreamModuleID     string
	upstreamReleaseID    string
	exportID             string
	sourceNamespaceID    string
	localNamespaceID     string
	extensionNamespaceID string
	targetNamespaceID    string
	targetDeclarationID  string
	resourceID           string
	workspaceID          string
}

func (workspaceSubject) isWorkspaceSubject() {}

// MarshalJSON projects exactly one API-owned tagged subject variant.
func (subject workspaceSubject) MarshalJSON() ([]byte, error) {
	return json.Marshal(struct {
		Kind                 string `json:"kind"`
		SourceID             string `json:"sourceId,omitempty"`
		ComponentID          string `json:"componentId,omitempty"`
		ModuleID             string `json:"moduleId,omitempty"`
		OwnerModuleID        string `json:"ownerModuleId,omitempty"`
		NamespaceID          string `json:"namespaceId,omitempty"`
		DeclarationID        string `json:"declarationId,omitempty"`
		ImporterModuleID     string `json:"importerModuleId,omitempty"`
		UpstreamModuleID     string `json:"upstreamModuleId,omitempty"`
		UpstreamReleaseID    string `json:"upstreamReleaseId,omitempty"`
		ExportID             string `json:"exportId,omitempty"`
		SourceNamespaceID    string `json:"sourceNamespaceId,omitempty"`
		LocalNamespaceID     string `json:"localNamespaceId,omitempty"`
		ExtensionNamespaceID string `json:"extensionNamespaceId,omitempty"`
		TargetNamespaceID    string `json:"targetNamespaceId,omitempty"`
		TargetDeclarationID  string `json:"targetDeclarationId,omitempty"`
		ResourceID           string `json:"resourceId,omitempty"`
		WorkspaceID          string `json:"workspaceId,omitempty"`
	}{
		Kind: subject.kind, SourceID: subject.sourceID, ComponentID: subject.componentID,
		ModuleID: subject.moduleID, OwnerModuleID: subject.ownerModuleID,
		NamespaceID: subject.namespaceID, DeclarationID: subject.declarationID,
		ImporterModuleID: subject.importerModuleID, UpstreamModuleID: subject.upstreamModuleID,
		UpstreamReleaseID: subject.upstreamReleaseID, ExportID: subject.exportID,
		SourceNamespaceID: subject.sourceNamespaceID, LocalNamespaceID: subject.localNamespaceID,
		ExtensionNamespaceID: subject.extensionNamespaceID, TargetNamespaceID: subject.targetNamespaceID,
		TargetDeclarationID: subject.targetDeclarationID, ResourceID: subject.resourceID,
		WorkspaceID: subject.workspaceID,
	})
}

// SourceSubject selects one exact source document.
func SourceSubject(sourceID string) WorkspaceSubject {
	return workspaceSubject{kind: "source", sourceID: sourceID}
}

// ComponentSubject selects one workspace-language component.
func ComponentSubject(componentID string) WorkspaceSubject {
	return workspaceSubject{kind: "component", componentID: componentID}
}

// ModuleSubject selects one reusable module.
func ModuleSubject(moduleID string) WorkspaceSubject {
	return workspaceSubject{kind: "module", moduleID: moduleID}
}

// DeclarationSubject selects one declaration inside its owning namespace.
func DeclarationSubject(ownerModuleID, namespaceID, declarationID string) WorkspaceSubject {
	return workspaceSubject{kind: "declaration", ownerModuleID: ownerModuleID, namespaceID: namespaceID, declarationID: declarationID}
}

// ImportSubject selects one admitted interface import.
func ImportSubject(importerModuleID, upstreamModuleID, upstreamReleaseID, exportID, sourceNamespaceID, localNamespaceID string) WorkspaceSubject {
	return workspaceSubject{
		kind: "import", importerModuleID: importerModuleID, upstreamModuleID: upstreamModuleID,
		upstreamReleaseID: upstreamReleaseID, exportID: exportID,
		sourceNamespaceID: sourceNamespaceID, localNamespaceID: localNamespaceID,
	}
}

// ExtensionSubject selects one declaration extension.
func ExtensionSubject(ownerModuleID, extensionNamespaceID, targetNamespaceID, targetDeclarationID string) WorkspaceSubject {
	return workspaceSubject{
		kind: "extension", ownerModuleID: ownerModuleID, extensionNamespaceID: extensionNamespaceID,
		targetNamespaceID: targetNamespaceID, targetDeclarationID: targetDeclarationID,
	}
}

// PolicySubject selects one policy declaration.
func PolicySubject(ownerModuleID, namespaceID, declarationID string) WorkspaceSubject {
	return workspaceSubject{kind: "policy", ownerModuleID: ownerModuleID, namespaceID: namespaceID, declarationID: declarationID}
}

// ResponsibilityClosureSubject selects the complete responsibility closure.
func ResponsibilityClosureSubject() WorkspaceSubject {
	return workspaceSubject{kind: "responsibility-closure"}
}

// OrganizationalTranslationClosureSubject selects the translation closure.
func OrganizationalTranslationClosureSubject() WorkspaceSubject {
	return workspaceSubject{kind: "organizational-translation-closure"}
}

// ExperienceCompositionSubject selects the experience-composition result.
func ExperienceCompositionSubject() WorkspaceSubject {
	return workspaceSubject{kind: "experience-composition"}
}

// ResourceSubject selects one module-owned resource.
func ResourceSubject(ownerModuleID, resourceID string) WorkspaceSubject {
	return workspaceSubject{kind: "resource", ownerModuleID: ownerModuleID, resourceID: resourceID}
}

// ResolutionLockSubject selects the admitted resolution lock.
func ResolutionLockSubject() WorkspaceSubject {
	return workspaceSubject{kind: "resolution-lock"}
}

// UniverseLockSubject selects the admitted universe lock.
func UniverseLockSubject() WorkspaceSubject {
	return workspaceSubject{kind: "universe-lock"}
}

// WorkspaceIdentitySubject selects the principal-owned workspace root.
func WorkspaceIdentitySubject(workspaceID string) WorkspaceSubject {
	return workspaceSubject{kind: "workspace", workspaceID: workspaceID}
}

// CompareSubject reads one subject in both retained compilations. The compiler
// compares typed outputs independently of impact. The result and canonical
// document survive comparison and client closure; neither is an admission handle.
func (comparison *WorkspaceComparison) CompareSubject(subject WorkspaceSubject) (WorkspaceSubjectComparison, CanonicalDocument, error) {
	if comparison == nil {
		return WorkspaceSubjectComparison{}, CanonicalDocument{}, &BridgeError{Code: "invalid_handle", Message: "opaque Pale value is unavailable or consumed"}
	}
	if err := comparison.valid(); err != nil {
		return WorkspaceSubjectComparison{}, CanonicalDocument{}, err
	}
	var value WorkspaceSubjectComparison
	result, err := comparison.client.callDecoded(struct {
		Handle  uint64           `json:"handle"`
		Subject WorkspaceSubject `json:"subject"`
	}{comparison.id(), subject}, opComparePortableWorkspaceSubject, func(response response) error {
		if response.object != 0 || len(response.metadata) != 0 || len(response.payload) == 0 {
			return fmt.Errorf("unexpected subject-comparison response envelope")
		}
		return decodeTransportJSON(response.payload, &value)
	})
	if err != nil {
		return WorkspaceSubjectComparison{}, CanonicalDocument{}, err
	}
	return value, CanonicalDocument{Bytes: result.payload}, nil
}

// Explain returns one typed causal answer and its exact canonical projection.
func (comparison *WorkspaceComparison) Explain(subject WorkspaceSubject) (WorkspaceExplanation, CanonicalDocument, error) {
	if comparison == nil {
		return WorkspaceExplanation{}, CanonicalDocument{}, &BridgeError{Code: "invalid_handle", Message: "opaque Pale value is unavailable or consumed"}
	}
	if err := comparison.valid(); err != nil {
		return WorkspaceExplanation{}, CanonicalDocument{}, err
	}
	var explanation WorkspaceExplanation
	result, err := comparison.client.callDecoded(struct {
		Handle  uint64           `json:"handle"`
		Subject WorkspaceSubject `json:"subject"`
	}{comparison.id(), subject}, opExplainPortableWorkspace, func(response response) error {
		if response.object != 0 || len(response.metadata) != 0 || len(response.payload) == 0 {
			return fmt.Errorf("unexpected workspace-explanation response envelope")
		}
		return decodeTransportJSON(response.payload, &explanation)
	})
	if err != nil {
		return WorkspaceExplanation{}, CanonicalDocument{}, err
	}
	return explanation, CanonicalDocument{Bytes: result.payload}, nil
}
