# Review a contract change

Fix the accepted predecessor and proposed contract versions for the review. Identify every affected producer and consumer. Compare field meanings, errors, state transitions, compatibility promises and the sources from which generated files were built. Use the HTTP guidance only when the interface uses HTTP.

In the review reply, explain each defect through an actual caller, triggering input and consequence. Name the negative and compatibility tests required before release, including what has already run, what failed and who owns the remaining checks.

## Before handing off

- [ ] Compare old and new producer and consumer behavior.
- [ ] Check compatibility, validation and migration at the actual interface.
- [ ] Name outstanding failures and who can resolve them.
