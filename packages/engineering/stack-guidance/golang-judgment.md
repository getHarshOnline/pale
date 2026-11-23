# Write Go with clear interfaces and cancellation

Apply the common coding method and the repository's Go formatting, import and checking rules. Prefer plain control flow, useful zero values and composition. Give packages clear responsibilities and keep their dependencies free of cycles.

## Design for the caller

Let consumers define small interfaces, and return concrete types when that makes the API clearer. Use functional options when a constructor has real variation to support. Inject dependencies through interfaces that serve an actual need.

When adding error context, preserve the identity used by `errors.Is` and `errors.As`. Handle the error where a useful response is possible, and reserve panic for a broken guarantee from which the program cannot recover.

## Own concurrent work and its shutdown

Pass context through I/O calls with explicit deadlines and cancellation. Give every goroutine an owner and a shutdown path that completes within its allowed time. Establish who owns and closes each channel, including what happens to senders when a receiver leaves. Avoid shared state where practical; otherwise protect it with the smallest suitable synchronization scope. Use the project's race checks for changes where concurrent access matters, within the task's permission.

Test observable behaviour. Named table cases help when inputs fall into meaningful classes; shared setup should still leave the failing call easy to see. Keep complex expected output readable and owned by the test. Use generated inputs for parsers and other wide input spaces, benchmarks for a stated performance question, and coverage to find gaps worth investigating.

## Cancel a provider request

Have the consumer define its small reader interface with `context.Context`, and pass the request context to the provider. Cancel that request in a test. Check both the returned error and whether the provider's work stops.

## Before handing off

- [ ] Do the interface and context lifetime fit the consumer?
- [ ] Were relevant cancellation and concurrent paths checked?
- [ ] Do errors reach the caller with the cause it needs?
