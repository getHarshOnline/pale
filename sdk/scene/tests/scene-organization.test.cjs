const assert = require("node:assert/strict");
const { test, mock } = require("node:test");
const { readFileSync } = require("node:fs");
const { join } = require("node:path");
const { Pale, OrganizationComparisonSide, OrganizationEndpointLookup,
  WorkspaceValueComparison } = require("@rtepl/pale");
const { ElkExecutor } = require("@rtepl/pale-scene");
const { OrganizationIntake, OrganizationIntakeError, OrganizationTopology, OrganizationSceneCompiler } = require("@rtepl/pale-scene/organization");

// Native qualification must supply the existing five-file SDK fixture, not a mock.
const fixture = process.env.PALE_SCENE_TEST_ORGANIZATION_FIXTURE;
assert(fixture, "Supply the canonical organization fixture before running this native test");
const files = ["pale-workspace.toml", "organization.toml", "responsibility.toml", "resolution.lock.toml", "universe.lock.toml"];
const compile = (mutate = documents => documents) => new Pale().compilePortableWorkspace({ rootManifest: files[0],
  documents: mutate(files.map(label => ({ label, bytes: readFileSync(join(fixture, label)) }))) });
const query = { inspected: 2, returned: 2, copiedStringBytes: 65536 };
const limits = { queries: 2000, recordVisits: 5000, sourceVisits: 5000, records: 1000,
  occurrences: 2000, sourceOccurrences: 5000, copiedStringBytes: 32 * 1024 * 1024 };
const organizationIntake = (owner, selectedQuery, selectedLimits, budget) =>
  new OrganizationIntake(owner, selectedQuery, selectedLimits, budget);
const retainedComparison = comparison => comparison;
function collect(intake) {
  const occurrences = [];
  for (let page = 0; page < limits.queries; page++) {
    const step = intake.next();
    if (step.kind === "enumeration-exhausted") return occurrences;
    assert.equal(step.kind, "page"); occurrences.push(...step.occurrences);
  }
  assert.fail("The bounded fixture did not exhaust");
}

const alphaRole = { kind: "role", role: { namespace: "alpha", roleId: "accountable-a" } };
const betaScope = { kind: "scope", scope: { namespace: "beta", scopeId: "scope" } };
function selectedPages(intake, method, endpoint) {
  const occurrences = [], pages = []; let after;
  for (let count = 0; count < limits.queries; count++) {
    const step = intake[method](endpoint, after); assert.equal(step.kind, "page");
    occurrences.push(...step.occurrences); pages.push(step.page);
    after = step.page.continuation(); assert.equal(step.page.isComplete, after === null);
    if (after === null) return { occurrences, pages };
  }
  assert.fail("The bounded selected query did not exhaust");
}

test("cancelled intake steps preserve state and charge no work", () => {
  const owner = compile(), intake = organizationIntake(owner, query, limits);
  const controller = new AbortController(); controller.abort(); const signal = controller.signal;
  try {
    const initialCharges = intake.charges;
    assert.deepEqual(intake.next(signal), { kind: "stopped", reason: "cancelled" });
    assert.deepEqual(intake.endpoint(null, undefined, signal), { kind: "stopped", reason: "cancelled" });
    assert.deepEqual(intake.incidentRelations(null, undefined, signal), { kind: "stopped", reason: "cancelled" });
    assert.deepEqual(intake.incidentServices(null, undefined, signal), { kind: "stopped", reason: "cancelled" });
    assert.deepEqual(intake.charges, initialCharges);

    const first = intake.next(); assert.equal(first.kind, "page");
    const expected = owner.inspectOrganization(query.inspected, query.returned, query.copiedStringBytes);
    assert.deepEqual(first.occurrences.map(item => intake.entry(item).row), expected.entries().map(entry => entry.row));
    assert.deepEqual(first.occurrences.map(item => item.id), expected.entries().map((_, index) => `o${index}`));

    const occurrence = first.occurrences[0]; assert(occurrence);
    const beforeSources = intake.charges;
    assert.deepEqual(intake.sources(occurrence, undefined, signal), { kind: "stopped", reason: "cancelled" });
    assert.deepEqual(intake.charges, beforeSources);
    assert.equal(intake.sources(occurrence).kind, "page");

    const comparison = retainedComparison(new Pale().comparePortableWorkspaces(owner, compile()));
    const beforeCorrespondence = intake.charges;
    assert.deepEqual(intake.correspond(comparison, OrganizationComparisonSide.Prior,
      occurrence, undefined, signal), { kind: "stopped", reason: "cancelled" });
    assert.deepEqual(intake.charges, beforeCorrespondence);
    assert.equal(intake.correspond(comparison, OrganizationComparisonSide.Prior, occurrence).kind, "result");

    const endpointBefore = intake.charges;
    assert.deepEqual(intake.endpoint(alphaRole, undefined, signal), { kind: "stopped", reason: "cancelled" });
    assert.deepEqual(intake.charges, endpointBefore);
    const endpoint = intake.endpoint(alphaRole); assert.equal(endpoint.kind, "page");
    assert(endpoint.occurrences.every(item => item.inclusion === "endpoint-query"));
    assert.equal(intake.incidentRelations(alphaRole).kind, "page");
    assert.equal(intake.incidentServices({ namespace: "alpha", scopeId: "scope" }).kind, "page");
  } finally { intake.close(); }
});

test("closed and exhausted states take precedence over cancellation", () => {
  const controller = new AbortController(); controller.abort(); const signal = controller.signal;
  const exhausted = organizationIntake(compile(), query, limits);
  try {
    collect(exhausted);
    assert.deepEqual(exhausted.next(signal), { kind: "enumeration-exhausted" });
  } finally { exhausted.close(); }
  assert.deepEqual(exhausted.next(signal), { kind: "closed" });
  assert.deepEqual(exhausted.endpoint(null, undefined, signal), { kind: "closed" });
  assert.deepEqual(exhausted.sources(null, undefined, signal), { kind: "closed" });
  assert.deepEqual(exhausted.correspond(null, null, null, undefined, signal), { kind: "closed" });
});

test("every intake resource stops before dispatch and keeps the existing charges", () => {
  const owner = compile();
  for (const resource of ["recordVisits", "records", "occurrences", "copiedStringBytes"]) {
    const intake = organizationIntake(owner, query, { ...limits, [resource]: 1 });
    try {
      const before = intake.charges;
      assert.deepEqual(intake.endpoint(null), { kind: "resource-stop", resource });
      assert.deepEqual(intake.charges, before);
    } finally { intake.close(); }
  }

  const queryLimited = organizationIntake(owner, query, { ...limits, queries: 1 });
  try {
    assert.equal(queryLimited.endpoint({ kind: "native-resource", nativeResource: "native.unobserved" }).kind, "page");
    const before = queryLimited.charges;
    assert.deepEqual(queryLimited.endpoint(null), { kind: "resource-stop", resource: "queries" });
    assert.deepEqual(queryLimited.charges, before);
  } finally { queryLimited.close(); }

  for (const resource of ["sourceVisits", "sourceOccurrences"]) {
    const intake = organizationIntake(owner, query, { ...limits, [resource]: 1 });
    try {
      const selected = selectedPages(intake, "endpoint", alphaRole).occurrences[0]; assert(selected);
      const before = intake.charges;
      assert.deepEqual(intake.sources(selected, {}), { kind: "resource-stop", resource });
      assert.deepEqual(intake.charges, before);
    } finally { intake.close(); }
  }
});

