import { isDeepStrictEqual as equal } from 'node:util';

export const contract = 'automation-export-verifier.v1';

export class InvalidRetainedEvidence extends TypeError {}

function index(records, key) {
  const result = new Map();
  for (const record of records) {
    const identity = record?.[key];
    if (typeof identity !== 'string' || !identity || result.has(identity)) {
      throw new InvalidRetainedEvidence(`Missing or duplicate ${key} in retained export`);
    }
    result.set(identity, record);
  }
  return result;
}

// Credential labels may change without changing the native binding.
function credentialReferences(node) {
  const credentials = node?.credentials ?? {};
  if (typeof credentials !== 'object' || Array.isArray(credentials)) {
    throw new InvalidRetainedEvidence('Invalid credential bindings in retained workflow export');
  }
  return Object.fromEntries(Object.entries(credentials).map(([kind, binding]) => {
    const id = binding?.id;
    if (!kind || !((typeof id === 'string' && id.trim()) || Number.isSafeInteger(id))) {
      throw new InvalidRetainedEvidence('Missing or invalid credential reference in retained workflow export');
    }
    return [kind, id];
  }));
}

function isRecord(value) {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function requireWorkflowEvidence(workflow) {
  if (!isRecord(workflow) || !Array.isArray(workflow.nodes) || !isRecord(workflow.connections)) {
    throw new InvalidRetainedEvidence('Expected a workflow node inventory and connection map');
  }
  for (const field of ['settings', 'pinData']) {
    if (Object.hasOwn(workflow, field) && !isRecord(workflow[field])) {
      throw new InvalidRetainedEvidence(`Invalid ${field} map in retained workflow export`);
    }
  }
  for (const node of workflow.nodes) {
    if (typeof node?.type !== 'string' || !node.type.trim() || !isRecord(node.parameters)) {
      throw new InvalidRetainedEvidence('Expected a node type and parameter object in retained workflow export');
    }
    credentialReferences(node);
  }
}

function requireConnectionEvidence(connections, nodes) {
  for (const [source, outputs] of Object.entries(connections)) {
    if (!nodes.has(source) || !isRecord(outputs)) {
      throw new InvalidRetainedEvidence('Expected a known connection source and an output map in retained workflow export');
    }
    for (const [output, slots] of Object.entries(outputs)) {
      if (!output.trim() || !Array.isArray(slots)) {
        throw new InvalidRetainedEvidence('Expected an output name and connection slots in retained workflow export');
      }
      for (const edges of slots) {
        if (edges === null) continue;
        if (!Array.isArray(edges) || edges.some(edge =>
          !isRecord(edge) || !nodes.has(edge.node) || typeof edge.type !== 'string' ||
          !edge.type.trim() || !Number.isSafeInteger(edge.index) || edge.index < 0)) {
          throw new InvalidRetainedEvidence('Expected a known connection destination, input type and index in retained workflow export');
        }
      }
    }
  }
}

function retainedCode(node) {
  if (!node.type.endsWith('.code')) return undefined;
  const { language = 'javaScript', jsCode, pythonCode } = node.parameters;
  if (node.type !== 'n8n-nodes-base.code') {
    // Preserve the JavaScript diagnostic for other known export shapes.
    return typeof jsCode === 'string' ? ['javaScript', jsCode.trim()] : undefined;
  }
  switch (language) {
    case 'javaScript':
      if (typeof jsCode === 'string') return [language, jsCode.trim()];
      break;
    case 'python':
    case 'pythonNative':
      if (typeof pythonCode === 'string') return [language, pythonCode];
      break;
  }
  throw new InvalidRetainedEvidence('Missing or invalid selected Code node body in retained workflow export');
}

function indexCode(nodes) {
  return new Map([...nodes].map(([name, node]) => [name, retainedCode(node)]));
}

const defaults = { disabled: false, retryOnFail: false, executeOnce: false, onError: 'stopWorkflow' };
const behaviorFields = ['disabled', 'retryOnFail', 'executeOnce', 'onError', 'typeVersion', 'alwaysOutputData'];
const retryFields = ['maxTries', 'waitBetweenTries'];

export function compareWorkflow(planned, native, normalize = node => node.parameters) {
  requireWorkflowEvidence(planned);
  requireWorkflowEvidence(native);
  const expected = index(planned.nodes, 'name');
  const observed = index(native.nodes, 'name');
  requireConnectionEvidence(planned.connections, expected);
  requireConnectionEvidence(native.connections, observed);
  const nodes = [...expected.values()];
  const expectedCode = indexCode(expected);
  const observedCode = indexCode(observed);
  const codeDrift = [...expectedCode].filter(([name, code]) =>
    code !== undefined && !equal(code, observedCode.get(name)));
  const bindingDrift = nodes.filter(node =>
    !equal(credentialReferences(node), credentialReferences(observed.get(node.name))));
  const parameterDrift = nodes.filter(node => !observed.has(node.name) ||
    !equal(normalize(node), normalize(observed.get(node.name))));
  const behaviorDrift = nodes.filter(node => {
    const nativeNode = observed.get(node.name);
    return behaviorFields.some(key =>
      !equal(node[key] ?? defaults[key], nativeNode?.[key] ?? defaults[key])) ||
      (node.retryOnFail === true && nativeNode?.retryOnFail === true &&
        retryFields.some(key => !equal(node[key], nativeNode[key])));
  });
  const expectedSettings = planned.settings ?? {};
  const observedSettings = native.settings ?? {};
  const settingKeys = new Set([...Object.keys(expectedSettings), ...Object.keys(observedSettings)]);
  const settingsDrift = [...settingKeys].filter(key =>
    Object.hasOwn(expectedSettings, key) !== Object.hasOwn(observedSettings, key) ||
    !equal(expectedSettings[key], observedSettings[key]));
  const graphMatches = equal(planned.connections, native.connections) &&
    expected.size === observed.size &&
    nodes.every(node => observed.get(node.name)?.type === node.type);
  return {
    codeDrift: codeDrift.map(([name]) => name),
    bindingDrift: bindingDrift.map(node => node.name),
    parameterDrift: parameterDrift.map(node => node.name),
    behaviorDrift: behaviorDrift.map(node => node.name),
    settingsDrift,
    graphMatches,
    pinnedDataEmpty: Object.keys(native.pinData ?? {}).length === 0,
  };
}

function pageEntries(app) {
  if (!Array.isArray(app?.pageList) || app.pageList.length === 0) {
    throw new InvalidRetainedEvidence('Expected a nonempty unpublished page inventory');
  }
  return app.pageList;
}

// A retained import and reload receipt can establish this narrower claim even
// when a complete native export was unavailable.
export function compareSavedApplication(planned, sourceSha256, observation, acceptedDeployments) {
  if (!isRecord(observation)) {
    throw new InvalidRetainedEvidence('Invalid saved application receipt in retained evidence');
  }
  const sourceMatches = observation.sourceSha256 === sourceSha256 &&
    observation.nativeImportAcknowledged === true &&
    observation.savedDraftReloadVerified === true &&
    acceptedDeployments.includes(observation.deployment);
  const pages = index(pageEntries(planned).map(entry => entry?.unpublishedPage), 'name');
  const results = [...pages.keys()].map(page => {
    const nativeLintClear = Object.hasOwn(observation.pages ?? {}, page) &&
      observation.pages[page]?.nativeLintClear === true;
    return { page, nativeLintClear, nativeUpdatePending: !sourceMatches || !nativeLintClear };
  });
  return { sourceMatches, pages: results };
}

// Export navigation references use page names after any caller normalization.
function unpublishedPageRoutes(app, pages) {
  const application = app.exportedApplication;
  if (!Object.hasOwn(application ?? {}, 'pages')) return null;
  if (!Array.isArray(application.pages)) {
    throw new InvalidRetainedEvidence('Invalid unpublished application page route inventory');
  }
  const pageNames = new Set(pages.map(page => page?.unpublishedPage?.name));
  let defaultPageCount = 0;
  const routes = application.pages.map((route, position) => {
    if (!isRecord(route) || !pageNames.has(route.id) ||
        ['isDefault', 'isHidden'].some(field =>
          Object.hasOwn(route, field) && typeof route[field] !== 'boolean')) {
      throw new InvalidRetainedEvidence('Invalid unpublished application page route');
    }
    if (route.isDefault) defaultPageCount++;
    return { id: route.id, position, isDefault: route.isDefault, isHidden: route.isHidden };
  });
  if (defaultPageCount > 1) {
    throw new InvalidRetainedEvidence('Multiple default unpublished application page routes');
  }
  return index(routes, 'id');
}

// Strict comparison of unpublished execution surfaces in complete exports.
// Native IDs and formats are evidence. The caller supplies any intentional
// environment translation before comparing.
export function compareAppsmithExport(planned, native) {
  const surfaces = {
    pages: ['pageList', 'unpublishedPage', ['name'], ['layouts', 'slug', 'isHidden'], ['deleted']],
    actions: ['actionList', 'unpublishedAction', ['pageId', 'name'], [
      'datasource', 'actionConfiguration', 'dynamicBindingPathList',
      'jsonPathKeys', 'runBehaviour', 'confirmBeforeExecute',
      'clientSideExecution', 'collectionId', 'fullyQualifiedName',
    ], ['pluginId', 'pluginType', 'deleted']],
    collections: ['actionCollectionList', 'unpublishedCollection', ['pageId', 'name'], [
      'body', 'variables', 'actions', 'archivedActions', 'pluginId', 'pluginType',
    ], ['deleted']],
    datasources: ['datasourceList', null, ['name'], null],
  };
  return Object.fromEntries(Object.entries(surfaces).map(([kind, [list, record, keys, fields, envelopeFields]]) => {
    function project(app) {
      const entries = kind === 'pages' ? pageEntries(app) : app?.[list];
      if (!Array.isArray(entries)) {
        throw new InvalidRetainedEvidence(`Missing ${kind} inventory in retained export`);
      }
      const routes = kind === 'pages' ? unpublishedPageRoutes(app, entries) : null;
      return index(entries.map(entry => {
        const source = record ? entry?.[record] : entry;
        const values = keys.map(key => source?.[key]);
        if (values.some(value => typeof value !== 'string' || !value.trim())) {
          throw new InvalidRetainedEvidence(`Missing ${kind} identity in retained export`);
        }
        if (kind === 'pages' && (!Array.isArray(source.layouts) || source.layouts.length === 0 ||
            source.layouts.some(layout => !isRecord(layout?.dsl)))) {
          throw new InvalidRetainedEvidence('Missing unpublished page layout or DSL in retained export');
        }
        if (kind === 'actions' && !isRecord(source.actionConfiguration)) {
          throw new InvalidRetainedEvidence('Missing or invalid unpublished action configuration in retained export');
        }
        if (kind === 'collections' &&
            (typeof source.body !== 'string' || !Array.isArray(source.variables))) {
          throw new InvalidRetainedEvidence('Missing or invalid unpublished collection body or variables in retained export');
        }
        const value = fields ? {
          payload: Object.fromEntries(fields.map(field => [field, source[field]])),
          envelope: Object.fromEntries(envelopeFields.map(field => [field, entry[field]])),
        } : source;
        if (routes) value.route = routes.get(source.name) ?? null;
        return { identity: JSON.stringify(values), value };
      }), 'identity');
    }
    const expected = project(planned);
    const observed = project(native);
    const missing = [...expected.keys()].filter(key => !observed.has(key));
    const unexpected = [...observed.keys()].filter(key => !expected.has(key));
    const changed = [...expected.keys()].filter(key => observed.has(key) &&
      !equal(expected.get(key).value, observed.get(key).value));
    return [kind, { missing, unexpected, changed }];
  }));
}
