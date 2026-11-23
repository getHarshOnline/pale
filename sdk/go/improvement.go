package pale

import "encoding/json"

// ConfigurationImprovementPhase is the API-owned current checkpoint.
type ConfigurationImprovementPhase string

const (
	ImprovementInspected ConfigurationImprovementPhase = "inspected"
	ImprovementExplained ConfigurationImprovementPhase = "explained"
	ImprovementProposed  ConfigurationImprovementPhase = "proposed"
	ImprovementEvaluated ConfigurationImprovementPhase = "evaluated"
	ImprovementDiagnosed ConfigurationImprovementPhase = "diagnosed"
	ImprovementPreviewed ConfigurationImprovementPhase = "previewed"
)

// UnmarshalJSON rejects unknown phase labels without changing the receiver.
func (phase *ConfigurationImprovementPhase) UnmarshalJSON(bytes []byte) error {
	var candidate string
	if err := json.Unmarshal(bytes, &candidate); err != nil {
		return err
	}
	switch ConfigurationImprovementPhase(candidate) {
	case ImprovementInspected, ImprovementExplained, ImprovementProposed, ImprovementEvaluated, ImprovementDiagnosed, ImprovementPreviewed:
		*phase = ConfigurationImprovementPhase(candidate)
		return nil
	default:
		return &BridgeError{Code: "invalid_response_metadata", Message: "unknown configuration improvement phase"}
	}
}

// ConfigurationImprovementInspectionState projects exact coordinates of one retained inspection.
type ConfigurationImprovementInspectionState struct {
	Phase                   ConfigurationImprovementPhase `json:"phase"`
	WorkspaceID             string                        `json:"workspace_id"`
	PrincipalUniverseDigest string                        `json:"principal_universe_digest"`
}

// State inspects this retained checkpoint without advancing or compiling it.
func (stage *ConfigurationImprovementInspection) State() (ConfigurationImprovementInspectionState, error) {
	if stage == nil {
		return ConfigurationImprovementInspectionState{}, invalidImprovementHandle()
	}
	if err := stage.valid(); err != nil {
		return ConfigurationImprovementInspectionState{}, err
	}
	result, err := stage.client.call(struct {
		Handle uint64 `json:"handle"`
	}{stage.id()}, opImprovementInspectionState)
	if err != nil {
		return ConfigurationImprovementInspectionState{}, err
	}
	return decodeMetadata[ConfigurationImprovementInspectionState](result)
}

// ConfigurationImprovementExplanationState projects exact coordinates of one retained explanation.
type ConfigurationImprovementExplanationState struct {
	Phase          ConfigurationImprovementPhase `json:"phase"`
	ProfileDigest  string                        `json:"profile_digest"`
	InstanceDigest string                        `json:"instance_digest"`
}

// State inspects this retained checkpoint without advancing or compiling it.
func (stage *ConfigurationImprovementExplanation) State() (ConfigurationImprovementExplanationState, error) {
	if stage == nil {
		return ConfigurationImprovementExplanationState{}, invalidImprovementHandle()
	}
	if err := stage.valid(); err != nil {
		return ConfigurationImprovementExplanationState{}, err
	}
	result, err := stage.client.call(struct {
		Handle uint64 `json:"handle"`
	}{stage.id()}, opImprovementExplanationState)
	if err != nil {
		return ConfigurationImprovementExplanationState{}, err
	}
	return decodeMetadata[ConfigurationImprovementExplanationState](result)
}

// ConfigurationImprovementEvaluationState projects exact coordinates of one retained evaluation.
type ConfigurationImprovementEvaluationState struct {
	Phase                            ConfigurationImprovementPhase `json:"phase"`
	CandidatePrincipalUniverseDigest string                        `json:"candidate_principal_universe_digest"`
	CandidateProfileDigest           string                        `json:"candidate_profile_digest"`
	CandidateInstanceDigest          string                        `json:"candidate_instance_digest"`
}

// State inspects this retained checkpoint without advancing or compiling it.
func (stage *ConfigurationImprovementEvaluation) State() (ConfigurationImprovementEvaluationState, error) {
	if stage == nil {
		return ConfigurationImprovementEvaluationState{}, invalidImprovementHandle()
	}
	if err := stage.valid(); err != nil {
		return ConfigurationImprovementEvaluationState{}, err
	}
	result, err := stage.client.call(struct {
		Handle uint64 `json:"handle"`
	}{stage.id()}, opImprovementEvaluationState)
	if err != nil {
		return ConfigurationImprovementEvaluationState{}, err
	}
	return decodeMetadata[ConfigurationImprovementEvaluationState](result)
}

