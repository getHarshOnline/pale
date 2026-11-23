# macOS signing helper

This adapter contains the selected XPC helper's Rust signing core and native Swift listener/client and an application-scoped XPC service entrypoint. It supplies no installed service, actual machine policy, real Keychain item or production identity. The resident must call the authenticated helper, keeping seed material inside that process. Loading the key-reading static library into the resident would violate that separation.

The [core record owners](../../crates/pale-core/src/records/resident/mod.rs) construct and admit canonical preimages, digests, signing authorities and transitions. This adapter owns native caller/helper authentication, configured key-slot selection, read-only Keychain access and Ed25519 mechanics. It grants no authority by signing a digest. The application must retain all contextual admission and verify the returned signature against the admitted public key.

## Compose the actual helper

Build the Rust static library without `qualification-fixture`, then compile `native/SigningProtocol.swift` and `native/SigningHelper.swift` with `include/pale_signing.h` and link the library. The calling host builds only `SigningProtocol.swift` and `SigningClient.swift`, with no C header, Rust signing library or Keychain lookup implementation. The native check compiles that client independently before testing the complete fixture. The native host constructs `SigningHelperListener` with an actual listener, client code-signing requirement, expected UID, positive connection ceiling (at most 64), and one to 64 configured aliases. Construction starts nothing; the host explicitly resumes and retains the listener.

`native/service/main.swift` is the separate XPC service host. Its signed bundle's `PaleSigningPolicyPath` selects one explicit protected file; there is no path search or environment fallback. Build it with `SigningProtocol.swift`, `SigningHelper.swift`, `SigningPolicy.swift`, the C header and the normal Rust library. `native/service/Info.plist.in` requires actual bundle identifier and policy-path substitutions before signing. The template declares application scope, not a system daemon or a privilege grant. The host captures policy before starting its service listener; missing/refused configuration exits 78 without logging paths or policy bodies. XPC owns the service's application lifecycle.

Policy capture uses the [filesystem adapter](../fs/src/private_configuration.rs), also used by resident endpoint selection: no-follow traversal, effective-UID private parent, regular single-link `0600` file, bounded read and change checks. Rust owns the closed native JSON shape in [policy.rs](src/policy.rs); Swift projects only validated captured bytes and checks the native code requirement. The overall policy ceiling is 16 KiB. Unknown fields, duplicate aliases or record kinds, unsupported kinds, empty scopes and invalid bounds refuse. `CapturedSigningHelperPolicy.sourceSHA256` identifies the original captured bytes; it is neither an authority signature nor a live configuration lease.

| Policy field | Native meaning |
| --- | --- |
| `schema_version` | Exactly 1 |
| `client_requirement`, `client_uid` | Explicit caller code requirement and supplementary kernel UID constraint |
| `maximum_connections` | Positive connection ceiling, at most 64 |
| `slots` | One to 64 unique aliases within the overall byte ceiling |
| Slot `alias`, `service`, `account`, optional `access_group` | Native key lookup binding; the caller selects only its alias |
| Slot `public_key`, `allowed_kinds` | Expected canonical Ed25519 public key and nonempty, duplicate-free supported kind list |

The actual protected path, bundle identities, item, accessibility/access control and client policy remain deployment inputs. Capturing a configuration does not admit its issuer, grant Pale authority or install a helper.

Each alias binds exact service/account, optional access group, expected 32-byte public key and a nonempty bounded set of record kinds. Requests carry only alias, kind, a 32-byte digest and a positive budget of at most 30 seconds. They cannot select another Keychain item, expected key or signature domain. Both peers set a code-signing requirement before resuming their XPC connection; UID is an additional constraint. Syntax validation does not establish that a requirement is sufficiently narrow: qualification must inspect the actual deployment identity and trust policy. No identity or lookup defaults are chosen.

The closed native Rust request selects one of nine core-owned domains: session lease, operation handle/observation, A2A task binding/observation, capability result, adapter failure, control checkpoint or principal trust checkpoint. Other record families remain unsupported rather than falling through to a generic signer. This is ordinary Ed25519 over the domain's exact bytes, including its NUL, followed by the decoded 32-byte canonical SHA-256 digest; there is no additional hash, arbitrary domain or Ed25519ph profile.

The read-only native query selects generic-password items by exact configured coordinates, explicitly excludes synchronising items, skips items requiring an authentication prompt and requests at most two results. Missing, ambiguous, non-data and non-32-byte results are refused. An unlike derived public key is refused. There is no create, update, delete, discovery or interactive-unlock operation. Item accessibility, access control, search-store selection and recovery still require actual owner configuration and qualification.

One process-global try-lock serializes Keychain access; occupied or poisoned access is refused without queuing another lookup. Seed buffers and Dalek keys use zeroization on drop. The seed, framework data and implementation temporaries still enter helper memory; this does not provide non-exportable keys, an independent rollback anchor or a guarantee that every compiler/OS copy is wiped.

