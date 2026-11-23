# Inspect one agent run

Identify the host event, selected route, run, expected next step and expected result. Distinguish permission checks, lookup, provider dispatch, execution, returned output, saved state and actual effects.

From the host's original events and results, find the first step whose output no longer supports what should happen next. Name the smallest authorised check that could show your explanation is wrong. In the diagnosis, propose the correction or recovery work, its permitted scope and the evidence needed before the stopped work can resume.

## Before handing off

- [ ] Has the run been traced from its original request and events?
- [ ] Is the first unsupported step or missing result identified?
- [ ] Does the proposed correction follow from what the run actually did?
