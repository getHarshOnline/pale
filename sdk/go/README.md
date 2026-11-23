# PaleGo

PaleGo is the Go SDK for Pale. Use it to compile workspace configuration, inspect validated results, or call the available local operations. It calls the canonical Rust SDK through cgo and returns Go values and errors. Each client retains its Rust-side handles; the Pale application implements the behavior.

## Build the native library

Use Go 1.26, Rust 1.94 or later, and a C compiler for cgo. The current linker configuration supports macOS and Linux. From `sdk/go` in this checkout, run:

```sh
CARGO_TARGET_DIR=target cargo build --release --locked
```

The target directory must stay at `target/release`, where cgo expects the library on macOS and Linux. This checkout's Cargo configuration resolves its local `pale-sdk = 0.0.1` dependency. If you build elsewhere, provide those matching dependencies explicitly. The build may download dependencies; it does not start a resident.

## Compile a workspace from memory

Pass an application's complete source batch to `CompilePortableWorkspace`. `RootManifest` is the logical relative label of the root; `Documents` contains that root and its selected module and lock documents as labelled byte slices. The SDK does not open those labels as host paths.

After building the native library, save this as `examples/workspace/main.go` under `sdk/go`, creating those directories if needed. Run `go run ./examples/workspace` from `sdk/go`. It loads the [complete workspace example](../../docs/workspace.md#try-a-complete-portable-workspace):

```go
package main

import (
    "fmt"
    "log"
    "os"
    "path/filepath"
    pale "github.com/getHarshOnline/pale/sdk/go"
)

func main() {
    if err := run(); err != nil { log.Fatal(err) }
}

func run() error {
    client, err := pale.NewClient()
    if err != nil { return err }
    defer client.Close()

    var documents []pale.Document
    for _, label := range []string{"pale-workspace.toml", "resolution.lock.toml", "universe.lock.toml"} {
        bytes, err := os.ReadFile(filepath.Join("../../docs/examples/workspace", label))
        if err != nil { return err }
        documents = append(documents, pale.Document{Label: label, Bytes: bytes})
    }
    workspace, summary, err := client.CompilePortableWorkspace(
        pale.PortableWorkspaceRequest{RootManifest: "pale-workspace.toml", Documents: documents},
    )
    if err != nil { return err }
    defer workspace.Close()
    fmt.Println(string(summary.Bytes))
    return nil
}
```

In the printed summary, expect `workspace_id` to be `notes-workspace` and `source_document_count` to be `3`. Keep `workspace` for inspection or comparison; close it when finished, then close its client. A rejected input returns an error; there is no partly compiled workspace to use.

The [workspace contract](../../docs/workspace.md) explains configuration selection. For command-line use instead of embedding, follow [Inspect your workspace](https://rawthoughts.in/pale/inspect-your-workspace/).

## Inspect a contract

This program creates one Pale client, checks the capabilities available to Go, reads the generated diagnostic schema, and handles an unavailable schema as a typed diagnostic. Save it as `examples/contract/main.go` under `sdk/go`, creating those directories if needed:

```go
package main

import (
	"errors"
	"fmt"
	"log"

	pale "github.com/getHarshOnline/pale/sdk/go"
)

func main() {
	if err := run(); err != nil {
		log.Fatal(err)
	}
}

func run() error {
	client, err := pale.NewClient()
	if err != nil {
		return err
	}
	defer client.Close()

	capabilities, err := client.Capabilities()
	if err != nil {
		return err
	}
	fmt.Printf("Go can call %d Pale capabilities\n", len(capabilities))

	schema, err := client.ContractSchema("diagnostic.v1")
	if err != nil {
		return err
	}
	fmt.Printf("%s\n%s\n", schema.RegistryID, schema.SchemaSHA256)

	_, err = client.ContractSchema("not-a-selected-schema")
	if err == nil {
		return errors.New("unavailable schema unexpectedly succeeded")
	}
	var diagnostic *pale.DiagnosticError
	if !errors.As(err, &diagnostic) {
		return err
	}
	fmt.Printf("%s: %s\n", diagnostic.Code, diagnostic.Summary)
	return nil
}
```

Run it from `sdk/go`:

```sh
go run ./examples/contract
```

## Find a method or result type

From `sdk/go`, use Go's local reference for the matching source:

```sh
go doc Client
go doc Client.ComparePortableWorkspaces
go doc WorkspaceComparison
```

`go doc` reads declarations without invoking the native operations. The native library is still required when compiling or running a consumer. This is a local reference, not a promise that this private version is on pkg.go.dev.

## Manage client and handle lifetimes

`NewClient()` verifies the embedded contract bundle and native-action registry. The returned client owns every opaque Rust handle created through it. Always close the client; `Close()` drops the API instance and all of its handles. It does not cancel a call or execute a Pale operation.

Share the returned `*Client`; do not copy the `Client` value, which contains its synchronization state. Opaque handles may be copied: copies refer to the same Rust-held state. `PortableWorkspace`, `WorkspaceComparison`, and `EffectiveProfile` have an idempotent `Close()` that invalidates their copies and releases that view. Independent derived comparisons, profiles, observations and review stages stay available; copied result documents remain readable. Close views when finished so a long-lived client need not retain each of these views until shutdown.

Observation snapshots, page cursors and subjects also have an idempotent `Close()` so a long-lived client need not retain them indefinitely. Closing a snapshot leaves its independent selectors available. `ObservationPageResult.Close()` closes all subjects and the continuation originally returned by that page, including copied handles; the copied fact data stays readable. Keep the page open while any selector is needed, or close unwanted children individually. During pagination, obtain the next page before closing the previous one to retain at most two page batches. These calls release local memory only.

Authoring navigation returns typed source locations and reusable resolved or completion identities. Close the navigation result when those identities are no longer needed; it releases the originally issued identities even if its public fields were reassigned. Individual identities can also be closed. Copied identity descriptions remain readable data. They cannot alter or recreate the native identities and relationships held by those handles.

### Read results and diagnose errors

`Capabilities()` lists the operations exposed by this Go binding. It does not check whether configured machines, providers or a resident are available. `ContractSchema()` returns Go-owned metadata and a copy of the exact generated schema bytes from the Rust contract bundle.

API failures use `DiagnosticError`. Read its typed fields or preserve `Canonical`; do not parse the error message as a protocol. A `BridgeError` reports client or handle misuse, or a bridge failure. It contains `Code` and `Message`, without SDK diagnostic or source-correction fields.

`DiagnosticError.Identity` comes directly from the SDK. `AuthoringCorrections` and nullable `AuthoringLocation` retain exact compiler-proved source evidence. Match a correction's expected source at its labelled path before applying it; these copied values do not authorize a write and remain readable after client closure. The Go source and private native library must be rebuilt together; missing diagnostic metadata is rejected, not silently treated as absent evidence.

## Compilation and configuration review

- `CompilePortableWorkspace()` compiles an explicit set of labelled source bytes. It does not scan the current directory.
- `InspectStandaloneAuthoring()` reads grammar facts for a selected document role. `InspectAuthoring()` on a workspace or profile reads those facts from its retained compilation. These methods return typed `AuthoringInspection` values; Go does not reconstruct the grammar.
- Effective-profile operations keep declared intent, portable effective state, and private machine realization separate.
- `InspectLocalControlStore()` observes one selected store without creating or repairing it. Physical `Ready` is not writer or resident admission.
- Configuration improvement uses consuming review stages. Copies of a stage share the same consumption state. Failed preparation leaves its explanation available for retry; commit requires the same explanation handle that prepared the candidate, not another explanation with equal contents.
- `PreviewLocalConfigurationImprovement(LocalConfigurationImprovementRequest{...})` captures `BaselineManifest` and `CandidateManifest` with common `ProjectID`, `ExecutionWorkspaceID`, and `MachineID` selectors. It returns the same preview handle. Only the baseline supplies Package content; no candidate Package installation is required.
- Observation delta and visual-lens requests expose only the API's cooperative preflight cancellation result. Other calls do not claim in-flight cancellation.

Calls are synchronous. A client serializes its native calls, response decoding, and closure. Do not treat a method call as background work merely because its input or result is small.

An accepted, rejected, cancelled, or caller-proved stale configuration preview is an effect-free review result. It does not change source or grant authority. Local publication and materialization methods are different: they cross bounded native I/O and require their exact admission, receipt, read-back, and reconciliation rules.

`Client.CompiledInterfaceIdentity()` reports compiled SDK version, interface and target claims. `LocalCompilation.VerifyCurrentExecutableBinding()` checks the current host executable against the captured workspace binding, not the loaded native library's bytes. Neither call observes a resident or establishes process health. Local settlement checks this same binding before making changes. A workspace bound to a separate Pale CLI executable cannot be settled directly from a Go host.

## Verify the package

After building the native library, run the Go tests from `sdk/go`:

```sh
go test ./...
```

The public Go API contains no C types. Requests are copied into bounded Rust-owned memory, and responses are copied back into Go-owned values. Tests under `tests/` load the real native library and replay the canonical cross-leaf contract, diagnostic, workspace, improvement, and native-action cases. The runtime capability assertion detects missing or extra Go operations. Effectful local publication and settlement tests use only a temporary workspace and require exact receipt and owner read-back convergence.

## Generated native contracts

The `contracts` package and its per-family `contracts/types` subpackages are a checked-in, deterministic Go projection of every canonical initial-v1 contract family. Generation runs in models-only mode. Go records and closed scalar enums improve authoring, while structural validation and semantic admission remain in `pale-sdk`. A constructed Go record is not proof that a contract is valid.

The generator verifies an explicit bundle directory and every registry digest. It records the exact bundle, pinned generator, dependency lock, behavior options, and output set in a pretty-printed source coordinate:

```sh
go -C tools/contract-codegen run . \
  --bundle /absolute/path/to/agent-surface.v1
go -C tools/contract-codegen run . \
  --check --bundle /absolute/path/to/agent-surface.v1
```
