import { createHash } from 'node:crypto';
import { isDeepStrictEqual as equal } from 'node:util';
import { TextDecoder } from 'node:util';
import { dirname, isAbsolute, normalize } from 'node:path';
import { verifyLocalRenderReceipt } from '../../tools/publication-verification/receipt.mjs';

const record = value => value !== null && typeof value === 'object' && !Array.isArray(value);
const nonempty = value => record(value) && Object.keys(value).length > 0;
const digest = value => typeof value === 'string' && /^[a-f0-9]{64}$/.test(value);
const inventory = value => nonempty(value) && record(value.identity) &&
  typeof value.identity.format === 'string' && value.identity.format.length > 0 &&
  digest(value.identity.sha256);

function projectableNumbers(value) {
  if (typeof value === 'number') {
    return Number.isFinite(value) && (!Number.isInteger(value) || Number.isSafeInteger(value));
  }
  if (Array.isArray(value)) return value.every(projectableNumbers);
  if (record(value)) return Object.values(value).every(projectableNumbers);
  return true;
}

function refuse(reason) {
  // The caller retains the private native bytes separately. Never include them
  // or a source/site locator in a diagnostic sent to a wider audience.
  throw new TypeError(`Gazette local render evidence refused: ${reason}`);
}

function readReport(bytes, maximumBytes) {
  if (!(bytes instanceof Uint8Array) || !Number.isSafeInteger(maximumBytes) ||
      maximumBytes < 1 || bytes.byteLength > maximumBytes) refuse('report byte boundary');
  let text;
  let report;
  try {
    text = new TextDecoder('utf-8', { fatal: true }).decode(bytes);
    report = JSON.parse(text);
  } catch {
    refuse('native report is not complete UTF-8 JSON');
  }
  return { report, nativeReport: text,
    nativeReportSha256: createHash('sha256').update(bytes).digest('hex') };
}

/** Check a complete native set without inventing files, admission or a new render. */
export function inspectLocalRenderReport(bytes, {
  expectedSelection, expectedConfiguration, requestSha256, exitCode, maximumBytes,
}) {
  const parsed = readReport(bytes, maximumBytes);
  if (exitCode !== 0) refuse('native process did not complete successfully');
  if (!nonempty(expectedConfiguration) || !digest(requestSha256)) refuse('request or configuration identity is unavailable');
  const sites = expectedSelection?.sites;
  if (!nonempty(expectedSelection) || !Array.isArray(sites) || sites.length === 0 ||
      sites.some(site => typeof site !== 'string' || !site.trim()) ||
      new Set(sites).size !== sites.length) refuse('expected selection is unavailable');
  const { report } = parsed;
  if (!record(report) || report.status !== 'local_set_rendered' ||
      !equal(report.build_selection, expectedSelection)) refuse('status or selection mismatch');
  if (!equal(report.configuration, expectedConfiguration)) refuse('configuration frontier differs');
  if (!nonempty(report.sources) || Object.values(report.sources).some(value => !nonempty(value)) ||
      !record(report.source_failures) || Object.keys(report.source_failures).length !== 0) {
    refuse('source capture is incomplete');
  }
  if (!record(report.sites) || !equal(Object.keys(report.sites).sort(), [...sites].sort())) {
    refuse('requested site membership differs');
  }
  for (const site of sites) {
    const outcome = report.sites[site];
    if (!record(outcome) || outcome.status !== 'rendered' ||
        !inventory(outcome.inventory) || !nonempty(outcome.evidence)) refuse('site output is incomplete');
  }
  return {
    ...parsed,
    // Keep exact emitted bytes. Parsing is for bounded interpretation, never a
    // replacement for the native record or its numeric/string lexemes.
    proofScope: 'local-publication-set-inventory',
    retainedFiles: false,
  };
}

/** Project only an exact singleton to the existing neutral local-render recipe. */
export function localRenderReceipt(bytes, expectation) {
  if (expectation.requireRetainedFiles !== false) {
    refuse('inventory-only route requires explicit acceptance; retained files are unavailable');
  }
  const result = inspectLocalRenderReport(bytes, expectation);
  if (expectation.expectedSelection.sites.length !== 1) refuse('singleton receipt cannot trim a site set');
  const selected = result.report.sites[expectation.expectedSelection.sites[0]];
  if (![result.report.build_selection, result.report.sources, selected.inventory].every(projectableNumbers)) {
    refuse('numeric projection would lose native integer evidence');
  }
  const receipt = {
    status: 'local-rendered',
    build_selection: result.report.build_selection,
    inputs: {
      sources: result.report.sources,
      configuration: result.report.configuration,
      request_sha256: expectation.requestSha256,
    },
    inventory: selected.inventory,
    native_report: {
      sha256: result.nativeReportSha256,
      json: result.nativeReport,
    },
    proof_scope: result.proofScope,
    retained_files: false,
  };
  verifyLocalRenderReceipt(receipt, { status: 'local-rendered', selection: expectation.expectedSelection });
  return receipt;
}

/** Project a native stage report; the caller separately verifies transport and retained bytes. */
export function retainedRenderReceipt(bytes, {
  expectedSite, expectedWorkspaceSha256, expectedOutputParent,
  requestSha256, exitCode, toolIsError, maximumBytes,
}) {
  const { report, nativeReport, nativeReportSha256 } = readReport(bytes, maximumBytes);
  if (exitCode !== 0 || toolIsError !== false) refuse('native MCP outcome is unavailable or unsuccessful');
  if (typeof expectedSite !== 'string' || !expectedSite.trim() ||
      !digest(expectedWorkspaceSha256) || !digest(requestSha256)) refuse('request or compiled workspace identity is unavailable');
  if (typeof expectedOutputParent !== 'string' || !isAbsolute(expectedOutputParent) ||
      normalize(expectedOutputParent) !== expectedOutputParent) refuse('canonical output parent is unavailable');
  if (!record(report) || report.status !== 'local_retained' ||
      report.inputs?.site !== expectedSite ||
      report.inputs?.compilation?.source_sha256 !== expectedWorkspaceSha256) refuse('retained site or compiled workspace differs');
  if (typeof report.directory !== 'string' || !isAbsolute(report.directory) ||
      normalize(report.directory) !== report.directory ||
      dirname(report.directory) !== expectedOutputParent) refuse('retained directory differs from the requested parent');
  if (!inventory(report.inventory) || report.inventory.identity.format !== 'gazette.inventory.v1' ||
      !nonempty(report.evidence)) refuse('retained output evidence is incomplete');
  if (![report.inputs.sources, report.inventory].every(projectableNumbers)) refuse('numeric projection would lose native integer evidence');
  const selection = { site: expectedSite };
  const receipt = {
    status: report.status,
    // This native result identifies one site, not render_sites' origin/draft selection.
    build_selection: { site: report.inputs.site },
    inputs: {
      sources: report.inputs.sources,
      configuration: { kind: 'compiled-workspace', source_sha256: report.inputs.compilation.source_sha256 },
      request_sha256: requestSha256,
    },
    inventory: report.inventory,
    directory: report.directory,
    native_report: { sha256: nativeReportSha256, json: nativeReport },
    proof_scope: 'native-single-site-retention-report',
    retained_files: true,
  };
  verifyLocalRenderReceipt(receipt, { status: 'local_retained', selection });
  return receipt;
}
