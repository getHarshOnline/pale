# Engineering

Package ID: `pale.engineering` · [Manifest](package.toml)

Make and review code changes, investigate failures, choose tests and work with the languages and frameworks a repository uses.

A workspace selects entries from this family for the duties its people and agents perform. To read it directly, open the page for your task and follow its reading links.

Start with [Change code so it fits](<change.md>). Its reasoning order is:

1. [Understand the change and what depends on it](<change/impact.md>)
2. [Keep the guarantees the change must respect](<change/constraints.md>)
3. [Choose the simplest complete solution](<change/choice.md>)
4. [Choose checks that can catch the failure](<testing.md>)
5. [Review a change](<review.md>)
6. [Make unfinished work and accepted limits explicit](<change/limits.md>)

## Guides

- [Build web interfaces around browser behaviour](<web.md>) · `web` · `doctrine`
  Build and review browser interfaces and content.
  Read first: [Change code so it fits](<change.md>).
- [Change code so it fits](<change.md>) · `change` · `doctrine`
  Understand a code change, fit it to its callers and check the whole result.
  Read as needed: [Make failures useful to the caller](<error-handling.md>), [Keep responses timely and data fresh](<response-time.md>), [Trace a security risk to the action it can reach](<security.md>).
- [Check JPA queries transactions and managed state](<spring-data-jpa.md>) · `spring-data-jpa` · `doctrine`
  Review Spring Data JPA where the project uses it for persistence.
  Read first: [Check the Spring service that actually runs](<spring.md>).
- [Check PyTorch training evaluation and recovery](<pytorch.md>) · `pytorch` · `doctrine`
  Review PyTorch training when PyTorch is a dependency or is selected in project configuration.
  Read first: [Write Python with explicit inputs and cleanup](<python.md>).
- [Check state-machine transitions completion and recovery](<spring-statemachine.md>) · `spring-statemachine` · `doctrine`
  Review Spring Statemachine where the project uses its state graph and dependencies. Select persistence guidance separately.
  Read first: [Check the Spring service that actually runs](<spring.md>).
- [Check the Spring service that actually runs](<spring.md>) · `spring` · `doctrine`
  Review a Spring Framework or Spring Boot service using its actual dependencies and configuration.
  Read first: [Model Java states and dependencies explicitly](<java.md>).
- [Check TypeScript types against runtime data](<typescript.md>) · `typescript` · `doctrine`
  Check TypeScript types against runtime data and review JavaScript or TypeScript caller failures.
  Read first: [Change code so it fits](<change.md>).
- [Check Zola templates content and generated pages](<zola.md>) · `zola` · `doctrine`
  Build and review Zola sites with the template engine version they use.
  Read first: [Change code so it fits](<change.md>).
- [Establish what actually ran](<execution.md>) · `execution` · `doctrine`
  Connect a claim to the actual command, inputs and structured result, including child tasks, restoration and cleanup.
- [Give React state effects and async work clear owners](<react.md>) · `react` · `doctrine`
  Review React behavior when React is a dependency or is selected in project configuration.
  Read first: [Build web interfaces around browser behaviour](<web.md>).
- [Give the next worker enough context to continue](<handoff.md>) · `handoff` · `doctrine`
  Prepare a handoff that lets the next worker continue the agreed task with its sources, limits and current state.
- [Keep C++ ownership and lifetimes explicit](<cpp.md>) · `cpp` · `doctrine`
  Write and review C and C++ code.
  Read first: [Change code so it fits](<change.md>).
- [Keep responses timely and data fresh](<response-time.md>) · `response-time` · `doctrine`
  Measure response-time distributions and data age under realistic load, then improve the dominant cost while keeping overload and recovery safe.
- [Keep Vue reactivity and component ownership clear](<vue.md>) · `vue` · `doctrine`
  Review Vue behavior on the web platform. Add TypeScript guidance when the project separately selects TypeScript.
  Read first: [Build web interfaces around browser behaviour](<web.md>).
- [Make failures useful to the caller](<error-handling.md>) · `error-handling` · `doctrine`
  Keep failures distinguishable, recover partial changes, retry safely and stop cancelled work without confusing local cancellation with a remote outcome.
- [Model Java states and dependencies explicitly](<java.md>) · `java` · `doctrine`
  Write and review Java with explicit domain states, dependencies, transactions and failure handling.
  Read first: [Change code so it fits](<change.md>).
- [Trace a security risk to the action it can reach](<security.md>) · `security` · `doctrine`
  Trace untrusted input to protected data and actions; check permission, secrets, exports, browser output and the actual failure path.
- [Write Go with clear interfaces and cancellation](<go.md>) · `go` · `doctrine`
  Write and review Go code using modules.
  Read first: [Change code so it fits](<change.md>).
