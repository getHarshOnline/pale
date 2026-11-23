# Review JavaScript and TypeScript runtime failures

Review the changed JavaScript or TypeScript code and configured lint or runtime results supplied by the requesting task. Apply TypeScript guidance and type-check results only when that stack is selected; React, Vue or JavaScript alone does not establish it.

Look for reachable failures from:

- in TypeScript, `any`, casts, non-null assertions or weakened compiler checks hiding a contract the caller needs; in either language, parsed or environment data reaching typed or trusted code without validation;
- promises with no owner or competing owners, unhandled rejection, `forEach(async ...)`, and sequencing that creates a race or avoidable measured latency;
- untrusted data reaching dynamic execution, HTML, URLs, queries, child processes, paths or object merging, and confidential values entering client bundles;
- synchronous I/O on request or event paths, mixed module systems that fail to load, and mutable module state unsafe under concurrent requests; and
- empty catches or thrown values that break the established error contract.

Style, broader type coverage and parallelization need concrete impact before becoming findings. Ask the requesting task for relevant repository-configured check results.
