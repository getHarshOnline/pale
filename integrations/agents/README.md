# Shared agent connection

This Rust component uses the SDK-owned native connection to connect Pale's SDK to an explicitly configured local resident on macOS and Linux. It supplies the shared connection for the Claude Code and Codex integrations being built. The host selects protected endpoint and credential files, pins both public keys and supplies independent admission of signed operation responses.

Call `LocalAgentCarrierSource::load` with those paths and pins, then pass the source to the SDK's `ResidentAttachment::provision_operations`. Authentication proves the resident before private application bytes are sent. Every request uses a fresh socket, the endpoint directory is checked again, and the same deadline covers connection, delivery and response checks. A failure after sending leaves completion uncertain; the connection never retries the payload. The loader uses operating-system randomness and the existing 1 MiB frame bound. A host with its own signer, randomness or transport can implement the SDK’s `AuthenticatedResidentCarrierSource` instead.

`LocalAgentNativeToolPreparation` preserves the exact native event while an independently selected caller signs its request. `LocalAgentNativeTool` retains that request through an uncertain start and consumes hook delivery at most once after a verified durable start. `deliver_hook` returns `None` when start has not succeeded or delivery was already attempted; otherwise it returns the SDK's admitted native output or error. Its retained start records describe that historical start, not the operation's current head. These process-local values do not implement recovery after host restart.

> [!IMPORTANT]
>
> **An authenticated connection does not grant tool permission.** Native response output requires the SDK's independent response authority. The ordinary `pale hook` command can select a protected hook-only connection through [`--resident-hook-config`](../../interfaces/cli/README.md); that configuration supplies neither response authority nor resident operations and continues to block native work. Running-host integration, deployment recovery after interrupted responses and production authority remain unfinished.

Source guidance for both hosts works today through the [guidance starter](../../docs/examples/guidance-workspace/README.md); plugin manifests and installation remain unfinished.

From the repository root:

```sh
cargo test --manifest-path integrations/agents/Cargo.toml --locked --offline
cargo clippy --manifest-path integrations/agents/Cargo.toml --locked --offline --all-targets -- -D warnings
RUSTDOCFLAGS='-D warnings' cargo doc --manifest-path integrations/agents/Cargo.toml --locked --offline --no-deps
```