The Rust core checks its monotonic deadline before/after lookup and after signing. It cannot interrupt a blocked OS call. The native client independently uses one absolute monotonic deadline, rejects late completion and performs no automatic retry or reconnection. Keep synchronous client work off the main thread and async executor. Native XPC decodes arguments before the helper's length checks; the policy bounds accepted requests, not all framework allocation by a malicious authenticated peer. The selected helper host still needs its process-level resource and termination policy.

## Native ABI and dependency boundary

The private C ABI passes a 4,100-byte request and 68-byte response by value. There are no raw-pointer arguments, Rust discriminants, allocator ownership or secret-key arguments. Rust owns the closed 4 KiB JSON request and refusal codes; the header describes only fixed integer/byte buffers. Locally allowed `no_mangle` attributes export the project-prefixed symbols; no unsafe Rust block is introduced. The native test calls that real symbol through the C header and checks both structures' sizes. Swift buffer access is confined to their native storage lifetimes. Protected policy capture returns four contiguous 4 KiB blocks after its status/length fields. Separate fields stay importable by Swift's C bridge while preserving the full 16 KiB ceiling. A pointer-free metadata export identifies fixture builds; the native test requires that marker before sending a signing request.

The implementation pins `ed25519-dalek` 2.2.0 (BSD-3-Clause, `std` and `zeroize`, without batch, key generation, hazardous or legacy compatibility features), `zeroize` 1.9.0 and `security-framework` 3.7.0 (MIT OR Apache-2.0). The root lock owns the exact closure. Core's existing SHA-256 assembly feature remains part of that closure. A pin and the older Dalek audit lineage do not establish a fresh release-specific audit or production dependency acceptance.

## Verify without real keys

From the repository root:

```sh
cargo test --locked -p pale-adapter-darwin-signing
cargo clippy --locked -p pale-adapter-darwin-signing --all-targets --no-deps -- -D warnings
RUSTDOCFLAGS='-D warnings' cargo doc --locked -p pale-adapter-darwin-signing --no-deps
```

Five Rust cases cover exact domain/digest signatures and early refusal, occupied access, wrong/short keys, expired work, bounded ABI input and closed policy/parser rules. A separate native policy-ABI case captures actual protected files and distinguishes unsafe custody, oversized bytes and rejected fields. These checks do not read Keychain. The ordinary suite explicitly skips both XPC journeys.

For that opt-in journey, build the **debug-only public test fixture**:

```sh
cargo build --locked -p pale-adapter-darwin-signing --features qualification-fixture
```

Select the resulting `target/debug/libpale_adapter_darwin_signing.a` through `PALE_SIGNING_TEST_CORE` and its independently captured SHA-256 through `PALE_SIGNING_TEST_CORE_SHA256`, then run:

```sh
cargo test --locked -p pale-adapter-darwin-signing --test native -- --ignored --nocapture
cargo test --locked -p pale-adapter-darwin-signing --test process -- --ignored --nocapture
```

The test verifies and copies the supplied archive into an owned private directory, compiles the maintained Swift sources, signs only that disposable executable ad hoc and supplies its observed CodeDirectory hash as the test identity. Nineteen checks exercise the real same-process native ABI/XPC route, all nine domains with independent CryptoKit verification, caller/helper identity refusals, UID, key and input refusals, and an actual late reply with no retry. The fixture uses only the published RFC 8032 test seed and has no Keychain path. Its feature is forbidden in release builds. Rebuild the normal library after qualification; never use a fixture library as a deployed helper.

The second journey builds a client executable without the signing core and a distinct helper inside an owned, ad-hoc-signed app/XPC bundle. The external protected policy is written after final code identities are known, avoiding a cycle between its caller requirement and bundle signatures. Its valid JSON source spans all four ABI blocks. The native connection witnesses distinct caller/helper PIDs; CryptoKit verifies the returned operation-handle signature. The helper is observed gone before artifact removal. The fixture-only host also has a 20-second self-exit ceiling, and a failed runtime retains its owned bundle for lifecycle inspection instead of deleting a possibly active executable.

This proves the selected native process/bundle and policy mechanics. It does not establish a real Keychain read or item permissions, production signing identity, installed machine configuration, resident attachment or hard-effect readiness. No launchd job is installed. Native tests remove their owned artifacts after process exit; no historical-wave directory or temporary executable is required.

Primary references: [Apple XPC code requirements](https://developer.apple.com/documentation/technotes/tn3127-inside-code-signing-requirements), [requirement syntax](https://developer.apple.com/library/archive/documentation/Security/Conceptual/CodeSigningGuide/RequirementLang/RequirementLang.html), [Dalek 2.2.0](https://docs.rs/ed25519-dalek/2.2.0/ed25519_dalek/), and [RFC 8032 vectors](https://www.rfc-editor.org/rfc/rfc8032#section-7.1).

Part of the [Pale source layout](../../docs/source-layout.md).
