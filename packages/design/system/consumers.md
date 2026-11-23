+++
description = "Use it to trace a design problem to the identity, shared component or page that owns it and to follow its impact."
uses = [{ scope = "local", resource_id = "system" }]

[guide]
exposure = "internal"
+++

# Trace a design problem to its owning source

## Diagnose at the source and follow the impact

| Observed pattern | Owning correction |
| --- | --- |
| Character is wrong across one identity's pages and components | Reconcile the accepted direction and its tokens; local spacing patches cannot repair it. |
| The same component fails wherever it appears | Correct the shared component or recipe once, then follow its consumers and overrides. |
| One composition fails while its shared components are sound | Repair that page within its accepted identity and permissions. |
| One gap, alignment or line break fails locally | Correct that instance; repeated instances may reveal a shared defect. |

Locate the producing token, winning style declaration, template or script. Use computed styles for a wrong rendered value, the accessibility tree for a wrong name or order, registered listeners for a dead control, console and network for a quiet load failure, and an actual trace for slowness. Source that looks plausible is only a hypothesis until connected to the symptom. Record an unclear cause as unresolved; do not compensate blindly elsewhere.

Trace the changed source through actual token references, component imports, template inheritance, script hooks and rendered uses. Word matches and page counts are not dependency proof. Include runtime-created controls and world overrides. Remove obsolete workarounds after consumers migrate; retain an intentional exception with its owner and reason.

For token rename or removal, include aliases, computed names, build references and structured output consumers. A literal search with no matches proves neither absence nor safe deletion. Source equality, resolver coverage, emitted property and actual computed value answer different questions. Reuse inspection and build tools rather than maintaining a second resolver. The [DTCG format](https://www.designtokens.org/tr/2025.10/format/) describes exchange structure; the selected producer owns its extensions and projection behavior.

Recheck the original failure and the reached consumers at the conditions the change can affect. Keep earlier evidence for unaffected source and claims. A changed future-authoring instruction does not alter an already rendered artifact. Keep owner acceptance, implemented behavior and current verification separate, with a source and scope for each. Update current rationale at its existing owner; routine output belongs with the change's verification, rather than another permanent decision register.

## Fix the source that produces the rendered result

A defect across an identity's material, light or type can require changing its own theme or design roles. The same control fault in several views points to the shared recipe. One page's rhythm or arrangement can be repaired in that page. A local break or alignment may need a small correction. Trace the observed value to its token, selector, template, script or source input before changing a second layer. If the level is uncertain, record what is known and test the likely source.

Start with the meaning, access and behavior the reader depends on, then apply the identity, local treatment and finer adjustments. A local decoration cannot change a published claim or remove the only keyboard route. Where two reasonable visual rules favor different treatments, compare them in the affected context and keep the reason local. Where the conflict affects required meaning or operability, settle it from the source and supported behavior before changing that part.

For each reached consumer, compare the old and new role or behavior and repeat the relevant check. Read the actual imports, templates or output to identify them. Count approved changes, observed implementation and verified cases separately. A screenshot that predates a shared-source change needs a new check only where it depended on that source. Keep unrelated results.

## Worked example

A caption is too close to the next figure across four templates. Inspect the winning rule, find the shared spacing recipe, and trace template inheritance to each consumer. Check the renderer’s computed spacing and a known narrow case after the change. If one template has a useful exception, keep it with its reason. For an alias rename also follow resolved and generated keys, because a literal search can miss composed names.
