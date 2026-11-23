# Identify which release, method and run support the claim

## Verify release and runtime evidence separately

Match each claim to evidence that can establish it. Release evidence identifies the exact manifest or tagged revision, its artifact inventory and who attested to the release. A runtime artifact identifies the result, producer, run, attempt, selected configuration and inputs. Permission to use inputs or deliver a result identifies the actor, protected material, operation, destination and validity. Keep these decisions separate even when one owner or key serves several roles.

Verify each signature under the rules for its evidence type. Resolve the canonical signed bytes, signature domain, signer role, key revision and applicable validity or revocation policy. Before accepting a runtime artifact, compare its verified bytes and identities with the selected attempt and result. A verified release manifest supports claims about its listed contents. The runtime result still needs the signature or attestation required by its own contract. Current use and delivery require their own permission checks.

Test a valid release with an unsigned runtime result, changed result bytes under an original signature, and a valid signature from a key or domain outside the runtime contract. Then test a correctly signed result whose input-use or delivery permission has expired or changed. Preserve the historical verification within its scope and hold the affected new action. Report release verification, artifact verification, permission and any downstream decision separately.

## Check a versioned measurement definition

Give each measurement one definition and revision controlled by its owner. State its stable identifier and labels, what it measures and which decisions it may inform, population, observation unit, units, whether higher or lower values mean more of the measured property, numerator and denominator or counting rule, source and coverage. Specify the time basis and lag, preprocessing order, fit window for learned transforms, aggregation and endpoint rules, and permitted handling of missing, invalid and fallback values. An observed field, proxy, estimate and permission decision remain different things even when their labels match.

Follow one selected revision from the producer's expression through every rename, conversion, transformation and aggregation into each consumer. Check calculation, prompt, notebook and report assumptions against that definition. Give a change in meaning its own revision and verify which consumers can still use earlier results. Reusing a symbol or producing equal output in one example does not establish equivalent meaning.

Use the lag to determine the requested observation period, then separately select a data revision available by the exact decision cutoff. Zero lag does not mean that period's data is already available. Fit learned preprocessing only on permitted training data. Distinguish missing values, suppression, invalid encoding, observed zero and policy fallback. Label any fallback in the consumer's evidence.

Test similar labels for different quantities, a reversed sign or cumulative direction, a blank beside a genuine zero, and a revision published after the cutoff. Inspect the source value, definition revision, derived value, handling of missing data and meaning used by the consumer. Return unresolved mappings to the owner before relying on the measure.

## Check the managed method that ran

Resolve mutable release labels when accepting a run. Fix a compatible set of workflow, adapter, prompt, components, provider settings, output contract and owner controls. Retain the supplied evidence, compiled request and settings actually applied as bytes or approved exact references, within the owner's retention and disclosure rules. A fixture using fixed text tests assembly; evidence from the selected adapter is needed to establish runtime retrieval and application.

Distinguish reuse of a cached prompt definition from reuse of a completed response. For the first, check the resolved prompt and configuration. For the second, require the same permitted request, method and execution scope, and count only the original attempt. A replay cannot count a cached live result as a fresh comparison attempt. Test rollback against the compatible method and both types of cache. Exercise a moved label, changed component or applied setting, and a response cached under another method or mode. If runtime evidence cannot identify the method, leave it unresolved.

## Separate assigned policy from observed execution

Keep the assigned candidate, intended route and each attempt's observed policy or provider identity separate. Use execution evidence from the owning system to connect the actual route with its case, attempt and outcome. An assignment, request label or successful response does not identify which implementation produced it. If the route changes after partial work, retain both stages and mark the outcome as mixed. Leave an unverified route unknown.

Keep every assigned case in the assignment denominator. Report coverage for the selected candidate, permitted fallback, unexpected routes, mixed routes, unavailable routes and unknown routes separately. Attribute results to the route that produced them. If the study estimates the effect of assigning a route including fallback, report that estimate with its assignment and missing-outcome rules. Grouping results by observed route alone does not establish a causal policy effect. Test a fallback that retains the assignment label and verify that its result cannot be counted as an observed result from the candidate.

## Worked example

Two runs carry the same evaluation label and produce different outputs. Compare the evaluator definition, version, actual execution path and result records. The policy assigned in configuration can differ from the provider that executed the run.
