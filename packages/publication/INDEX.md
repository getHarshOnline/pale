# Publication

Package ID: `pale.publication` · [Manifest](package.toml)

Write and adapt articles and documentation, check claims and voice, prepare media, and inspect a local publication build.

A workspace selects entries from this family for the duties its people and agents perform. To read it directly, open the page for your task and follow its reading links.

## Guides

- [Adapt content across media](<adapt-media.md>) · `adapt-media` · `doctrine`
  Adapt content across the requested media while preserving claims, evidence and voice.
  Read first: [Preserve meaning across presentations](<../design/presentation.md>).
- [Check a reference in editing and published output](<references.md>) · `references` · `doctrine`
  Check a reference through editor navigation, incoming-reference search and the actual reader destination.
  Read as needed: [Write for the reader's actual job](<documentation.md>).
- [Check the media version shown in a publication](<media.md>) · `media` · `doctrine`
  Check which media rendition a publication actually shows and whether it preserves the intended meaning.
- [Check what the reader can infer at each stage](<reveals.md>) · `reveals` · `doctrine`
  Check what a first-time reader can infer from a clue or reveal before seeing later answers.
- [Compose content from accepted sources](<write.md>) · `write` · `doctrine`
  Compose content from accepted sources for a defined reader, purpose and medium.
  Read first: [Write so the reader can act on the answer](<clear-writing.md>).
  Read as needed: [Write an article from its sources](<articles.md>), [Write for the reader's actual job](<documentation.md>), [Develop a voice profile from original work](<voice-profile.md>).
- [Develop a voice profile from original work](<voice-profile.md>) · `voice-profile` · `doctrine`
  Develop a voice profile from owner-approved original work and test it against further examples.
- [Find the owner of a publication change](<change.md>) · `change` · `doctrine`
  Find whether a publication change belongs in its source, workspace, theme or engine, then check the affected output.
  Read as needed: [Inspect an interface in its rendered context](<../design/inspect.md>), [Change code so it fits](<../engineering/change.md>), [Choose the checks for a local publication build](<local-build.md>), [Compose content from accepted sources](<write.md>).
- [Keep the reviewed asset through storage and selection](<reviewed-media.md>) · `reviewed-media` · `doctrine`
  Check that archive installation or asset selection uses the bytes actually reviewed and accepted.
- [Review an editorial draft](<edit.md>) · `edit` · `doctrine`
  Review a draft for its reader, source support, clarity and depth; use specialist checks when the question needs them.
  Read first: [Write so the reader can act on the answer](<clear-writing.md>).
  Read as needed: [Write for the reader's actual job](<documentation.md>), [Check what the reader can infer at each stage](<reveals.md>), [Keep the reviewed asset through storage and selection](<reviewed-media.md>), [Review how a draft depends on its sources](<../research/source-expression.md>), [Check the evidence for a proposed source use](<../research/source-rights.md>).
- [Write an article from its sources](<articles.md>) · `articles` · `doctrine`
  Develop a long-form argument from its sources for the intended reader.
- [Write for the reader's actual job](<documentation.md>) · `documentation` · `doctrine`
  Write or review documentation for the reader’s actual job, keeping prerequisites, examples and next steps connected.
  Read first: [Write so the reader can act on the answer](<clear-writing.md>).
- [Write so the reader can act on the answer](<clear-writing.md>) · `clear-writing` · `doctrine`
  Write directly for the reader, preserving reasoning and examples.

## Tasks

- [Adapt content across media](<adapt-media.md>) · `adapt-media` · `operation`
  Adapt content across the requested media while preserving claims, evidence and voice.
  Read first: [Preserve meaning across presentations](<../design/presentation.md>).
- [Check the media version shown in a publication](<media.md>) · `media` · `operation`
  Check which media rendition a publication actually shows and whether it preserves the intended meaning.
  Read first: [Review the current design independently](<../design/independent.md>).
- [Choose the checks for a local publication build](<local-build.md>) · `local-build` · `operation`
  Choose what to check in the build and its output.
  Read first: [Establish what actually ran](<../engineering/execution.md>), [Check a reference in editing and published output](<references.md>).
  Read as needed: [Check actions in their intended page slots](<build/actions.md>), [Compare artifacts for the claim being checked](<build/compare.md>), [Check assembled documentation and generated reference](<build/documentation.md>), [Match a preview to its built files](<build/preview.md>), [Check the bibliography in the rendered document](<build/references.md>), [Check resources through the actual build and renderer](<build/resources.md>), [Check every build stage and cached input](<build/stages.md>), [Check material that must stay out of the output](<build/withheld.md>).
- [Compose content from accepted sources](<write.md>) · `write` · `operation`
  Compose content from accepted sources for a defined reader, purpose and medium.
  Read first: [Write so the reader can act on the answer](<clear-writing.md>).
  Read as needed: [Write an article from its sources](<articles.md>), [Write for the reader's actual job](<documentation.md>), [Develop a voice profile from original work](<voice-profile.md>).
- [Review a site's search visibility](<search.md>) · `search` · `operation`
  Review current evidence for a defined search-visibility question.
  Read first: [Form an audience hypothesis from discovery evidence](<../research/audience.md>).
  Read as needed: [Compare changes separately and together](<../evaluation/compare.md>), [Review an editorial draft](<edit.md>), [Review a site's search visibility](<search.md>), [Compare new evidence with an accepted baseline](<../research/updates.md>).
- [Review an editorial draft](<edit.md>) · `edit` · `operation`
  Review a draft for its reader, source support, clarity and depth; use specialist checks when the question needs them.
  Read first: [Write so the reader can act on the answer](<clear-writing.md>).
  Read as needed: [Write for the reader's actual job](<documentation.md>), [Check what the reader can infer at each stage](<reveals.md>), [Keep the reviewed asset through storage and selection](<reviewed-media.md>), [Review how a draft depends on its sources](<../research/source-expression.md>), [Check the evidence for a proposed source use](<../research/source-rights.md>).

## Roles

- [Review a site's search visibility](<search.md>) · `search` · `role`
  Review current evidence for a defined search-visibility question.
  Read first: [Form an audience hypothesis from discovery evidence](<../research/audience.md>).
  Read as needed: [Compare changes separately and together](<../evaluation/compare.md>), [Review an editorial draft](<edit.md>), [Compare new evidence with an accepted baseline](<../research/updates.md>).

## Stacks and tools

- [render-local](<render-local.toml>) · `render-local` · `tool_recipe`
  Render the requested publication locally and return the renderer’s result record.

## Policies and other resources

- [build-policy](<build-policy.toml>) · `build-policy` · `capability_policy`
