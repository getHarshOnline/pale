# Check local knowledge and repair its links

Use `tools/vault.py` to inspect Markdown pages and local Cargo links, find links into a page, repair references after a move, or apply reviewed subject tags. Supply the folders and conventions for this workspace explicitly. The tool does not infer a publication boundary from a Git remote, move files or decide which material to delete. A writing standard can rely on `check` for its links: it reports a missing page and a heading fragment that no longer exists in a linked Markdown page outside original records, but it does not interpret wiki links or publication IDs.

Run it from the Pale repository with Python 3.11 or later. Put shared options before the command:

```sh
python3 tools/vault.py --root /absolute/knowledge \
  --root /absolute/repositories --vault /absolute/knowledge --json check
```

`--root` selects a complete search tree and can be repeated. `--vault` adds private-page checks inside a search tree. `--public-root` selects a publication tree whose local links must stay within it. Use `--working-area` for temporary work and `--protect` for material that must remain unchanged. Repairs additionally need a `--writable` root; declaring a search root grants no write permission. Original `Records` and `Sources` folders stay read-only; `--original-name NAME` adds a workspace's own folder name for original material. The check reads past the links inside original and protected material, which keep the paths they were written with.

## Choose the command

| Task | Command | Result |
| --- | --- | --- |
| Find broken references and page inconsistencies | `check` | Findings with paths, lines and reasons; no writes |
| Find which files link into a page or local path | `incoming PAGE` | Source paths, line numbers and link destinations; no writes |
| Repair links after a file or folder has moved | `relink OLD NEW` | A list of files that would change; `--apply` applies the newly computed plan |
| Reconcile topic queries and reviewed subjects | `tags` | Proposed graph changes and subject questions; `--apply` writes permitted changes |
| Measure selected build directories | `builds` | A native Pale measurement for every supplied build root; no page scan or cleanup |

Without `--json`, findings and proposed changes print as readable lines. JSON output separates `findings` and `changes`; `builds` also includes the measurements. Exit status `0` means no error finding, though questions marked for review may remain. Status `1` means at least one error finding. Invalid inputs or a failed operation return `2` with an explanation on standard error. Read the findings before treating an exit status as a decision about the content.

## Find links into a page

Supply the page's exact path and every tree whose incoming links you need:

```sh
python3 tools/vault.py --root /absolute/knowledge \
  --root /absolute/repositories --json \
  incoming /absolute/knowledge/Project/README.md
```

The destination must be inside a selected root, but it need not exist: a former path can reveal links still needing repair. Matching uses the complete local path, ignoring query strings and heading fragments. The command omits links from the destination to itself and skips the same symlinks and generated directories as the ordinary scan. It does not follow remote URLs or interpret publication IDs or wiki links.

Each result gives the source `path`, one-based `line`, original link `target`, link `kind` and whether the source is `protected`. A reference-style Markdown link points to the line defining its destination, not every use of that definition. HTML links and local Cargo path declarations use the existing parsers. Original records and explicitly protected sources can appear in the result; the command never changes them or any other file and requires no writable root.

JSON places the results in `incoming` and the selected destination in `target`. It keeps read failures in `findings` alongside successful matches and returns status `1` for an incomplete scan. An empty result without read failures means no matching parsed links were found in the selected trees. It says nothing about folders outside that selection or link forms the parser does not support.

## Inspect before changing

`check` reports missing targets and headings, undefined Markdown references and selected publication-boundary violations. Private-page checks include metadata, aliases, subject and topic tags, navigation from the owning README and links into temporary work. A folder or matching subject name can suggest a review; it cannot settle who owns a page.

Git history may suggest a destination for a missing link. Review that candidate against the actual move; the tool never applies it automatically. Use `check --no-move-suggestions` when you want findings without the history search. `--task-end PATH=YYYY-MM-DD` supplies an actual end date for temporary work; file age does not establish that the task ended. `--hook FILE` names an agent hook configuration to read, and each `--retired-command NAME` names an executable those hooks must no longer call; a hook line that still calls one is reported as a `retired-hook` finding.

The scan skips symlinks and common generated directories, including `.git`, `node_modules`, `target`, `build` and `dist`. Files over the tool's text limit or unreadable files produce findings. A clean result covers the selected trees and implemented checks; it does not establish that an argument is complete or a private fact may be published.

## Repair a completed move

Move the material through the owner's approved process first. `OLD` must be absent, `NEW` must exist, and both must be within the search roots. The destination and every file needing a repair must be writable and unprotected.

```sh
python3 tools/vault.py --root /absolute/knowledge \
  --writable /absolute/knowledge \
  relink /absolute/knowledge/Old.md /absolute/knowledge/New.md
```

Review the affected files, then repeat the command with `--apply` when those changes are authorised. The command repairs incoming references and relative links inside the moved material. It preserves query strings and heading fragments; run `check` afterward to find a fragment that no longer exists.

Each invocation computes its plan from current files. The CLI preview lists affected files rather than displaying or saving an exact diff. Before applying, the tool checks its inputs again and refuses protected or changed files. Replacements are atomic per file, not across the whole operation: a later write failure can leave earlier files changed. Read the reported completed paths and the repository diff before retrying.

## Apply a reviewed subject or graph query

Supply `--topic-standard` with the workspace's Markdown standard containing one `Root` / `Topics` table. Repeat `--graph` for the Obsidian graph settings to reconcile. The tool changes recognised topic queries while preserving colours and custom filters. It does not choose a page's subject from its physical folder.

```sh
python3 tools/vault.py --root /absolute/knowledge \
  --vault /absolute/knowledge --writable /absolute/knowledge \
  --topic-standard /absolute/knowledge/Standard.md \
  --graph /absolute/knowledge/.obsidian/graph.json \
  tags --subject /absolute/knowledge/Project/README.md=project
```

`--subject PAGE=TAG` supplies a reviewed subject choice. It can deliberately retain a subject that differs from the containing folder. Other subject questions remain for review. Inspect the proposal before adding `--apply`; the same write protections and partial-failure limits as link repair apply.

## Measure build directories

The `builds` command delegates to an explicitly selected native Pale executable. It reports logical regular-file bytes, not reclaimable disk space. See [Measure build directories](doctor.md#measure-build-directories) for the command, ceilings, incomplete results and symlink rules. The same measurement options can accompany `check`. No command deletes build output.