// ConfigurationImprovementDiagnosisState projects exact coordinates of one retained diagnosis.
type ConfigurationImprovementDiagnosisState struct {
	Phase                   ConfigurationImprovementPhase `json:"phase"`
	DerivationReceiptDigest string                        `json:"derivation_receipt_digest"`
}

// State inspects this retained checkpoint without advancing or compiling it.
func (stage *ConfigurationImprovementDiagnosis) State() (ConfigurationImprovementDiagnosisState, error) {
	if stage == nil {
		return ConfigurationImprovementDiagnosisState{}, invalidImprovementHandle()
	}
	if err := stage.valid(); err != nil {
		return ConfigurationImprovementDiagnosisState{}, err
	}
	result, err := stage.client.call(struct {
		Handle uint64 `json:"handle"`
	}{stage.id()}, opImprovementDiagnosisState)
	if err != nil {
		return ConfigurationImprovementDiagnosisState{}, err
	}
	return decodeMetadata[ConfigurationImprovementDiagnosisState](result)
}

// Phase reads the API-owned phase only while this handle is retained.
func (stage *ConfigurationImprovementProposal) Phase() (ConfigurationImprovementPhase, error) {
	if stage == nil {
		return "", invalidImprovementHandle()
	}
	if err := stage.valid(); err != nil {
		return "", err
	}
	result, err := stage.client.call(struct {
		Handle uint64 `json:"handle"`
	}{stage.id()}, opImprovementProposalPhase)
	if err != nil {
		return "", err
	}
	return decodeMetadata[ConfigurationImprovementPhase](result)
}

// Phase reads the API-owned phase only while this handle is retained.
func (stage *ConfigurationImprovementPreview) Phase() (ConfigurationImprovementPhase, error) {
	if stage == nil {
		return "", invalidImprovementHandle()
	}
	if err := stage.valid(); err != nil {
		return "", err
	}
	result, err := stage.client.call(struct {
		Handle uint64 `json:"handle"`
	}{stage.id()}, opImprovementPreviewPhase)
	if err != nil {
		return "", err
	}
	return decodeMetadata[ConfigurationImprovementPhase](result)
}

// String omits captured private source and topology selectors.
func (ConfigurationImprovementCandidateRequest) String() string {
	return "ConfigurationImprovementCandidateRequest(<redacted>)"
}

// GoString preserves redaction with Go-syntax formatting.
func (request ConfigurationImprovementCandidateRequest) GoString() string { return request.String() }

// String omits candidate source and private configuration.
func (ConfigurationImprovementProposalRequest) String() string {
	return "ConfigurationImprovementProposalRequest(<redacted>)"
}

// GoString preserves redaction with Go-syntax formatting.
func (request ConfigurationImprovementProposalRequest) GoString() string { return request.String() }

// ConfigurationImprovementCandidateRequest selects the exact private/profile
// context for a candidate. The API validates it against the explained baseline;
// this request carries no Package source or authority.
type ConfigurationImprovementCandidateRequest struct {
	ProjectID            string
	ExecutionWorkspaceID string
	MachineID            string
	PrivateInputs        PrivateInputs
}

// ConfigurationImprovementProposalRequest supplies source for the consuming
// proposal route. Use Prepare with an admitted workspace to avoid reparsing it.
type ConfigurationImprovementProposalRequest struct {
	Candidate ConfigurationImprovementCandidateRequest
	Workspace PortableWorkspaceRequest
}

// ConfigurationImprovementInspection retains an inspected baseline.
// Copies share consumption state. Closing its Client drops it without an effect.
type ConfigurationImprovementInspection struct{ handle }

// ConfigurationImprovementExplanation retains a baseline joined to its exact profile.
// Copies share consumption state. Closing its Client drops it without an effect.
type ConfigurationImprovementExplanation struct{ handle }

// ConfigurationImprovementProposal retains an unadmitted source proposal.
// Copies share consumption state. Closing its Client drops it without an effect.
type ConfigurationImprovementProposal struct{ handle }

// ConfigurationImprovementEvaluation retains an admitted candidate.
// Copies share consumption state. Closing its Client drops it without an effect.
type ConfigurationImprovementEvaluation struct{ handle }

