# Gazette integration

Convert Gazette's native render result into the receipt used by Pale's publication checks. The adapter checks that the report belongs to the request you made and keeps the original report bytes with the receipt. It does not render a site, reopen the output tree or install a Pale provider.

To create a publication, use [Gazette's first-publication guide](https://rawthoughts.in/gazette/first-publication/). This guide is for an application that already invokes Gazette and needs to verify its result.

## Choose the report shape

| Gazette result | Adapter | What you receive |
| --- | --- | --- |
| CLI `render-sites --local` or MCP `render_sites` | `inspectLocalRenderReport` | The complete site-set report, original JSON and digest |
| An inventory-only set containing exactly one site | `localRenderReceipt` | A receipt with `retained_files: false`; pass `requireRetainedFiles: false` explicitly |
| MCP `stage_rendered_site` | `retainedRenderReceipt` | A single-site receipt and its fresh output directory |
| Single-site supplied-TOML CLI result with retained files | `verifyLocalRenderReceipt` from the shared verifier | Verification of the existing neutral receipt, without another projection |

The [Gazette render contract](https://docs.rawthoughts.in/gazette/local-rendering/#local-site-sets) defines each request and its output. Inventory-only reports describe in-memory output, not files you can open. The singleton projection refuses a multi-site report rather than dropping the other sites.

## Verify a staged result

::::steps
:::step "Record the request before invoking Gazette"
Choose the site and canonical output parent. Use Gazette's [workspace load](https://docs.rawthoughts.in/gazette/mcp/#load-a-selected-workspace) to obtain the compiled-workspace digest, and hash the exact request you will send.

Keep these values independently of the response. Taking the expected site or digest from the response would allow a different successful render to pass as the requested one.
:::
:::step "Check the native call's outcome"
Invoke [stage_rendered_site](https://docs.rawthoughts.in/gazette/mcp/#complete-local-rendering) with its required local permissions and limits. Check the RPC and tool outcomes before passing the original UTF-8 report bytes to this adapter. A JSON success field cannot stand in for a successful transport.
:::
:::step "Project the original report"
The following helper can sit in your caller next to `receipt.mjs`. Each input comes from the request or the actual invocation, rather than an invented successful report:

```javascript
import { retainedRenderReceipt } from './receipt.mjs';

export function verifyStagedPublication({
  reportBytes,
  rpcSucceeded,
  exitCode,
  toolIsError,
  expectedSite,
  expectedWorkspaceSha256,
  expectedOutputParent,
  requestSha256,
  maximumBytes,
}) {
  if (!rpcSucceeded) throw new Error('Gazette RPC did not succeed');
  return retainedRenderReceipt(reportBytes, {
    expectedSite,
    expectedWorkspaceSha256,
    expectedOutputParent,
    requestSha256,
    exitCode,
    toolIsError,
    maximumBytes,
  });
}
```

`reportBytes` must contain complete JSON. `expectedOutputParent` is the absolute normalized parent you requested; `maximumBytes` is a positive integer report limit. The adapter requires `exitCode: 0` and `toolIsError: false`, and verifies the requested site, compiled workspace, inventory and direct-child directory.
:::
:::step "Inspect the output before using it"
The receipt keeps the inventory, directory, request digest and original JSON with its SHA-256. Its `build_selection` is the single requested site. Open or inspect the staged tree and compare its complete membership and bytes with the original inventory.

The adapter verifies the report, not the current files on disk. A successful check here cannot establish that an output file was not changed later.
:::
::::

## Work with inventory-only results

For `inspectLocalRenderReport`, supply `expectedSelection`, `expectedConfiguration`, `requestSha256`, the actual `exitCode` and `maximumBytes`. The adapter compares the complete site set and configuration, then checks source failures and every site's output evidence.

Use `localRenderReceipt` only when that set contains one site and your caller explicitly accepts the absence of retained files. If your application needs the actual trees in memory, use Gazette's [SDK set-rendering API](https://docs.rawthoughts.in/gazette/local-rendering/#use-the-sdk-set-entrypoints) instead of reconstructing them from an inventory.

## Keep native bytes and limits distinct

The original JSON text is preserved even after parsing. A neutral receipt projection refuses integers JavaScript cannot represent safely; a parsed number must not silently replace exact native evidence. The set inspector can still preserve the original report for another consumer.

The report-size limit is separate from Gazette's rendering limits. For example, `directory_bytes` limits the returned path, while output budgets limit rendered data. The native request also decides source reads, staging and math or diagram execution.

The [Publication manifest](../../packages/publication/package.toml) declares Pale's build operation, and its [local-render recipe](../../packages/publication/render-local.toml) defines the receipt requirement. Using these report functions does not admit or activate that provider.

## Diagnose a refused report

A selection mismatch needs the correct request or a source correction. Do not change the expectation to fit the response. Other failures include incomplete source capture, missing or extra sites, a different output parent, invalid UTF-8 and unsuccessful native outcomes.

Diagnostics omit private source and site locators. Keep the original response privately when investigating a refusal. If the report passes but a file is absent or changed, inspect the staged directory. A new render is a new result, not a repair of the old report.

## Extend the adapter

Keep Gazette report interpretation in `receipt.mjs` and shared receipt rules in `tools/publication-verification/receipt.mjs`. When a native shape changes, update the interpretation and its success and refusal cases. From the Pale root:

```sh
node --test integrations/gazette/receipt.test.mjs
```

These tests check parsing and projection. A change to native rendering also needs its actual Gazette invocation and output checks.
