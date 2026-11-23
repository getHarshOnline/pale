package pale

// NativeActionDescriptor describes one compiled effect-free native action.
type NativeActionDescriptor struct {
	ActionID           string `json:"actionId"`
	MCPToolName        string `json:"mcpToolName"`
	Description        string `json:"description"`
	ContractBytes      []byte `json:"contractBytes"`
	PayloadSchemaBytes []byte `json:"payloadSchemaBytes"`
}

// NativeActionCatalog returns the closed compiled action inventory.
func (client *Client) NativeActionCatalog() ([]NativeActionDescriptor, error) {
	result, err := client.call(struct{}{}, opNativeActionCatalog)
	if err != nil {
		return nil, err
	}
	var descriptors []NativeActionDescriptor
	if err := decodeJSON(result.metadata, &descriptors); err != nil {
		return nil, err
	}
	return descriptors, nil
}

// InvokeNativeAction admits and executes one complete tagged effect-free request.
func (client *Client) InvokeNativeAction(actionID string, bytes []byte) (CanonicalDocument, error) {
	return client.invokeNativeAction(opInvokeNativeAction, actionID, bytes)
}

// InvokeNativeActionPayload binds a carrier-selected action tag to one payload.
func (client *Client) InvokeNativeActionPayload(actionID string, bytes []byte) (CanonicalDocument, error) {
	return client.invokeNativeAction(opInvokeNativeActionPayload, actionID, bytes)
}

func (client *Client) invokeNativeAction(operation operation, actionID string, bytes []byte) (CanonicalDocument, error) {
	result, err := client.call(struct {
		ActionID string `json:"actionId"`
		Bytes    []byte `json:"bytes"`
	}{actionID, bytes}, operation)
	return CanonicalDocument{Bytes: result.payload}, err
}

// AdmittedRepositoryNativeActions is the path-free action set bound to one repository.
type AdmittedRepositoryNativeActions struct {
	ProfileDigest  string                   `json:"profileDigest"`
	InstanceDigest string                   `json:"instanceDigest"`
	RepositoryID   string                   `json:"repositoryId"`
	Descriptors    []NativeActionDescriptor `json:"descriptors"`
}

// AdmitRepositoryNativeActions binds a canonical catalog to its exact compilation identities.
func (client *Client) AdmitRepositoryNativeActions(bytes []byte, profileDigest, instanceDigest, repositoryID string) (AdmittedRepositoryNativeActions, error) {
	result, err := client.call(struct {
		Bytes          []byte `json:"bytes"`
		ProfileDigest  string `json:"profileDigest"`
		InstanceDigest string `json:"instanceDigest"`
		RepositoryID   string `json:"repositoryId"`
	}{bytes, profileDigest, instanceDigest, repositoryID}, opAdmitRepositoryNativeActions)
	if err != nil {
		return AdmittedRepositoryNativeActions{}, err
	}
	var admitted AdmittedRepositoryNativeActions
	if err := decodeJSON(result.metadata, &admitted); err != nil {
		return AdmittedRepositoryNativeActions{}, err
	}
	return admitted, nil
}
