# Understand the change and what depends on it

Before choosing a solution, understand the requested outcome, how the current system produces it, and what else depends on that behaviour. A small edit can change a shared rule used throughout the system. Read enough to explain the cause and its consequences; the number of changed lines cannot tell you how much investigation is needed.

For a focused repair, the owning source, the affected path and its direct callers may be enough. Follow further dependencies when those reads expose shared rules, uncertain effects or other users. Reuse earlier findings while their sources and the system's state still support them.

## Establish the outcome

In your working analysis, name who uses the behaviour, the starting condition, the result they need and anything explicitly outside the request. Distinguish the owner's facts and decisions from your interpretation, assumptions and preferences. If two plausible readings would lead to materially different behaviour or effects, get the missing decision before doing that part. Otherwise use the narrowest reading that fulfils the request and state the assumption.

Keep the analysis tied to the repository or artifact version you inspected, its governing instructions and any outside system whose rules it must follow. A plan, issue, example or generated view can explain the intended behaviour. Where it differs from the current source, establish who can settle the difference before treating it as a direction to change the code.

## Follow the behaviour through the system

Start where the user or caller enters and follow the path to its result. Read calls, data transformations, state changes, errors, retries, fallbacks and changes to outside systems. For declarations and content, also trace identity, references, ordering, selection, interpretation and how readers or other programs use the result.

At each step, establish:

- which component owns the meaning or state;
- what passes between the components and which rules govern it;
- what must be true for the step to be valid;
- what failure or a missing value means;
- who uses this result directly; and
- which later users inherit its consequences.

Search aliases, generated forms, fixtures, configuration, documentation, migrations and rejection tests as well as the main implementation. Use structural tools to find callers and dependencies, then read the source and run the relevant checks to understand those connections.

When work crosses components, follow the caller down to the provider and the eventual change, then follow the result back to the receiver that accepts it. Check other routes that use the same rule or artifact. In your analysis, note when a connection applies only under a particular condition. A provider can produce a valid result that its receiver cannot accept because it belongs to a different subject or is too old.

## Find the cause and the right place to change

For a defect, find the earliest incorrect step needed to produce the observed failure. Compare plausible causes against the same observations, and note which observation supports or rules out each one. An exception or wrong output helps locate the problem; check what produced it. If the available evidence cannot distinguish the causes, name the missing observation and obtain it before changing code on that assumption.

For a new capability, find the existing component responsible for the behaviour and extend it there. Check whether its meaning and lifecycle fit before reusing a mechanism. Two pieces of code that look alike may still belong to different owners or follow different rules. A second policy, field, cache, adapter or special case needs a reason the owning component cannot already satisfy.

## Work out the consequences

Group the consequences by what changes, so a reviewer can understand why each affected file matters:

- meaning, guarantees, decisions and who may make them;
- accepted inputs, outputs, errors, compatibility and callers' expectations;
- data formats, validation, migration, retention and recovery;
- creation, updates, replacement, rollback, removal and interpretation of old records;
- performance, security, availability, diagnostics and how failures spread; and
- direct callers, generated artifacts, integrations and people's workflows.

If you claim an area is unaffected, support that claim: there is no dependency path, an unchanged interface isolates it, compatibility handling keeps its behaviour, or a focused check demonstrates it. Pay particular attention to shared schemas, defaults, identity rules, ordering, serialization and fallbacks, even when the proposed edit is short.

Check compatibility through behaviour. Which old inputs and stored artifacts remain readable? Can old and new versions work together? Who performs any required migration, and how does it recover from partial failure? Distinguish accidental behaviour from behaviour people rely on before deciding what to keep or change.

## Choose the work and its checks

Once the consequences are understood, propose the smallest complete repair. Name the owning source, the dependent components that must change, generated or migrated artifacts, and the parts outside this change. If it reaches another owner's decisions, repository or data, or an irreversible effect, split the work or obtain the missing decision before extending the scope. Use permission already given for the agreed work.

Choose checks for the cause or requested outcome, each changed interface, important failure paths, migration and rollback where relevant, and representative areas expected to stay unchanged. Separate checks already run from those still planned. A nearby passing test or a broad passing suite is useful only to the extent that it exercises the behaviour being claimed.

For a focused repair, explain the cause, affected components, correction, check and remaining uncertainty in the reply. For a change spanning interfaces, migration, ownership, recovery or unresolved dependencies, put the necessary map and decisions in the existing handoff or requested analysis. Include what changes implementation or acceptance; these categories need no separate dossier or compulsory report sections.

The analysis is complete when it supports a specific implementation decision. Revisit it if the inspected state changes, another user or dependency appears, a check contradicts the explanation, or an owner resolves an open question. Carry the proposed checks into implementation and report their results after running them.
