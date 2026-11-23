package pale

import "fmt"

// PlanKind selects apply, explicit adoption, or removal law.
type PlanKind string

const (
	PlanApply  PlanKind = "apply"
	PlanAdopt  PlanKind = "adopt"
	PlanRemove PlanKind = "remove"
)

// UnmarshalJSON accepts the SDK's existing three plan classes.
func (kind *PlanKind) UnmarshalJSON(data []byte) error {
	return decodeTransportTag(data, kind, PlanApply, PlanAdopt, PlanRemove)
}

// OneShotCompilation is an opaque complete captured compilation.
type OneShotCompilation struct{ handle }

// PlanCandidate is an opaque effect-free candidate consumed by machine-effect binding.
type PlanCandidate struct{ handle }

// BoundPlan is an opaque complete plan consumed by saved-plan admission.
type BoundPlan struct{ handle }

// AdmittedPlan is an opaque authority proof retained for later eligible effects.
type AdmittedPlan struct{ handle }

// OneShotRequest is one complete path-free compilation input.
type OneShotRequest struct {
	ProjectID            string
	ExecutionWorkspaceID string
	MachineID            string
	PrivateInputs        PrivateInputs
	PackageDocuments     []Document
	Workspace            PortableWorkspaceRequest
}

// PriorMaterialization identifies trusted prior ownership for planning.
type PriorMaterialization struct {
	CanonicalState []byte
}

func (value PriorMaterialization) wire() any {
	if value.CanonicalState == nil {
		return struct {
			State string `json:"state"`
		}{State: "unowned"}
	}
	return struct {
		State          string `json:"state"`
		CanonicalState []byte `json:"canonical_state"`
	}{State: "owned", CanonicalState: value.CanonicalState}
}

// BaselineEntry is one exact pre-effect repository observation.
type BaselineEntry struct {
	RelativePath string
	SHA256       string
	UnixMode     uint32
	Present      bool
}

func (entry BaselineEntry) wire() any {
	if !entry.Present {
		return struct {
			RelativePath string `json:"relativePath"`
			Observation  struct {
				State string `json:"state"`
			} `json:"observation"`
		}{RelativePath: entry.RelativePath, Observation: struct {
			State string `json:"state"`
		}{State: "absent"}}
	}
	return struct {
		RelativePath string `json:"relativePath"`
		Observation  struct {
			State    string `json:"state"`
			SHA256   string `json:"sha256"`
			UnixMode uint32 `json:"unix_mode"`
		} `json:"observation"`
	}{RelativePath: entry.RelativePath, Observation: struct {
		State    string `json:"state"`
		SHA256   string `json:"sha256"`
		UnixMode uint32 `json:"unix_mode"`
	}{State: "present", SHA256: entry.SHA256, UnixMode: entry.UnixMode}}
}

// CompileOneShot admits one complete captured workspace, private, and Package source family.
func (client *Client) CompileOneShot(request OneShotRequest) (*OneShotCompilation, CanonicalDocument, error) {
	result, err := client.callDecoded(struct {
		ProjectID            string                   `json:"projectId"`
		ExecutionWorkspaceID string                   `json:"executionWorkspaceId"`
		MachineID            string                   `json:"machineId"`
		PrivateInputs        privateInputs            `json:"privateInputs"`
		PackageDocuments     []Document               `json:"packageDocuments"`
		Workspace            PortableWorkspaceRequest `json:"workspace"`
	}{request.ProjectID, request.ExecutionWorkspaceID, request.MachineID, request.PrivateInputs.wire(), request.PackageDocuments, request.Workspace}, opCompileOneShot, decodeCanonicalHandleResponse)
	if err != nil {
		return nil, CanonicalDocument{}, err
	}
	return &OneShotCompilation{newHandle(client, result.object)}, CanonicalDocument{Bytes: result.metadata}, nil
}

// Summary returns the same canonical compilation summary produced at admission.
func (compilation *OneShotCompilation) Summary() (CanonicalDocument, error) {
	if compilation == nil {
		return CanonicalDocument{}, &BridgeError{Code: "invalid_handle", Message: "opaque Pale value is unavailable or consumed"}
	}
	if err := compilation.valid(); err != nil {
		return CanonicalDocument{}, err
	}
	result, err := compilation.client.call(struct {
		Handle uint64 `json:"handle"`
	}{compilation.id()}, opOneShotSummary)
	return CanonicalDocument{Bytes: result.metadata}, err
}

