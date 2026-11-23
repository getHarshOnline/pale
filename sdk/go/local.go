package pale

import "fmt"

// CapturedNativeAction is an opaque path-free request captured from a confined file.
type CapturedNativeAction struct{ handle }

// LocalCompilation retains an admitted compilation and private host context.
type LocalCompilation struct{ handle }

// LocalPlan retains a complete local plan and its private host context.
type LocalPlan struct{ handle }

// LocalRun retains saved-plan admission and any outcome-unknown reconciliation state.
type LocalRun struct{ handle }

// LocalConfigurationImprovementRequest selects two local configurations and
// their common profile context. Only the baseline supplies Package content.
type LocalConfigurationImprovementRequest struct {
	BaselineManifest     string `json:"baselineManifest"`
	CandidateManifest    string `json:"candidateManifest"`
	ProjectID            string `json:"projectId"`
	ExecutionWorkspaceID string `json:"executionWorkspaceId"`
	MachineID            string `json:"machineId"`
}

// String keeps private locators and selectors out of ordinary diagnostic text.
func (LocalConfigurationImprovementRequest) String() string {
	return "LocalConfigurationImprovementRequest{<redacted>}"
}

// GoString also redacts the request in Go-syntax formatting.
func (request LocalConfigurationImprovementRequest) GoString() string { return request.String() }

// PreviewLocalConfigurationImprovement captures and compares local sources.
// Accepting its preview returns a disposition; it does not persist the decision,
// write files, or authorize an effect. The handle uses the existing review lifecycle.
func (client *Client) PreviewLocalConfigurationImprovement(request LocalConfigurationImprovementRequest) (*ConfigurationImprovementPreview, error) {
	result, err := client.call(request, opPreviewLocalConfigurationImprovement)
	if err != nil {
		return nil, err
	}
	return &ConfigurationImprovementPreview{newHandle(client, result.object)}, nil
}

// LocalWorkspaceRequest selects one manifest, project, execution workspace,
// and machine for Rust-owned local capture.
type LocalWorkspaceRequest struct {
	Manifest             string `json:"manifest"`
	ProjectID            string `json:"projectId"`
	ExecutionWorkspaceID string `json:"executionWorkspaceId"`
	MachineID            string `json:"machineId"`
}

// CompileLocalOneShot resolves, captures, and compiles one local workspace.
func (client *Client) CompileLocalOneShot(request LocalWorkspaceRequest) (*LocalCompilation, CanonicalDocument, error) {
	result, err := client.callDecoded(request, opCompileLocalOneShot, decodeCanonicalHandleResponse)
	if err != nil {
		return nil, CanonicalDocument{}, err
	}
	return &LocalCompilation{newHandle(client, result.object)}, CanonicalDocument{Bytes: result.metadata}, nil
}

// InspectLocalPackageDistribution captures only the selected Package tree and
// returns its canonical release coordinates.
func (client *Client) InspectLocalPackageDistribution(request LocalWorkspaceRequest) (CanonicalDocument, error) {
	result, err := client.call(request, opInspectLocalPackageDistribution)
	return CanonicalDocument{Bytes: result.payload}, err
}

// CaptureLocalNativeActionRequest confines and reads one local file without
// invoking the selected effect-free action.
func (client *Client) CaptureLocalNativeActionRequest(actionID, root, relativePath string) (*CapturedNativeAction, error) {
	result, err := client.call(struct {
		ActionID     string `json:"actionId"`
		Root         string `json:"root"`
		RelativePath string `json:"relativePath"`
	}{actionID, root, relativePath}, opCaptureLocalNativeAction)
	if err != nil {
		return nil, err
	}
	return &CapturedNativeAction{newHandle(client, result.object)}, nil
}

// Invoke consumes this captured request through the same action registry used
// by InvokeNativeAction.
func (request *CapturedNativeAction) Invoke() (CanonicalDocument, error) {
	if request == nil || request.client == nil {
		return CanonicalDocument{}, &BridgeError{Code: "invalid_handle", Message: "captured native-action request is unavailable"}
	}
	client := request.client
	client.mu.Lock()
	defer client.mu.Unlock()
	if err := request.valid(); err != nil {
		return CanonicalDocument{}, err
	}
	result, err := client.callLocked(struct {
		Handle uint64 `json:"handle"`
	}{request.id()}, func(client, input uint64) uint64 {
		request.consume()
		return opInvokeCapturedNativeAction(client, input)
	})
	return CanonicalDocument{Bytes: result.payload}, err
}

