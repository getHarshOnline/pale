+++
description = "When the project chose Midjourney and you need to prepare a request and inspect what comes back."
uses = [{ scope = "local", resource_id = "generate" }]
official_sources = [{ label = "Prepare a Midjourney request and inspect the result", url = "https://docs.midjourney.com/hc/en-us" }]

[guide]
exposure = "internal"

[stack]
+++

# Prepare a Midjourney request and inspect the result

Use this overlay when the project chose Midjourney. Read the provider’s current parameter reference for the version and task. A style reference changes treatment. An image prompt can influence subject and composition. A subject reference and a seed answer different questions. Check availability and meanings in the version actually used.

The workspace's image engine prepares request text and imports a downloaded file; it does not operate the service. Submit through the provider’s own interface using the project's selected operator. Keep credentials, accounts, uploaded source decisions and job settings with the project and the provider.

## Choose the surface for the decision

| Job | Useful surface | What still needs judgment |
| --- | --- | --- |
| Explore territories | Draft batches | Range and direction; a draft is not an approved keyframe |
| Find words for a reference | Describe | Borrow useful vocabulary, then write the project's request |
| Carry a visual world | Moodboard | Curate the board from the approved references |
| Discover a treatment | Style Creator | Select by treatment and compare against the project's board |
| Test a style or make a keyframe | Full image generation at the selected resolution | Judge the actual delivered detail and framing |
| Preserve or combine subjects | Edit model and supported reference slots | Inspect identity, silhouette and relationships across the whole image |
| Repair a region or extend the canvas | Editor inpainting or outpainting | Compare outside the intended change; composite when exact pixels must survive |
| Animate an approved image | Video with a starting frame and optional ending frame | Judge motion, continuity and the ending, not just the still |
| Organise candidates | Gallery folders and likes | Keep the project record and retained originals independently |