// ObservationTargets derives the exact effect-free observation set for planning.
func (compilation *OneShotCompilation) ObservationTargets(repositoryID string, kind PlanKind, prior PriorMaterialization) (CanonicalDocument, error) {
	if compilation == nil {
		return CanonicalDocument{}, &BridgeError{Code: "invalid_handle", Message: "opaque Pale value is unavailable or consumed"}
	}
	if err := compilation.valid(); err != nil {
		return CanonicalDocument{}, err
	}
	result, err := compilation.client.call(struct {
		Compilation  uint64   `json:"compilation"`
		RepositoryID string   `json:"repositoryId"`
		Kind         PlanKind `json:"kind"`
		Prior        any      `json:"prior"`
	}{compilation.id(), repositoryID, kind, prior.wire()}, opOneShotObservationTargets)
	return CanonicalDocument{Bytes: result.payload}, err
}

// PlanCandidateInfo copies complete SDK metadata; it does not admit a plan.
// Native results stay response-owned until their Go metadata is decoded. The
// accompanying canonical bytes remain unchanged and are not re-admitted here.
type PlanCandidateInfo struct {
	Kind                     PlanKind `json:"kind"`
	RepositoryID             string   `json:"repositoryId"`
	PlanDigest               string   `json:"planDigest"`
	OneShotCompilationDigest string   `json:"oneShotCompilationDigest"`
	BaselineCount            uint32   `json:"baselineCount"`
	OperationCount           uint32   `json:"operationCount"`
	CreateCount              uint32   `json:"createCount"`
	AdoptCount               uint32   `json:"adoptCount"`
	UpdateCount              uint32   `json:"updateCount"`
	UnchangedCount           uint32   `json:"unchangedCount"`
	DeleteCount              uint32   `json:"deleteCount"`
	SpliceCount              uint32   `json:"spliceCount"`
	DeferredEffectCount      uint32   `json:"deferredEffectCount"`
	LocalOnlyTargets         []string `json:"localOnlyTargets"`
}

// UnmarshalJSON checks transport shape, not plan identities or path admission.
func (info *PlanCandidateInfo) UnmarshalJSON(data []byte) error {
	type plain PlanCandidateInfo
	var next struct {
		plain
		Targets []*string `json:"localOnlyTargets"`
	}
	if err := decodeTransportObject(data, &next, []string{
		"kind", "repositoryId", "planDigest", "oneShotCompilationDigest",
		"baselineCount", "operationCount", "createCount", "adoptCount", "updateCount",
		"unchangedCount", "deleteCount", "spliceCount", "deferredEffectCount", "localOnlyTargets",
	}); err != nil {
		return err
	}
	next.plain.LocalOnlyTargets = make([]string, len(next.Targets))
	for index, target := range next.Targets {
		if target == nil {
			return fmt.Errorf("one-shot target must be a string")
		}
		next.plain.LocalOnlyTargets[index] = *target
	}
	*info = PlanCandidateInfo(next.plain)
	return nil
}

// Plan derives one effect-free candidate from admitted observations.
func (compilation *OneShotCompilation) Plan(repositoryID string, kind PlanKind, prior PriorMaterialization, baseline []BaselineEntry) (*PlanCandidate, PlanCandidateInfo, CanonicalDocument, error) {
	if compilation == nil {
		return nil, PlanCandidateInfo{}, CanonicalDocument{}, &BridgeError{Code: "invalid_handle", Message: "opaque Pale value is unavailable or consumed"}
	}
	if err := compilation.valid(); err != nil {
		return nil, PlanCandidateInfo{}, CanonicalDocument{}, err
	}
	entries := make([]any, len(baseline))
	for index, entry := range baseline {
		entries[index] = entry.wire()
	}
	var info PlanCandidateInfo
	result, err := compilation.client.callDecoded(struct {
		Compilation  uint64   `json:"compilation"`
		RepositoryID string   `json:"repositoryId"`
		Kind         PlanKind `json:"kind"`
		Prior        any      `json:"prior"`
		Baseline     []any    `json:"baseline"`
	}{compilation.id(), repositoryID, kind, prior.wire(), entries}, opPlanOneShot, func(result response) error {
		var err error
		info, err = decodeOneShotCandidateResponse(result)
		return err
	})
	if err != nil {
		return nil, PlanCandidateInfo{}, CanonicalDocument{}, err
	}
	return &PlanCandidate{newHandle(compilation.client, result.object)}, info, CanonicalDocument{Bytes: result.payload}, nil
}

// BoundPlanInfo identifies one complete plan after machine effects are bound.
type BoundPlanInfo struct {
	Kind         PlanKind `json:"kind"`
	RepositoryID string   `json:"repositoryId"`
}

// UnmarshalJSON preserves the exact SDK field set before native custody transfers.
func (info *BoundPlanInfo) UnmarshalJSON(data []byte) error {
	type plain BoundPlanInfo
	var next plain
	if err := decodeTransportObject(data, &next, []string{"kind", "repositoryId"}); err != nil {
		return err
	}
	*info = BoundPlanInfo(next)
	return nil
}

