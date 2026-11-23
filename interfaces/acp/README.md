# Pale ACP conductor

Use `pale-acp` when a coding session runs through Pale: each call conducts one turn with an established coding agent over stable ACP v1, and the caller supplies the already-connected stdio byte streams. It is a Rust conductor, not an executable launcher, an agent server or a Pale-owned language-model loop.

## Conduct a turn

::::steps

:::step "Construct the request"

Create `AcpConductor::new()`, then an `AcpTurnRequest` with the absolute working directory and prompt text. The directory coordinate is sent to the agent. Construction reads no local files and starts no action.

:::

:::step "Conduct one turn"

Pass the connected input/output streams, `AcpCancellation`, a finite deadline future and an update observer to `conduct_turn`. Your application supplies the clock and executor. The observer runs inline and must not block; the deadline cannot preempt it.

:::

:::step "Read the outcome"

Each call negotiates ACP v1, creates a fresh native session and submits one prompt. Read the typed outcome or error. The conductor does not keep a session for another prompt or resumption.

:::

::::

The [session tests](tests/session.rs) exercise this sequence against a synthetic agent, including cancellation, silent-peer deadlines, permission rejection and transport loss. They are local protocol tests, not qualification of a deployed agent. See [`conduct_turn`](src/conductor.rs) for the exact async stream bounds and error contract rather than introducing a second executor or launch wrapper.

No filesystem, terminal, or MCP capability is advertised to the agent. Permission requests select a supplied reject-once option or return cancellation; they never grant permission. These limits govern this connection, not any authority the independently operated agent already holds elsewhere.

## Cancellation and incomplete outcomes

Before a prompt starts, cancellation drops the connection without inventing a native session-cancellation acknowledgement. During a prompt, it forwards one cancellation request and awaits the native terminal reason, still bounded by the caller's deadline. A cancellation request is not proof of terminal cancellation. Deadline expiry or transport loss preserves the known native session coordinate, if one was obtained; neither proves that the agent stopped or that an external effect settled.

Incoming newline-delimited JSON-RPC frames, aggregate inbound bytes and messages, prompt bytes, projected text chunks, and ordered update count have independent bounds. Aggregate limits include negotiation, permission, control, update, and terminal traffic. Text-bearing updates with non-text content fail closed instead of silently losing their payload. Stable-v1 tool calls, tool-call updates, and plans retain the exact structured values decoded by the pinned ACP SDK. They remain native agent data, not Pale operations or effect evidence. The interface creates no second update queue or transcript store.

## Review Pale configuration

Use `PaleApi` directly for local, effect-free configuration review. Do not route SDK calls through `AcpConductor`. A host can give an agent admitted context, receive a proposed change, and then pass that proposal to the SDK's review workflow. The ACP session coordinate and Pale review handles remain separate. A verified person's explicit acceptance can authorize the exact proposed write when the actor, target, current source, and proposed change still match. The write remains a separate admitted effect. Acceptance does not apply the change or prove its native read-back.

The SDK can compile and compare portable workspaces, inspect source, compile an effective profile, explain findings, and review a candidate through preview and an explicit accept, reject, cancel, or stale disposition. It receives no local source capture, effect admission, action, resident observation, installation, activation, or effect authority through this ACP interface.

The [review tests](tests/improvement.rs) compose the two independent surfaces. Agent output does not automatically enter or approve a review. An ACP session ID and stop reason never substitute for a Pale profile, operation result, acceptance, or native settlement.