// LocalMaterializationDrift is one compiler-owned reason a local repository
// differs from its admitted projection. Fields irrelevant to Kind are empty.
type LocalMaterializationDrift struct {
	Kind         string `json:"kind"`
	RelativePath string `json:"relativePath,omitempty"`
	Action       string `json:"action,omitempty"`
	Change       string `json:"change,omitempty"`
	Effect       string `json:"effect,omitempty"`
}

// LocalRepositoryStatus is one complete read-only local comparison.
type LocalRepositoryStatus struct {
	RepositoryID string                      `json:"repositoryId"`
	Clean        bool                        `json:"clean"`
	Drift        []LocalMaterializationDrift `json:"drift"`
}

// RepositoryStatus compares one local repository with its admitted projection.
func (compilation *LocalCompilation) RepositoryStatus(repositoryID string) (LocalRepositoryStatus, error) {
	if compilation == nil {
		return LocalRepositoryStatus{}, &BridgeError{Code: "invalid_handle", Message: "opaque Pale value is unavailable or consumed"}
	}
	if err := compilation.valid(); err != nil {
		return LocalRepositoryStatus{}, err
	}
	result, err := compilation.client.call(struct {
		Compilation  uint64 `json:"compilation"`
		RepositoryID string `json:"repositoryId"`
	}{compilation.id(), repositoryID}, opInspectLocalRepositoryStatus)
	if err != nil {
		return LocalRepositoryStatus{}, err
	}
	var status LocalRepositoryStatus
	if err := decodeJSON(result.metadata, &status); err != nil {
		return LocalRepositoryStatus{}, err
	}
	return status, nil
}

// LocalPlanInfo contains transition counts derived from the same complete plan.
type LocalPlanInfo struct {
	Kind                    PlanKind `json:"kind"`
	RepositoryID            string   `json:"repositoryId"`
	BaselineCount           uint32   `json:"baselineCount"`
	OperationCount          uint32   `json:"operationCount"`
	TotalOperationCount     uint32   `json:"totalOperationCount"`
	CreateCount             uint32   `json:"createCount"`
	AdoptCount              uint32   `json:"adoptCount"`
	UpdateCount             uint32   `json:"updateCount"`
	UnchangedCount          uint32   `json:"unchangedCount"`
	DeleteCount             uint32   `json:"deleteCount"`
	SpliceCount             uint32   `json:"spliceCount"`
	DeferredEffectCount     uint32   `json:"deferredEffectCount"`
	MachineLocalEffectCount uint32   `json:"machineLocalEffectCount"`
}

// UnmarshalJSON checks complete transport shape, not plan or filesystem law.
func (info *LocalPlanInfo) UnmarshalJSON(data []byte) error {
	type plain LocalPlanInfo
	var next plain
	if err := decodeTransportObject(data, &next, []string{
		"kind", "repositoryId", "baselineCount", "operationCount", "totalOperationCount",
		"createCount", "adoptCount", "updateCount", "unchangedCount", "deleteCount",
		"spliceCount", "deferredEffectCount", "machineLocalEffectCount",
	}); err != nil {
		return err
	}
	*info = LocalPlanInfo(next)
	return nil
}

// Plan constructs one effect-free complete plan from trusted local observations.
func (compilation *LocalCompilation) Plan(repositoryID string, kind PlanKind) (*LocalPlan, LocalPlanInfo, CanonicalDocument, error) {
	if compilation == nil {
		return nil, LocalPlanInfo{}, CanonicalDocument{}, &BridgeError{Code: "invalid_handle", Message: "opaque Pale value is unavailable or consumed"}
	}
	if err := compilation.valid(); err != nil {
		return nil, LocalPlanInfo{}, CanonicalDocument{}, err
	}
	var info LocalPlanInfo
	result, err := compilation.client.callDecoded(struct {
		Compilation  uint64   `json:"compilation"`
		RepositoryID string   `json:"repositoryId"`
		Kind         PlanKind `json:"kind"`
	}{compilation.id(), repositoryID, kind}, opPlanLocalOneShot, func(result response) error {
		var err error
		info, err = decodeLocalPlanResponse(result)
		return err
	})
	if err != nil {
		return nil, LocalPlanInfo{}, CanonicalDocument{}, err
	}
	return &LocalPlan{newHandle(compilation.client, result.object)}, info, CanonicalDocument{Bytes: result.payload}, nil
}

