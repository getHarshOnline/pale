package pale

// AuthoringCorrectionKind identifies the correction proved by the compiler.
type AuthoringCorrectionKind string

const (
	// AuthoringNarrowImportVisibility narrows an import to its selected export ceiling.
	AuthoringNarrowImportVisibility AuthoringCorrectionKind = "narrow-import-visibility"
)

// UnmarshalJSON rejects correction kinds outside the SDK's closed vocabulary.
func (value *AuthoringCorrectionKind) UnmarshalJSON(data []byte) error {
	return decodeTransportTag(data, value, AuthoringNarrowImportVisibility)
}

// AuthoringCorrection copies one compiler-proved source edit. Before applying it,
// a caller must match ExpectedSource at SourceLabel and Path. The diagnostic
// identity correlates evidence; neither it nor ProofSubject grants write authority.
type AuthoringCorrection struct {
	Kind               AuthoringCorrectionKind    `json:"kind"`
	SourceLabel        string                     `json:"source_label"`
	Path               []AuthoringLocationSegment `json:"path"`
	ExpectedSource     string                     `json:"expected_source"`
	ReplacementSource  string                     `json:"replacement_source"`
	ProofSubject       AuthoringIdentityValue     `json:"proof_subject"`
	DiagnosticIdentity string                     `json:"diagnostic_identity"`
}

// UnmarshalJSON requires every SDK field and preserves the receiver on rejection.
func (value *AuthoringCorrection) UnmarshalJSON(data []byte) error {
	type plain AuthoringCorrection
	var next plain
	if err := decodeTransportObject(data, &next, []string{
		"kind", "source_label", "path", "expected_source", "replacement_source",
		"proof_subject", "diagnostic_identity",
	}); err != nil {
		return err
	}
	*value = AuthoringCorrection(next)
	return nil
}
