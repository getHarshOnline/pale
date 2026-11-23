# Public test certificates

These certificates and private keys exist only for local A2A transport tests.
Never install either CA into a system trust store or use these keys outside tests.

The leaf covers `localhost` and `127.0.0.1`. The wrong-host leaf covers only
`wrong-host.invalid`. Both use the test CA; the unrelated CA has a different key.
Validity: 2026-09-07 15:38:20 UTC through 2036-09-04 15:38:20 UTC.

Generated offline with `/usr/bin/openssl` (LibreSSL 3.3.6), RSA 2048, SHA-256,
CA/server-auth extensions and ten-year test validity. The test runner reads DER
files directly and does not invoke OpenSSL. Keys are intentionally public fixtures.
