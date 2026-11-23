# Review Vue reactivity lifecycle and server rendering

Apply this role when Vue is selected. Add these risks to the shared review method:

- ref or prop destructuring, watcher sources or reactive-object replacement that lose tracking or leave state stale under the selected Vue version;
- composable effects outside their instance lifecycle, missing cleanup and shared module state leaking across components or server requests;
- mismatched props, emits, slots, store or router-guard contracts that cause runtime failure or a gap in permission checks;
- unsanitized `v-html` or user URLs, exposed confidential runtime configuration and server routes without effective input validation or permission checks; and
- server/client hydration mismatch, unstable list identity, broken focus or semantics and measured rendering cost.

Recommend memoization or reactivity changes only for a concrete failure or measured cost. Ask the requesting task for another review role when needed; do not delegate from this role.