test("intake charges report actual work and never decrease between completed calls", () => {
  const owner = compile(), whole = { ...query, inspected: 256, returned: 256 };
  const intake = organizationIntake(owner, whole, limits);
  const snapshots = [intake.charges];
  try {
    assert.deepEqual(snapshots[0], { queries: 0, recordVisits: 0, sourceVisits: 0,
      records: 0, occurrences: 0, sourceOccurrences: 0, copiedStringBytes: 0 });
    const selected = intake.endpoint(alphaRole); assert.equal(selected.kind, "page");
    const selectedEntries = selected.page.entries();
    snapshots.push(intake.charges);
    assert.equal(intake.charges.queries, 1);
    assert.equal(intake.charges.recordVisits, selected.page.inspected);
    assert.equal(intake.charges.records, selectedEntries.length);
    assert.equal(intake.charges.occurrences, selectedEntries.length);
    assert.equal(intake.charges.copiedStringBytes, selected.page.copiedStringBytes);
    assert.equal(intake.charges.sourceVisits, 0); assert.equal(intake.charges.sourceOccurrences, 0);

    const occurrence = selected.occurrences[0]; assert(occurrence);
    const beforeSource = intake.charges, source = intake.sources(occurrence); assert.equal(source.kind, "page");
    snapshots.push(intake.charges);
    const sourceCount = source.mapping.kind === "named" ? source.mapping.occurrences.length : 0;
    const sourceInspected = source.mapping.kind === "named" ? source.mapping.inspected : 0;
    assert.equal(intake.charges.queries, beforeSource.queries + 1);
    assert.equal(intake.charges.sourceVisits, beforeSource.sourceVisits + sourceInspected);
    assert.equal(intake.charges.sourceOccurrences, beforeSource.sourceOccurrences + sourceCount);
    assert.equal(intake.charges.recordVisits, beforeSource.recordVisits);
    assert.equal(intake.charges.records, beforeSource.records);
    assert.equal(intake.charges.occurrences, beforeSource.occurrences);
    assert.equal(intake.charges.copiedStringBytes,
      beforeSource.copiedStringBytes + source.page.copiedStringBytes);

    const comparison = retainedComparison(new Pale().comparePortableWorkspaces(owner, compile()));
    const beforeCorrespondence = intake.charges;
    const correspondence = intake.correspond(comparison, OrganizationComparisonSide.Prior, occurrence);
    assert.equal(correspondence.kind, "result"); snapshots.push(intake.charges);
    assert.equal(intake.charges.records, beforeCorrespondence.records);
    assert.equal(intake.charges.occurrences, beforeCorrespondence.occurrences);
    assert.equal(intake.charges.sourceVisits, beforeCorrespondence.sourceVisits);
    assert.equal(intake.charges.sourceOccurrences, beforeCorrespondence.sourceOccurrences);

    for (let index = 1; index < snapshots.length; index++) {
      for (const resource of Object.keys(snapshots[index])) {
        assert(snapshots[index][resource] >= snapshots[index - 1][resource], `${resource} charge decreased`);
      }
    }
  } finally { intake.close(); }
});

test("cancellation during a synchronous SDK call stops only the following call", () => {
  const owner = compile(), intake = organizationIntake(owner, query, limits);
  const controller = new AbortController();
  const endpoint = { kind: "role", role: { get namespace() {
    controller.abort(); return "alpha";
  }, roleId: "accountable-a" } };
  try {
    const completed = intake.endpoint(endpoint, undefined, controller.signal);
    assert.equal(completed.kind, "page");
    const before = intake.charges;
    assert.deepEqual(intake.endpoint(alphaRole, undefined, controller.signal), { kind: "stopped", reason: "cancelled" });
    assert.deepEqual(intake.charges, before);
  } finally { intake.close(); }
});

test("organization intake uses the SDK owner rather than instance method replacements", () => {
  const owner = compile(), intake = organizationIntake(owner, query, limits);
  const replacements = ["inspectOrganization", "inspectOrganizationEndpoint", "inspectOrganizationIncidentRelations",
    "inspectOrganizationIncidentServices"].map(method => mock.method(owner, method,
      () => assert.fail(`Scene must not obtain semantic rows from an instance replacement for ${method}`)));
  try {
    assert.equal(intake.next().kind, "page");
    assert.equal(intake.endpoint(alphaRole).kind, "page");
    assert.equal(intake.incidentRelations(alphaRole).kind, "page");
    assert.equal(intake.incidentServices({ namespace: "alpha", scopeId: "scope" }).kind, "page");
    assert(replacements.every(replacement => replacement.mock.callCount() === 0));
  } finally {
    for (const replacement of replacements) replacement.mock.restore();
    intake.close();
  }
});

test("endpoint and incident pages preserve tiny versus whole SDK results without advancing enumeration", () => {
  const owner = compile(), tiny = organizationIntake(owner, { ...query, inspected: 1, returned: 1 }, limits);
  const whole = organizationIntake(owner, { ...query, inspected: 256, returned: 256 }, limits);
  try {
    const first = tiny.next(); assert.equal(first.kind, "page");
    const enumerationCursor = first.page.continuation(); assert(enumerationCursor);
    for (const [method, reason] of [["endpoint", "endpoint-query"], ["incidentRelations", "incident-relation-query"]]) {
      const before = tiny.charges;
      const small = selectedPages(tiny, method, alphaRole), large = selectedPages(whole, method, alphaRole);
      const entries = small.occurrences.map(item => tiny.entry(item));
      assert.deepEqual(entries.map(entry => entry.row), large.occurrences.map(item => whole.entry(item).row));
      assert(small.occurrences.every(item => item.inclusion === reason));
      assert(entries.every(entry => entry.handle.belongsTo(owner)));
      assert.equal(tiny.charges.recordVisits - before.recordVisits, small.pages.reduce((sum, page) => sum + page.inspected, 0));
      assert.equal(tiny.charges.copiedStringBytes - before.copiedStringBytes, small.pages.reduce((sum, page) => sum + page.copiedStringBytes, 0));
      assert.equal(tiny.charges.records - before.records, entries.length);
      assert.equal(tiny.charges.occurrences - before.occurrences, entries.length);
      if (method === "incidentRelations") assert(entries.every(entry => ["relation", "derived-relation"].includes(entry.row.kind)));
    }
    const continued = tiny.next(); assert.equal(continued.kind, "page");
    const expected = owner.inspectOrganization(1, 1, query.copiedStringBytes, enumerationCursor);
    assert.deepEqual(continued.occurrences.map(item => tiny.entry(item).row), expected.entries().map(entry => entry.row));
    assert(continued.occurrences.every(item => item.inclusion === "enumerated-record"));
  } finally { tiny.close(); whole.close(); }
});

