package pale

// ContractSchema is one exact generated schema and its registry identity.
type ContractSchema struct {
	RegistryID string
	SchemaURI  string
	// WireSchemaID is nil when the contract has no standalone wire identity.
	WireSchemaID *string
	SchemaSHA256 string
	Bytes        []byte
}

// ValidatedContract is one schema-admitted canonical contract document.
type ValidatedContract struct {
	SchemaID     string
	SchemaSHA256 string
	Bytes        []byte
}

// ContractBundleIdentity returns the canonical embedded bundle coordinate.
func (client *Client) ContractBundleIdentity() (CanonicalDocument, error) {
	result, err := client.call(struct{}{}, opContractBundleIdentity)
	return CanonicalDocument{Bytes: result.metadata}, err
}

// ContractSchema looks up one exact generated schema.
func (client *Client) ContractSchema(schemaID string) (ContractSchema, error) {
	result, err := client.call(struct {
		SchemaID string `json:"schemaId"`
	}{schemaID}, opContractSchema)
	if err != nil {
		return ContractSchema{}, err
	}
	var metadata struct {
		RegistryID   string  `json:"registryId"`
		SchemaURI    string  `json:"schemaUri"`
		WireSchemaID *string `json:"wireSchemaId"`
		SchemaSHA256 string  `json:"schemaSha256"`
	}
	if err := decodeJSON(result.metadata, &metadata); err != nil {
		return ContractSchema{}, err
	}
	return ContractSchema{
		RegistryID: metadata.RegistryID, SchemaURI: metadata.SchemaURI,
		WireSchemaID: metadata.WireSchemaID, SchemaSHA256: metadata.SchemaSHA256,
		Bytes: result.payload,
	}, nil
}

// ValidateContractDocument admits and canonicalizes one generated contract.
func (client *Client) ValidateContractDocument(schemaID string, bytes []byte) (ValidatedContract, error) {
	result, err := client.call(struct {
		SchemaID string `json:"schemaId"`
		Bytes    []byte `json:"bytes"`
	}{schemaID, bytes}, opValidateContractDocument)
	if err != nil {
		return ValidatedContract{}, err
	}
	var metadata struct {
		SchemaID     string `json:"schemaId"`
		SchemaSHA256 string `json:"schemaSha256"`
	}
	if err := decodeJSON(result.metadata, &metadata); err != nil {
		return ValidatedContract{}, err
	}
	return ValidatedContract{SchemaID: metadata.SchemaID, SchemaSHA256: metadata.SchemaSHA256, Bytes: result.payload}, nil
}