- [Write Python with explicit inputs and cleanup](<python.md>) · `python` · `doctrine`
  Write and review Python code and its packaging.
  Read first: [Change code so it fits](<change.md>).
  Read as needed: [Trace a security risk to the action it can reach](<security.md>).
- [Write Rust with clear ownership](<rust.md>) · `rust` · `doctrine`
  Write and review Rust with clear ownership, typed failures, deliberate concurrency and checks that reach the required behaviour.
  Read first: [Change code so it fits](<change.md>).

## Tasks

- [Check a journey for races and lost changes](<review/journeys.md>) · `review/journeys` · `operation`
  Trace a journey taken by a person or an agent through competing writes, repeated requests, interrupted replies and permission checks to find races or lost changes.
  Read first: [Review a change](<review.md>).
- [Check whether the design needs its complexity](<review/complexity.md>) · `review/complexity` · `operation`
  Check which present requirements justify added layers, concepts and dependencies, and propose simpler ways to keep the required behaviour.
  Read first: [Review a change](<review.md>).
- [Choose checks that can catch the failure](<testing.md>) · `testing` · `operation`
  Choose checks that can catch the required failure, design a useful test and review what the result supports.
  Read first: [Change code so it fits](<change.md>).
  Read as needed: [Choose checks that can catch the failure](<testing.md>).
- [Find and fix the cause of a failed build](<build.md>) · `build` · `operation`
  Find the cause of a failed build, review the explanation and correct it within the agreed work.
  Read first: [Change code so it fits](<change.md>), [Make failures useful to the caller](<error-handling.md>).
  Read as needed: [Find and fix the cause of a failed build](<build.md>).
- [Find consequential gaps in the tests](<testing/coverage.md>) · `testing/coverage` · `operation`
  Use coverage to find consequential gaps and strengthen tests that would catch a regression.
  Read first: [Choose checks that can catch the failure](<testing.md>).
- [Give the next worker enough context to continue](<handoff.md>) · `handoff` · `operation`
  Prepare a handoff that lets the next worker continue the agreed task with its sources, limits and current state.
- [Improve a measured bottleneck](<performance.md>) · `performance` · `operation`
  Improve a measured bottleneck by changing one cause and comparing correctness, performance and costs under the same conditions.
  Read first: [Change code so it fits](<change.md>), [Keep responses timely and data fresh](<response-time.md>).
- [Prepare an input without losing what matters](<input-conversion.md>) · `input-conversion` · `operation`
  Compare an input conversion with its original, preserving the quality, integrity, privacy, fallback and cancellation the task needs.
  Read first: [Change code so it fits](<change.md>), [Make failures useful to the caller](<error-handling.md>).
- [Review a change](<review.md>) · `review` · `operation`
  Review a local change or pull request, establish each finding and explain what the code and checks support.
  Read first: [Change code so it fits](<change.md>).
  Read as needed: [Review a change](<review.md>).
- [Run the repository check and read its result](<testing/repository.md>) · `testing/repository` · `operation`
  Run the project's check, confirm the required stages and assertions ran, and read the result the consumer receives.
  Read first: [Choose checks that can catch the failure](<testing.md>), [Establish what actually ran](<execution.md>).
- [Simplify code while keeping its behaviour](<refactor.md>) · `refactor` · `operation`
  Simplify code after tracing its callers, keeping the behaviour and compatibility they need.
  Read first: [Change code so it fits](<change.md>).
- [Trace a security risk to the action it can reach](<security.md>) · `security` · `operation`
  Trace untrusted input to protected data and actions; check permission, secrets, exports, browser output and the actual failure path.
  Read first: [Review a change](<review.md>).
  Read as needed: [Trace a security risk to the action it can reach](<security.md>).

## Roles

- [Change code so it fits](<change.md>) · `change` · `role`
  Understand a code change, fit it to its callers and check the whole result.
  Read as needed: [Make failures useful to the caller](<error-handling.md>), [Keep responses timely and data fresh](<response-time.md>), [Trace a security risk to the action it can reach](<security.md>).
- [Check TypeScript types against runtime data](<typescript.md>) · `typescript` · `role`
  Check TypeScript types against runtime data and review JavaScript or TypeScript caller failures.
  Read first: [Change code so it fits](<change.md>).
  Read as needed: [Review a change](<review.md>), [Check TypeScript types against runtime data](<typescript.md>).
- [Check Zola templates content and generated pages](<zola.md>) · `zola` · `role`
  Build and review Zola sites with the template engine version they use.
  Read first: [Change code so it fits](<change.md>).
  Read as needed: [Review a change](<review.md>), [Check Zola templates content and generated pages](<zola.md>).
