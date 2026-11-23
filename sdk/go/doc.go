// Package pale compiles workspace configuration, inspects results, and calls
// local Pale operations through the Rust SDK.
//
// Create a [Client] with [NewClient] and close it when finished. Go translates
// requests and manages references to Rust values; the Rust SDK implements their
// behavior. [DiagnosticError] reports an SDK operation failure. [BridgeError]
// reports a bridge failure or misuse of a client or retained value.
//
// Calls are synchronous. Observation delta and visual-lens requests can report
// cancellation checked before computation. Other methods do not promise to
// interrupt a running call.
//
// Copyright 2026 RawThoughts Enterprises Private Limited. Authored and
// originated by Harsh Joshi. Licensed under Apache-2.0.
package pale
