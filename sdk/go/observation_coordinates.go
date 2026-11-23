package pale

// ObservationGenerationKind separates immutable profile meaning from resident authority.
type ObservationGenerationKind string

const (
	ObservationEffectiveProfileGeneration ObservationGenerationKind = "effective-profile"
	ObservationResidentGeneration         ObservationGenerationKind = "resident"
)

// ObservationGeneration copies exact SDK coordinates; it grants no handle or authority.
// Only the fields belonging to Kind are populated. Resident absence is explicit.
type ObservationGeneration struct {
	Kind                   ObservationGenerationKind `json:"kind"`
	ProfileDigest          string                    `json:"profileDigest,omitempty"`
	InstanceDigest         string                    `json:"instanceDigest,omitempty"`
	CompositionPlanDigest  string                    `json:"compositionPlanDigest,omitempty"`
	ControlStoreID         string                    `json:"controlStoreId,omitempty"`
	ControlIncarnationID   string                    `json:"controlIncarnationId,omitempty"`
	ActiveGenerationDigest *string                   `json:"activeGenerationDigest,omitempty"`
}

// UnmarshalJSON rejects unknown or mixed generation payloads atomically.
func (value *ObservationGeneration) UnmarshalJSON(data []byte) error {
	kind, err := transportVariant(data, "kind", map[string][]string{
		"effective-profile": {"profileDigest", "instanceDigest", "compositionPlanDigest"},
		"resident":          {"controlStoreId", "controlIncarnationId", "activeGenerationDigest"},
	})
	if err != nil {
		return err
	}
	type plain ObservationGeneration
	var next plain
	required := []string{"profileDigest", "instanceDigest", "compositionPlanDigest"}
	if kind == "resident" {
		required = []string{"controlStoreId", "controlIncarnationId"}
	}
	if err := decodeObservationObject(data, &next, required...); err != nil {
		return err
	}
	*value = ObservationGeneration(next)
	return nil
}

// ObservationRevisionKind identifies the revision domain, not process liveness.
type ObservationRevisionKind string

const (
	ObservationStaticRevision          ObservationRevisionKind = "static"
	ObservationResidentJournalRevision ObservationRevisionKind = "resident-journal"
)

// ObservationRevision retains an exact uint64 sequence, absent before the first append.
type ObservationRevision struct {
	Kind     ObservationRevisionKind `json:"kind"`
	Sequence *uint64                 `json:"sequence,omitempty"`
}

// UnmarshalJSON preserves absent journal heads without accepting foreign fields.
func (value *ObservationRevision) UnmarshalJSON(data []byte) error {
	if _, err := transportVariant(data, "kind", map[string][]string{
		"static": {}, "resident-journal": {"sequence"},
	}); err != nil {
		return err
	}
	type plain ObservationRevision
	var next plain
	if err := decodeObservationObject(data, &next, "kind"); err != nil {
		return err
	}
	*value = ObservationRevision(next)
	return nil
}

// ObservationCursor is inspectable coordinate data, never a fabricated request handle.
type ObservationCursor struct {
	Generation ObservationGeneration `json:"generation"`
	Revision   ObservationRevision   `json:"revision"`
}

// UnmarshalJSON requires both independently typed coordinates.
func (value *ObservationCursor) UnmarshalJSON(data []byte) error {
	type plain ObservationCursor
	var next plain
	if err := decodeTransportObject(data, &next, []string{"generation", "revision"}); err != nil {
		return err
	}
	*value = ObservationCursor(next)
	return nil
}

// ObservationResumeCoordinate records the last completely applied stream cursor.
type ObservationResumeCoordinate struct {
	Cursor ObservationCursor `json:"cursor"`
}

// UnmarshalJSON rejects an absent cursor without changing the receiver.
func (value *ObservationResumeCoordinate) UnmarshalJSON(data []byte) error {
	type plain ObservationResumeCoordinate
	var next plain
	if err := decodeTransportObject(data, &next, []string{"cursor"}); err != nil {
		return err
	}
	*value = ObservationResumeCoordinate(next)
	return nil
}