test("scope-local services build exact directional routes without enumeration or client filtering", async () => {
  const owner = compile(), intake = organizationIntake(owner, { ...query, inspected: 1, returned: 1 }, limits);
  const enumeration = mock.method(owner, "inspectOrganization", () => assert.fail("Service expansion must not enumerate"));
  const executor = new ElkExecutor({ requestBytes: 1e6, resultBytes: 1e6, deadlineMs: 10000,
    totalRequestBytes: 10e6, totalResultBytes: 10e6, totalActiveMs: 60000, oldHeapMb: 128, youngHeapMb: 32 });
  let compiler;
  try {
    const selected = selectedPages(intake, "incidentServices", { namespace: "alpha", scopeId: "scope" });
    assert.equal(selected.occurrences.length, 2);
    assert(selected.occurrences.every(item => item.inclusion === "incident-service-query"));
    assert(selected.pages.every(page => page.endpointLookup === null));
    const endpoints = ["alpha", "beta"].flatMap(namespace => selectedPages(intake, "endpoint",
      { kind: "scope", scope: { namespace, scopeId: "scope" } }).occurrences);
    const topology = new OrganizationTopology(intake, [...selected.occurrences, ...endpoints],
      { records: 10, appearances: 10, routeAppearances: 10, identityCodeUnits: 10000 });
    const routes = selected.occurrences.map(relation => {
      const entry = topology.entry(relation), row = entry.row; assert.equal(row.kind, "service");
      assert(entry.handle.belongsTo(owner)); assert.equal(intake.sources(relation).mapping.kind, "named");
      assert.equal(new Set([row.providerCompletion, row.providerAcceptance, row.consumerAcceptance,
        row.nativeReadBack, row.paleSettlement].map(item => item.evidenceRef)).size, 5);
      const ends = topology.endpoints(relation); assert.equal(ends.from.kind, "included"); assert.equal(ends.to.kind, "included");
      assert.equal(ends.from.appearances.length, 1); assert.equal(ends.to.appearances.length, 1);
      assert.deepEqual(ends.from.endpoint.scope, row.contract.providerScope);
      assert.deepEqual(ends.to.endpoint.scope, row.contract.consumerScope);
      return { relation, source: ends.from.appearances[0], target: ends.to.appearances[0] };
    });
    assert.equal(topology.appearances.length, 2); assert.equal(routes[0].source, routes[1].target);
    const assembly = topology.geometry(topology.appearances.map(appearance => ({ appearance, width: 80, height: 40 })),
      routes, { nodes: 2, ports: 4, relations: 2, points: 1000, depth: 1, coordinateMagnitude: 10000 });
    compiler = new OrganizationSceneCompiler(intake); compiler.select(topology, assembly);
    const result = await compiler.layout(executor); assert.equal(result.kind, "completed");
    assert.equal(result.scene.geometry.routes.length, 2); assert.deepEqual(result.scene.declaredGates, []);
    assert(result.scene.assembly.routes.every(binding => binding.witness.kind === "service"
      && selected.occurrences.includes(binding.witness.occurrence)));
    assert.equal(enumeration.mock.callCount(), 0); compiler.close(); intake.close();
    assert(result.scene.topology.entry(selected.occurrences[0]).handle.belongsTo(owner));
  } finally { enumeration.mock.restore(); compiler?.close(); intake.close(); await executor.close(); }
});

test("incident service paging retains sparse cursors, reservation failures and closure custody", () => {
  const owner = compile(), one = { ...query, inspected: 1, returned: 1 };
  const intake = organizationIntake(owner, one, limits), whole = organizationIntake(owner, { ...query, inspected: 256, returned: 256 }, limits);
  const scope = { namespace: "alpha", scopeId: "scope" };
  try {
    const enumeration = intake.next(); assert.equal(enumeration.kind, "page"); const enumerationCursor = enumeration.page.continuation();
    const small = selectedPages(intake, "incidentServices", scope), large = selectedPages(whole, "incidentServices", scope);
    assert.deepEqual(small.occurrences.map(item => intake.entry(item).row), large.occurrences.map(item => whole.entry(item).row));
    assert.equal(small.pages.reduce((sum, page) => sum + page.inspected, 0), 2);
    const cursor = small.pages[0].continuation(); assert(cursor);
    const incident = intake.incidentRelations(alphaRole); assert.equal(incident.kind, "page"); const incidentCursor = incident.page.continuation(); assert(incidentCursor);
    for (const call of [() => intake.incidentServices(scope, enumerationCursor), () => intake.incidentServices(scope, incidentCursor),
      () => intake.incidentRelations(alphaRole, cursor), () => intake.endpoint(alphaRole, cursor),
      () => intake.incidentServices({ ...scope, namespace: "beta" }, cursor)]) {
      assert.throws(call, error => error.diagnostic.code === "workspace_organization_custody_mismatch");
    }
    assert.equal(intake.incidentServices(scope, cursor).kind, "page");
    const sparse = selectedPages(intake, "incidentServices", { namespace: "gamma", scopeId: "scope" });
    assert.equal(sparse.occurrences.length, 0); assert.equal(sparse.pages[0].isComplete, false);
    assert(sparse.pages.every(page => page.endpointLookup === null)); assert.equal(sparse.pages.at(-1).isComplete, true);
    const continued = intake.next(); assert.equal(continued.kind, "page");
    assert.deepEqual(continued.page.entries().map(entry => entry.row), owner.inspectOrganization(1, 1, one.copiedStringBytes, enumerationCursor).entries().map(entry => entry.row));
  } finally { intake.close(); whole.close(); }
  for (const resource of ["recordVisits", "records", "occurrences", "copiedStringBytes"]) {
    const stopped = organizationIntake(owner, query, { ...limits, [resource]: 1 });
    assert.deepEqual(stopped.incidentServices(null), { kind: "resource-stop", resource }); stopped.close();
  }
  const failed = organizationIntake(owner, { ...one, copiedStringBytes: 1 }, limits);
  try {
    assert.throws(() => failed.incidentServices(scope), error => error.diagnostic.code === "workspace_organization_copy_budget");
    assert.equal(failed.charges.records, 0); assert.equal(failed.charges.occurrences, 0);
    assert.equal(failed.charges.recordVisits, 1); assert.equal(failed.charges.copiedStringBytes, 1);
  } finally { failed.close(); }
  const closed = organizationIntake(owner, one, limits);
  const closing = { get namespace() { closed.close(); return "alpha"; }, scopeId: "scope" };
  assert.deepEqual(closed.incidentServices(closing), { kind: "closed" });
  assert.equal(closed.charges.records, 0); assert.equal(closed.charges.occurrences, 0);
});

