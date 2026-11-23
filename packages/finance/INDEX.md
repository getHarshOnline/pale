# Finance

Package ID: `pale.finance` · [Manifest](package.toml)

Frame a money question, project an owner's books to see what fails first, and close each owner's books for review.

A workspace selects entries from this family for the duties its people and agents perform. To read it directly, open the page for your task and follow its reading links.

## Tasks

- [Close an owner's books for a period](<close.md>) · `close` · `operation`
  Close one owner's books for a period from statements and the accountant's books, and prepare the differences for review.
  Read first: [Reconcile a register with its controlling records](<../assurance/register.md>), [Check a workbook export or reconstructed view](<../evaluation/workbook.md>).
- [Frame a money question](<question.md>) · `question` · `operation`
  Frame a money question so one run can answer it: whose books, which facts and rules, what is unknown and what counts as failing.
  Read first: [Research which rules apply to a proposed act](<../assurance/rules.md>), [Judge what research evidence supports](<../research/evidence.md>).
- [Project the books and read what fails first](<project.md>) · `project` · `operation`
  Project an owner's books under a framed question, sweep the amount set aside and read what fails first.
  Read first: [Check a calculation request and its authority](<../assurance/calculation.md>), [Check numerical results against independent references](<../evaluation/numbers.md>), [Check the candidate an optimiser produces](<../evaluation/optimisation.md>), [Make decision notebooks reproducible](<../evaluation/notebook.md>), [Check a workbook export or reconstructed view](<../evaluation/workbook.md>), [Trace evaluation inputs and compare runs](<../evaluation/runs.md>), [Compare changes separately and together](<../evaluation/compare.md>).

## Stacks and tools

- [check-run](<check-run.toml>) · `check-run` · `tool_recipe`
  Check the question's declaration, captured snapshot and selected Packs, and report what would stop a run.
- [close-run](<close-run.toml>) · `close-run` · `tool_recipe`
  Reconcile one owner's opening position, movements and closing position for the period, and return the open differences and the run record.
- [project-run](<project-run.toml>) · `project-run` · `tool_recipe`
  Project one owner's books under the question's scenarios or sweep, and return results, findings and the run record.

## Policies and other resources

- [run-policy](<run-policy.toml>) · `run-policy` · `capability_policy`
