package pale

// EffectiveAuthoringInspection contains ordered associations retained by the SDK.
// Truncated reports the owner's omitted rows; it is not recomputed by Go.
type EffectiveAuthoringInspection struct {
	Coordinates EffectiveProfileCoordinates `json:"coordinates"`
	Entries     []EffectiveProfileEntry     `json:"entries"`
	Truncated   bool                        `json:"truncated"`
}

// UnmarshalJSON checks exact fields and commits only a complete result.
func (value *EffectiveAuthoringInspection) UnmarshalJSON(data []byte) error {
	type plain EffectiveAuthoringInspection
	var next plain
	if err := decodeTransportObject(data, &next, []string{"coordinates", "entries", "truncated"}); err != nil {
		return err
	}
	*value = EffectiveAuthoringInspection(next)
	return nil
}

// InspectEffectiveAuthoring joins a resolved source identity to retained profile facts.
// It borrows both handles and allocates no retained native result. Entries are
// copied facts; their explanation strings resolve only while the profile is open.
func (profile *EffectiveProfile) InspectEffectiveAuthoring(identity *AuthoringIdentity) (EffectiveAuthoringInspection, error) {
	if profile == nil || identity == nil || identity.client != profile.client {
		return EffectiveAuthoringInspection{}, &BridgeError{Code: "invalid_handle", Message: "authoring identity must belong to this client"}
	}
	if err := profile.valid(); err != nil {
		return EffectiveAuthoringInspection{}, err
	}
	if err := identity.valid(); err != nil {
		return EffectiveAuthoringInspection{}, err
	}
	result, err := profile.client.call(struct {
		Handle   uint64 `json:"handle"`
		Identity uint64 `json:"identity"`
	}{profile.id(), identity.id()}, opJoinEffectiveAuthoring)
	if err != nil {
		return EffectiveAuthoringInspection{}, err
	}
	return decodeMetadata[EffectiveAuthoringInspection](result)
}
