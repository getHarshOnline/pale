+++
description = "Inspect the required Hugging Face metadata at the intended repository revision."
official_sources = [{ label = "Hub API endpoints", url = "https://huggingface.co/docs/hub/api" }, { label = "Hugging Face MCP and host setup", url = "https://huggingface.co/docs/hub/agents-mcp" }, { label = "Hugging Face agent skills", url = "https://huggingface.co/docs/hub/agents-skills" }, { label = "Revision-specific file reads", url = "https://huggingface.co/docs/huggingface_hub/guides/download" }, { label = "Gated access", url = "https://huggingface.co/docs/hub/models-gated" }]

see_also = [{ scope = "external", resource = { package_id = "pale.research", resource_id = "source-rights" } }]

[guide]

[stack]
+++

# Inspect the Hugging Face Hub

Use a connected Hub MCP search, browser or API to find the artifact. Follow results to the original repository, paper or documentation. Resolve a branch or tag to a full commit hash through commit history or the Hub API. Read the required card, file tree and configuration at that revision. The installed `hf` CLI or `huggingface_hub` can perform the same read when appropriate. In the review, say what the route actually observed.

Use the datasets skill or Dataset Viewer when inspection of the chosen data rows is permitted. A model or dataset card gives the publisher's account. Check architecture against the pinned configuration, list the actual artifacts and keep versioned licence and access evidence with the proposed use. Identify a missing card, gated configuration or blocked preview as a gap. For a model, follow the source-rights guide in the optional reading: its worked case separates licence classes, upstream and repackaged terms, model-use permission, output rights and gated access. Apply it to the actual revision and proposed use, keeping the project’s selected policy in its workspace.

Use Hugging Face skills and client-specific MCP setup when accepted by the host. Search access does not permit jobs, Space execution, repository changes or large transfers. Handle tokens, endpoint configuration, access requests, downloads and execution through their separately approved routes. Before claiming a model can load, compare its actual configuration, weights and tokenizer with the chosen backend.

## Worked example

Two model cards describe different revisions. Read metadata for the actual repository and revision needed for the work before downloading. Check which files, terms and weights belong to that revision. Report a missing revision instead of substituting the default branch.

## Before handing off

- [ ] Are the repository, revision and artifact type clear?
- [ ] Was metadata read through the appropriate route before using files?
- [ ] Were access and terms checked for the planned use?