- [Choose checks that can catch the failure](<testing.md>) · `testing` · `role`
  Choose checks that can catch the required failure, design a useful test and review what the result supports.
  Read first: [Change code so it fits](<change.md>).
- [Find and fix the cause of a failed build](<build.md>) · `build` · `role`
  Find the cause of a failed build, review the explanation and correct it within the agreed work.
  Read first: [Change code so it fits](<change.md>).
- [Give React state effects and async work clear owners](<react.md>) · `react` · `role`
  Review React behavior when React is a dependency or is selected in project configuration.
  Read first: [Build web interfaces around browser behaviour](<web.md>).
  Read as needed: [Give React state effects and async work clear owners](<react.md>), [Review a change](<review.md>).
- [Keep C++ ownership and lifetimes explicit](<cpp.md>) · `cpp` · `role`
  Write and review C and C++ code.
  Read first: [Change code so it fits](<change.md>).
  Read as needed: [Keep C++ ownership and lifetimes explicit](<cpp.md>), [Review a change](<review.md>).
- [Keep Vue reactivity and component ownership clear](<vue.md>) · `vue` · `role`
  Review Vue behavior on the web platform. Add TypeScript guidance when the project separately selects TypeScript.
  Read first: [Build web interfaces around browser behaviour](<web.md>).
  Read as needed: [Review a change](<review.md>), [Keep Vue reactivity and component ownership clear](<vue.md>).
- [Model Java states and dependencies explicitly](<java.md>) · `java` · `role`
  Write and review Java with explicit domain states, dependencies, transactions and failure handling.
  Read first: [Change code so it fits](<change.md>).
  Read as needed: [Model Java states and dependencies explicitly](<java.md>), [Review a change](<review.md>).
- [Review a change](<review.md>) · `review` · `role`
  Review a local change or pull request, establish each finding and explain what the code and checks support.
  Read first: [Change code so it fits](<change.md>).
- [Trace a security risk to the action it can reach](<security.md>) · `security` · `role`
  Trace untrusted input to protected data and actions; check permission, secrets, exports, browser output and the actual failure path.
  Read as needed: [Review a change](<review.md>).
- [Write Go with clear interfaces and cancellation](<go.md>) · `go` · `role`
  Write and review Go code using modules.
  Read first: [Change code so it fits](<change.md>).
  Read as needed: [Write Go with clear interfaces and cancellation](<go.md>), [Review a change](<review.md>).
- [Write Python with explicit inputs and cleanup](<python.md>) · `python` · `role`
  Write and review Python code and its packaging.
  Read first: [Change code so it fits](<change.md>).
  Read as needed: [Write Python with explicit inputs and cleanup](<python.md>), [Review a change](<review.md>), [Trace a security risk to the action it can reach](<security.md>).
- [Write Rust with clear ownership](<rust.md>) · `rust` · `role`
  Write and review Rust with clear ownership, typed failures, deliberate concurrency and checks that reach the required behaviour.
  Read first: [Change code so it fits](<change.md>).
  Read as needed: [Review a change](<review.md>), [Write Rust with clear ownership](<rust.md>).

## Stacks and tools

- [Build web interfaces around browser behaviour](<web.md>) · `web` · `stack`
  Build and review browser interfaces and content.
  Read first: [Change code so it fits](<change.md>).
  Read as needed: [Build web interfaces around browser behaviour](<web.md>).
- [Check JPA queries transactions and managed state](<spring-data-jpa.md>) · `spring-data-jpa` · `stack`
  Review Spring Data JPA where the project uses it for persistence.
  Read first: [Check the Spring service that actually runs](<spring.md>).
  Read as needed: [Check JPA queries transactions and managed state](<spring-data-jpa.md>).
- [Check PyTorch training evaluation and recovery](<pytorch.md>) · `pytorch` · `stack`
  Review PyTorch training when PyTorch is a dependency or is selected in project configuration.
  Read first: [Write Python with explicit inputs and cleanup](<python.md>).
  Read as needed: [Check PyTorch training evaluation and recovery](<pytorch.md>).
- [Check state-machine transitions completion and recovery](<spring-statemachine.md>) · `spring-statemachine` · `stack`
  Review Spring Statemachine where the project uses its state graph and dependencies. Select persistence guidance separately.
  Read first: [Check the Spring service that actually runs](<spring.md>).
  Read as needed: [Check state-machine transitions completion and recovery](<spring-statemachine.md>).
- [Check the Spring service that actually runs](<spring.md>) · `spring` · `stack`
  Review a Spring Framework or Spring Boot service using its actual dependencies and configuration.
  Read first: [Model Java states and dependencies explicitly](<java.md>).
  Read as needed: [Check the Spring service that actually runs](<spring.md>).