// LocalPlanPublication identifies one durably published owner-only plan file.
type LocalPlanPublication struct {
	RepositoryID   string `json:"repositoryId"`
	RelativeTarget string `json:"relativeTarget"`
	DisplayTarget  string `json:"displayTarget"`
}

// Publish durably replaces this plan in a confined directory. This is a native
// effect and has no Engine-owned cancellation transition.
func (plan *LocalPlan) Publish(outputDirectory string) (LocalPlanPublication, error) {
	if plan == nil {
		return LocalPlanPublication{}, &BridgeError{Code: "invalid_handle", Message: "opaque Pale value is unavailable or consumed"}
	}
	if err := plan.valid(); err != nil {
		return LocalPlanPublication{}, err
	}
	result, err := plan.client.call(struct {
		Plan            uint64 `json:"plan"`
		OutputDirectory string `json:"outputDirectory"`
	}{plan.id(), outputDirectory}, opPublishLocalOneShotPlan)
	if err != nil {
		return LocalPlanPublication{}, err
	}
	var publication LocalPlanPublication
	if err := decodeJSON(result.metadata, &publication); err != nil {
		return LocalPlanPublication{}, err
	}
	return publication, nil
}

// LocalRunTargetAdmission binds a repository to the exact saved-plan digest.
type LocalRunTargetAdmission struct {
	RepositoryID string `json:"repositoryId"`
	PlanDigest   string `json:"planDigest"`
}

// UnmarshalJSON requires both SDK-projected target fields without re-admitting them.
func (target *LocalRunTargetAdmission) UnmarshalJSON(data []byte) error {
	type plain LocalRunTargetAdmission
	var next plain
	if err := decodeTransportObject(data, &next, []string{"repositoryId", "planDigest"}); err != nil {
		return err
	}
	*target = LocalRunTargetAdmission(next)
	return nil
}

// LocalRunAdmission is the effect-free authority proof for one complete run.
type LocalRunAdmission struct {
	Kind    PlanKind                  `json:"kind"`
	Targets []LocalRunTargetAdmission `json:"targets"`
}

// UnmarshalJSON preserves the SDK target order and accepts no partial records.
func (admission *LocalRunAdmission) UnmarshalJSON(data []byte) error {
	type plain LocalRunAdmission
	var next plain
	if err := decodeTransportObject(data, &next, []string{"kind", "targets"}); err != nil {
		return err
	}
	*admission = LocalRunAdmission(next)
	return nil
}

// AdmitRun reconstructs and admits every saved plan without writing.
func (compilation *LocalCompilation) AdmitRun(kind PlanKind, savedPlanDirectory, evidenceDirectory string) (*LocalRun, LocalRunAdmission, error) {
	if compilation == nil {
		return nil, LocalRunAdmission{}, &BridgeError{Code: "invalid_handle", Message: "opaque Pale value is unavailable or consumed"}
	}
	if err := compilation.valid(); err != nil {
		return nil, LocalRunAdmission{}, err
	}
	var admission LocalRunAdmission
	result, err := compilation.client.callDecoded(struct {
		Compilation        uint64   `json:"compilation"`
		Kind               PlanKind `json:"kind"`
		SavedPlanDirectory string   `json:"savedPlanDirectory"`
		EvidenceDirectory  string   `json:"evidenceDirectory"`
	}{compilation.id(), kind, savedPlanDirectory, evidenceDirectory}, opAdmitLocalMaterializationRun, func(result response) error {
		var err error
		admission, err = decodeLocalRunResponse(result)
		return err
	})
	if err != nil {
		return nil, LocalRunAdmission{}, err
	}
	return &LocalRun{newHandle(compilation.client, result.object)}, admission, nil
}

// These are newly constructed outputs, not settled runs. A failed decode
// discards the pending native value and does not alter any retained input.
func decodeLocalPlanResponse(result response) (LocalPlanInfo, error) {
	if result.object == 0 || len(result.payload) == 0 {
		return LocalPlanInfo{}, fmt.Errorf("incomplete local plan response")
	}
	info, err := decodeMetadata[LocalPlanInfo](result)
	if err != nil {
		return LocalPlanInfo{}, fmt.Errorf("invalid local plan metadata")
	}
	return info, nil
}

