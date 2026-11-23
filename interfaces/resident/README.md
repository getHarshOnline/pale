# Pale resident native host

Use this Rust component to connect a Unix-domain socket listener to an assembled Pale resident service. `ResidentLocalHost` borrows `ResidentFrameService`; `ResidentWorkTurnCoordinator` schedules frame, retained-phase and recovery turns. Admission, custody and execution remain in `pale-application`; framing and kernel credentials remain in the [Unix-socket adapter](../../adapters/uds/src/lib.rs).

The composition root supplies the service, an exact endpoint, frame limits, absolute connection deadlines, a positive retained-connection limit and `ResidentConnectionAuthentication`. The authentication input borrows a selected connection signer, operating-system random source and a bounded lookup of allowed caller keys. The listener challenges the caller, verifies the signed request and signs the response for that exact request. Connection keys establish neither a Pale principal nor application permission; the existing signed application request still passes its own authority, session and recovery checks. `ResidentLocalHost::bind` never discovers a principal, opens authority stores or chooses a signer. It refuses an occupied endpoint without removing it. Directory confinement, permissions and final endpoint removal belong to that deployment's lifecycle owner.

Call `run_one` when the native event loop offers a turn. An optional deadline pair permits one new connection. Accept and readiness observation do not wait; reading a ready partial frame remains bounded by its original deadline. The bounded connection queue is retained across turns. A later offer does not renew existing deadlines. Each authentication turn reads at most one ready frame, so a caller that stops after the resident proof does not block another ready client. Application recovery can advance while a handshake waits or is rejected. The caller-key lookup runs again before application dispatch, including after a ready request was deferred behind other work. After one frame or a frame failure, the connection closes without replay. Transport expiry does not cancel an operation or discard application recovery.

Native hook frames carry the exact repository command arguments and the original event as a UTF-8 string. The application retains the verified caller key with the event decoded from that same frame. The complete canonical frame is limited to 262,144 bytes; malformed, unauthenticated or mixed-family requests reach no session lookup. The application checks current caller and session authority, the original native occurrence and its protected operation state. A refusal returns its bounded failure class. With explicit response inputs, the application can retain preparation and dispatch with their checkpoints, recheck permission and disclosure, and return the exact prepared output. Without a selected handler it returns `handler_unavailable`; the listener supplies no response authority itself. The caller must independently admit the response before release to a native host. The SDK preserves the distinction between rejected input, denied admission, missing configuration and unresolved recovery; recovery and replay conflicts require reconciliation before another attempt.

An explicitly configured deployment can load `LocalEndpointConfiguration` from the Unix-socket adapter, then call `ResidentLocalHost::bind_configured`. Its local JSON file contains only a version and an absolute endpoint:

```json
{ "schema_version": 1, "endpoint": "/absolute/private/runtime/resident.sock" }
```

The loader selects no default or fallback and creates nothing. It bounds the read, refuses symlink traversal and hard links, requires effective-UID ownership, a `0600` regular file and owner-private parent directories, and retains the endpoint parent's identity for the next bind or connect. It captures configuration; it does not establish a live lease.

Those checks cover POSIX ownership and mode bits. They do not inspect ACLs, exclude another process with the same UID or authenticate the server. Mutual connection authentication supplies the peer proof independently. The deployment still qualifies its ACL and administration boundary, selects the actual connection credentials and supplies signed application request admission. This native endpoint file contains no principal, grant or credential and does not replace a Pale workspace or machine binding.

> [!IMPORTANT]
>
> This is a native listener library, not an installed resident service.

Its Rust composition entrypoint is available; a standalone configured process still needs the selected writer/trust, connection credentials, record signer and independent anchor implementations. Ordinary CLI/hook wiring to the explicit configuration loader, native-tool-to-operation registration and provider admission remain separate unfinished integrations. No production credentials, test authority or default endpoint is built into this component.

Build and check this independent component from the repository root:

```sh
cargo test --manifest-path interfaces/resident/Cargo.toml --locked --offline
```

Its socket tests use actual kernel credentials, transport signatures made with isolated fixture keys and the application-owned in-memory conformance service. They exercise the authenticated listener and protected request boundary; application authority signatures, custody and anchors are synthetic. They do not qualify a production deployment.