// ObservationSubjectKind names the SDK's closed nominal subject vocabulary.
type ObservationSubjectKind string

const (
	ObservationEffectiveProfileSubject      ObservationSubjectKind = "effective-profile"
	ObservationMachineSubject               ObservationSubjectKind = "machine"
	ObservationRepositorySubject            ObservationSubjectKind = "repository"
	ObservationOperationSubject             ObservationSubjectKind = "operation"
	ObservationCapabilityRequirementSubject ObservationSubjectKind = "capability-requirement"
	ObservationExecutionPathSubject         ObservationSubjectKind = "execution-path"
	ObservationCompiledGenerationSubject    ObservationSubjectKind = "compiled-generation"
	ObservationResidentProcessSubject       ObservationSubjectKind = "resident-process"
)

// ObservationSubjectValue is inspectable data. Only an ObservationSubject handle
// obtained from Page can be supplied back to the native SDK.
type ObservationSubjectValue struct {
	Kind                     ObservationSubjectKind `json:"kind"`
	ProfileDigest            string                 `json:"profileDigest,omitempty"`
	MachineID                string                 `json:"machineId,omitempty"`
	RepositoryID             string                 `json:"repositoryId,omitempty"`
	OperationID              string                 `json:"operationId,omitempty"`
	RequirementID            string                 `json:"requirementId,omitempty"`
	CapabilityID             string                 `json:"capabilityId,omitempty"`
	ExecutionPathID          string                 `json:"executionPathId,omitempty"`
	CompiledGenerationDigest string                 `json:"compiledGenerationDigest,omitempty"`
}

// UnmarshalJSON checks the selected subject payload without admitting coordinates.
func (value *ObservationSubjectValue) UnmarshalJSON(data []byte) error {
	variants := map[string][]string{
		"effective-profile": {"profileDigest"}, "machine": {"machineId"},
		"repository": {"repositoryId"}, "operation": {"repositoryId", "operationId"},
		"capability-requirement": {"repositoryId", "operationId", "requirementId", "capabilityId"},
		"execution-path":         {"executionPathId"}, "compiled-generation": {"compiledGenerationDigest"},
		"resident-process": {},
	}
	kind, err := transportVariant(data, "kind", variants)
	if err != nil {
		return err
	}
	type plain ObservationSubjectValue
	var next plain
	if err := decodeObservationObject(data, &next, variants[kind]...); err != nil {
		return err
	}
	*value = ObservationSubjectValue(next)
	return nil
}

// ObservationFreshness qualifies evidence currency, independently of readiness.
type ObservationFreshness struct {
	Kind            ObservationFreshnessClass `json:"kind"`
	RevisionsBehind uint64                    `json:"revisionsBehind,omitempty"`
}

// UnmarshalJSON requires an exact integer lag only for stale evidence.
func (value *ObservationFreshness) UnmarshalJSON(data []byte) error {
	kind, err := transportVariant(data, "kind", map[string][]string{
		"fresh": {}, "unknown": {}, "stale": {"revisionsBehind"},
	})
	if err != nil {
		return err
	}
	type plain ObservationFreshness
	var next plain
	required := []string{"kind"}
	if kind == "stale" {
		required = append(required, "revisionsBehind")
	}
	if err := decodeObservationObject(data, &next, required...); err != nil {
		return err
	}
	*value = ObservationFreshness(next)
	return nil
}

// ObservationSnapshotSummary carries coordinates and bounded cardinality, not live status.
type ObservationSnapshotSummary struct {
	Cursor           ObservationCursor `json:"cursor"`
	FactCount        uint32            `json:"factCount"`
	OmittedFactCount uint32            `json:"omittedFactCount"`
}

// UnmarshalJSON requires the complete SDK summary atomically.
func (value *ObservationSnapshotSummary) UnmarshalJSON(data []byte) error {
	type plain ObservationSnapshotSummary
	var next plain
	if err := decodeTransportObject(data, &next, []string{"cursor", "factCount", "omittedFactCount"}); err != nil {
		return err
	}
	*value = ObservationSnapshotSummary(next)
	return nil
}
