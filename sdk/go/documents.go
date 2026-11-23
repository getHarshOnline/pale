package pale

// Document binds an untrusted logical source label to owned bytes.
type Document struct {
	Label string `json:"label"`
	Bytes []byte `json:"bytes"`
}

// CanonicalDocument contains exact RFC 8785 JSON bytes produced or admitted by
// the Rust façade. Callers may decode it into generated contract types without
// changing the authoritative byte identity.
type CanonicalDocument struct {
	Bytes []byte
}

// Projection contains an exact bounded API-owned JSON DTO for which no public
// schema exists. It is deliberately distinguished from CanonicalDocument: its
// bytes are transport data, not a durable contract identity.
type Projection struct {
	JSON []byte
}

type privateInputs struct {
	Capabilities      Document   `json:"capabilities"`
	MachineBinding    Document   `json:"machineBinding"`
	DistributionLock  Document   `json:"distributionLock"`
	LifecycleGuidance []Document `json:"lifecycleGuidance,omitempty"`
}

// PrivateInputs binds the singular private documents used for compilation.
type PrivateInputs struct {
	Capabilities      Document
	MachineBinding    Document
	DistributionLock  Document
	LifecycleGuidance []Document
}

func (value PrivateInputs) wire() privateInputs {
	return privateInputs{
		Capabilities:      value.Capabilities,
		MachineBinding:    value.MachineBinding,
		DistributionLock:  value.DistributionLock,
		LifecycleGuidance: value.LifecycleGuidance,
	}
}