// ConfigurationImprovementDiagnosis retains a candidate with canonical impact proof.
// Copies share consumption state. Closing its Client drops it without an effect.
type ConfigurationImprovementDiagnosis struct{ handle }

// ConfigurationImprovementPreparedPreview retains a prepared candidate still requiring its exact explanation.
// Copies share consumption state. Closing its Client drops it without an effect.
type ConfigurationImprovementPreparedPreview struct{ handle }

// ConfigurationImprovementPreview retains a frozen human-review candidate.
// Copies share consumption state. Closing its Client drops it without an effect.
type ConfigurationImprovementPreview struct{ handle }

// InspectConfigurationImprovement captures an admitted baseline without consuming it.
func (workspace *PortableWorkspace) InspectConfigurationImprovement() (*ConfigurationImprovementInspection, error) {
	if workspace == nil {
		return nil, &BridgeError{Code: "invalid_handle", Message: "opaque Pale value is unavailable or consumed"}
	}
	if err := workspace.valid(); err != nil {
		return nil, err
	}
	result, err := workspace.client.call(struct {
		Handle uint64 `json:"handle"`
	}{workspace.id()}, opImprovementInspect)
	if err != nil {
		return nil, err
	}
	return &ConfigurationImprovementInspection{newHandle(workspace.client, result.object)}, nil
}

// Explain consumes the inspection and joins the exact API-owned profile.
// API rejection consumes this stage; a cross-client argument is rejected first.
func (inspection *ConfigurationImprovementInspection) Explain(profile *EffectiveProfile) (*ConfigurationImprovementExplanation, error) {
	if inspection == nil || profile == nil || inspection.client == nil {
		return nil, invalidImprovementHandle()
	}
	client := inspection.client
	client.mu.Lock()
	defer client.mu.Unlock()
	if err := sameImprovementClient(&inspection.handle, &profile.handle); err != nil {
		return nil, err
	}
	inspectionID := inspection.id()
	result, err := client.callLocked(struct {
		Handle  uint64 `json:"handle"`
		Profile uint64 `json:"profile"`
	}{inspectionID, profile.id()}, func(client, request uint64) uint64 {
		inspection.consume()
		// Raw C preflight may preserve the stage; Go has explicitly surrendered it.
		defer dropImprovementInspection(client, inspectionID)
		return opImprovementExplain(client, request)
	})
	if err != nil {
		return nil, err
	}
	return &ConfigurationImprovementExplanation{newHandle(inspection.client, result.object)}, nil
}

// Propose consumes this explanation and captures a bounded source candidate.
// Capture/admission errors do not leave a reusable explanation on this route.
func (explanation *ConfigurationImprovementExplanation) Propose(request ConfigurationImprovementProposalRequest) (*ConfigurationImprovementProposal, error) {
	if explanation == nil || explanation.client == nil {
		return nil, invalidImprovementHandle()
	}
	client := explanation.client
	client.mu.Lock()
	defer client.mu.Unlock()
	if err := explanation.valid(); err != nil {
		return nil, err
	}
	candidate := request.Candidate
	explanationID := explanation.id()
	result, err := client.callLocked(struct {
		Handle               uint64                   `json:"handle"`
		ProjectID            string                   `json:"projectId"`
		ExecutionWorkspaceID string                   `json:"executionWorkspaceId"`
		MachineID            string                   `json:"machineId"`
		Workspace            PortableWorkspaceRequest `json:"workspace"`
		PrivateInputs        privateInputs            `json:"privateInputs"`
	}{explanationID, candidate.ProjectID, candidate.ExecutionWorkspaceID, candidate.MachineID, request.Workspace, candidate.PrivateInputs.wire()}, func(client, request uint64) uint64 {
		explanation.consume()
		// Even a private request-decoding failure must not strand a surrendered input.
		defer dropImprovementExplanation(client, explanationID)
		return opImprovementPropose(client, request)
	})
	if err != nil {
		return nil, err
	}
	return &ConfigurationImprovementProposal{newHandle(explanation.client, result.object)}, nil
}