test("empty selected pages distinguish continuing search, complete miss, external reference and unselected input", () => {
  const owner = compile(), intake = organizationIntake(owner, { ...query, inspected: 1, returned: 1 }, limits);
  try {
    const missing = { kind: "role", role: { namespace: "alpha", roleId: "missing" } };
    const first = intake.endpoint(missing); assert.equal(first.kind, "page");
    assert.equal(first.occurrences.length, 0); assert.equal(first.page.isComplete, false);
    assert.equal(first.page.endpointLookup, OrganizationEndpointLookup.SearchIncomplete); assert(first.page.organizationSelected);
    const searched = selectedPages(intake, "endpoint", missing);
    assert.equal(searched.occurrences.length, 0); assert.equal(searched.pages.at(-1).endpointLookup, OrganizationEndpointLookup.NotFound);
    const found = selectedPages(intake, "endpoint", betaScope);
    assert.equal(found.pages[0].endpointLookup, OrganizationEndpointLookup.SearchIncomplete);
    assert.equal(found.pages[0].entries().length, 0); assert.equal(found.pages.at(-1).endpointLookup, OrganizationEndpointLookup.Found);
    const external = { kind: "native-resource", nativeResource: "native.unobserved" };
    const referenced = intake.endpoint(external); assert.equal(referenced.kind, "page");
    assert.equal(referenced.page.endpointLookup, OrganizationEndpointLookup.ExternalReference);
    assert.equal(referenced.page.inspected, 0); assert.equal(referenced.occurrences.length, 0);
    assert.equal(referenced.page.isComplete, true);
    const incident = selectedPages(intake, "incidentRelations", external);
    assert.equal(incident.occurrences.length, 0); assert(incident.pages.every(page => page.endpointLookup === null));
    const unselected = compile(documents => documents.map(document => {
      if (document.label !== "pale-workspace.toml") return document;
      const source = document.bytes.toString();
      const changed = source.replace('organizational_translations = ["organization.toml"]', 'organizational_translations = []')
        .replace('responsibility_derivations = ["responsibility.toml"]', 'responsibility_derivations = []');
      assert.notEqual(changed, source); return { ...document, bytes: Buffer.from(changed) };
    }));
    const absent = organizationIntake(unselected, query, limits);
    try {
      const page = absent.endpoint(alphaRole); assert.equal(page.kind, "page");
      assert.equal(page.page.organizationSelected, false); assert.equal(page.page.endpointLookup, OrganizationEndpointLookup.NotFound);
      assert.equal(page.page.inspected, 0); assert.equal(page.occurrences.length, 0);
    } finally { absent.close(); }
  } finally { intake.close(); }
});

test("selected query cursor mismatches preserve diagnostics and the original continuation", () => {
  const owner = compile(), one = { ...query, inspected: 1, returned: 1 };
  const intake = organizationIntake(owner, one, limits), foreign = organizationIntake(compile(), one, limits);
  const unlike = [organizationIntake(owner, { ...one, inspected: 2 }, limits),
    organizationIntake(owner, { ...one, returned: 2 }, limits),
    organizationIntake(owner, { ...one, copiedStringBytes: one.copiedStringBytes - 1 }, limits)];
  try {
    const first = intake.endpoint(alphaRole); assert.equal(first.kind, "page");
    const cursor = first.page.continuation(); assert(cursor);
    const wrong = { kind: "role", role: { namespace: "beta", roleId: "accountable-a" } };
    for (const call of [() => intake.endpoint(wrong, cursor), () => intake.incidentRelations(alphaRole, cursor),
      () => foreign.endpoint(alphaRole, cursor), ...unlike.map(value => () => value.endpoint(alphaRole, cursor))]) {
      assert.throws(call, error => error.diagnostic.code === "workspace_organization_custody_mismatch");
    }
    const continued = intake.endpoint(alphaRole, cursor); assert.equal(continued.kind, "page");
    assert(continued.page.belongsTo(owner));
    const expected = owner.inspectOrganizationEndpoint(alphaRole, 1, 1, one.copiedStringBytes, cursor);
    assert.deepEqual(continued.occurrences.map(item => intake.entry(item).row), expected.entries().map(entry => entry.row));
    const incident = intake.incidentRelations(alphaRole); assert.equal(incident.kind, "page");
    const incidentCursor = incident.page.continuation(); assert(incidentCursor);
    assert.throws(() => intake.endpoint(alphaRole, incidentCursor), error => error.diagnostic.code === "workspace_organization_custody_mismatch");
    assert.equal(intake.incidentRelations(alphaRole, incidentCursor).kind, "page");
  } finally { intake.close(); foreign.close(); unlike.forEach(value => value.close()); }
});

test("selected queries reserve before dispatch and keep failed SDK copy charges without enrolling records", () => {
  const owner = compile();
  for (const method of ["endpoint", "incidentRelations"]) {
    for (const resource of ["recordVisits", "records", "occurrences", "copiedStringBytes"]) {
      const stopped = organizationIntake(owner, query, { ...limits, [resource]: 1 });
      try {
        const before = stopped.charges;
        assert.deepEqual(stopped[method](null), { kind: "resource-stop", resource });
        assert.deepEqual(stopped.charges, before);
      } finally { stopped.close(); }
    }
    const limited = organizationIntake(owner, query, { ...limits, queries: 1 });
    try {
      assert.equal(limited.endpoint({ kind: "native-resource", nativeResource: "native.unobserved" }).kind, "page");
      assert.deepEqual(limited[method](null), { kind: "resource-stop", resource: "queries" });
      limited.close(); assert.deepEqual(limited[method](null), { kind: "closed" });
    } finally { limited.close(); }
    const failed = organizationIntake(owner, { inspected: 256, returned: 256, copiedStringBytes: 1 }, limits);
    try {
      const sdkMethod = method === "endpoint" ? "inspectOrganizationEndpoint" : "inspectOrganizationIncidentRelations";
      let original; try { owner[sdkMethod](alphaRole, 256, 256, 1); } catch (error) { original = error; }
      assert.equal(original?.diagnostic.code, "workspace_organization_copy_budget");
      assert.throws(() => failed[method](alphaRole), error => error.diagnostic.code === original.diagnostic.code);
      assert.equal(failed.charges.queries, 1); assert.equal(failed.charges.recordVisits, 256);
      assert.equal(failed.charges.copiedStringBytes, 1); assert.equal(failed.charges.records, 0); assert.equal(failed.charges.occurrences, 0);
      assert.equal(failed.endpoint({ kind: "native-resource", nativeResource: "native.unobserved" }).kind, "page");
    } finally { failed.close(); }
  }
});