[Describe](https://docs.midjourney.com/hc/en-us/articles/32497889043981-Describe) offers suggestions, not a reconstruction of the reference. Preserve useful suggestions before refreshing the page. [Draft and Conversational modes](https://docs.midjourney.com/hc/en-us/articles/35577175650957-Draft-Conversational-Modes) have different jobs: Draft explores cheaply, while Conversational mode rewrites the request through another model. Use direct prompt entry for an exact comparison. Regenerating a draft at full resolution creates a new candidate to judge. Read the selected version's behavior instead of assuming the same draft batch or enhancement route across versions.

The [Edit model](https://docs.midjourney.com/hc/en-us/articles/48495453462797-Edit-Model) supports up to four reference images in V8.1 and V8.2, replacing the older Omni and Character reference routes there. The first reference influences the default aspect ratio; set the intended ratio explicitly. Edit output currently has tile and Remix restrictions. An edit instruction can ask for a change directly, unlike an ordinary scene description. None of these controls guarantees unchanged surrounding pixels.

## Keep settings and reference roles explicit

For a still, describe the world and shot, then append parameters with a space before each `--flag`. Keep prose before the flags and avoid stray punctuation between them. Commas inside the supported `--no` list are a separate syntax. The [parameter reference](https://docs.midjourney.com/hc/en-us/articles/32859204029709-Parameter-List) owns version compatibility and ranges.

| Choice | Control | Reason to record it |
| --- | --- | --- |
| Model and image shape | `--v` or `--niji`, `--ar` | Version and framing affect every comparison |
| Interpretation and variety | `--raw`, `--s`, `--chaos`, `--weird` | Raw reduces added interpretation; it does not lock reference pixels |
| Treatment reference | `--sref`, `--sw`, supported `--sv` | Reference, strength and interpretation version are separate |
| Curated board or taste profile | `--p` with an exact code | Ambient defaults can silently add a different look |
| Starting noise | `--seed` | Controls one source of variation, not identity or style |
| Resolution and service mode | Supported SD/HD, Draft, Fast, Relax or Turbo settings | Cost, output and repeatability differ |
| Visibility | Actual public or Stealth setting | A private project does not make an upload private |

Image prompts influence content and composition; their supported image weight is separate from style weight. In Discord, an image-prompt URL precedes the descriptive words. Use the reference slot the chosen version actually supports, including its older `--oref`/`--ow` route where applicable. Do not carry a subject-reference flag into a different version by analogy.

[Exclusions](https://docs.midjourney.com/hc/en-us/articles/32173351982093-No) are read word by word. For example, excluding “modern clothing” can also be read as excluding clothing. Describe the wanted clothing positively instead. A multiword warning calls for inspection, not an automatic ban on every compound term. [Multi-prompts and weights](https://docs.midjourney.com/hc/en-us/articles/32658968492557-Multi-Prompts-Weights) use `::` only on supported versions; negative weights require a positive total. Check that the selected model accepted the intended syntax rather than treating it as ordinary text.

Use an integer seed from 0 through 4294967295 for a controlled trial and copy the actual job seed afterwards. The provider's [seed guide](https://docs.midjourney.com/hc/en-us/articles/32604356340877-Seeds) describes improved V8.X consistency but also warns about changed sessions, settings and Turbo. A seed cannot preserve a character or style across prompts, and it never replaces the downloaded master.

### A code is a provider reference, not the board itself

Keep the approved board, its image rights, revision and manifest with the project. A provider board or code is a derivative of that source. Record its kind, exact submitted code, model/reference version, date, board revision and strength. Another provider starts from the board and judgment, not from an assumed translation of the code.

[Style reference codes](https://docs.midjourney.com/hc/en-us/articles/32180011136653-Style-Reference) address Midjourney's internal style library. An uploaded image can serve as a style reference, but does not yield an equivalent numeric code. Random and popular codes need the same visual comparison as any other candidate. Older codes can change appearance under a different model or reference version. [Style Creator](https://docs.midjourney.com/hc/en-us/articles/41308374558221-Style-Creator) learns from selected treatments; a simple preview prompt makes that treatment easier to see. Preview rounds consume GPU time. Entering with an existing code can stack a second code rather than replace it, so retain every code that produced the chosen preview.

A [moodboard ID](https://docs.midjourney.com/hc/en-us/articles/39193335040013-Moodboards) selects its current contents and resolves to a version code on submission. Retain that submitted code, not just the mutable ID. The provider says existing codes remain usable after board deletion; that is no guarantee of permanent reproducibility. Moodboards do not accept `--sw` or `--sv`; their influence uses stylize. Turn off unselected default personalization. If the project deliberately chose a taste profile, pin the submitted version and hold it through the comparison; later ratings or board changes must not silently alter the trial.

## Prepare the handoff and recover what actually ran

Prepare the prompt, version and mode, reference attachments and their roles, any personalization or profile setting, relevant visibility choice and what original files to download. Include material metered cost when it affects the choice. Follow the project's division of operation and selection: an agent may be asked to operate at a human pace while the person chooses the results to keep. Stop at any CAPTCHA or bot check and hand it to the person. Prepare one changed input for a controlled comparison; exploration can vary more but cannot isolate the cause.

The [provider’s terms](https://docs.midjourney.com/hc/en-us/articles/32083055291277-Terms-of-Service) prohibit automated access and generation. Keep that service condition distinct from the project's selected workflow: a request for agent operation does not change the terms, and human pacing does not establish permission from the provider. Read the current terms when choosing the route and state any mismatch explicitly; do not rewrite the owner's choice as a universal manual-only policy.

Check the [plan](https://docs.midjourney.com/hc/en-us/articles/27870484040333-Comparing-Midjourney-Plans) and remaining budget before the round: Relax images require Standard or above; Relax SD video and Stealth require Pro or Mega, while HD video uses Fast. The terms make content public and remixable by default; Stealth does not conceal work in a shared room. They grant the provider continuing rights over submitted and generated content, retain third-party rights, restrict account sharing and require Pro or Mega for companies above the stated revenue threshold. Upscaling someone else's image does not transfer its ownership. Match those conditions to the project's source permissions before upload. If the selected budget or disclosure limits cannot support the route, use an approved alternative rather than changing the account or buying capacity implicitly.

Render the request with the engine's request renderer, from the project's existing binding and request. The renderer checks field shapes and reference bytes and prints text, attachment instructions and warnings. It does not validate every changing provider range, submit a job or witness an upload. Still-image `extra` text needs review as actual provider syntax. The current warning tells territory/style-test users to disable any profile; apply the selected-profile rule above rather than treating that warning as authority to discard the project's choice. Keep the selected interface's settings visible during submission. If a long-prompt adjustment or other rewrite appears, inspect the resulting command; shorten and render again when the exact prepared wording is required.

After the run, compare the full command and seed from the actual job with the prepared request. UI settings, prompt adjustment or a change at the keyboard can alter what ran. Keep those differences in the existing run record. Download the original file, rather than a grid screenshot or social-media copy. The provider's [authenticity check](https://docs.midjourney.com/hc/en-us/articles/48207461163661-Content-Authenticity) depends on embedded metadata that edits and screenshots may remove; absence of that metadata does not prove another origin. Preserve the downloaded bytes before editing or re-encoding. A filename-only change does not change those bytes, but retain the original name privately because it may carry the account's display name.

Put this run's downloads in the source library's `imports/inbox/`. Preview the import first; it scans all supported media in that inbox, not only files selected by a request ID:

```sh
python3 image/midjourney/scripts/mj_import.py --project PROJECT --request REQUEST --binding /path/to/studio.local.toml --dry-run
```

Run the same command without `--dry-run` to import. Add `--ran` with the actual full command and `--seed` with the observed seed when available. The importer preserves bytes under date/hash names in `generated/midjourney/` and starts a workspace run record with `complete = false`. Existing identical outputs and duplicate bytes stay in the inbox; a conflicting destination is refused. Record the actual model, mode, codes, attachment roles, operator actions and the owner's kept/rejected verdicts. The prepared request and current reference hashes do not prove what was uploaded; unknown fields remain unknown.

The importer stages a complete pending record before moving files with no-replace hard links. Caught interruptions roll moves back where possible; a cross-volume move refuses rather than silently copying. An uncatchable termination may leave a pending record and links in both locations. Reconcile source/destination hashes and the record before retrying, preserving every unmatched original. Byte-preserving import and a completed record do not approve an image. Show comparable candidates with the one remaining choice when an owner's selection is needed; continue within a selection already authorized.

## A film request carries motion, not still-image flags

Begin from the approved frame. The [video controls](https://docs.midjourney.com/hc/en-us/articles/37460773864589-Video) support low/high motion, Raw, a loop or separate ending frame, and batches of one, two or four. Inspect the whole clip for drift, unwanted movement and its ending. Download the raw video master; a social export is a delivery derivative.

The engine's `film` request renders the shot's motion text. It retains a required model version as provenance but emits no still-image `--v`, world paragraph or exclusion list. Attach references to `Starting Frame` and, when needed, `Ending Frame`, at most one of each. A separate `end` URL must be absolute HTTP(S); it cannot duplicate an attached ending frame. A loop cannot also name a separate end. The film renderer refuses still fields even when empty, flags hidden in shot text, and braces that would expand into multiple jobs. Its `extra` field accepts only `--stealth`, `--relax` or `--fast`, with Fast and Relax mutually exclusive. Use the explicit film fields for motion, Raw, loop and batch size. These checks establish request syntax, not service admission or a generated film.

## Worked example: keep an approved object through two treatments

Prepare a base prompt with the approved subject, framing and version. Supply the allowed subject reference to both trials and change one style input. Keep the source and version the same, then compare silhouette, crop and material separately. Save the prompt, provider parameters, job identity and downloaded original if the project has them. Import that original before editing it. A filename or screenshot of a browser grid cannot identify the exact file that was imported.