// Evaluate consumes this stage through the canonical API transition, including on API rejection.
func (prior *ConfigurationImprovementProposal) Evaluate() (*ConfigurationImprovementEvaluation, error) {
	if prior == nil || prior.client == nil {
		return nil, invalidImprovementHandle()
	}
	client := prior.client
	client.mu.Lock()
	defer client.mu.Unlock()
	if err := prior.valid(); err != nil {
		return nil, err
	}
	result, err := client.callLocked(struct {
		Handle uint64 `json:"handle"`
	}{prior.id()}, func(client, request uint64) uint64 {
		prior.consume()
		return opImprovementEvaluate(client, request)
	})
	if err != nil {
		return nil, err
	}
	return &ConfigurationImprovementEvaluation{newHandle(prior.client, result.object)}, nil
}

// Diagnose consumes this stage through the canonical API transition, including on API rejection.
func (prior *ConfigurationImprovementEvaluation) Diagnose() (*ConfigurationImprovementDiagnosis, error) {
	if prior == nil || prior.client == nil {
		return nil, invalidImprovementHandle()
	}
	client := prior.client
	client.mu.Lock()
	defer client.mu.Unlock()
	if err := prior.valid(); err != nil {
		return nil, err
	}
	result, err := client.callLocked(struct {
		Handle uint64 `json:"handle"`
	}{prior.id()}, func(client, request uint64) uint64 {
		prior.consume()
		return opImprovementDiagnose(client, request)
	})
	if err != nil {
		return nil, err
	}
	return &ConfigurationImprovementDiagnosis{newHandle(prior.client, result.object)}, nil
}

// Preview consumes this stage through the canonical API transition, including on API rejection.
func (prior *ConfigurationImprovementDiagnosis) Preview() (*ConfigurationImprovementPreview, error) {
	if prior == nil || prior.client == nil {
		return nil, invalidImprovementHandle()
	}
	client := prior.client
	client.mu.Lock()
	defer client.mu.Unlock()
	if err := prior.valid(); err != nil {
		return nil, err
	}
	result, err := client.callLocked(struct {
		Handle uint64 `json:"handle"`
	}{prior.id()}, func(client, request uint64) uint64 {
		prior.consume()
		return opImprovementPreview(client, request)
	})
	if err != nil {
		return nil, err
	}
	return &ConfigurationImprovementPreview{newHandle(prior.client, result.object)}, nil
}

// Prepare borrows this explanation and an already admitted candidate.
// Failed preparation leaves both handles available for retry. No portable
// documents are parsed; the API compiles the candidate's private profile once.
func (explanation *ConfigurationImprovementExplanation) Prepare(candidate *PortableWorkspace, request ConfigurationImprovementCandidateRequest) (*ConfigurationImprovementPreparedPreview, error) {
	if explanation == nil || candidate == nil {
		return nil, invalidImprovementHandle()
	}
	if err := sameImprovementClient(&explanation.handle, &candidate.handle); err != nil {
		return nil, err
	}
	result, err := explanation.client.call(struct {
		Handle               uint64        `json:"handle"`
		Candidate            uint64        `json:"candidate"`
		ProjectID            string        `json:"projectId"`
		ExecutionWorkspaceID string        `json:"executionWorkspaceId"`
		MachineID            string        `json:"machineId"`
		PrivateInputs        privateInputs `json:"privateInputs"`
	}{explanation.id(), candidate.id(), request.ProjectID, request.ExecutionWorkspaceID, request.MachineID, request.PrivateInputs.wire()}, opImprovementPrepare)
	if err != nil {
		return nil, err
	}
	return &ConfigurationImprovementPreparedPreview{newHandle(explanation.client, result.object)}, nil
}

// Commit consumes both this explanation and a preparation. Equal coordinates
// from another explanation do not establish owner equality: the API checks the
// exact preparation owner. An API owner-mismatch consumes both supplied values.
// A concurrent caller that cannot acquire both values consumes neither.
func (explanation *ConfigurationImprovementExplanation) Commit(prepared *ConfigurationImprovementPreparedPreview) (*ConfigurationImprovementPreview, error) {
	if explanation == nil || prepared == nil || explanation.client == nil {
		return nil, invalidImprovementHandle()
	}
	client := explanation.client
	client.mu.Lock()
	defer client.mu.Unlock()
	if err := sameImprovementClient(&explanation.handle, &prepared.handle); err != nil {
		return nil, err
	}
	explanationID, preparedID := explanation.id(), prepared.id()
	result, err := client.callLocked(struct {
		Handle   uint64 `json:"handle"`
		Prepared uint64 `json:"prepared"`
	}{explanationID, preparedID}, func(client, request uint64) uint64 {
		// The client lock makes this pair atomic with every competing consumer.
		explanation.consume()
		prepared.consume()
		defer dropImprovementExplanation(client, explanationID)
		defer dropImprovementPrepared(client, preparedID)
		return opImprovementCommit(client, request)
	})
	if err != nil {
		return nil, err
	}
	return &ConfigurationImprovementPreview{newHandle(explanation.client, result.object)}, nil
}

