# Review a workspace configuration change

Identify the current and proposed workspace declarations, pinned Package releases, project, execution scope, machine connection and target system. In the review, state the owner's requested change and the candidate being considered. Keep portable choices separate from machine paths and credentials. Establish permission and tool availability before planning an action.

When an approved tool provides a structured review, give it the same captured version of the owner's inputs. Send only documents it may receive. Keep the preview's full identity through impact inspection and the review result. If an evidence page is incomplete, continue through the review tool's saved result or state the remaining gap. A favourable review and permission to apply are separate decisions.

## Trace stack guidance to its declaration

Distinguish the Package release available to a project from the resources chosen for each repository. Inspect project-wide `semantic.stacks` in the capabilities file and `stacks` on each `project_repositories` membership in workspace topology. A repository receives its project-wide and membership selections. An empty project selection leaves the membership selection in force. Selecting the same exact stack at both locations contributes its guidance once.

Choose a stack project-wide when it applies across the relevant repositories; use repository membership for a narrower need. To remove guidance, find and remove the selections or other permitted dependency paths that actually bring it in. Check the rendered guidance and confirm it is absent from repositories where it should not apply. Keep the stack, operation, tool recipe and actual command connection distinguishable. Stack guidance grants neither tool permission nor runtime availability.

Exercise a project with two different repository stacks, a project-wide stack reaching both, a duplicate exact selection and a removal. Compare CLI and MCP plans and read back each host's generated instructions. Keep an independently chosen operation and tool request in the control case so their continued presence cannot be mistaken for a stack-selection effect.

## Check ownership before taking over or retiring generated content

Compare current and proposed selections. Inspect the system's plan for files or blocks it will create, update, retire or take over. Distinguish whole owned files, managed blocks and surrounding user content. Matching bytes or a familiar marker cannot establish ownership. Trace the target to its accepted origin and confirm that the local ownership record is backed by the responsible owner. Leave unsupported ownership claims unresolved.

Taking over unowned content needs the owner's explicit permission for the target, observed baseline and proposed plan. Before the first change, recheck the baseline, including metadata used by the plan. Changed premises need a new review. For a managed block, check its beginning and end and preserve surrounding content. For a whole file, preserve the original bytes separately when recovery needs them.

The authorised operator applies the reviewed plan through the owning system and keeps its result records. Independently read back the target and ownership. Distinguish planned, written, active and invoked states. If application or recording of the result fails, check restored bytes and ownership before retrying. A successful retry needs its own evidence.

For retirement, distinguish removing the owned content from separately permitted restoration of the original state. Establish what is removed, what user content remains and who owns recovery. Exercise fresh installation, matching unowned bytes, explicit takeover, baseline changes and removal. If rollback can race an owner edit, check that it preserves the edit and identify any unresolved restoration by path in the review.

### Rehearse removal of an older launcher

Find every place the host can start the predecessor, including hooks, tool-server entries and entry-page references. Compare observed bytes and metadata with saved plans and ownership records. Give the owner each mismatch against the observed baseline to resolve. A valid successor plan can leave an older marker or launcher behind; inspect effective host configuration for both generations.

On disposable copies, remove only the approved entries or marker span. Preserve unrelated matchers, servers, settings and surrounding text. Check the before and after configurations independently, then rehearse restoration of the exact original bytes and metadata. A change after either baseline stops the corresponding removal or restoration. Keep the changed path in the review.

For each retired entry, name its duty, successor or separately accepted end, and the proof needed at the actual host. Reading back a generated file does not establish host trust, session acceptance, tool invocation or completion of the action. The rehearsal informs the operator's decision; live retirement still needs the owner's permission and proof that replacement duties work.

### Distinguish launchers, permissions and history

For the host and workspace in question, trace which configuration takes effect, including inheritance, overrides and disabled routes. Identify the event, trigger or server entry that can start the runtime. Follow wrappers to the resolved executable, arguments, configuration, context and catalog it consumes. Identify the runtime by its accepted path and content. Two generations can share a command name; similarly named files or archives can contain different bytes or serve different purposes.

Distinguish launch entries from allow rules, environment and directory grants, instruction text, sealed action records and installed but unused files. A permission rule may change what a caller can do without scheduling a process. A historical mention may support recovery without being a live dependency. An entry page may still tell a person or agent to invoke the older runtime; inspect that route too.

