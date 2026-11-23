# Pale A2A interface

Use this interface when Pale hands one task it has checked and authorised to an agent system someone else operates. Each call starts, advances, observes or cancels that outward `Delegate` through an authenticated Pale resident attachment, reusing the Rust SDK's request and result types. The interface keeps no operation state machine or A2A wire types of its own.

## Choose the operation

Build this crate against the matching local `pale-sdk` with Rust 1.94 or later. The public functions take the SDK's typed requests and return its results and `ApiError` diagnostics:

| Function | Use it to |
| --- | --- |
| `inspect_delegate_handle` | Check a supplied Delegate handle's structure without contacting a resident |
| `start_delegate` | Create an authorized child operation and receive its durable handle before dispatch |
| `advance_delegate` | Perform at most one resident-owned turn on that child |
| `observe_operation` | Read the operation's current signed records |
| `cancel_operation` | Request cancellation, then observe its outcome |

The last four require both `&PaleApi` and `&ResidentAttachment`. See the [Rust SDK](../../sdk/rust/README.md#choose-portable-native-or-resident-operations) for the attachment's lifetime and trust requirements.

> [!IMPORTANT]
>
> Supply a working authenticated resident attachment before using the operation functions. This crate does not discover or start the service; constructing a request is not a connection.

## Construct a call in your application

The start function has this signature:

```rust
pub fn start_delegate(
    api: &PaleApi,
    attachment: &ResidentAttachment,
    request: ResidentDelegateStartRequest,
) -> Result<ResidentOperationObservation, ApiError>
```

Construct it with `ResidentDelegateStartRequest::new(parent, signed_request)`, where `parent` is the retained `OperationHandleLocator` and `signed_request` contains the exact signed common-request bytes. The request identifies an already admitted Delegate route. The caller's deployment supplies the signing and resident connection; this interface does not construct either from a task description. To inspect the exact constructor, signed-record inputs and carrier traits, run from this crate's directory:

```sh
cargo doc --no-deps --open
```

This opens the interface's local Rustdoc. For the request and attachment types, build the [Rust SDK method reference](../../sdk/rust/README.md#build-the-method-reference) and open `ResidentDelegateStartRequest`, `ResidentAttachment`, `AuthenticatedResidentCarrierSource` and `ResidentResponseAdmission`. The first defines the input; the latter three define how the host connects and checks returned records. These local references are generated from the matching checkout, not a separate hosted API version.

## Keep remote and local outcomes separate

The effect-free inspection operation remains available for a supplied `Delegate` handle. It proves only canonical structure and a syntax-checked record reference. Resident custody, signatures, current authority, and disclosure still require the live attachment.

The application owns admission, durable `Delegate` start and dispatch, first-response and remote-task custody, observation, cancellation, and pre-Task recovery. The native A2A transport adapter owns JSON-RPC encoding, bounded response decoding, and verified HTTPS. This interface imports neither the adapter nor its private wire types.

The caller must provide an attachment whose carrier authenticates the resident and whose response-admission owner verifies signatures, authority rotation, and anti-rollback evidence. Deployment-qualified HTTPS remains an adapter concern.

The selected route is A2A v1 JSON-RPC over verified HTTPS. Streaming requires the exact admitted Agent Card's advertised support. An unidentified task remains unknown after a lost response; it must not trigger a repeated send. A cancellation response requires native read-back and does not prove Pale settlement.

This crate does not discover or start a resident. It provides no server, Agent Card publication, task listing, push delivery, alternate binding, compatibility reader, installation, or activation.