// Summary returns exact API-owned candidate coordinates before commit.
func (prepared *ConfigurationImprovementPreparedPreview) Summary() (CanonicalDocument, error) {
	if prepared == nil {
		return CanonicalDocument{}, invalidImprovementHandle()
	}
	return improvementDocument(&prepared.handle, opImprovementPreparedSummary)
}

// Summary returns exact retained preview coordinates and impact counts.
func (preview *ConfigurationImprovementPreview) Summary() (CanonicalDocument, error) {
	if preview == nil {
		return CanonicalDocument{}, invalidImprovementHandle()
	}
	return improvementDocument(&preview.handle, opImprovementSummary)
}

// ComparisonSummary returns the canonical comparison owner's coordinates and counts.
func (preview *ConfigurationImprovementPreview) ComparisonSummary() (CanonicalDocument, error) {
	if preview == nil {
		return CanonicalDocument{}, invalidImprovementHandle()
	}
	return improvementDocument(&preview.handle, opImprovementComparisonSummary)
}

// ChangeRoots pages the retained canonical change-root proof.
func (preview *ConfigurationImprovementPreview) ChangeRoots(offset uint32, limit uint16) (CanonicalDocument, error) {
	return preview.page(offset, limit, opImprovementChangeRoots)
}

// Impacts pages the retained canonical impact partition.
func (preview *ConfigurationImprovementPreview) Impacts(offset uint32, limit uint16) (CanonicalDocument, error) {
	return preview.page(offset, limit, opImprovementImpacts)
}

func (preview *ConfigurationImprovementPreview) page(offset uint32, limit uint16, invoke operation) (CanonicalDocument, error) {
	if preview == nil {
		return CanonicalDocument{}, invalidImprovementHandle()
	}
	if err := preview.valid(); err != nil {
		return CanonicalDocument{}, err
	}
	result, err := preview.client.call(struct {
		Handle uint64 `json:"handle"`
		Offset uint32 `json:"offset"`
		Limit  uint16 `json:"limit"`
	}{preview.id(), offset, limit}, invoke)
	return CanonicalDocument{Bytes: result.payload}, err
}

// Explain queries the retained comparison proof without compiling again.
func (preview *ConfigurationImprovementPreview) Explain(subject WorkspaceSubject) (CanonicalDocument, error) {
	if preview == nil {
		return CanonicalDocument{}, invalidImprovementHandle()
	}
	if err := preview.valid(); err != nil {
		return CanonicalDocument{}, err
	}
	result, err := preview.client.call(struct {
		Handle  uint64           `json:"handle"`
		Subject WorkspaceSubject `json:"subject"`
	}{preview.id(), subject}, opImprovementExplainComparison)
	return CanonicalDocument{Bytes: result.payload}, err
}

// ConfigurationImprovementProfile selects a retained profile, not a native observation.
type ConfigurationImprovementProfile string

const (
	ImprovementBaselineProfile  ConfigurationImprovementProfile = "baseline"
	ImprovementCandidateProfile ConfigurationImprovementProfile = "candidate"
)

// InspectProfile pages either exact retained profile without a second compilation.
func (preview *ConfigurationImprovementPreview) InspectProfile(side ConfigurationImprovementProfile, section EffectiveProfileSection, offset uint32, limit uint16) (CanonicalDocument, error) {
	if preview == nil {
		return CanonicalDocument{}, invalidImprovementHandle()
	}
	if err := preview.valid(); err != nil {
		return CanonicalDocument{}, err
	}
	result, err := preview.client.call(struct {
		Handle  uint64                          `json:"handle"`
		Side    ConfigurationImprovementProfile `json:"side"`
		Section EffectiveProfileSection         `json:"section"`
		Offset  uint32                          `json:"offset"`
		Limit   uint16                          `json:"limit"`
	}{preview.id(), side, section, offset, limit}, opImprovementProfilePage)
	return CanonicalDocument{Bytes: result.payload}, err
}