test("queried records retain independent occurrence reasons and original source and detail custody", () => {
  const owner = compile(), intake = organizationIntake(owner, query, limits);
  try {
    const a = selectedPages(intake, "endpoint", alphaRole).occurrences[0]; assert(a);
    const b = selectedPages(intake, "endpoint", alphaRole).occurrences[0]; assert(b);
    assert.notEqual(a, b); assert.notEqual(a.id, b.id); assert(intake.entry(a).handle.sameRecord(intake.entry(b).handle));
    assert.equal(a.inclusion, "endpoint-query"); assert.equal(b.inclusion, "endpoint-query");
    const repeat = intake.repeat(a); assert.equal(repeat.inclusion, "additional-occurrence");
    assert.equal(intake.entry(repeat), intake.entry(a));
    assert.equal(intake.sources(a).mapping.kind, "named");
    const role = intake.entry(a).handle;
    assert.equal(role.axisResources(256, 256, query.copiedStringBytes).rows.kind, "axis-resources");
    const occupant = selectedPages(intake, "endpoint", { kind: "occupant", occupant: "occupant.alpha" }).occurrences[0]; assert(occupant);
    assert.equal(intake.sources(occupant).mapping.kind, "unmapped-record");
    const binding = intake.entry(occupant).handle;
    intake.close(); assert(role.belongsTo(owner) && binding.belongsTo(owner));
    assert.equal(role.sources(256, 256, query.copiedStringBytes).mapping.kind, "named");
  } finally { intake.close(); }
});

test("actual endpoint and incident queries build selected topology and a real scene without enumeration", async () => {
  const owner = compile(), intake = organizationIntake(owner, query, limits);
  const enumeration = mock.method(owner, "inspectOrganization", () => assert.fail("Selected expansion must not enumerate the workspace"));
  const executor = new ElkExecutor({ requestBytes: 1e6, resultBytes: 1e6, deadlineMs: 10000,
    totalRequestBytes: 10e6, totalResultBytes: 10e6, totalActiveMs: 60000, oldHeapMb: 128, youngHeapMb: 32 });
  let compiler;
  try {
    const incident = selectedPages(intake, "incidentRelations", alphaRole);
    const relation = incident.occurrences.find(item => intake.entry(item).row.relationId === "relation.collaboration"); assert(relation);
    const row = intake.entry(relation).row; assert.equal(row.kind, "relation");
    const from = selectedPages(intake, "endpoint", row.from), to = selectedPages(intake, "endpoint", row.to);
    assert.equal(from.occurrences.length, 1); assert.equal(to.occurrences.length, 1);
    const selected = [relation, ...from.occurrences, ...to.occurrences];
    const topology = new OrganizationTopology(intake, selected,
      { records: 10, appearances: 10, routeAppearances: 10, identityCodeUnits: 10000 });
    const endpoints = topology.endpoints(relation); assert.equal(endpoints.from.kind, "included"); assert.equal(endpoints.to.kind, "included");
    const source = endpoints.from.appearances[0], target = endpoints.to.appearances[0];
    const assembly = topology.geometry([source, target].map(appearance => ({ appearance, width: 80, height: 40 })),
      [{ relation, source, target }], { nodes: 2, ports: 2, relations: 1, points: 1000, depth: 1, coordinateMagnitude: 10000 });
    compiler = new OrganizationSceneCompiler(intake); compiler.select(topology, assembly);
    const result = await compiler.layout(executor); assert.equal(result.kind, "completed");
    assert.equal(result.scene.topology.records.length, 3); assert.equal(result.scene.geometry.routes.length, 1);
    assert.equal(result.scene.assembly.routes[0].witness.occurrence, relation);
    assert.equal(relation.inclusion, "incident-relation-query");
    assert.deepEqual(result.scene.declaredGates, []);
    assert.equal(enumeration.mock.callCount(), 0);
    compiler.close(); intake.close(); assert(result.scene.topology.entry(relation).handle.belongsTo(owner));
  } finally { enumeration.mock.restore(); compiler?.close(); intake.close(); await executor.close(); }
});

test("real selector getters cannot enroll a page after closing its intake", () => {
  for (const method of ["endpoint", "incidentRelations"]) {
    const intake = organizationIntake(compile(), query, limits); let calls = 0;
    const endpoint = { kind: "role", role: { get namespace() {
      calls++; intake.close(); return "alpha";
    }, roleId: "accountable-a" } };
    try {
      assert.deepEqual(intake[method](endpoint), { kind: "closed" });
      assert.equal(calls, 1); assert.equal(intake.charges.records, 0); assert.equal(intake.charges.occurrences, 0);
      assert.equal(intake.charges.queries, 1); assert.equal(intake.charges.recordVisits, query.inspected);
      assert.equal(intake.charges.copiedStringBytes, query.copiedStringBytes);
      assert.deepEqual(intake.next(), { kind: "closed" });
    } finally { intake.close(); }
  }
});

test("real nested selector reads share retention capacity and preserve distinct appearance IDs", () => {
  const owner = compile(), one = { ...query, inspected: 256, returned: 1 };
  for (const capacity of [1, 2]) {
    const intake = organizationIntake(owner, one, { ...limits, records: capacity, occurrences: capacity });
    let nested, calls = 0;
    const endpoint = { kind: "role", role: { get namespace() {
      calls++; nested = intake.endpoint(alphaRole); return "alpha";
    }, roleId: "accountable-a" } };
    try {
      const outer = intake.endpoint(endpoint); assert.equal(outer.kind, "page"); assert.equal(calls, 1);
      assert.equal(outer.occurrences.length, 1);
      if (capacity === 1) {
        assert.deepEqual(nested, { kind: "resource-stop", resource: "records" });
        assert.equal(intake.charges.queries, 1);
      } else {
        assert.equal(nested.kind, "page"); assert.equal(nested.occurrences.length, 1);
        assert.notEqual(outer.occurrences[0].id, nested.occurrences[0].id);
        assert.notEqual(outer.occurrences[0], nested.occurrences[0]);
        assert(intake.entry(outer.occurrences[0]).handle.sameRecord(intake.entry(nested.occurrences[0]).handle));
        assert.equal(intake.charges.queries, 2);
      }
      assert.equal(intake.charges.records, capacity); assert.equal(intake.charges.occurrences, capacity);
    } finally { intake.close(); }
  }
  const intake = organizationIntake(owner, one, { ...limits, records: 2, occurrences: 2 });
  try {
    const first = intake.endpoint(alphaRole); assert.equal(first.kind, "page");
    let repeated;
    const endpoint = { kind: "role", role: { get namespace() {
      repeated = intake.repeat(first.occurrences[0]); return "alpha";
    }, roleId: "accountable-a" } };
    const second = intake.endpoint(endpoint); assert.equal(second.kind, "page");
    assert.deepEqual(repeated, { kind: "resource-stop", resource: "occurrences" });
    assert.equal(intake.charges.occurrences, 2);
    assert.notEqual(first.occurrences[0].id, second.occurrences[0].id);
  } finally { intake.close(); }
});

