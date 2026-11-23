# Choose checks that can catch the failure

Choose checks from the requested outcome and the consequences found while tracing the change. For each claim, identify what can be observed, which component produces it, what a wrong result would look like, and the environment needed to distinguish correct behaviour from failure. Begin with the smallest useful check, then cover the risks the change actually introduces.

Use deterministic checks for mechanical rules. Use scenarios, rendered output, performance measurements, security checks or expert review when the claim depends on those conditions. A quick self-check is the starting point; the guarantees the system must keep still determine the required work.

For regressions, parsers, state changes, calculations and untrusted input, observing the missing behaviour before the fix often makes the test stronger. Use that sequence when it helps establish the cause, and leave a runnable regression check for non-trivial logic. Prose and metadata may need inspection; a connection to another system may need a real integration check. Choose the check that can answer the question.

Read the project's declared checks and find out what they do before running them. A command called `check` can generate files, synchronize state or contact a service. Use the project's actual runner and the permission already given for those effects; an example command in a guide does not establish either.

Expand the checks according to the risk: the code that causes the behaviour, the changed interface, direct users, a representative journey, then wider regressions. Make sure the checks exercise the claim: a large suite can miss the failure, and a focused test supports only the path it covers. For an area expected to remain unchanged, check the interface that isolates it or the representative behaviour identified in the analysis.

For generated agent instructions, check each step separately: compilation, writing the output, the host making it available, invoking the tool and using its result. Start a fresh session to observe that route. Open links from the generated file and confirm they reach the named sections. Where a procedure claims to catch a missing input or a wrong tool route, deliberately introduce that failure in a safe test and check the response.

In the reply or existing verification record, pair each claim with the check run, its result, the artifact or environment tested, and any remaining limit. Distinguish a product bug, a broken test, an environment problem, an unavailable capability and a check that was not run. A required check that fails, is stale or remains unknown leaves the work open unless the responsible owner has explicitly accepted the limit.
