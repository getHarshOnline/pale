# Review Python inputs resources and async work

Add Python-specific risks to the shared review method. Look for:

- untrusted data reaching queries, shells, paths, dynamic evaluation, deserialization or unsafe YAML parsing, and weak cryptography used for a security guarantee;
- broad catches or fallback values creating false success, lost exception context, leaked resources or corrupted state;
- mutable defaults or shared state leaking across calls, incorrectly mixed synchronous and asynchronous work, and unsynchronized concurrent writes;
- `Any`, casts or schema translations hiding a required contract or dropping data; and
- observed N+1 I/O, blocking work in an asynchronous path, or measured repeated allocation and computation.

Type coverage, formatter preferences and Pythonic style alone do not establish a defect. Ask the requesting task for the relevant configured type, lint, security and test results.
