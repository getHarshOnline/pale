# Decide whether work can run unattended

Autonomy changes who is present when decisions are made. The work still needs permission for its effects. Use this guide to decide whether a defined task can run repeatedly or unattended, and what controls must exist before the engine or service starts it. Establish scheduling, memory, dispatch, computer use and external actions from the actual tools and runtime.

## Define the work and its limits

In the proposed design, state an observable objective, its trigger, accepted inputs, allowed output, maximum effects, the owners of the affected systems and when the work ends. Check which capabilities are connected and working, including how they fail. A proposed capability, apparently installed configuration or written description cannot establish availability.

Set separate budgets for elapsed time, attempts, spending, rate, concurrency, stored state and external actions. Say which decisions use deterministic rules, which need model judgment, which need fresh approval and which are forbidden. Permission for one action does not grant permission for later repetitions.

## Make progress and intervention observable

Use progress measures that are harder to game than the activity being measured: externally observable state, independent checks and milestones whose completed work remains valid. Define checkpoints, signs of stale work, retryable and terminal failures, who receives an escalation, when to stop the process and where recovery can resume. For work that changes state, establish whether retrying is safe, how to undo or compensate for changes, and which result record distinguishes an attempt from a completed action.

Persistent state is optional and has its own owner. If needed, specify its purpose, minimum content, scope, retention, confidentiality and what invalidates it. The Package helps decide these requirements. The engine or owning service supplies storage, scheduling, queues, process control and action records.

## Choose the permitted way forward

Allow autonomous execution only when the objective and stopping condition can be observed well enough for the risk, all required capabilities work, effects have limits, budgets can be enforced, recovery has an owner and missing required evidence stops the work. Otherwise propose a smaller task: one attended execution, a read-only observation, a plan for review or a manual checkpoint between iterations.

Leave execution unresolved when the goal can drift, the verifier shares the actor's blind spot, effects cannot be contained, private state has no owner, withdrawn permission cannot reach the runner, budgets cannot be enforced, or no person or system can intervene. Return the design with the work and effect limits, capability checks, budgets, required evidence, decisions and permissions, persistent-state rules, stopping and recovery rules, chosen approach and conditions for resuming. A completed design establishes readiness for review; use runtime evidence to establish whether a process started.