test("a throwing real selector releases active retention but keeps unknown-work charges", () => {
  const one = { ...query, inspected: 256, returned: 1 };
  const intake = organizationIntake(compile(), one, { ...limits, records: 1, occurrences: 1 });
  let calls = 0;
  try {
    assert.throws(() => intake.endpoint({ kind: "role", role: { get namespace() {
      calls++; throw new Error("fixture selector failed");
    }, roleId: "accountable-a" } }));
    assert.equal(calls, 1); assert.equal(intake.charges.records, 0); assert.equal(intake.charges.occurrences, 0);
    assert.equal(intake.charges.queries, 1); assert.equal(intake.charges.recordVisits, one.inspected);
    assert.equal(intake.charges.copiedStringBytes, one.copiedStringBytes);
    const retry = intake.endpoint(alphaRole); assert.equal(retry.kind, "page");
    assert.equal(retry.occurrences.length, 1); assert.equal(retry.occurrences[0].id, "o0");
    assert.equal(intake.charges.records, 1); assert.equal(intake.charges.occurrences, 1);
  } finally { intake.close(); }
});

test("real organization pages retain all eighteen SDK families and exact handles", () => {
  const owner = compile(), intake = organizationIntake(owner, query, limits);
  try {
    const occurrences = collect(intake);
    const entries = occurrences.map(occurrence => intake.entry(occurrence));
    assert.deepEqual([...new Set(entries.map(entry => entry.row.kind))].sort(), [
      "scope", "organization", "group", "discipline-tree", "role", "occupant-binding", "provider-binding",
      "communication-actor", "leadership", "relation", "object", "population-rule", "group-lens", "service",
      "native-projection", "inspection", "derived-relation", "responsibility",
    ].sort());
    const direct = []; let after;
    for (let page = 0; page < limits.queries; page++) {
      const found = owner.inspectOrganization(query.inspected, query.returned, query.copiedStringBytes, after);
      direct.push(...found.entries()); after = found.continuation(); if (after === null) break;
    }
    assert.deepEqual(entries.map(entry => entry.row), direct.map(entry => entry.row));
    assert(entries.every(entry => entry.handle.belongsTo(owner)));
    assert.equal(new Set(occurrences.map(occurrence => occurrence.id)).size, occurrences.length);
    assert.deepEqual(intake.next(), { kind: "enumeration-exhausted" });
  } finally { intake.close(); }
});

test("appearance IDs are not semantic keys or custody tokens", () => {
  const owner = compile(), first = organizationIntake(owner, query, limits);
  const second = organizationIntake(owner, query, limits);
  try {
    const a = first.next().occurrences[0], b = second.next().occurrences[0]; assert(a && b);
    assert.equal(a.id, b.id);
    assert.throws(() => first.entry(b), error => error instanceof OrganizationIntakeError && error.kind === "foreign-occurrence");
    assert.throws(() => first.entry({ ...a }), error => error.kind === "foreign-occurrence");
    const repeated = first.repeat(a);
    assert.equal(repeated.inclusion, "additional-occurrence"); assert.notEqual(repeated.id, a.id);
    assert.equal(first.entry(repeated), first.entry(a));
    const retained = first.entry(a).handle;
    first.close(); assert(retained.belongsTo(owner));
    assert.deepEqual(first.next(), { kind: "closed" });
    assert.throws(() => first.entry(a), error => error.kind === "closed");
  } finally { first.close(); second.close(); }
});

test("whole-query reservations stop without changing SDK limits or inventing completeness", () => {
  const intake = organizationIntake(compile(), query, { ...limits, recordVisits: 1 });
  assert.deepEqual(intake.next(), { kind: "resource-stop", resource: "recordVisits" });
  assert.equal(intake.charges.queries, 0); intake.close();
  const oneQuery = organizationIntake(compile(), query, { ...limits, queries: 1 });
  assert.equal(oneQuery.next().kind, "page");
  assert.deepEqual(oneQuery.next(), { kind: "resource-stop", resource: "queries" });
  oneQuery.close();
});

test("sources retain named paths, roles and continuation from the real record owner", () => {
  const owner = compile(), intake = organizationIntake(owner, query, limits);
  try {
    const occurrence = collect(intake).find(occurrence => intake.entry(occurrence).row.kind === "role"); assert(occurrence);
    const selected = intake.entry(occurrence);
    let after, matched = 0;
    for (let page = 0; page < 1000; page++) {
      const step = intake.sources(occurrence, after); assert.equal(step.kind, "page");
      assert(step.page.record.sameRecord(selected.handle));
      assert(step.page.record.belongsTo(owner));
      const expectedPage = selected.handle.sources(query.inspected, query.returned, query.copiedStringBytes, after);
      const expected = expectedPage.mapping;
      assert.equal(step.page.copiedStringBytes, expectedPage.copiedStringBytes);
      assert.equal(step.mapping.kind, expected.kind);
      assert.equal(step.mapping.kind, "named");
      assert.deepEqual(step.mapping.occurrences, expected.occurrences);
      matched += step.mapping.occurrences.length;
      assert.equal(step.mapping.inspected, expected.inspected);
      assert.equal(step.mapping.continuation === null, expected.continuation === null);
      after = step.mapping.continuation; if (after === null) { assert(matched > 0); return; }
    }
    assert.fail("The bounded source fixture did not exhaust");
  } finally { intake.close(); }
});

test("unmapped record evidence remains distinct from an exhausted named source scan", () => {
  const intake = organizationIntake(compile(), query, limits);
  try {
    const occurrence = collect(intake).find(occurrence => intake.entry(occurrence).row.kind === "occupant-binding"); assert(occurrence);
    const step = intake.sources(occurrence);
    assert.equal(step.kind, "page"); assert.deepEqual(step.mapping, { kind: "unmapped-record" });
  } finally { intake.close(); }
});

test("SDK errors retain conservative work charges and their canonical diagnostics", () => {
  const owner = compile(), intake = organizationIntake(owner, { ...query, copiedStringBytes: 1 }, limits);
  let expected;
  try { owner.inspectOrganization(query.inspected, query.returned, 1); } catch (error) { expected = error; }
  assert(expected?.diagnostic);
  assert.throws(() => intake.next(), error => error.diagnostic.code === expected.diagnostic.code);
  assert.equal(intake.charges.queries, 1); assert.equal(intake.charges.recordVisits, query.inspected);
  assert.equal(intake.charges.copiedStringBytes, 1); intake.close();
});

const replaceOrganization = (from, to) => documents => documents.map(document => {
  if (document.label !== "organization.toml") return document;
  const source = document.bytes.toString(), changed = source.replace(from, to);
  assert.notEqual(source, changed, "actual fixture mutation must apply");
  return { ...document, bytes: Buffer.from(changed) };
});
const priorSide = OrganizationComparisonSide.Prior;
const one = { ...query, inspected: 1, returned: 1 };
function finishCorrespondence(intake, comparison, side, occurrence) {
  let after;
  for (let count = 0; count < limits.queries; count++) {
    const step = intake.correspond(comparison, side, occurrence, after);
    assert.equal(step.kind, "result");
    if (step.result.state.kind !== "searching") return step.result;
    after = step.result.state.cursor;
  }
  assert.fail("bounded correspondence did not exhaust");
}