Check a permission-only reference, a disabled or overridden launcher, an indirect wrapper and a second installed generation. Compare actual targets and dependency digests. In the review, state which configuration scopes were inspected and any unresolved precedence or indirection. Static inspection establishes configured references and blockers within that scope. Observe host loading and invocation separately when retirement depends on them.

### Recheck saved trust when reusing a project path

Inspect saved project trust separately from repository launchers, individual hook approvals, tool-server approvals and tool permissions. Associate each observation with its host, configuration layer, resolved project root and checkout identity. Include inherited or overridden entries and trust for paths that no longer exist. A new checkout can reuse a path whose old trust record remains.

Before replacing or restoring a checkout at a former path, inspect effective configuration for the replacement in a disposable host context. Rehearse retained trust, a fresh path with no entry and an explicitly untrusted path. Keep the project declaration fixed while varying trust. Observe which layers load and whether the successor becomes eligible for a hook or server. Where approval belongs to a specific definition, change the definition and check its approval separately. Configuration may establish eligibility; startup and invocation need their own observed results.

Treat stale trust as an owner decision during replacement. Identify the record to change, its exact baseline and its effect on the successor. Keep configuration cleanup, a new trust decision, runtime activation and retirement as distinct steps. Preserve unrelated trust entries and limit the finding to the host tested.

### Check which runtime a launcher actually starts

Trace the host entry through shims and wrappers to the platform package, executable and required runtime. Establish their source identities and the locations the resolver may search. Identical launcher bytes and reported versions can still select different implementations. Inspect cache, download, update and cleanup behaviour separately from tool invocation.

In a disposable cache, test an absent selected package with an empty cache and with a same-version substitute. Require refusal before an unapproved implementation runs or cache maintenance changes state. An approved fallback needs its own identified implementation and checks. Preserve evidence of the original failure, and read back the implementation and state actually used by each control case.

### Check that the tool server uses the chosen catalog

Associate the permitted launch with its repository, profile, resolved instance, exact catalog and schema, and executable. Compare the effective host entry and actual startup arguments with that configuration. A sealed catalog beside a launch command cannot restrict a process that never reads it.

For repository-level server registration, identify the config file, server key, accepted executable and argument list. Generated output may contain only a request to attach the server. Review the actual registration edit separately, applying the ownership and baseline checks above. Reparse the edited file and check both the chosen entry and unrelated servers and settings. Exercise installation, unchanged reapplication, removal and preservation of owner edits before judging effective host invocation.

Take expected roots, context and digests from the accepted launch configuration. Read regular files within those roots, with size limits and without following redirects. Register tools from verified catalog bytes and establish that the launched executable is the one checked. Test changed bytes and a symbolic link to matching bytes separately. If several hosts share a repository catalog, check the actions exposed to each host.

Inspect tool discovery from the actual process and invoke the chosen tool through it. Try direct calls to an unselected tool as well as discovery. Test a changed catalog, wrong instance and unavailable restricted startup. Require refusal before serving tools, and check that the host cannot silently use a broader server instead.

Record process scope separately from host registration. Observe the actual host's protocol handshake and startup refusal before crediting discovery. Explicitly choose any compatibility transport for the permitted host, preserving the catalog, tool inputs, typed result and action record across transports. Check refusal of an unselected protocol or lifecycle. For asynchronous discovery, inspect which tools reached the model request and whether the host returned the result. A direct invocation establishes only the transport it used.

Compare the host's registration listing with the execution mode actually used. Identify project trust, server approval, launch flags and tool allow rules for that invocation. A listing can show pending approval while a command-line allowance permits execution. Keep the catalog fixed and vary approval or allowance separately. Observe server startup, discovery, delivery of tools to the model and invocation independently. The listing alone cannot establish which permissions execution enforced.

Return the plan, the scope of the checks, unresolved items and the owner's next decision in the requested review. Application belongs to the authorised operator.

## Before handing off

- [ ] Are the workspace, entry and configuration changed clearly identified?
- [ ] Has the real host's behaviour been checked after the change?
- [ ] Are recoverable failures and unresolved owner decisions clear?
