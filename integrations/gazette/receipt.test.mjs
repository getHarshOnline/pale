import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import test from 'node:test';
import { inspectLocalRenderReport, localRenderReceipt, retainedRenderReceipt } from './receipt.mjs';
import { join, parse } from 'node:path';

const selection = { origin: 'invocation', sites: ['site:example'], local_drafts: 'include' };
const configuration = { kind: 'supplied', source_sha256: 'b'.repeat(64) };
const output = () => ({ status: 'rendered', inventory: {
  identity: { format: 'native-inventory', sha256: 'a'.repeat(64) }, entries: [],
}, evidence: { source_frontier: 'native-example' } });
const report = () => ({ status: 'local_set_rendered', build_selection: structuredClone(selection),
  configuration: structuredClone(configuration),
  sources: { 'source:example': { revision: 'retained-native-example' } }, source_failures: {},
  sites: { 'site:example': output() } });
const encode = value => Buffer.from(JSON.stringify(value));
const expectation = (extra = {}) => ({ expectedSelection: selection, exitCode: 0,
  expectedConfiguration: configuration, requestSha256: 'c'.repeat(64),
  maximumBytes: 64 * 1024, requireRetainedFiles: false, ...extra });

test('singleton keeps exact native evidence and never claims retained files', () => {
  const bytes = Buffer.from(` \n${JSON.stringify(report(), null, 2)}\n`);
  const receipt = localRenderReceipt(bytes, expectation());
  assert.equal(receipt.native_report.json, bytes.toString('utf8'));
  assert.equal(receipt.native_report.sha256, createHash('sha256').update(bytes).digest('hex'));
  assert.equal(receipt.retained_files, false);
  assert.equal(receipt.proof_scope, 'local-publication-set-inventory');
  assert.deepEqual(receipt.build_selection, selection);
});

test('a successful outer status cannot hide a failed, missing or surplus sibling or source', () => {
  for (const mutate of [
    value => { value.status = 'local_set_incomplete'; },
    value => { value.sites['site:example'] = { status: 'failed', code: 'native-failure', message: 'private-canary' }; },
    value => { delete value.sites['site:example']; },
    value => { value.sites['site:extra'] = output(); },
    value => { value.source_failures['source:failed'] = { code: 'missing', message: 'private-canary' }; },
    value => { value.sources = {}; },
    value => { value.sites['site:example'].inventory.identity.sha256 = 'unobserved'; },
    value => { value.sites['site:example'].evidence = {}; },
  ]) {
    const value = report(); mutate(value); const original = encode(value);
    assert.throws(() => localRenderReceipt(original, expectation()), error => {
      assert(!error.message.includes('private-canary'));
      return true;
    });
    assert.deepEqual(original, encode(value));
  }
});

test('the exact admitted selection and terminal process outcome remain necessary', () => {
  for (const change of [
    { exitCode: null }, { exitCode: 1 }, { maximumBytes: 1 },
    { expectedSelection: { ...selection, origin: 'workspace' } },
    { expectedSelection: { ...selection, sites: [] } },
    { expectedSelection: { ...selection, sites: ['site:example', 'site:example'] } },
    { expectedConfiguration: { ...configuration, source_sha256: 'd'.repeat(64) } },
    { expectedConfiguration: null }, { requestSha256: null },
  ]) assert.throws(() => localRenderReceipt(encode(report()), expectation(change)));
  for (const bytes of [Buffer.from('{} trailing'), Buffer.from([0xff]), Buffer.from('{}')]) {
    assert.throws(() => localRenderReceipt(bytes, expectation()));
  }
});

test('set inspection preserves membership but cannot fabricate a singleton or retained artifact', () => {
  const value = report();
  value.build_selection.sites.push('site:second');
  value.sites['site:second'] = output();
  const options = expectation({ expectedSelection: value.build_selection });
  const result = inspectLocalRenderReport(encode(value), options);
  assert.deepEqual(Object.keys(result.report.sites), ['site:example', 'site:second']);
  assert.throws(() => localRenderReceipt(encode(value), options), /cannot trim/);
  assert.throws(() => localRenderReceipt(encode(report()), expectation({ requireRetainedFiles: true })), /unavailable/);
  const { requireRetainedFiles, ...unspecified } = expectation();
  assert.throws(() => localRenderReceipt(encode(report()), unspecified), /explicit acceptance/);
});

test('an exact raw report can be retained without silently rounding its integer projection', () => {
  const text = JSON.stringify(report()).replace('"entries":[]', '"entries":[],"bytes":9007199254740993');
  const bytes = Buffer.from(text);
  assert.equal(inspectLocalRenderReport(bytes, expectation()).nativeReport, text);
  assert.throws(() => localRenderReceipt(bytes, expectation()), /numeric projection/);
});

const outputParent = join(parse(process.cwd()).root, 'native-output');
const retainedReport = () => ({ status: 'local_retained',
  inputs: { site: 'site:example', compilation: { source_sha256: 'b'.repeat(64) },
    sources: { 'source:example': { revision: 'native-source' } } },
  evidence: { pages: ['index.html'] },
  inventory: { identity: { format: 'gazette.inventory.v1', sha256: 'a'.repeat(64) }, entries: [] },
  directory: join(outputParent, 'fresh-stage'),
});
const retainedExpectation = (extra = {}) => ({ expectedSite: 'site:example',
  expectedWorkspaceSha256: 'b'.repeat(64), expectedOutputParent: outputParent,
  requestSha256: 'c'.repeat(64), exitCode: 0, toolIsError: false, maximumBytes: 65536, ...extra });

test('native stage shape projects its actual site and preserves the original report', () => {
  const bytes = Buffer.from(` \n${JSON.stringify(retainedReport(), null, 2)}\n`);
  const receipt = retainedRenderReceipt(bytes, retainedExpectation());
  assert.deepEqual(receipt.build_selection, { site: 'site:example' });
  assert.equal(receipt.native_report.json, bytes.toString('utf8'));
  assert.equal(receipt.native_report.sha256, createHash('sha256').update(bytes).digest('hex'));
  assert.equal(receipt.proof_scope, 'native-single-site-retention-report');
  assert.equal(receipt.retained_files, true);
});

test('a stage result must match independent site, workspace, parent and native outcome', () => {
  for (const change of [
    { expectedSite: 'site:other' }, { expectedWorkspaceSha256: 'd'.repeat(64) },
    { expectedOutputParent: join(outputParent, 'other') }, { expectedOutputParent: '.' },
    { requestSha256: null }, { exitCode: 1 }, { exitCode: null },
    { toolIsError: true }, { toolIsError: undefined }, { maximumBytes: 1 },
  ]) assert.throws(() => retainedRenderReceipt(encode(retainedReport()), retainedExpectation(change)));
  for (const mutate of [
    value => { value.status = 'local_set_rendered'; },
    value => { value.inputs.sources = {}; },
    value => { value.directory = join(outputParent, 'nested', 'fresh-stage'); },
    value => { value.inventory.identity.format = 'other-format'; },
    value => { value.evidence = {}; },
  ]) {
    const value = retainedReport(); mutate(value);
    assert.throws(() => retainedRenderReceipt(encode(value), retainedExpectation()));
  }
});

test('native stage projection refuses unsafe integers without altering the original bytes', () => {
  const text = JSON.stringify(retainedReport()).replace('"entries":[]', '"entries":[],"bytes":9007199254740993');
  const bytes = Buffer.from(text);
  assert.throws(() => retainedRenderReceipt(bytes, retainedExpectation()), /numeric projection/);
  assert.equal(bytes.toString('utf8'), text);
});