test("correspondence returns actual SDK states and refunds work without enrolling rows", () => {
  const prior = compile(), current = compile();
  const pair = retainedComparison(new Pale().comparePortableWorkspaces(prior, current));
  const intake = organizationIntake(prior, one, limits);
  try {
    const occurrences = collect(intake), scopes = occurrences.filter(value => intake.entry(value).row.kind === "scope");
    assert(scopes.length >= 2);
    const selected = scopes[1], before = intake.charges;
    const first = intake.correspond(pair, priorSide, selected);
    assert.equal(first.kind, "result"); assert.equal(first.result.state.kind, "searching");
    assert(first.result.belongsTo(pair));
    assert(first.result.selected.sameRecord(intake.entry(selected).handle));
    assert.equal(first.result.inspected, 1); assert.equal(first.result.copiedStringBytes, 0);
    assert.equal(intake.charges.queries, before.queries + 1);
    assert.equal(intake.charges.recordVisits, before.recordVisits + 1);
    assert.equal(intake.charges.copiedStringBytes, before.copiedStringBytes);
    const next = intake.correspond(pair, priorSide, selected, first.result.state.cursor);
    assert.equal(next.kind, "result"); assert.equal(next.result.state.kind, "matched");
    assert.equal(next.result.state.values, WorkspaceValueComparison.Equal);
    assert(next.result.state.other.handle.belongsTo(current));
    assert.equal(intake.charges.records, before.records);
    assert.equal(intake.charges.occurrences, before.occurrences);
    assert.equal(intake.charges.sourceVisits, before.sourceVisits);
    assert.equal(intake.charges.copiedStringBytes, before.copiedStringBytes + next.result.copiedStringBytes);
    const retained = next.result; intake.close();
    assert(retained.selected.belongsTo(prior)); assert(retained.state.other.handle.belongsTo(current));
    assert.deepEqual(intake.correspond(pair, priorSide, selected), { kind: "closed" });
  } finally { intake.close(); }
});

test("SDK pair side record cursor and raw-limit rejection leaves original continuation usable", () => {
  const prior = compile(), current = compile(), duplicate = compile(), api = new Pale();
  const pair = retainedComparison(api.comparePortableWorkspaces(prior, current));
  const otherPair = retainedComparison(api.comparePortableWorkspaces(prior, duplicate));
  const reversed = retainedComparison(api.comparePortableWorkspaces(current, prior));
  const intake = organizationIntake(prior, one, limits);
  const unlikeLimits = organizationIntake(prior, { ...one, inspected: 2 }, limits);
  const unlikeCopy = organizationIntake(prior, { ...one, copiedStringBytes: one.copiedStringBytes - 1 }, limits);
  try {
    const scopes = collect(intake).filter(value => intake.entry(value).row.kind === "scope");
    const selected = scopes.at(-1), first = intake.correspond(pair, priorSide, selected);
    assert.equal(first.result.state.kind, "searching"); const cursor = first.result.state.cursor;
    const changed = collect(unlikeLimits).find(value => unlikeLimits.entry(value).handle.sameRecord(intake.entry(selected).handle));
    const changedCopy = collect(unlikeCopy).find(value => unlikeCopy.entry(value).handle.sameRecord(intake.entry(selected).handle));
    assert(changed);
    assert(changedCopy);
    for (const call of [
      () => intake.correspond(otherPair, priorSide, selected, cursor),
      () => intake.correspond(reversed, OrganizationComparisonSide.Current, selected, cursor),
      () => intake.correspond(pair, OrganizationComparisonSide.Current, selected, cursor),
      () => intake.correspond(pair, priorSide, scopes[0], cursor),
      () => unlikeLimits.correspond(pair, priorSide, changed, cursor),
      () => unlikeCopy.correspond(pair, priorSide, changedCopy, cursor),
    ]) assert.throws(call, error => error.diagnostic.code === "workspace_organization_custody_mismatch");
    const before = intake.charges;
    assert.throws(() => intake.correspond(pair, priorSide, { ...selected }, cursor), error => error.kind === "foreign-occurrence");
    assert.deepEqual(intake.charges, before, "foreign local token rejects before reservation");
    assert.throws(() => intake.correspond(pair, priorSide, changed, cursor), error => error.kind === "foreign-occurrence");
    assert.equal(intake.correspond(pair, priorSide, selected, cursor).kind, "result");
    const currentIntake = organizationIntake(current, one, limits);
    try {
      const selectedCurrent = currentIntake.next().occurrences[0];
      assert.equal(finishCorrespondence(currentIntake, pair, OrganizationComparisonSide.Current, selectedCurrent).state.kind, "matched");
    } finally { currentIntake.close(); }
  } finally { intake.close(); unlikeLimits.close(); unlikeCopy.close(); }
});

test("correspondence keeps conservative failed reservations and stops before dispatch", () => {
  const prior = compile(), current = compile();
  const pair = retainedComparison(new Pale().comparePortableWorkspaces(prior, current));
  for (const resource of ["queries", "recordVisits", "copiedStringBytes"]) {
    const maximum = resource === "copiedStringBytes" ? one.copiedStringBytes : 1;
    const intake = organizationIntake(prior, one, { ...limits, [resource]: maximum });
    try {
      const selected = intake.next().occurrences[0], before = intake.charges;
      // An invalid comparison would throw if reservation did not stop dispatch.
      assert.deepEqual(intake.correspond(null, priorSide, selected), { kind: "resource-stop", resource });
      assert.deepEqual(intake.charges, before);
    } finally { intake.close(); }
  }
  const exact = organizationIntake(prior, one, { ...limits, queries: 2 });
  try {
    const selected = exact.next().occurrences[0];
    assert.equal(exact.correspond(pair, priorSide, selected).kind, "result");
    assert.equal(exact.charges.queries, 2);
    assert.deepEqual(exact.correspond(null, priorSide, selected), { kind: "resource-stop", resource: "queries" });
  } finally { exact.close(); }
  const intake = organizationIntake(prior, one, limits);
  try {
    const selected = intake.next().occurrences[0], before = intake.charges;
    for (let attempt = 1; attempt <= 2; attempt++) {
      assert.throws(() => intake.correspond(pair, OrganizationComparisonSide.Current, selected),
        error => error.diagnostic.code === "workspace_organization_custody_mismatch");
      assert.equal(intake.charges.queries, before.queries + attempt);
      assert.equal(intake.charges.recordVisits, before.recordVisits + attempt * one.inspected);
      assert.equal(intake.charges.copiedStringBytes, before.copiedStringBytes + attempt * one.copiedStringBytes);
    }
    assert.equal(intake.correspond(pair, priorSide, selected).result.state.kind, "matched");
  } finally { intake.close(); }
});

