+++
description = "Check the revision, files, access and use permissions behind a proposed model or dataset."

[guide]

[task]
outputs = ["In the review: repository, revision, inspected files and stated access.", "Unverified claims and the evidence needed to establish compatibility or use."]
+++

# Check a model or dataset at its intended revision

Start with the proposed use and the inspection its owner permits. In the review, identify the repository kind and identity, immutable revision, file paths, stated origin and licence, access state and what each observation supports. Follow upstream links for base-model, dataset or conversion history. Distinguish the publisher's account from inspected configuration, listed files, verified bytes and tested behaviour.

Inspect the smallest source that can settle each claim. Use a published card for stated intent and pinned configuration for architecture or dimensions. List weights, shard indexes, tokenizers, templates and dataset splits before deciding what to retrieve. Record digests for files actually read. A listed weight file, inferred licence, gated file not inspected or documentation for another revision leaves the corresponding claim unresolved. Include the observation date and whose policy or permission allows the proposed use.

Access, acceptance of terms, transfer, code execution, runtime compatibility and use rights are separate questions. Hold transfers, gated-access requests, execution, uploads and jobs when the needed permission is missing. Review the route independently before enabling code supplied with an artifact. Return the pinned versions, observations, missing or restricted items, and separate compatibility and use findings for the owner.

Use provider guidance selected by the project for the permitted inspection route. Installing or configuring a provider is separate from inspecting the artifact.

## Before handing off

- [ ] Are the model or dataset and source version identified?
- [ ] Have the relevant files, terms and recorded results been checked?
- [ ] Are metadata claims distinguished from observed behaviour?
