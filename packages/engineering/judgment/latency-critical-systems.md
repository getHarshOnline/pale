# Keep responses timely and data fresh

Use this when a result's usefulness depends on response time, completion time or age. Establish the user or downstream deadline, workload and arrival pattern, target percentile, maximum data age, capacity limit and acceptable reduced service. Keep those requirements with the performance analysis or benchmark so the measurements have a clear target.

Break the critical path into queueing, startup, network, serialization, storage, computation, coordination and rendering. Measure the distribution, including slow requests, and find which part grows under contention. An average can hide a large delay for the users at the tail.

Reduce work while keeping correctness and freshness within their limits. Precomputation and caching reduce waiting but introduce invalidation risk. Batching reduces per-item cost while adding waiting time. Concurrency can shorten independent work while increasing total load or contention for shared resources.

Before overload, define which work is admitted, how producers are slowed, timeouts, what is dropped first and which reduced result remains useful. Returning stale or incomplete data immediately is useful only if the task permits it; otherwise make unavailability explicit.

In the reply or benchmark report, show where time is spent, measurements under representative load, the dominant cost, freshness trade-offs, capacity limit, overload response and remaining risk. Stop when the target is met, measurement cannot isolate the cause or the next change would break correctness or recovery.

## Search waits for a worker

Search meets its deadline in a quiet test. Under the usual arrival burst, the 95th percentile misses it while database time stays steady. Measure the wait for a worker, then run the same burst after changing the queue limit. Report both the response-time distribution and rejected requests. Check that any proposed cache remains fresh enough for the task.

## Before handing off

- [ ] Does the test use a tail metric and realistic load?
- [ ] Were freshness, queueing and backpressure measured alongside response time?
- [ ] Does the comparison use the same workload after the change?
