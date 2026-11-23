+++
description = "Use it to choose reading density, grouping and visible tools from what most readers came to do."
uses = [{ scope = "local", resource_id = "presentation" }]
see_also = [{ scope = "local", resource_id = "presentation/preserve-the-payload" }]

[guide]
exposure = "internal"
+++

# Choose the presentation from the reader’s task

Name what most readers came to do. That determines the reading density, grouping and visible tools. Identity applies across those choices. A site can contain several page tasks, and a project or content family may need different treatments for different requests.

| Main task | Give the reader |
| --- | --- |
| Arrive and orient | What this place is, what is here and visible entry routes. |
| Read a long argument | Continuous reading, with contents, notes and tools where they can be reached. |
| Learn or do a specific job | A usable outline, prerequisites where needed, worked examples and a path to deeper reference. |
| Decide whether to act | Scope, comparison, material terms and an appropriate next action. |
| Follow parts in order | The parts, current position, and a reachable next or previous step. |
| Read and cite a formal document | Its title, version, abstract, references and citation facts. |
| Understand a product | What it does, for whom, its actual state, availability and useful demonstrations. |
| Follow a narrative | Pace, continuity, and a way to resume or continue. |
| Scan and choose | Grouped results, understandable filters and sort, and actual destinations. |
| Do one small task | Search, settings, an error or another focused action with usable recovery. |

Treat the table as a way to reason about a page, then apply the project's supported layouts and route mapping. A publication can use its established articles, collections and journeys for product help. Its documentation category need not imply a new visual family. Developer reference for the repository can have its own outline and destination. A page-wide exception is appropriate only when its actual content shares the task. Keep a narrower exception open if the current templates cannot express it safely.

Let a secondary region support the dominant task: an intake can follow an explanation, or a film can occupy a meaningful passage without turning the whole page into a player. Raise density through grouping and hierarchy before shrinking text. Put reading tools within reach without having them compete with the argument; give filtering and sorting more prominence when choosing among records is the task. Use the native navigation, search and complementary regions that describe those jobs, rather than changing roles to match a visual style.

Preserve the [Keep facts and relationships through a change of form](preserve-the-payload.md) when changing the mode. An article and its index entry can have different reading rhythms. The published state of the subject, the current navigation and the reader's history remain separate.

Mark the current navigation page with `aria-current="page"`, or the current step with `aria-current="step"` when that is the relation. Keep one current item in the relevant set. A tab's selection uses the tab pattern and `aria-selected`; `aria-current` is not its replacement. Inspect the accessibility tree after navigation, then check that the published status text has not changed. `:visited` is a privacy-restricted color treatment, not a readable history database or a reliable “read” flag. If the product offers saved or read state, use its explicit state model and labels.

## Worked example

A release has a long account of one important change and a dated list of earlier changes. Give the account a reading layout. Keep the earlier list easy to scan, with dates and links. An install guide uses an ordered path with a next step, even if it shares parts of the article template. Opening the guide should not change what the site claims about the release's availability.

## Sources

- [WAI-ARIA landmark regions](https://www.w3.org/WAI/ARIA/apg/practices/landmark-regions/): navigation, search and supporting content with roles that follow their purpose.
- [WAI-ARIA `aria-current`](https://www.w3.org/TR/wai-aria-1.2/#aria-current): current page or step, including its distinction from selection in widgets.
- [MDN `:visited`](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Selectors/:visited): allowed color properties and browser privacy restrictions on observing visited state.