func decodeLocalRunResponse(result response) (LocalRunAdmission, error) {
	if result.object == 0 || len(result.payload) != 0 {
		return LocalRunAdmission{}, fmt.Errorf("invalid local run admission response")
	}
	admission, err := decodeMetadata[LocalRunAdmission](result)
	if err != nil {
		return LocalRunAdmission{}, fmt.Errorf("invalid local run admission metadata")
	}
	return admission, nil
}

// LocalMaterializationRunOutcome reports how far the ordered repository run completed.
type LocalMaterializationRunOutcome string

const (
	// LocalRunSucceeded means every selected repository has a success receipt.
	LocalRunSucceeded LocalMaterializationRunOutcome = "succeeded"
	// LocalRunPartial means earlier repositories succeeded before a failed target
	// stopped the run. Later repositories were not attempted.
	LocalRunPartial LocalMaterializationRunOutcome = "partial"
	// LocalRunFailedNoEffect means the first target retained no repository or
	// machine-local materialization effect. Its diagnostic scopes coordination state.
	LocalRunFailedNoEffect LocalMaterializationRunOutcome = "failed-no-effect"
)

// UnmarshalJSON accepts only the three outcomes recorded by the Rust SDK.
func (outcome *LocalMaterializationRunOutcome) UnmarshalJSON(data []byte) error {
	return decodeTransportTag(data, outcome, LocalRunSucceeded, LocalRunPartial, LocalRunFailedNoEffect)
}

// LocalMaterializationTargetOutcome reports one attempted repository's result.
type LocalMaterializationTargetOutcome string

const (
	// LocalTargetSucceeded means the target has an immutable success receipt.
	LocalTargetSucceeded LocalMaterializationTargetOutcome = "succeeded"
	// LocalTargetFailedNoEffect excludes retained materialization effects, not
	// the persistent coordination state described in the target's diagnostic.
	LocalTargetFailedNoEffect LocalMaterializationTargetOutcome = "failed-no-effect"
)

// UnmarshalJSON accepts only the two per-target outcomes recorded by the Rust SDK.
func (outcome *LocalMaterializationTargetOutcome) UnmarshalJSON(data []byte) error {
	return decodeTransportTag(data, outcome, LocalTargetSucceeded, LocalTargetFailedNoEffect)
}

// LocalRunTargetResult is one attempted repository settlement.
type LocalRunTargetResult struct {
	RepositoryID           string                            `json:"repositoryId"`
	Outcome                LocalMaterializationTargetOutcome `json:"outcome"`
	ChangedSurfaceCount    uint32                            `json:"changedSurfaceCount"`
	DeferredEffectCount    uint32                            `json:"deferredEffectCount"`
	LocalGitExcludeChanged bool                              `json:"localGitExcludeChanged"`
	// Failure contains canonical diagnostic bytes, or nil when no failure was recorded.
	Failure []byte `json:"failure"`
}

// UnmarshalJSON checks the complete target projection without re-evaluating its outcome.
func (target *LocalRunTargetResult) UnmarshalJSON(data []byte) error {
	type plain LocalRunTargetResult
	var next plain
	if err := decodeTransportObject(data, &next, []string{
		"repositoryId", "outcome", "changedSurfaceCount", "deferredEffectCount", "localGitExcludeChanged",
	}, "failure"); err != nil {
		return err
	}
	*target = LocalRunTargetResult(next)
	return nil
}

// LocalRunResult contains recorded outcomes and the canonical run-result bytes.
// A nil Settle error means this result was recorded, not that every target succeeded.
type LocalRunResult struct {
	Kind    PlanKind                       `json:"kind"`
	Outcome LocalMaterializationRunOutcome `json:"outcome"`
	// TargetCount counts recorded target rows, excluding repositories not attempted.
	TargetCount          uint32                 `json:"targetCount"`
	SucceededTargetCount uint32                 `json:"succeededTargetCount"`
	Targets              []LocalRunTargetResult `json:"targets"`
	Canonical            []byte                 `json:"-"`
}

