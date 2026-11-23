# Match a preview to its built files

Use this when a screenshot, interaction check or review must refer to one local output, especially when a watch process can rebuild it. The renderer owns its input and output inventories. The preview owner supplies the address, active build, and session handle. The browser observes what it actually received.

## Check the served document and the files it uses

1. Build through the normal publishing engine. Open its output inventory for the intended publication and record the active preview state.
2. Start from the intended reader route. Check the final address after any redirects and the document the browser received. Refresh a previously open tab when reviewing a rebuild.
3. Where the claim depends on matching bytes, compare the browser's observed document and same-origin resource bodies with one build's inventory. Include only responses the tool actually captured. Separate partial or range responses, off-origin dependencies and later interactions when the capture cannot account for them.
4. Inspect the resulting page in the relevant states and contexts. Compare an affected page with a neutral fixture where each can answer a different question. Read visual or accessibility results separately from the file match.

An output directory, URL or build label can stay the same while the underlying files change. A watched page can load part of an earlier build and part of a later one. If the inventories disagree or the capture is incomplete, refresh against a stable build or state what remains unknown. Identical bytes can be served in several preview generations. The preview owner decides when to stop and release it. A local preview does not publish the artifact.

## Worked example

A reviewer opens an article during a watch rebuild. The article's HTML matches the new inventory, but its stylesheet matches the previous one. Wait for the preview to report the new build. Reload from the intended route and compare both responses with the same inventory. Keep the screenshot from that run and check the changed layout in the actual browser. A screenshot with the expected shape alone cannot establish which stylesheet was used.
