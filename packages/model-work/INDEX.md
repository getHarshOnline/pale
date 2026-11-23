# Model Work

Package ID: `pale.model-work` · [Manifest](package.toml)

Choose and adopt models, compare representations and artifacts, estimate cost and runtime, and review their lifecycle.

A workspace selects entries from this family for the duties its people and agents perform. To read it directly, open the page for your task and follow its reading links.

## Guides

- [Build, release and maintain a model reproducibly](<build.md>) · `build` · `doctrine`
  Build, evaluate, release and monitor a reproducible model with clear requirements for promotion and rollback.
- [Check a model or dataset at its intended revision](<artifact.md>) · `artifact` · `doctrine`
  Check the revision, files, access and use permissions behind a proposed model or dataset.
- [Compare a model's full runtime on its intended hardware](<compare-runtime.md>) · `compare-runtime` · `doctrine`
  Compare quality, latency, memory and failure behaviour for the full model runtime on its intended hardware.
- [Compare learned representations and sparse dictionaries](<representations.md>) · `representations` · `doctrine`
  Compare paired activations, sparse reconstruction and feature stability for claims about internal representations.
  Read first: [Check numerical results against independent references](<../evaluation/numbers.md>), [Compare changes separately and together](<../evaluation/compare.md>).
- [Compare models with the same resource budget](<compare-resources.md>) · `compare-resources` · `doctrine`
  Compare model routes using the same resource budget, required capabilities and controls.
- [Decide whether a product needs machine learning](<adopt.md>) · `adopt` · `doctrine`
  Decide whether machine learning improves a product enough to justify its data needs, operating cost and risk.
- [Inspect the Hugging Face Hub](<hugging-face.md>) · `hugging-face` · `doctrine`
  Inspect the required Hugging Face metadata at the intended repository revision.
  Read as needed: [Check the evidence for a proposed source use](<../research/source-rights.md>).
- [Keep model-assisted work within its value and budget](<cost.md>) · `cost` · `doctrine`
  Choose a model route that meets the task’s quality and budget, including context, retries, caches and repeated calls.
  Read as needed: [Compare changes separately and together](<../evaluation/compare.md>), [Check what an analysis counts](<../evaluation/rows.md>).

## Tasks

- [Build, release and maintain a model reproducibly](<build.md>) · `build` · `operation`
  Build, evaluate, release and monitor a reproducible model with clear requirements for promotion and rollback.
  Read first: [Trace evaluation inputs and compare runs](<../evaluation/runs.md>), [Compare changes separately and together](<../evaluation/compare.md>).
  Read as needed: [Review a machine-learning change](<review.md>).
- [Check a model or dataset at its intended revision](<artifact.md>) · `artifact` · `operation`
  Check the revision, files, access and use permissions behind a proposed model or dataset.
- [Compare a model's full runtime on its intended hardware](<compare-runtime.md>) · `compare-runtime` · `operation`
  Compare quality, latency, memory and failure behaviour for the full model runtime on its intended hardware.
  Read first: [Trace evaluation inputs and compare runs](<../evaluation/runs.md>).
- [Compare learned representations and sparse dictionaries](<representations.md>) · `representations` · `operation`
  Compare paired activations, sparse reconstruction and feature stability for claims about internal representations.
  Read first: [Check numerical results against independent references](<../evaluation/numbers.md>), [Compare changes separately and together](<../evaluation/compare.md>).
- [Compare models with the same resource budget](<compare-resources.md>) · `compare-resources` · `operation`
  Compare model routes using the same resource budget, required capabilities and controls.
  Read first: [Keep model-assisted work within its value and budget](<cost.md>), [Trace evaluation inputs and compare runs](<../evaluation/runs.md>).
- [Decide whether a product needs machine learning](<adopt.md>) · `adopt` · `operation`
  Decide whether machine learning improves a product enough to justify its data needs, operating cost and risk.
- [Keep model-assisted work within its value and budget](<cost.md>) · `cost` · `operation`
  Choose a model route that meets the task’s quality and budget, including context, retries, caches and repeated calls.
  Read as needed: [Compare changes separately and together](<../evaluation/compare.md>), [Check what an analysis counts](<../evaluation/rows.md>).

## Roles

- [Review a machine-learning change](<review.md>) · `review` · `role`
  Independently review a model change and the evidence supporting its acceptance.

## Stacks and tools

- [Inspect the Hugging Face Hub](<hugging-face.md>) · `hugging-face` · `stack`
  Inspect the required Hugging Face metadata at the intended repository revision.
  Read as needed: [Check the evidence for a proposed source use](<../research/source-rights.md>).