- [Check TypeScript types against runtime data](<typescript.md>) · `typescript` · `stack`
  Check TypeScript types against runtime data and review JavaScript or TypeScript caller failures.
  Read first: [Change code so it fits](<change.md>).
  Read as needed: [Check TypeScript types against runtime data](<typescript.md>).
- [Check Zola templates content and generated pages](<zola.md>) · `zola` · `stack`
  Build and review Zola sites with the template engine version they use.
  Read first: [Change code so it fits](<change.md>).
  Read as needed: [Check Zola templates content and generated pages](<zola.md>).
- [Give React state effects and async work clear owners](<react.md>) · `react` · `stack`
  Review React behavior when React is a dependency or is selected in project configuration.
  Read first: [Build web interfaces around browser behaviour](<web.md>).
  Read as needed: [Give React state effects and async work clear owners](<react.md>).
- [Keep C++ ownership and lifetimes explicit](<cpp.md>) · `cpp` · `stack`
  Write and review C and C++ code.
  Read first: [Change code so it fits](<change.md>).
  Read as needed: [Keep C++ ownership and lifetimes explicit](<cpp.md>).
- [Keep Vue reactivity and component ownership clear](<vue.md>) · `vue` · `stack`
  Review Vue behavior on the web platform. Add TypeScript guidance when the project separately selects TypeScript.
  Read first: [Build web interfaces around browser behaviour](<web.md>).
  Read as needed: [Keep Vue reactivity and component ownership clear](<vue.md>).
- [Model Java states and dependencies explicitly](<java.md>) · `java` · `stack`
  Write and review Java with explicit domain states, dependencies, transactions and failure handling.
  Read first: [Change code so it fits](<change.md>).
  Read as needed: [Model Java states and dependencies explicitly](<java.md>).
- [repository-verifier](<repository-verifier.toml>) · `repository-verifier` · `tool_recipe`
  Run the repository’s chosen check and report its result and limits.
- [Write Go with clear interfaces and cancellation](<go.md>) · `go` · `stack`
  Write and review Go code using modules.
  Read first: [Change code so it fits](<change.md>).
  Read as needed: [Write Go with clear interfaces and cancellation](<go.md>).
- [Write Python with explicit inputs and cleanup](<python.md>) · `python` · `stack`
  Write and review Python code and its packaging.
  Read first: [Change code so it fits](<change.md>).
  Read as needed: [Write Python with explicit inputs and cleanup](<python.md>), [Trace a security risk to the action it can reach](<security.md>).
- [Write Rust with clear ownership](<rust.md>) · `rust` · `stack`
  Write and review Rust with clear ownership, typed failures, deliberate concurrency and checks that reach the required behaviour.
  Read first: [Change code so it fits](<change.md>).
  Read as needed: [Write Rust with clear ownership](<rust.md>).

## Policies and other resources

- [Change code so it fits](<change.md>) · `change` · `policy_family`
  Understand a code change, fit it to its callers and check the whole result.
  Read as needed: [Make failures useful to the caller](<error-handling.md>), [Keep responses timely and data fresh](<response-time.md>), [Trace a security risk to the action it can reach](<security.md>).
- [Choose checks that can catch the failure](<testing.md>) · `testing` · `policy_family`
  Choose checks that can catch the required failure, design a useful test and review what the result supports.
  Read first: [Change code so it fits](<change.md>).
- [Choose the simplest complete solution](<change/choice.md>) · `change/choice` · `policy_family`
  After understanding the change and its guarantees, check what is necessary, reuse suitable code and platform facilities, and choose a complete solution that people can maintain.
  Read first: [Change code so it fits](<change.md>).
- [governing-order](<governing-order.toml>) · `governing-order` · `governing_order`
- [Keep the guarantees the change must respect](<change/constraints.md>) · `change/constraints` · `policy_family`
  Keep correctness, security, accessibility, input validation, recovery and requested behaviour intact while simplifying. Check who owns the affected data and decisions, and how the consuming system enforces those limits.
  Read first: [Change code so it fits](<change.md>).
- [Make unfinished work and accepted limits explicit](<change/limits.md>) · `change/limits` · `policy_family`
  Put unfinished work and accepted limits on the owning work list, with the responsible owner, checks and an observable reason to revisit them.
  Read first: [Change code so it fits](<change.md>).
- [Review a change](<review.md>) · `review` · `policy_family`
  Review a local change or pull request, establish each finding and explain what the code and checks support.
  Read first: [Change code so it fits](<change.md>).
- [Understand the change and what depends on it](<change/impact.md>) · `change/impact` · `policy_family`
  Trace the current behaviour from its entry to its result, find the cause or the right place for a new capability, and check which users and guarantees the change affects before choosing a solution.
  Read first: [Change code so it fits](<change.md>).
- [verification-policy](<verification-policy.toml>) · `verification-policy` · `capability_policy`
