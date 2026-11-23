package pale

// InspectPackageDistribution derives canonical Package release coordinates.
func (client *Client) InspectPackageDistribution(documents []Document) (CanonicalDocument, error) {
	result, err := client.call(struct {
		Documents []Document `json:"documents"`
	}{documents}, opPackageDistribution)
	return CanonicalDocument{Bytes: result.payload}, err
}
