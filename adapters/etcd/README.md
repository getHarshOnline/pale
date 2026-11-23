# Independent anchor custody through etcd

`pale-adapter-etcd` implements the existing `AntiRollbackAnchorPort` using the native etcd v3 JSON gateway. Transition meaning stays in [the port](../../crates/pale-ports/src/anti_rollback.rs). This adapter owns bounded HTTPS transport, exact-key representation and physical compare-and-swap. It neither provisions a cluster nor admits a deployment.

## Use the admitted native inputs

Construct `EtcdAnchorStore` with the exact HTTPS member origin, nonzero cluster ID, namespace, CA, account token, optional transport client identity and positive per-request timeout. The namespace begins and ends with `/`, contains an interior byte and is at most 256 bytes. Tokens are at most 16 KiB, each PEM input 64 KiB, and the timeout at most 30 seconds. Endpoint userinfo, paths, queries and fragments are refused. No defaults, ambient proxy, redirects, automatic retries, member discovery or failover participate.

The supplied CA is the client's only trust root. Native credentials and network routing must be scoped to the admitted cluster; checking a response's cluster ID does not authenticate an arbitrarily rebound endpoint before a write. The administrator must provide namespace-scoped credentials through native secret handling. This adapter has no password login, token refresh, user administration, key deletion, lease, snapshot or restore operation. Run its synchronous port in the host's blocking context, outside an async executor.

The JSON gateway authenticates the account token, not the client certificate's Common Name. With native account authentication enabled, a transport client certificate with a nonempty CN is rejected. Use a separate CA-issued client certificate without a CN when mTLS is required. Renewal and expired/refused credentials remain explicit native responsibilities.

## Comparison and uncertain outcomes

Each typed principal minimum or control-domain anchor has one namespaced key. Versioned canonical values retain the complete typed coordinate. Malformed, foreign, expiring or oversized custody is unavailable, never interpreted as a missing anchor. Responses are limited to 64 KiB and root response headers must identify the selected cluster, a positive member and a positive revision.

A default linearizable exact-key read classifies the observation through the port. A missing initial anchor compares key version with zero. A successor compares both the exact predecessor value and its modification revision, so an intervening replacement cannot pass merely by restoring the same bytes. One native transaction performs the put and its read-back together; a failed compare reads the retained value without writing. An already retained exact successor is an idempotent replay.

A failed preliminary read is `Unavailable`. Once a transaction is dispatched, transport failure, HTTP failure or invalid response evidence is `OutcomeUnknown`. There is no automatic retry. Observe the exact transition using the existing port and classify the retained frontier before deciding any next action. A successful advance requires the exact successor and matching transaction revision, not merely a successful HTTP response.

The adapter exposes no rollback operation. Native administrators or credentials with write privilege can still replace or delete etcd data. Production qualification must establish independent administration, credentials, backups and restore domains outside the database and signing-key restore domain. Three members alone establish none of those properties. Hard effects stay disabled until the actual deployment qualifies.

## Verify from maintained sources

From the repository root, the ordinary checks run three decoder/custody cases and explicitly skip the native quorum journey:

```sh
cargo test --locked -p pale-adapter-etcd
cargo clippy --locked -p pale-adapter-etcd --all-targets --no-deps -- -D warnings
RUSTDOCFLAGS='-D warnings' cargo doc --locked -p pale-adapter-etcd --no-deps
```

The native journey requires an independently obtained and hash-verified **etcd 3.6.16** executable selected by `PALE_ETCD_TEST_BINARY` and its lowercase SHA-256 in `PALE_ETCD_TEST_BINARY_SHA256`, plus native `openssl`. After supplying those explicit inputs:

```sh
cargo test --locked -p pale-adapter-etcd --test native -- --ignored --nocapture
```

The test verifies the binary bytes and version before starting three owned loopback members with throwaway PKI and account authentication. It exercises initial/advancing/replayed/conflicting custody, distinct domains, foreign cluster and bad-token refusal, competing native CAS, a committed transaction whose reply is deliberately lost, member and full-quorum restart, corrupt values and the response-size ceiling. The fixture owner refreshes its token explicitly after restarts. TLS intermediaries only inject test faults; actual etcd performs the transactions. Children are stopped and owned temporary state removed on exit. No installed service, production key, real independent administration or live resident assembly is proved. A skipped native test is not native operation.

The 11 November 2026 macOS ARM64 run used the official [v3.6.16 release](https://github.com/etcd-io/etcd/releases/tag/v3.6.16), archive SHA-256 `228cbc84959a0eb3e48463a0daa3be8985b9edbba78e0d880840075a39e7b7c5`, and executable SHA-256 `69b1d6fe1a81d2735ae7b0de31add9f7f4a5dd626f54f39f67f86d9da2302714`. Tests have no dependency on that run's temporary download directory.

Native contracts: [API guarantees](https://etcd.io/docs/v3.6/learning/api_guarantees/), [API and transactions](https://etcd.io/docs/v3.6/learning/api/), and [JSON gateway and authentication](https://etcd.io/docs/v3.6/dev-guide/api_grpc_gateway/).

Part of the [Pale source layout](../../docs/source-layout.md).
