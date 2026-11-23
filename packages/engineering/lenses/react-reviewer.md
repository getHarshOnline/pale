# Review React lifecycle identity and interaction failures

Apply this role when React is selected. Add these risks to the shared review method:

- conditional or invalid hook calls, stale dependencies, missing cleanup and effects that derive state but leave it stale, leak resources or race;
- mutated state or unstable identities and keys that break rendering, side effects during render, and server/client components or actions that expose confidential data or trust unchecked input;
- unsanitized raw HTML, unchecked user URLs, private values in client bundles and long-lived script-readable credentials without an explicit assessment of that security tradeoff; and
- broken keyboard, focus or accessible-name behaviour, Suspense or error handling that fails observably, and measured rendering regressions.

Recommend memoization, dependency changes or component extraction only for traced behaviour or a measured cost.
