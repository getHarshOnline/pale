+++
description = "Exercise the requested desktop task in isolated state and keep useful failure diagnostics."

[task]
uses = [{ scope = "local", resource_id = "user-task" }, { scope = "local", resource_id = "access" }]
roles = [{ scope = "external", resource = { package_id = "pale.engineering", resource_id = "testing" } }]
outputs = ["Scenario results and diagnostics in the maintained test output or existing review.", "Product failures, environment limitations and unobserved outcomes stated in the reply."]
+++

# Check a task in the desktop application

Choose one claim that depends on the desktop application. Identify the permitted build and platform, starting state, user-visible path and relevant accessibility, privacy and recovery requirements. Follow the shared end-to-end method, selecting the transitions and observations needed to decide this claim. Keep expected behavior, observed behavior and what the test can establish separate.

Use the actual application build, platform version and input route. Check input permissions, initial application state, test isolation and cleanup before calling the result reproducible. OS focus, accessibility, persistence and restoration need observations from the desktop application; a browser test or plan cannot establish them.

Return scenario results and a verdict in the existing review or reply. Distinguish a demonstrated product mismatch from a result made inconclusive by the environment or observation tools. Keep product repairs outside this verification task, describe only observed results and leave installation, release and deployment decisions with their owners.

## Before handing off

- [ ] Did the actual desktop application receive the intended input?
- [ ] Were the task, state changes and recovery checked in its real window?
- [ ] Does the verdict say what the tested environment establishes and what remains untested?
