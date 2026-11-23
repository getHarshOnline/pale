package pale

import "sync/atomic"

// Consuming methods preserve inputs when host preflight rejects a call. Once
// native invocation begins, they surrender those inputs even on rejection.
// The client mutex serializes that handover with calls and Client.Close.
type handle struct {
	client *Client
	value  *atomic.Uint64
}

func newHandle(client *Client, value uint64) handle {
	coordinate := &atomic.Uint64{}
	coordinate.Store(value)
	return handle{client: client, value: coordinate}
}

func (value *handle) id() uint64 {
	if value == nil || value.value == nil {
		return 0
	}
	return value.value.Load()
}

func (value *handle) consume() uint64 {
	if value == nil || value.value == nil {
		return 0
	}
	return value.value.Swap(0)
}

func (value *handle) valid() error {
	if value == nil || value.client == nil || value.id() == 0 {
		return &BridgeError{Code: "invalid_handle", Message: "opaque Pale value is unavailable or consumed"}
	}
	return nil
}
