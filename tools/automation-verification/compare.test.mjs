import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { copyFileSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';
import { compareAppsmithExport, compareSavedApplication, compareWorkflow } from './compare.mjs';

test('workflow parity detects parameter, binding, behavior, graph, and pinned-data drift', () => {
  const plan = {
    nodes: [{ name: 'Transform', type: 'example.code', typeVersion: 1,
      parameters: { jsCode: 'return items;' }, credentials: { demo: { id: 'ref-1' } } }],
    connections: {}, settings: { executionOrder: 'v1' },
  };
  const native = structuredClone(plan);
  native.nodes[0].disabled = false;
  assert.deepEqual(compareWorkflow(plan, native), {
    codeDrift: [], bindingDrift: [], parameterDrift: [], behaviorDrift: [],
    settingsDrift: [], graphMatches: true, pinnedDataEmpty: true,
  });
  Object.assign(native.nodes[0], {
    typeVersion: 2, parameters: { jsCode: 'return [];' },
    credentials: { demo: { id: 'ref-2' } },
  });
  native.connections = { Transform: {} };
  native.settings.executionOrder = 'v0';
  native.pinData = { Transform: [] };
  assert.deepEqual(compareWorkflow(plan, native), {
    codeDrift: ['Transform'], bindingDrift: ['Transform'], parameterDrift: ['Transform'],
    behaviorDrift: ['Transform'], settingsDrift: ['executionOrder'], graphMatches: false,
    pinnedDataEmpty: false,
  });
  native.nodes.push(native.nodes[0]);
  assert.throws(() => compareWorkflow(plan, native), /duplicate name/);
});

test('workflow parity detects settings added or removed by the native export', () => {
  const workflow = { nodes: [], connections: {} };
  const planned = { ...workflow, settings: { executionOrder: 'v1' } };
  const native = { ...workflow, settings: { executionOrder: 'v1', errorWorkflow: 'retained-route' } };

  assert.deepEqual(compareWorkflow(planned, native).settingsDrift, ['errorWorkflow']);
  assert.deepEqual(compareWorkflow(native, planned).settingsDrift, ['errorWorkflow']);
  assert.deepEqual(compareWorkflow(workflow, native).settingsDrift, ['executionOrder', 'errorWorkflow']);
  assert.deepEqual(compareWorkflow(workflow, { ...workflow, settings: {} }).settingsDrift, []);
});

test('workflow parity checks execute-once and active retry settings', () => {
  const plan = {
    nodes: [{ name: 'Fetch', type: 'example.request', parameters: {},
      retryOnFail: true, maxTries: 2, waitBetweenTries: 1000 }],
    connections: {},
  };
  const native = structuredClone(plan);
  native.nodes[0].executeOnce = false;
  assert.deepEqual(compareWorkflow(plan, native).behaviorDrift, []);
  for (const change of [
    { executeOnce: true }, { maxTries: 5 }, { waitBetweenTries: 0 }, { waitBetweenTries: undefined },
  ]) {
    const changed = structuredClone(native);
    Object.assign(changed.nodes[0], change);
    assert.deepEqual(compareWorkflow(plan, changed).behaviorDrift, ['Fetch']);
  }
  plan.nodes[0].retryOnFail = false;
  native.nodes[0].retryOnFail = false;
  native.nodes[0].maxTries = 5;
  native.nodes[0].waitBetweenTries = 0;
  assert.deepEqual(compareWorkflow(plan, native).behaviorDrift, []);
});

test('workflow parity refuses incomplete or malformed graph evidence', () => {
  const complete = { nodes: [], connections: {} };
  assert.equal(compareWorkflow(complete, complete).graphMatches, true);
  for (const [field, value, error] of [
    ['connections', undefined, /connection map/],
    ['connections', null, /connection map/],
    ['nodes', {}, /node inventory/],
    ['settings', [], /Invalid settings map/],
    ['pinData', false, /Invalid pinData map/],
  ]) {
    const partial = { ...complete, [field]: value };
    if (value === undefined) delete partial[field];
    assert.throws(() => compareWorkflow(partial, partial), error);
    assert.throws(() => compareWorkflow(complete, partial), error);
    assert.throws(() => compareWorkflow(partial, complete), error);
  }
});

test('workflow parity refuses incomplete node evidence before comparing', () => {
  const validNode = { name: 'Read', type: 'example.request', parameters: {} };
  const complete = { nodes: [validNode], connections: {} };
  assert.equal(compareWorkflow(complete, complete).graphMatches, true);
  for (const node of [
    { ...validNode, type: ' ' },
    { name: validNode.name, type: validNode.type },
    { ...validNode, parameters: [] },
  ]) {
    const partial = { nodes: [node], connections: {} };
    for (const [planned, observed] of [[partial, partial], [complete, partial], [partial, complete]]) {
      assert.throws(() => compareWorkflow(planned, observed), /node type and parameter object/);
    }
  }
});

test('workflow parity validates connection endpoints and slots in both exports', () => {
  const edge = { node: 'Sink', type: 'ai_tool', index: 1 };
  const complete = {
    nodes: ['Source', 'Sink'].map(name => ({ name, type: 'example.step', parameters: {} })),
    connections: { Source: { ai_tool: [null, [], [edge]] } },
  };
  assert.equal(compareWorkflow(complete, complete).graphMatches, true);
  const moved = { ...complete, connections: { Source: { ai_tool: [[edge], [], null] } } };
  assert.equal(compareWorkflow(complete, moved).graphMatches, false);

  for (const [connections, error] of [
    [{ Unknown: { ai_tool: [[edge]] } }, /connection source/],
    [{ Source: [] }, /output map/],
    [{ Source: { ' ': [[edge]] } }, /output name/],
    [{ Source: { ai_tool: {} } }, /connection slots/],
    [{ Source: { ai_tool: [edge] } }, /connection destination/],
    ...[{ node: 'Missing' }, { type: '' }, { index: -1 }, { index: 0.5 }, { index: '1' }].map(
      change => [{ Source: { ai_tool: [[{ ...edge, ...change }]] } }, /destination, input type and index/],
    ),
  ]) {
    const partial = { ...complete, connections };
    for (const [planned, observed] of [[partial, partial], [complete, partial], [partial, complete]]) {
      assert.throws(() => compareWorkflow(planned, observed), error);
    }
  }
});

test('workflow parity reads the selected core Code body and preserves Python indentation', () => {
  const node = { name: 'Transform', type: 'n8n-nodes-base.code', typeVersion: 2,
    parameters: { language: 'pythonNative', pythonCode: '    return []', jsCode: 'stale' } };
  const planned = { nodes: [node], connections: {} };
  const observed = structuredClone(planned);
  const compareCode = () => compareWorkflow(planned, observed, () => ({})).codeDrift;
  assert.deepEqual(compareCode(), []);

  observed.nodes[0].parameters.jsCode = 'changed stale JavaScript';
  assert.deepEqual(compareCode(), []);
  observed.nodes[0].parameters.pythonCode = 'return []';
  assert.deepEqual(compareCode(), ['Transform']);
  observed.nodes[0].parameters.pythonCode = node.parameters.pythonCode;
  observed.nodes[0].parameters.language = 'python';
  assert.deepEqual(compareCode(), ['Transform']);
  delete observed.nodes[0].parameters.pythonCode;
  assert.throws(compareCode, /selected Code node body/);
  assert.throws(() => compareWorkflow(observed, planned), /selected Code node body/);
  observed.nodes[0].parameters = { language: 'custom', jsCode: 'return []' };
  assert.throws(compareCode, /selected Code node body/);
  observed.nodes[0].parameters = { language: 'javaScript', jsCode: 'return []' };
  assert.deepEqual(compareCode(), ['Transform']);
  delete observed.nodes[0].parameters.language;
  assert.deepEqual(compareWorkflow(observed, observed).codeDrift, []);
});

test('saved application parity requires a matching revision and per-page observation', () => {
  const app = { pageList: [{ unpublishedPage: { name: 'Queue' } }] };
  const observed = {
    sourceSha256: 'current', nativeImportAcknowledged: true, savedDraftReloadVerified: true,
    deployment: 'draft', pages: { Queue: { nativeLintClear: true } },
  };
  assert.deepEqual(compareSavedApplication(app, 'current', observed, ['draft']), {
    sourceMatches: true, pages: [{ page: 'Queue', nativeLintClear: true, nativeUpdatePending: false }],
  });
  assert.equal(compareSavedApplication(app, 'new', observed, ['draft']).pages[0].nativeUpdatePending, true);
  assert.equal(compareSavedApplication(app, 'current', observed, ['release']).sourceMatches, false);
  assert.equal(compareSavedApplication(app, 'current', { ...observed, pages: {} }, ['draft'])
    .pages[0].nativeUpdatePending, true);
});

test('Appsmith export detects action metadata, collection variables, layout and datasource drift', () => {
  const app = {
    pageList: [{ unpublishedPage: { name: 'Queue', layouts: [{ dsl: {} }] } }],
    actionList: [{ unpublishedAction: { pageId: 'Queue', name: 'load',
      actionConfiguration: { jsArguments: [{ name: 'limit', value: 10 }] } } }],
    actionCollectionList: [{ unpublishedCollection: {
      pageId: 'Queue', name: 'State', body: 'export default {};', variables: [] } }],
    datasourceList: [{ name: 'Local', id: 'original' }],
  };
  const native = structuredClone(app);
  for (const result of Object.values(compareAppsmithExport(app, native))) {
    assert.deepEqual(result, { missing: [], unexpected: [], changed: [] });
  }
  native.pageList[0].unpublishedPage.layouts[0].dsl.visible = false;
  native.actionList[0].unpublishedAction.actionConfiguration.jsArguments[0].value = 5;
  native.actionCollectionList[0].unpublishedCollection.variables.push({ name: 'flag' });
  native.datasourceList[0].id = 'replacement';
  for (const result of Object.values(compareAppsmithExport(app, native))) {
    assert.equal(result.changed.length, 1);
  }
});

test('Appsmith full export detects page routing and native action binding drift', () => {
  const plan = {
    pageList: [{ deleted: false, unpublishedPage: {
      name: 'Queue', slug: 'queue', isHidden: false, layouts: [{ dsl: {} }],
    } }],
    actionList: [{ deleted: false, pluginId: 'restapi-plugin', pluginType: 'API',
      unpublishedAction: { pageId: 'Queue', name: 'load', actionConfiguration: {},
        clientSideExecution: true, collectionId: 'queue-state', fullyQualifiedName: 'State.load' } }],
    actionCollectionList: [{ deleted: false, unpublishedCollection: {
      pageId: 'Queue', name: 'State', pluginId: 'js-plugin', pluginType: 'JS',
      body: 'export default {};', variables: [],
    } }],
    datasourceList: [],
  };
  for (const [surface, list, wrapper, field, value, identity] of [
    ['pages', 'pageList', 'unpublishedPage', 'slug', 'another-queue', ['Queue']],
    ['pages', 'pageList', 'unpublishedPage', 'isHidden', true, ['Queue']],
    ['pages', 'pageList', null, 'deleted', true, ['Queue']],
    ['actions', 'actionList', null, 'pluginId', 'different-plugin', ['Queue', 'load']],
    ['actions', 'actionList', null, 'pluginType', 'DB', ['Queue', 'load']],
    ['actions', 'actionList', null, 'deleted', true, ['Queue', 'load']],
    ['actions', 'actionList', 'unpublishedAction', 'clientSideExecution', false, ['Queue', 'load']],
    ['actions', 'actionList', 'unpublishedAction', 'clientSideExecution', undefined, ['Queue', 'load']],
    ['actions', 'actionList', 'unpublishedAction', 'collectionId', 'other-state', ['Queue', 'load']],
    ['actions', 'actionList', 'unpublishedAction', 'fullyQualifiedName', 'Other.load', ['Queue', 'load']],
    ['collections', 'actionCollectionList', 'unpublishedCollection', 'pluginId', 'different-plugin', ['Queue', 'State']],
    ['collections', 'actionCollectionList', null, 'deleted', true, ['Queue', 'State']],
  ]) {
    const observed = structuredClone(plan);
    const entry = observed[list][0];
    (wrapper ? entry[wrapper] : entry)[field] = value;
    assert.deepEqual(compareAppsmithExport(plan, observed)[surface], {
      missing: [], unexpected: [], changed: [JSON.stringify(identity)],
    });
  }
});

test('Appsmith compares ordered application routing apart from page definitions', () => {
  const planned = {
    pageList: ['Queue', 'Review'].map(name => ({ unpublishedPage: { name, layouts: [{ dsl: {} }] } })),
    exportedApplication: { pages: [
      { id: 'Queue', isDefault: true, isHidden: false },
      { id: 'Review', isDefault: false, isHidden: true },
    ] },
    actionList: [], actionCollectionList: [], datasourceList: [],
  };
  const changes = app => compareAppsmithExport(planned, app).pages.changed;
  const observed = structuredClone(planned);
  observed.pageList.reverse();
  assert.deepEqual(changes(observed), []);
  observed.exportedApplication.publishedPages = [{ id: 'Published' }];
  assert.deepEqual(changes(observed), []);
  observed.exportedApplication.pages.reverse();
  assert.deepEqual(changes(observed), ['["Queue"]', '["Review"]']);
  observed.exportedApplication.pages.reverse();
  observed.exportedApplication.pages[0].isDefault = false;
  observed.exportedApplication.pages[1].isDefault = true;
  assert.deepEqual(changes(observed), ['["Queue"]', '["Review"]']);
  observed.exportedApplication.pages = structuredClone(planned.exportedApplication.pages);
  observed.exportedApplication.pages[1].isHidden = false;
  assert.deepEqual(changes(observed), ['["Review"]']);
  observed.exportedApplication.pages.pop();
  assert.deepEqual(changes(observed), ['["Review"]']);
  delete observed.exportedApplication.pages;
  assert.deepEqual(changes(observed), ['["Queue"]', '["Review"]']);
  assert.deepEqual(compareAppsmithExport(observed, observed).pages.changed, []);
});

test('Appsmith refuses malformed or contradictory supplied navigation on either side', () => {
  const planned = {
    pageList: ['Queue', 'Review'].map(name => ({ unpublishedPage: { name, layouts: [{ dsl: {} }] } })),
    actionList: [], actionCollectionList: [], datasourceList: [],
  };
  for (const [pages, error] of [
    [null, /route inventory/],
    [[{ id: 'Absent' }], /page route/],
    [[{ id: 'Queue', isHidden: 'yes' }], /page route/],
    [[{ id: 'Queue' }, { id: 'Queue' }], /duplicate id/],
    [[{ id: 'Queue', isDefault: true }, { id: 'Review', isDefault: true }], /Multiple default/],
  ]) {
    const partial = { ...planned, exportedApplication: { pages } };
    assert.throws(() => compareAppsmithExport(partial, planned), error);
    assert.throws(() => compareAppsmithExport(planned, partial), error);
  }
});

test('workflow parity validates bindings on unmatched nodes before reporting drift', () => {
  const empty = { nodes: [], connections: {} };
  const node = { name: 'Extra', type: 'example.request', parameters: {} };
  for (const credentials of [[], { auth: { name: 'No reference' } }, { auth: { id: ' ' } }]) {
    const malformed = { nodes: [{ ...node, credentials }], connections: {} };
    const baseline = structuredClone(malformed);
    assert.throws(() => compareWorkflow(empty, malformed), /credential bindings|credential reference/);
    assert.throws(() => compareWorkflow(malformed, empty), /credential bindings|credential reference/);
    assert.deepEqual(malformed, baseline);
  }
  const valid = { nodes: [{ ...node, credentials: { auth: { id: 'ref-1' } } }], connections: {} };
  assert.equal(compareWorkflow(empty, valid).graphMatches, false);
  assert.equal(compareWorkflow(valid, empty).graphMatches, false);
});

test('workflow binding parity checks both directions without comparing labels', () => {
  const planned = {
    nodes: [
      { name: 'Source', type: 'example.request', parameters: {},
        credentials: { auth: { id: 'ref-1', name: 'Original' } } },
      { name: 'Map', type: 'example.map', parameters: {} },
    ],
    connections: {},
  };
  const observed = structuredClone(planned);
  observed.nodes[0].credentials.auth.name = 'Renamed';
  assert.deepEqual(compareWorkflow(planned, observed).bindingDrift, []);

  observed.nodes[0].credentials.extra = { id: 'ref-2' };
  observed.nodes[1].credentials = { auth: { id: 'ref-3' } };
  assert.deepEqual(compareWorkflow(planned, observed).bindingDrift, ['Source', 'Map']);

  delete observed.nodes[0].credentials;
  assert.deepEqual(compareWorkflow(planned, observed).bindingDrift, ['Source', 'Map']);

  observed.nodes[0].credentials = { auth: { name: 'Missing reference' } };
  assert.throws(() => compareWorkflow(planned, observed), /invalid credential reference/);
});

test('Appsmith rejects empty or partial evidence without widening saved-receipt scope', () => {
  const observation = {
    sourceSha256: 'current', nativeImportAcknowledged: true,
    savedDraftReloadVerified: true, deployment: 'draft',
    pages: { Queue: { nativeLintClear: true } },
  };
  const plan = { pageList: [{ unpublishedPage: { name: 'Queue' } }] };
  assert.throws(
    () => compareSavedApplication({ pageList: [] }, 'current', observation, ['draft']),
    /nonempty unpublished page inventory/,
  );
  assert.equal(compareSavedApplication(plan, 'current', observation, ['draft'])
    .pages[0].nativeUpdatePending, false);

  const exportPlan = { ...plan, actionList: [], actionCollectionList: [], datasourceList: [] };
  assert.throws(() => compareAppsmithExport(exportPlan, exportPlan), /Missing unpublished page layout/);
  const complete = structuredClone(exportPlan);
  complete.pageList[0].unpublishedPage.layouts = [{ dsl: {} }];
  const noDsl = structuredClone(complete);
  noDsl.pageList[0].unpublishedPage.layouts = [{}];
  assert.throws(() => compareAppsmithExport(complete, noDsl), /Missing unpublished page layout/);
  assert.throws(() => compareAppsmithExport(complete, { ...complete, actionList: undefined }),
    /Missing actions inventory/);
  assert.deepEqual(compareAppsmithExport(complete, complete).pages, {
    missing: [], unexpected: [], changed: [],
  });
});

test('Appsmith export requires action configuration and collection execution evidence', () => {
  const complete = {
    pageList: [{ unpublishedPage: { name: 'Queue', layouts: [{ dsl: {} }] } }],
    actionList: [{ unpublishedAction: { pageId: 'Queue', name: 'load', actionConfiguration: {} } }],
    actionCollectionList: [{ unpublishedCollection: {
      pageId: 'Queue', name: 'State', body: 'export default {};', variables: [],
    } }],
    datasourceList: [],
  };
  for (const [list, wrapper, field, invalid, error] of [
    ['actionList', 'unpublishedAction', 'actionConfiguration', [], /invalid unpublished action configuration/],
    ['actionCollectionList', 'unpublishedCollection', 'body', null, /invalid unpublished collection body or variables/],
    ['actionCollectionList', 'unpublishedCollection', 'variables', {}, /invalid unpublished collection body or variables/],
  ]) {
    const partial = structuredClone(complete);
    delete partial[list][0][wrapper][field];
    assert.throws(() => compareAppsmithExport(partial, partial), error);
    partial[list][0][wrapper][field] = invalid;
    assert.throws(() => compareAppsmithExport(complete, partial), error);
  }
  assert.deepEqual(compareAppsmithExport(complete, complete).collections, {
    missing: [], unexpected: [], changed: [],
  });
});

const verifier = fileURLToPath(new URL('./verify.mjs', import.meta.url));
const sha256 = text => createHash('sha256').update(text).digest('hex');

function inVerifierFixture(run) {
  const directory = mkdtempSync(join(tmpdir(), 'automation-verifier-'));
  try {
    const expected = join(directory, 'expected.json');
    const observed = join(directory, 'private-fixture-path.json');
    const compare = (kind, ...rest) => invokeVerifier(verifier, kind, expected, observed, ...rest);
    run({ directory, expected, observed, compare });
  } finally {
    rmSync(directory, { force: true, recursive: true });
  }
}

function invokeVerifier(entry, ...args) {
  const result = spawnSync(process.execPath, [entry, ...args], { encoding: 'utf8', timeout: 10000 });
  assert.ifError(result.error);
  assert.equal(result.signal, null);
  return result;
}

test('verifier preserves structured comparisons for all three modes', () => inVerifierFixture(
  ({ expected, observed, compare }) => {
    const workflow = { nodes: [], connections: {} };
    const app = { pageList: [{ unpublishedPage: { name: 'Queue', layouts: [{ dsl: {} }] } }],
      actionList: [], actionCollectionList: [], datasourceList: [] };
    const bytes = JSON.stringify(app);
    const saved = { sourceSha256: sha256(bytes), nativeImportAcknowledged: true,
      savedDraftReloadVerified: true, deployment: 'draft', pages: { Queue: { nativeLintClear: true } } };
    const cases = [
      ['n8n', workflow, workflow, { ...workflow, settings: { executionOrder: 'v1' } }, []],
      ['appsmith-export', app, app, { ...app, datasourceList: [{ name: 'fixture' }] }, []],
      ['appsmith-saved', app, saved, { ...saved, pages: {} }, ['draft']],
    ];
    for (const [kind, plan, equal, different, args] of cases) {
      writeFileSync(expected, JSON.stringify(plan));
      for (const [value, status] of [[equal, 0], [different, 1]]) {
        writeFileSync(observed, JSON.stringify(value));
        const result = compare(kind, ...args);
        assert.equal(result.status, status);
        assert.equal(result.stderr, '');
        const report = JSON.parse(result.stdout);
        assert.deepEqual(Object.keys(report), ['contract','coverage','inputSha256','matches','findings']);
        assert.equal(report.matches, status === 0);
        assert.equal(report.inputSha256.observed, sha256(JSON.stringify(value)));
      }
    }
  },
));

test('verifier distinguishes unusable evidence and omits paths and raw payload', () => inVerifierFixture(
  ({ expected, observed, compare }) => {
    const good = '{"nodes":[],"connections":{}}';
    writeFileSync(expected, good);
    const failures = [
      ['n8n', null, 'unreadable-artifact'],
      ['n8n', '{ "private-fixture-text" ', 'invalid-json'],
      ['n8n', '{"nodes":[]}', 'invalid-retained-evidence'],
      ['appsmith-saved', 'null', 'invalid-retained-evidence'],
      ['unknown', good, 'invalid-invocation'],
    ];
    for (const [kind, value, code] of failures) {
      if (value !== null) writeFileSync(observed, value);
      const result = compare(kind, ...(kind === 'appsmith-saved' ? ['draft'] : []));
      assert.equal(result.status, 2);
      assert.equal(result.stdout, '');
      assert.doesNotMatch(result.stderr, /private-fixture/);
      const { contract, diagnostic } = JSON.parse(result.stderr);
      assert.equal(contract, 'automation-export-verifier.v1');
      assert.equal(diagnostic.code, code);
      assert.equal(Object.hasOwn(diagnostic, 'findings'), false);
      if (code === 'unreadable-artifact' || code === 'invalid-json') {
        assert.deepEqual(diagnostic.artifacts, ['observed']);
      }
      if (code === 'invalid-json' || code === 'invalid-retained-evidence') {
        assert.deepEqual(diagnostic.inputSha256, { expected: sha256(good), observed: sha256(value) });
      }
    }
  },
));

test('verifier contains an unexpected implementation error separately', () => inVerifierFixture(
  ({ directory, expected, observed }) => {
    const entry = join(directory, 'verify.mjs');
    copyFileSync(verifier, entry);
    writeFileSync(join(directory, 'compare.mjs'), [
      "export const contract = 'automation-export-verifier.v1';",
      'export class InvalidRetainedEvidence extends TypeError {}',
      "export function compareWorkflow() { throw new TypeError('private-fixture-error'); }",
      'export { compareWorkflow as compareAppsmithExport, compareWorkflow as compareSavedApplication };',
    ].join('\n'));
    writeFileSync(expected, '{}');
    writeFileSync(observed, '{}');
    const result = invokeVerifier(entry, 'n8n', expected, observed);
    assert.equal(result.status, 3);
    assert.equal(result.stdout, '');
    assert.deepEqual(JSON.parse(result.stderr), {
      contract: 'automation-export-verifier.v1', diagnostic: { code: 'unexpected-verifier-failure' },
    });
  },
));