test("copy-budget rejection returns no result and leaves the selected handle reusable", () => {
  const prior = compile(), current = compile(replaceOrganization('controller = "controller.alpha-scope"',
    `controller = "controller.${"a".repeat(100)}"`));
  const api = new Pale(), pair = retainedComparison(api.comparePortableWorkspaces(prior, current));
  const budget = prior.inspectOrganization(1, 1, one.copiedStringBytes).copiedStringBytes;
  assert(budget > 0 && budget < one.copiedStringBytes);
  const intake = organizationIntake(prior, { ...one, copiedStringBytes: budget }, limits);
  try {
    const selected = intake.next().occurrences[0], original = intake.entry(selected).handle;
    const before = intake.charges;
    for (let attempt = 1; attempt <= 2; attempt++) {
      assert.throws(() => intake.correspond(pair, priorSide, selected),
        error => error.diagnostic.code === "workspace_organization_copy_budget");
      assert.equal(intake.charges.queries, before.queries + attempt);
      assert.equal(intake.charges.recordVisits, before.recordVisits + attempt);
      assert.equal(intake.charges.copiedStringBytes, before.copiedStringBytes + attempt * budget);
      assert.equal(intake.charges.records, before.records);
      assert.equal(intake.charges.occurrences, before.occurrences);
      assert(intake.entry(selected).handle.sameRecord(original));
    }
    const equalPair = retainedComparison(api.comparePortableWorkspaces(prior, compile()));
    assert.equal(intake.correspond(equalPair, priorSide, selected).result.state.kind, "matched");
  } finally { intake.close(); }
});

test("searching and terminal unmatched retain original binding source meaning", () => {
  const prior = compile(), current = compile(replaceOrganization(
    "[occupant_bindings.interval]\nstart = 1", "[occupant_bindings.interval]\nstart = 2"));
  const pair = retainedComparison(new Pale().comparePortableWorkspaces(prior, current));
  for (const [owner, side] of [[prior, priorSide], [current, OrganizationComparisonSide.Current]]) {
    const intake = organizationIntake(owner, one, limits);
    try {
      const selected = collect(intake).find(value => intake.entry(value).row.kind === "occupant-binding");
      const first = intake.correspond(pair, side, selected);
      assert.equal(first.result.state.kind, "searching"); assert.equal(first.result.inspected, 1);
      const before = intake.charges;
      const terminal = intake.correspond(pair, side, selected, first.result.state.cursor);
      assert.equal(terminal.result.state.kind, "unmatched"); assert.equal(terminal.result.inspected, 0);
      assert.equal(intake.charges.recordVisits, before.recordVisits);
      assert.equal(intake.charges.copiedStringBytes, before.copiedStringBytes);
      assert.equal(terminal.result.selected.sources(1, 1, one.copiedStringBytes).mapping.kind, "unmapped-record");
    } finally { intake.close(); }
  }
});

test("Changed nested values are SDK evidence even when shallow rows are equal", () => {
  const prior = compile(), current = compile(replaceOrganization(
    'resources = ["resource.role-a"]', 'resources = ["resource.changed"]'));
  const pair = retainedComparison(new Pale().comparePortableWorkspaces(prior, current));
  const intake = organizationIntake(prior, one, limits);
  try {
    let changed = 0;
    for (const selected of collect(intake).filter(value => intake.entry(value).row.kind === "role")) {
      const result = finishCorrespondence(intake, pair, priorSide, selected);
      assert.equal(result.state.kind, "matched");
      if (result.state.values === WorkspaceValueComparison.Changed) {
        changed++;
        assert.deepEqual(intake.entry(selected).row, result.state.other.row);
        assert.notDeepEqual(result.selected.axisResources(256, 256, one.copiedStringBytes).rows,
          result.state.other.handle.axisResources(256, 256, one.copiedStringBytes).rows);
      }
    }
    assert.equal(changed, 1);
  } finally { intake.close(); }
});

test("runtime-private custody cannot be overwritten by shadow properties or enrollment hooks", () => {
  const owner = compile(), current = compile();
  const pair = retainedComparison(new Pale().comparePortableWorkspaces(owner, current));
  const intake = organizationIntake(owner, one, limits), foreignIntake = organizationIntake(owner, one, limits);
  try {
    const selected = intake.next().occurrences[0], foreign = foreignIntake.next().occurrences[0];
    const entry = intake.entry(selected), charges = intake.charges;
    intake.state = { kind: "open", owner: current, position: { kind: "initial" } };
    intake.query = { inspected: 256, returned: 256, copiedStringBytes: 1 };
    intake.limits = { ...limits, queries: 0 }; intake.used = { queries: -100 };
    intake.entries = new Map([[foreign, entry]]);
    intake.include = () => foreign; intake.shortage = () => undefined;
    intake.entry = () => entry;
    assert.throws(() => intake.correspond(pair, priorSide, foreign), error => error.kind === "foreign-occurrence");
    assert.deepEqual(intake.charges, charges);
    const result = intake.correspond(pair, priorSide, selected).result;
    assert(result.selected.sameRecord(entry.handle));
    const repeated = intake.repeat(selected); assert.notEqual(repeated, foreign);
    assert.equal(intake.charges.occurrences, charges.occurrences + 1);
    intake.close(); intake.state = { kind: "open", owner };
    assert.deepEqual(intake.correspond(null, priorSide, foreign), { kind: "closed" });
    assert.deepEqual(intake.next(), { kind: "closed" });
  } finally { intake.close(); foreignIntake.close(); }
});

test("declared limits are read once and validated as the exact stored snapshot", () => {
  const owner = compile(), reads = new Map();
  const accessors = (name, values) => {
    const input = {};
    for (const [key, value] of Object.entries(values)) {
      const label = `${name}.${key}`;
      Object.defineProperty(input, key, { enumerable: true, get() {
        const count = (reads.get(label) ?? 0) + 1;
        reads.set(label, count);
        return count === 1 ? value : NaN;
      } });
    }
    Object.defineProperty(input, "unrelated", { enumerable: true, get() {
      assert.fail("Only declared limit fields may be read");
    } });
    return input;
  };
  const intake = organizationIntake(owner, accessors("query", one), accessors("limits", limits));
  try {
    assert.equal(reads.size, Object.keys(one).length + Object.keys(limits).length);
    assert([...reads.values()].every(count => count === 1));
    assert.equal(intake.next().kind, "page");
    assert.equal(intake.charges.queries, 1);
    assert([...reads.values()].every(count => count === 1));
  } finally { intake.close(); }
  let invalidReads = 0;
  const invalid = { ...limits, get queries() { invalidReads++; return NaN; } };
  assert.throws(() => organizationIntake(owner, one, invalid), RangeError);
  assert.equal(invalidReads, 1);
});
