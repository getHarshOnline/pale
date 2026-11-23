package pale

// CapabilityID is one stable operation identity selected for the Go leaf by
// the API-owned eligibility inventory.
type CapabilityID string

// Capabilities returns the exact API-owned operations eligible for Go in
// stable inventory order.
func (client *Client) Capabilities() ([]CapabilityID, error) {
	result, err := client.call(struct{}{}, opCapabilities)
	if err != nil {
		return nil, err
	}
	var capabilities []CapabilityID
	if err := decodeJSON(result.metadata, &capabilities); err != nil {
		return nil, err
	}
	return capabilities, nil
}
