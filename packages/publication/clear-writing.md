+++
description = "Write directly for the reader, preserving reasoning and examples."

[guide]
+++

# Write so the reader can act on the answer

Use this when writing or editing a publication, manual or explanation. Start with the reader’s question and put the answer where they can use it. Name who does what, what they can observe and what decision follows. Use the terms they see in the product or source.

Keep the reasoning, conditions, useful detail and worked example that make the answer trustworthy. Put exhaustive lookup material where the reader can find it when needed. Use a reference for syntax and a guide for a task. Read the whole delivered page, including its links, headings and examples, from the intended entry.

## Match the form to the reader's job

The same clear voice serves comments, help and an essay, but each form answers a different question. A code reader needs the contract and its reason; a person using the product needs the task and next action. Preserve the speaker's confirmed position and the destination's conventions. Private voice examples may guide the writing without becoming links in a public repository.

### Comments and generated reference

Use short, active sentences with one instruction or claim at a time, in the spirit of Simplified Technical English. Explain what a declaration is for, the inputs it needs, what it returns and the corner case that changes how it is used. Keep technical names exact. Two or three lines can serve a small function; a complex interface needs its actual conditions and examples, not an arbitrary length limit.

A crate, package or module comment first places the code: what it is for in the system, what it builds on and what builds on it. Its contract follows. Say what does not exist yet in one plain line after that opening, never as the opening: “MUON and HENA import this kernel” tells a reader where they are before any statement of what remains unbuilt. As in command help, state a negative only where a reader would expect the opposite; a list of what a crate does not acquire, render or grant leaves the reader to infer what it is for.

Explain the reason the code cannot show. “Increment the index” adds little beside `index += 1`. “Skip the header row so it is not imported as a customer” explains the choice. For an interface, “Find the customer by their stable ID. Return no match when the customer was deleted; an empty display name is still a match” preserves the distinction a caller needs.

Put parameter, return, error, panic and safety details in the language's documentation form where applicable. Include an example when the order of calls or a boundary case matters. Generated reference and editor help inherit these comments; fix their source. Link within the repository or to public documentation, and keep task history and private decisions elsewhere.

### Command help

Start with the action: “Import customers from a CSV file.” Name the required input, useful options and what the person gets back. Give a command they can run and explain a consequential default beside its option. A negative belongs where the person might reasonably expect the opposite: “Preview the changes without writing files” explains preview; a catalogue of unrelated things the command does not do adds noise.

Use the actual flags, paths and accepted values. Keep exhaustive syntax in the reference and walk the task in the guide. Check examples against the command before presenting them as working instructions.

### Agent instructions

Server instructions, tool and skill descriptions and plugin guides are read by an agent deciding what to do next. Give them four parts: when to reach for the tool and when not to; what one call gives; what to do when it cannot help; and which step is someone else's decision. Name a tool and its description for the question the agent has, such as “what applies to this repository?”, rather than for the implementation's internal categories. Keep custody and schema rules exact in the input and output schemas, where the agent reads them when it needs them.

Gazette's agent skill shows the form. Its description reads: “Check, edit and preview publications configured with Gazette. Use for Gazette workspaces, authored pages, publication links, Themes and compiler feedback; an unrelated Markdown file or website does not imply a Gazette workspace.” Its guide then says where to start, which tool answers a publication check, what a result does and does not establish, and how to report it.

### Errors and recovery

Name the failed action in the user's terms, the cause you actually know, what changed or remains intact, and what to do next. A retry instruction is useful only if repeating the action is appropriate. When the outcome is uncertain, say how to check it before retrying. Do not expose credentials or private input in an error.

Separate invalid input, unavailable dependencies and an internal fault. Do not blame the user for an invariant the program broke. For example: “Pale produced a workspace record it cannot decode. This is a bug in Pale. Keep the diagnostic ID and report it with the command you ran.” Include only a diagnostic or recovery action the product actually supplies; keep internal identifiers in a separate detail when they help support investigate.

### Commits and release notes

Give a commit a short imperative subject naming the change, such as “Preserve the previous import when validation fails.” Use plain paragraphs in the body to explain the concrete failure, the resulting behaviour and the relevant check or limit. Follow the repository's established conventions. Avoid a transcript of the editing session or a claim that all problems are fixed.

Release notes tell a user what they can now do, what behaves differently and whether they need to act. Group related changes around their consequence rather than reproducing commit subjects. A useful example is: “If a CSV row is invalid, Import now shows the row and keeps your previous import available. Correct the row and run Import again.” Describe a breaking change with its affected users and migration step, and state a known limit beside the feature it qualifies. Preparation, a local test and a published release are different states; use release language only for the state the evidence establishes.

## Worked example: explain a failed import

“Resolve input eligibility and rerun the selected workflow” leaves the reader to guess what failed. Write “The importer could not open the file because it is a folder. Choose the CSV file inside that folder and run Import again. The previous import is still available.” If the file was received and only its second column failed, name that column and explain what values are allowed. Keep the real failure, its consequence and the recovery path together.

## Before handing off

- [ ] Put the answer and next action where the reader expects them.
- [ ] Keep the conditions and example that explain when the answer applies.
- [ ] Check comments, command examples and messages against the actual behaviour, including failures and uncertain outcomes.
- [ ] Read the delivered text and follow its relevant links.
