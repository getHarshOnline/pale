package pale

// ResidentHookAdmission is an opaque proof of route/payload agreement.
type ResidentHookAdmission struct{ handle }

// ResidentHookRequest is one bounded native event and static repository route.
type ResidentHookRequest struct {
	Event                   string
	Workspace               string
	Repository              string
	ProviderBinding         string
	ProviderBindingIdentity string
	Payload                 []byte
}

// AdmitResidentHook validates route/payload agreement without selecting a carrier.
func (client *Client) AdmitResidentHook(request ResidentHookRequest) (*ResidentHookAdmission, error) {
	result, err := client.call(struct {
		Event                   string `json:"event"`
		Workspace               string `json:"workspace"`
		Repository              string `json:"repository"`
		ProviderBinding         string `json:"providerBinding"`
		ProviderBindingIdentity string `json:"providerBindingIdentity"`
		Payload                 []byte `json:"payload"`
	}{request.Event, request.Workspace, request.Repository, request.ProviderBinding, request.ProviderBindingIdentity, request.Payload}, opResidentHookAdmission)
	if err != nil {
		return nil, err
	}
	return &ResidentHookAdmission{newHandle(client, result.object)}, nil
}
