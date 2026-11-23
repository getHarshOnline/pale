+++
description = "Choose and test Redis caches, leases, counters and event streams for their required guarantees."

[guide]

[stack]
role_uses = [{ scope = "local", resource_id = "redis-storage" }]
+++

# Choose Redis structures for the state they hold

Start with the lifetime or coordination guarantee the task needs. A disposable cache, expiring lease, rate counter, lossy notification channel and retained event stream serve different purposes even when one server supports them all.

For a cache, identify the source that owns the value, key identity, freshness, invalidation, stampede prevention and safe handling of misses. For a lease or lock, define holder tokens, expiry, renewal, fencing and delayed-client recovery. Rate or admission counters need atomic updates and explicit clock and network-partition behaviour. Use ephemeral fan-out only when losing messages is acceptable; use a retained log when ordering, recovery or replay is required.

Assess memory limits, eviction, persistence, replication, failover, connection pooling, network partitions, duplicate delivery and recovery. A healthy Redis instance does not make a derived value the governing record.

In the design or review reply, explain the required guarantee, chosen structure, value owner and expiry, failure cases, capacity observations, fallback and checks. The project supplies actual commands, deployment, topology and current values.

## Worked example

Cache a profile under a versioned key with a TTL. Update the database and invalidate the old key. Simulate the invalidation arriving after a stale reader writes a value, and check that a later reader cannot treat the old version as current.

## Before handing off

- [ ] Name the purpose, key and lifetime of stored data.
- [ ] Check invalidation, ordering and behavior after a lost value.
- [ ] Test the owner’s recovery path where state matters.