// ExplainProfile resolves an explanation against the selected retained profile.
// A valid unmatched handle returns nil. This copied evidence authorizes no effect.
func (preview *ConfigurationImprovementPreview) ExplainProfile(side ConfigurationImprovementProfile, explanation string) (*EffectiveProfileExplanation, error) {
	if preview == nil {
		return nil, invalidImprovementHandle()
	}
	if err := preview.valid(); err != nil {
		return nil, err
	}
	result, err := preview.client.call(struct {
		Handle      uint64                          `json:"handle"`
		Side        ConfigurationImprovementProfile `json:"side"`
		Explanation string                          `json:"explanation"`
	}{preview.id(), side, explanation}, opImprovementExplainProfile)
	if err != nil {
		return nil, err
	}
	return decodeEffectiveProfileExplanation(result, explanation)
}

// ConfigurationImprovementDisposition is a human-governed, effect-free review result.
type ConfigurationImprovementDisposition string

const (
	ImprovementAccepted  ConfigurationImprovementDisposition = "accepted"
	ImprovementRejected  ConfigurationImprovementDisposition = "rejected"
	ImprovementCancelled ConfigurationImprovementDisposition = "cancelled"
	ImprovementStale     ConfigurationImprovementDisposition = "stale"
)

// ConfigurationImprovementTerminal retains the exact preview governed by a
// disposition. Acceptance grants no write, activation or runtime authority.
type ConfigurationImprovementTerminal struct {
	Disposition ConfigurationImprovementDisposition
	Preview     CanonicalDocument
}

// Accept consumes the preview and accepts for presentation to a separate authority/effect gate. It performs no mutation.
func (preview *ConfigurationImprovementPreview) Accept() (ConfigurationImprovementTerminal, error) {
	return preview.finish(opImprovementAccept)
}

// Reject consumes the preview and rejects the candidate. It performs no mutation.
func (preview *ConfigurationImprovementPreview) Reject() (ConfigurationImprovementTerminal, error) {
	return preview.finish(opImprovementReject)
}

// Cancel consumes the preview and cancels this review, not an external operation. It performs no mutation.
func (preview *ConfigurationImprovementPreview) Cancel() (ConfigurationImprovementTerminal, error) {
	return preview.finish(opImprovementCancel)
}

// MarkStale consumes the preview and marks this review stale after the caller establishes outer-coordinate drift. It performs no mutation.
func (preview *ConfigurationImprovementPreview) MarkStale() (ConfigurationImprovementTerminal, error) {
	return preview.finish(opImprovementMarkStale)
}

func (preview *ConfigurationImprovementPreview) finish(invoke operation) (ConfigurationImprovementTerminal, error) {
	if preview == nil || preview.client == nil {
		return ConfigurationImprovementTerminal{}, invalidImprovementHandle()
	}
	client := preview.client
	client.mu.Lock()
	defer client.mu.Unlock()
	if err := preview.valid(); err != nil {
		return ConfigurationImprovementTerminal{}, err
	}
	result, err := client.callLocked(struct {
		Handle uint64 `json:"handle"`
	}{preview.id()}, func(client, request uint64) uint64 {
		preview.consume()
		return invoke(client, request)
	})
	if err != nil {
		return ConfigurationImprovementTerminal{}, err
	}
	var disposition ConfigurationImprovementDisposition
	if err := decodeJSON(result.metadata, &disposition); err != nil {
		return ConfigurationImprovementTerminal{}, err
	}
	switch disposition {
	case ImprovementAccepted, ImprovementRejected, ImprovementCancelled, ImprovementStale:
		return ConfigurationImprovementTerminal{disposition, CanonicalDocument{Bytes: result.payload}}, nil
	default:
		return ConfigurationImprovementTerminal{}, &BridgeError{Code: "invalid_response_metadata", Message: "unknown improvement disposition"}
	}
}

func improvementDocument(value *handle, invoke operation) (CanonicalDocument, error) {
	if err := value.valid(); err != nil {
		return CanonicalDocument{}, err
	}
	result, err := value.client.call(struct {
		Handle uint64 `json:"handle"`
	}{value.id()}, invoke)
	return CanonicalDocument{Bytes: result.payload}, err
}

func invalidImprovementHandle() error {
	return &BridgeError{Code: "invalid_handle", Message: "improvement value is unavailable, consumed or belongs to another client"}
}

func sameImprovementClient(first, second *handle) error {
	if err := first.valid(); err != nil {
		return err
	}
	if err := second.valid(); err != nil {
		return err
	}
	if first.client != second.client {
		return invalidImprovementHandle()
	}
	return nil
}
