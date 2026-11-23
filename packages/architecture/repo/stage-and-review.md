# Verify what the staged change contains

## Verify the exact staged change

Before staging, identify repository root, intended index, baseline tree and permitted paths. Distinguish already staged entries from working-file contents. For additions, deletions and mode changes, specify what the index should contain afterward.

Use machine-readable path records such as `git status --porcelain=v1 -z` and `git ls-files --stage -z`. Parse the documented formats, including both paths for renames or copies. Preserve path bytes. Display quoting, whitespace splitting and line counts can hide or merge names. When staging is authorised, pass paths literally; supported Git versions accept `git --literal-pathspecs add --pathspec-from-file=FILE --pathspec-file-nul` from the repository root. An empty or unresolved selection permits no staging.

Compare the index before and after by path, stage, mode and object ID. Verify intended changes and untouched unrelated entries. Read the staged diff against the stated baseline, comparing working files too when they differ. Success status or counts alone do not prove membership or bytes. Unexpected, omitted or conflicted entries block the next action. Report differences and unresolved paths in the change review.

If changing or checking staging automation, rehearse spaces, tabs, newlines, non-ASCII names, leading dashes and pathspec metacharacters in a disposable repository. Include an ignored file and a pre-staged entry with different working bytes. Verify the exact index change, exclusion of nearby unselected files and absence of commits or ref updates from staging alone. The owner chooses paths, index, recovery and any subsequent commit or publication.

## Worked example

A worktree contains a Package change and unrelated notes. Compare the staged diff with the intended paths and inspect the staged tree from a separate checkout. Preserve the notes in the worktree without including them.
