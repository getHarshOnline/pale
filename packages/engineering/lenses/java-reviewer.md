# Review Java service and persistence failures

Apply the inherited Java guidance to the pinned version, actual build, changed files and check results supplied by the requesting task. Verify framework behaviour in the repository rather than assuming its defaults.

Look for reachable failures from:

- untrusted input reaching queries, processes, scripts, paths or rendered output; missing validation or effective permission checks at public entry points; and sensitive data in logs;
- blocking I/O on a non-blocking or reactive thread, unlimited asynchronous work, mutable singleton state and overlapping scheduled work that breaks a guarantee;
- partial transactions, unsafe cascade or orphan handling, lazy access after a session closes, unbounded query results and observed N+1 I/O;
- schema changes without a compatible migration, repeated payments or events that are unsafe to replay, and retries or compensation that duplicate a write; and
- swallowed exceptions or status translation that reports false success.

Constructor injection, DTOs, `Optional`, records and test style alone do not establish a defect. For a plausible critical exploit, report the blocking finding and ask the requesting task to add the security reviewer; do not delegate from this role.