// UnmarshalJSON checks complete metadata; Settle supplies the separate canonical bytes.
func (result *LocalRunResult) UnmarshalJSON(data []byte) error {
	type plain LocalRunResult
	var next plain
	if err := decodeTransportObject(data, &next, []string{
		"kind", "outcome", "targetCount", "succeededTargetCount", "targets",
	}); err != nil {
		return err
	}
	*result = LocalRunResult(next)
	return nil
}

// Settle performs the admitted repository-scoped effects. An error may carry
// outcome-unknown policy; retain this run for Reconciliation in that case.
func (run *LocalRun) Settle() (LocalRunResult, error) {
	if run == nil {
		return LocalRunResult{}, &BridgeError{Code: "invalid_handle", Message: "opaque Pale value is unavailable or consumed"}
	}
	if err := run.valid(); err != nil {
		return LocalRunResult{}, err
	}
	result, err := run.client.call(struct {
		Run uint64 `json:"run"`
	}{run.id()}, opSettleLocalMaterializationRun)
	if err != nil {
		return LocalRunResult{}, err
	}
	var output LocalRunResult
	if err := decodeJSON(result.metadata, &output); err != nil {
		return LocalRunResult{}, err
	}
	output.Canonical = result.payload
	return output, nil
}

// LocalReconciliation is bounded read-back after an outcome-unknown settlement.
type LocalReconciliation struct {
	RepositoryID string `json:"repositoryId"`
	PlanDigest   string `json:"planDigest"`
	Receipt      string `json:"receipt"`
	Ownership    string `json:"ownership"`
}

// Reconciliation inspects the exact receipt slot and independent ownership relation.
func (run *LocalRun) Reconciliation() (LocalReconciliation, error) {
	if run == nil {
		return LocalReconciliation{}, &BridgeError{Code: "invalid_handle", Message: "opaque Pale value is unavailable or consumed"}
	}
	if err := run.valid(); err != nil {
		return LocalReconciliation{}, err
	}
	result, err := run.client.call(struct {
		Run uint64 `json:"run"`
	}{run.id()}, opInspectLocalReconciliation)
	if err != nil {
		return LocalReconciliation{}, err
	}
	var observation LocalReconciliation
	if err := decodeJSON(result.metadata, &observation); err != nil {
		return LocalReconciliation{}, err
	}
	return observation, nil
}

// CompiledInterfaceIdentity reports SDK version, protocol and target claims.
// Its canonical handshake record does not establish executable bytes or health.
type CompiledInterfaceIdentity struct {
	Version            string `json:"version"`
	Protocol           string `json:"protocol"`
	OS                 string `json:"os"`
	Arch               string `json:"arch"`
	DurablePublication string `json:"durablePublication"`
	Canonical          []byte `json:"-"`
}

// CompiledInterfaceIdentity returns compiled claims without reading an executable.
func (client *Client) CompiledInterfaceIdentity() (CompiledInterfaceIdentity, error) {
	return client.compiledInterfaceIdentity(struct{}{}, opCompiledInterfaceIdentity)
}

// VerifyCurrentExecutableBinding checks this host executable against the captured
// workspace binding. It does not verify the loaded native library's bytes, observe
// resident state, or establish process health. Success returns the compiled claims.
func (compilation *LocalCompilation) VerifyCurrentExecutableBinding() (CompiledInterfaceIdentity, error) {
	if compilation == nil {
		return CompiledInterfaceIdentity{}, &BridgeError{Code: "invalid_handle", Message: "opaque Pale value is unavailable or consumed"}
	}
	if err := compilation.valid(); err != nil {
		return CompiledInterfaceIdentity{}, err
	}
	return compilation.client.compiledInterfaceIdentity(struct {
		Compilation uint64 `json:"compilation"`
	}{compilation.id()}, opVerifyCurrentExecutableBinding)
}

func (client *Client) compiledInterfaceIdentity(request any, operation operation) (CompiledInterfaceIdentity, error) {
	result, err := client.call(request, operation)
	if err != nil {
		return CompiledInterfaceIdentity{}, err
	}
	var identity CompiledInterfaceIdentity
	if err := decodeJSON(result.metadata, &identity); err != nil {
		return CompiledInterfaceIdentity{}, err
	}
	identity.Canonical = result.payload
	return identity, nil
}