// BindMachineEffects consumes a candidate and binds canonical machine-local candidates.
func (candidate *PlanCandidate) BindMachineEffects(effects [][]byte) (*BoundPlan, BoundPlanInfo, CanonicalDocument, error) {
	if candidate == nil || candidate.client == nil {
		return nil, BoundPlanInfo{}, CanonicalDocument{}, &BridgeError{Code: "invalid_handle", Message: "one-shot candidate is unavailable"}
	}
	client := candidate.client
	client.mu.Lock()
	defer client.mu.Unlock()
	if err := candidate.valid(); err != nil {
		return nil, BoundPlanInfo{}, CanonicalDocument{}, err
	}
	if effects == nil {
		effects = [][]byte{}
	}
	handleValue := candidate.id()
	var info BoundPlanInfo
	result, err := client.callDecodedLocked(struct {
		Candidate uint64   `json:"candidate"`
		Effects   [][]byte `json:"effects"`
	}{handleValue, effects}, func(client, request uint64) uint64 {
		candidate.consume()
		// Private decoding can reject before Rust removes the surrendered input.
		defer dropOneShotCandidate(client, handleValue)
		return opBindOneShotMachineEffects(client, request)
	}, func(result response) error {
		var err error
		info, err = decodeOneShotBoundResponse(result)
		return err
	})
	if err != nil {
		return nil, BoundPlanInfo{}, CanonicalDocument{}, err
	}
	return &BoundPlan{newHandle(candidate.client, result.object)}, info, CanonicalDocument{Bytes: result.payload}, nil
}

// AdmittedPlanInfo identifies one plan admitted against exact saved bytes.
type AdmittedPlanInfo struct {
	Kind         PlanKind `json:"kind"`
	RepositoryID string   `json:"repositoryId"`
	PlanDigest   string   `json:"planDigest"`
}

// UnmarshalJSON checks the projection without minting saved-plan authority.
func (info *AdmittedPlanInfo) UnmarshalJSON(data []byte) error {
	type plain AdmittedPlanInfo
	var next plain
	if err := decodeTransportObject(data, &next, []string{"kind", "repositoryId", "planDigest"}); err != nil {
		return err
	}
	*info = AdmittedPlanInfo(next)
	return nil
}

// AdmitSaved consumes a rebuilt complete plan and admits the exact saved bytes.
func (plan *BoundPlan) AdmitSaved(saved []byte) (*AdmittedPlan, AdmittedPlanInfo, error) {
	if plan == nil || plan.client == nil {
		return nil, AdmittedPlanInfo{}, &BridgeError{Code: "invalid_handle", Message: "one-shot bound plan is unavailable"}
	}
	client := plan.client
	client.mu.Lock()
	defer client.mu.Unlock()
	if err := plan.valid(); err != nil {
		return nil, AdmittedPlanInfo{}, err
	}
	handleValue := plan.id()
	var info AdmittedPlanInfo
	result, err := client.callDecodedLocked(struct {
		BoundPlan  uint64 `json:"boundPlan"`
		SavedBytes []byte `json:"savedBytes"`
	}{handleValue, saved}, func(client, request uint64) uint64 {
		plan.consume()
		defer dropOneShotBoundPlan(client, handleValue)
		return opAdmitOneShotSavedPlan(client, request)
	}, func(result response) error {
		var err error
		info, err = decodeOneShotAdmittedResponse(result)
		return err
	})
	if err != nil {
		return nil, AdmittedPlanInfo{}, err
	}
	return &AdmittedPlan{newHandle(plan.client, result.object)}, info, nil
}

func decodeOneShotCandidateResponse(result response) (PlanCandidateInfo, error) {
	if result.object == 0 || len(result.payload) == 0 {
		return PlanCandidateInfo{}, fmt.Errorf("incomplete one-shot candidate response")
	}
	info, err := decodeMetadata[PlanCandidateInfo](result)
	if err != nil {
		return PlanCandidateInfo{}, fmt.Errorf("invalid one-shot candidate metadata")
	}
	return info, nil
}

func decodeOneShotBoundResponse(result response) (BoundPlanInfo, error) {
	if result.object == 0 || len(result.payload) == 0 {
		return BoundPlanInfo{}, fmt.Errorf("incomplete one-shot bound-plan response")
	}
	info, err := decodeMetadata[BoundPlanInfo](result)
	if err != nil {
		return BoundPlanInfo{}, fmt.Errorf("invalid one-shot bound-plan metadata")
	}
	return info, nil
}

func decodeOneShotAdmittedResponse(result response) (AdmittedPlanInfo, error) {
	if result.object == 0 || len(result.payload) != 0 {
		return AdmittedPlanInfo{}, fmt.Errorf("invalid one-shot admitted-plan response")
	}
	info, err := decodeMetadata[AdmittedPlanInfo](result)
	if err != nil {
		return AdmittedPlanInfo{}, fmt.Errorf("invalid one-shot admitted-plan metadata")
	}
	return info, nil
}
