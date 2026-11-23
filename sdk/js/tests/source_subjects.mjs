// Copyright © 2026 RawThoughts Enterprises Private Limited. Apache-2.0.
import assert from "node:assert/strict";
import fs from "node:fs";

// These local fixtures were exported by the existing Rust test helper
// unadopted_documents(). This test neither seals locks nor supplies owner proof.
function unselectedRequest(renamed) {
  const derived = new URL("./fixtures/source-subjects/", import.meta.url);
  const alpha = renamed ? "renamed-alpha.toml" : "alpha.toml";
  return {
    rootManifest: "pale-workspace.toml",
    documents: [
      { label: "pale-workspace.toml", bytes: fs.readFileSync(new URL(renamed ? "renamed-root.toml" : "original-root.toml", derived)) },
      { label: "universe.lock.toml", bytes: fs.readFileSync(new URL("universe.lock.toml", derived)) },
      ...["module.toml", "alpha.toml", "beta.toml", "resolution.lock.toml"].map((label) => ({
        label: label === "alpha.toml" ? alpha : label,
        bytes: fs.readFileSync(new URL(label, derived)),
      })),
    ],
  };
}

function apiRejection(bindings, operation, code) {
  assert.throws(operation, (error) => {
    assert(error instanceof Error);
    assert.equal(error.diagnostic.code, code);
    assert.equal(error.message, error.diagnostic.summary);
    assert(Buffer.isBuffer(error.diagnostic.canonicalBytes));
    const canonical = JSON.parse(error.diagnostic.canonicalBytes);
    assert.equal(canonical.code, code);
    assert.equal(canonical.summary, error.diagnostic.summary);
    assert.equal(error.diagnostic.effect, bindings.DiagnosticEffectCertainty.NotApplicable);
    assert.equal(canonical.source, undefined);
    return true;
  });
}

export function exerciseSourceSubjects(bindings, api, request) {
  const { PortableWorkspaceSourceState: State, PortableWorkspaceSubjectMembership: Membership } = bindings;
  const field = (name) => bindings.AuthoringInstancePathSegment.field(name);
  const index = (value) => bindings.AuthoringInstancePathSegment.index(value);
  const read = (workspace, label, path = []) => api.portableWorkspaceSourceSubject(workspace, label, path);
  let workspace = api.compilePortableWorkspace(request());
  let rebuilt = api.compilePortableWorkspace(request());
  const root = request().rootManifest;
  const source = read(workspace, root);
  assert(source instanceof bindings.PortableWorkspaceSourceInspection);
  assert.equal(source.state, State.Selected);
  assert.equal(source.belongsTo(workspace), true);
  assert.equal(source.belongsTo(rebuilt), false);
  assert.equal(workspace.semanticIrDigest, rebuilt.semanticIrDigest);
  assert.deepEqual(source.location, { sourceLabel: root, path: [] });
  const occurrence = source.occurrence();
  assert.deepEqual(occurrence, { location: source.location, role: bindings.AuthoringOccurrenceRole.Definition });
  const subject = source.subject();
  assert(subject instanceof bindings.WorkspaceSubject);
  assert.equal(subject.kind, bindings.WorkspaceSubjectKind.Source);
  assert.equal(subject.sourceId, "workspace-manifest");
  assert.equal(subject.ownerModuleId, null);
  assert.equal(subject.declarationId, null);
  const originalLocation = source.location;
  source.location.path.push({ field: "caller mutation" });
  occurrence.location.sourceLabel = "caller mutation";
  assert.deepEqual(source.location, originalLocation);
  assert.equal(source.occurrence().location.sourceLabel, root);
  assert.equal(source.subject().sourceId, subject.sourceId);

  const unsupported = read(workspace, root, [field("workspace"), field("distribution_lock")]);
  assert.equal(unsupported.state, State.NoDenotation);
  const unsupportedNavigation = api.navigatePortableWorkspaceAuthoring(workspace, root,
    [field("workspace"), field("distribution_lock")]);
  assert.deepEqual(unsupported.occurrence(), {
    location: unsupported.location, role: unsupportedNavigation.occurrenceRole,
  });
  assert.equal(unsupported.subject(), null);
  const missing = read(workspace, root, [field("no_such_field")]);
  assert.equal(missing.state, State.NoOccurrence);
  assert.equal(missing.occurrence(), null);
  assert.equal(missing.subject(), null);
  assert.deepEqual(missing.location, { sourceLabel: root, path: [{ field: "no_such_field" }] });
  assert(missing.belongsTo(workspace));
  apiRejection(bindings, () => read(workspace, root, Array.from({ length: 65 }, () => index(0))), "authoring_path_too_deep");

  const subjectRows = (owner, target, inspected, returned, after) =>
    api.portableWorkspaceSubjectOccurrences(owner, target, inspected, returned, after);
  const identity = api.navigatePortableWorkspaceAuthoring(workspace, root, []).identity();
  const first = subjectRows(workspace, subject, 1, 1);
  assert(first instanceof bindings.PortableWorkspaceSubjectOccurrences);
  assert.equal(first.membership, Membership.Selected);
  const cursor = first.page().continuation();
  assert(cursor instanceof bindings.AuthoringOccurrencePageCursor);
  const identityFirst = api.portableWorkspaceAuthoringOccurrences(workspace, identity, 1, 1);
  const other = read(workspace, root, [field("workspace"), field("workspace_id")]).subject();
  assert(other);
  assert.equal(other.kind, bindings.WorkspaceSubjectKind.Workspace);
  assert.equal(other.workspaceId, workspace.workspaceId);
  const mismatch = (operation) => apiRejection(bindings, operation, "authoring_continuation_mismatch");
  for (const operation of [
    () => subjectRows(rebuilt, subject, 1, 1, cursor),
    () => subjectRows(workspace, other, 1, 1, cursor),
    () => subjectRows(workspace, subject, 2, 1, cursor),
    () => subjectRows(workspace, subject, 1, 2, cursor),
    () => subjectRows(workspace, subject, 1, 1, identityFirst.continuation()),
    () => api.portableWorkspaceAuthoringOccurrences(workspace, identity, 1, 1, cursor),
  ]) mismatch(operation);
  // A rejected request borrows the cursor; it does not consume or restart it.
  assert.equal(subjectRows(workspace, subject, 1, 1, cursor).membership, Membership.Selected);

  function collect(owner, target, inspected, returned, membership) {
    let after;
    const rows = [];
    let pages = 0;
    let emptyIncomplete = false;
    do {
      assert(++pages < 4096, "the actual fixture scan must finish");
      const result = subjectRows(owner, target, inspected, returned, after);
      assert.equal(result.membership, membership);
      const page = result.page();
      assert(page instanceof bindings.AuthoringOccurrencePage);
      assert(page.inspected <= Math.min(inspected || 64, 256));
      const current = page.occurrences();
      assert(current.length <= Math.min(returned || 64, 256));
      assert.deepEqual(result.page().occurrences(), current);
      rows.push(...current);
      after = page.continuation();
      assert.equal(page.isComplete, after === null);
      if (!current.length && after !== null) emptyIncomplete = true;
    } while (after !== null);
    assert.equal(new Set(rows.map((row) => JSON.stringify(row))).size, rows.length);
    return { rows, emptyIncomplete };
  }
  const expected = collect(workspace, subject, 256, 256, Membership.Selected).rows;
  assert(expected.length > 0);
  const identityRows = api.portableWorkspaceAuthoringOccurrences(workspace, identity, 256, 256);
  assert(identityRows.isComplete, "the empty workspace fits this single control page");
  assert.deepEqual(expected, identityRows.occurrences());
  for (const [inspected, returned] of [[1, 1], [1, 2], [2, 1], [0, 0], [64, 256], [257, 65535]]) {
    assert.deepEqual(collect(workspace, subject, inspected, returned, Membership.Selected).rows, expected);
  }
  for (const [inspected, returned, changedInspected, changedReturned] of [[0, 1, 64, 1], [1, 0, 1, 64], [257, 1, 65535, 1], [1, 257, 1, 65535]]) {
    // Keep the scan below exhaustion when checking a large return ceiling.
    const rawCursor = subjectRows(workspace, subject, inspected, returned).page().continuation();
    assert(rawCursor instanceof bindings.AuthoringOccurrencePageCursor);
    mismatch(() => subjectRows(workspace, subject, changedInspected, changedReturned, rawCursor));
  }
  for (const bad of [-1, 0.5, 65536, 2 ** 32, NaN, Infinity, -Infinity]) {
    for (const after of [undefined, cursor]) {
      for (const [inspected, returned] of [[bad, 1], [1, bad]]) {
        assert.throws(() => subjectRows(workspace, subject, inspected, returned, after), (error) => {
          assert(error instanceof Error);
          assert.equal(error.code, "InvalidArg");
          assert.match(error.message, /integer from 0 through 65535/);
          assert.equal(error.diagnostic, undefined);
          return true;
        });
      }
    }
  }
  const equal = api.comparePortableWorkspaces(workspace, rebuilt);
  const lock = equal.impacts(0, 256).entries.map((impact) => impact.subject())
    .find((item) => item.kind === bindings.WorkspaceSubjectKind.UniverseLock);
  assert(lock);
  const noNamedRows = collect(workspace, lock, 1, 1, Membership.Selected);
  assert.deepEqual(noNamedRows.rows, []);
  assert(noNamedRows.emptyIncomplete);

  let unselected = api.compilePortableWorkspace(unselectedRequest(false));
  let renamed = api.compilePortableWorkspace(unselectedRequest(true));
  assert.equal(unselected.semanticIrDigest, renamed.semanticIrDigest);
  const declarationPath = [field("declarations"), index(0), field("id")];
  const definition = read(unselected, "alpha.toml", declarationPath);
  const shifted = read(renamed, "renamed-alpha.toml", declarationPath);
  assert.equal(definition.state, State.Unselected);
  assert.equal(shifted.state, State.Unselected);
  assert(definition.belongsTo(unselected));
  assert(!definition.belongsTo(renamed));
  const declaration = definition.subject();
  const renamedDeclaration = shifted.subject();
  for (const value of [declaration, renamedDeclaration]) {
    assert.equal(value.kind, bindings.WorkspaceSubjectKind.Declaration);
    assert.equal(value.ownerModuleId, "candidate");
    assert.equal(value.declarationNamespaceId, "alpha");
    assert.equal(value.declarationId, "common");
    assert.equal(value.sourceId, null);
    assert.equal(value.extensionNamespaceId, null);
  }
  assert.equal(definition.occurrence().role, bindings.AuthoringOccurrenceRole.Definition);
  assert.equal(read(renamed, "alpha.toml", declarationPath).state, State.NoOccurrence);
  const unselectedRows = collect(unselected, declaration, 1, 1, Membership.Unselected);
  assert(unselectedRows.rows.some((row) => row.location.sourceLabel === "alpha.toml"));
  assert(unselectedRows.emptyIncomplete);
  for (const row of unselectedRows.rows) {
    const path = row.location.path.map((segment) => segment.field != null ? field(segment.field) : index(segment.index));
    const answer = read(unselected, row.location.sourceLabel, path);
    assert.equal(answer.state, State.Unselected);
    assert.deepEqual(answer.occurrence(), row);
    assert.equal(answer.subject().ownerModuleId, declaration.ownerModuleId);
    assert.equal(answer.subject().declarationNamespaceId, declaration.declarationNamespaceId);
    assert.equal(answer.subject().declarationId, declaration.declarationId);
  }
  const unselectedCursor = subjectRows(unselected, declaration, 1, 1).page().continuation();
  assert(unselectedCursor);
  mismatch(() => subjectRows(renamed, declaration, 1, 1, unselectedCursor));
  const beta = read(unselected, "beta.toml", declarationPath).subject();
  assert.equal(beta.ownerModuleId, declaration.ownerModuleId);
  assert.equal(beta.declarationId, declaration.declarationId);
  assert.notEqual(beta.declarationNamespaceId, declaration.declarationNamespaceId);
  mismatch(() => subjectRows(unselected, beta, 1, 1, unselectedCursor));
  assert.equal(subjectRows(unselected, declaration, 1, 1, unselectedCursor).membership, Membership.Unselected);
  assert.deepEqual(collect(workspace, declaration, 1, 1, Membership.Unselected).rows, []);
  assert.throws(() => read(workspace, root, [{}]));
  assert.throws(() => subjectRows(workspace, {}, 1, 1));
  assert.throws(() => subjectRows(workspace, identity, 1, 1));
  assert.throws(() => subjectRows(workspace, subject, 1, 1, {}));
  assert.throws(() => source.belongsTo({}));
  for (const Type of [bindings.PortableWorkspaceSourceInspection, bindings.PortableWorkspaceSubjectOccurrences]) {
    assert.throws(() => new Type());
  }
  // Other handles still retain compilations: this proves usability, not sole-owner release.
  workspace = rebuilt = unselected = renamed = null;
  global.gc?.();
  assert.equal(source.subject().sourceId, "workspace-manifest");
  assert.equal(definition.subject().declarationId, "common");
  assert.equal(definition.occurrence().location.sourceLabel, "alpha.toml");
  assert.equal(first.page().inspected, 1);
  return "workspace.portable-source-subjects";
}

export function exerciseSubjectInspection(bindings, api, request) {
  const field = (name) => bindings.AuthoringInstancePathSegment.field(name);
  const index = (value) => bindings.AuthoringInstancePathSegment.index(value);
  const inspect = (owner, subject, limits = [1, 1, 1], after) =>
    owner.inspectSubject(subject, ...limits, after);
  const mismatch = (call) => apiRejection(bindings, call, "workspace_subject_inspection_continuation_mismatch");
  const locationCopiedStringBytes = (location) => Buffer.byteLength(location.sourceLabel)
    + location.path.reduce((total, segment) => total
      + (segment.field == null ? 0 : Buffer.byteLength(segment.field)), 0);
  let workspace = api.compilePortableWorkspace(request());
  let rebuilt = api.compilePortableWorkspace(request());
  const subjectAt = (owner, label, path = []) => api.portableWorkspaceSourceSubject(owner, label, path).subject();
  const source = subjectAt(workspace, request().rootManifest);
  const workspaceSubject = subjectAt(workspace, request().rootManifest, [field("workspace"), field("workspace_id")]);
  const first = inspect(workspace, source);
  const cursor = first.continuation();
  assert(cursor instanceof bindings.PortableWorkspaceSubjectInspectionCursor);
  assert(first.belongsTo(workspace));
  assert(!first.belongsTo(rebuilt));
  assert.equal(workspace.semanticIrDigest, rebuilt.semanticIrDigest);

  function check(page, owner) {
    assert(page instanceof bindings.PortableWorkspaceSubjectInspection);
    assert(page.belongsTo(owner));
    const bytes = page.canonicalBytes;
    assert(Buffer.isBuffer(bytes));
    const encoded = JSON.parse(bytes);
    assert.deepEqual(Object.keys(encoded).sort(), ["workspaceId", "principalUniverseDigest", "semanticIrDigest",
      "admissionEvidenceDigest", "subject", "root", "dependencies", "occurrences", "inspected",
      "recordAnchor", "dependencyCopiedStringBytes", "sourceCopiedStringBytes",
      "inputsComplete", "sourcesComplete"].sort());
    for (const key of ["workspaceId", "principalUniverseDigest", "semanticIrDigest", "admissionEvidenceDigest"]) {
      assert.equal(page[key], owner[key]);
      assert.equal(encoded[key], page[key]);
    }
    assert.equal(encoded.inspected, page.inspected);
    assert.equal(encoded.dependencyCopiedStringBytes, page.dependencyCopiedStringBytes);
    assert.equal(encoded.sourceCopiedStringBytes, page.sourceCopiedStringBytes);
    assert.equal(page.sourceCopiedStringBytes,
      page.occurrences().reduce((total, row) => total + locationCopiedStringBytes(row.location), 0)
        + (page.recordAnchor === null ? 0 : locationCopiedStringBytes(page.recordAnchor)));
    assert.equal(encoded.inputsComplete, page.inputsComplete);
    assert.equal(encoded.sourcesComplete, page.sourcesComplete);
    assert.equal(page.isComplete, page.inputsComplete && page.sourcesComplete);
    assert.equal(page.continuation() === null, page.isComplete);
    assert(page.subject() instanceof bindings.WorkspaceSubject);
    const expectedSubject = JSON.parse(api.comparePortableWorkspaces(owner, owner)
      .compareSubject(page.subject()).canonicalBytes).subject;
    assert.deepEqual(encoded.subject, expectedSubject);
    const root = page.root();
    assert.equal(root === null, encoded.root === null);
    if (root !== null) {
      assert(root instanceof bindings.WorkspaceCausalStep);
      assert.equal(root.nodeDigest, encoded.root.nodeDigest);
      assert.equal(typeof root.fact().kind, "number");
      assert.equal(typeof root.provenance().kind, "number");
      assert.equal(root.subject().kind, page.subject().kind);
    }
    assert.deepEqual(page.dependencies().map((step) => step.nodeDigest), encoded.dependencies.map((step) => step.nodeDigest));
    for (const step of page.dependencies()) {
      assert(step instanceof bindings.WorkspaceCausalStep);
      assert.equal(typeof step.fact().kind, "number");
      assert.equal(typeof step.provenance().kind, "number");
    }
    const roleName = new Map([
      [bindings.AuthoringOccurrenceRole.Definition, "definition"],
      [bindings.AuthoringOccurrenceRole.Reference, "reference"],
      [bindings.AuthoringOccurrenceRole.Projection, "projection"],
    ]);
    const location = (value) => ({ source_label: value.sourceLabel, path: value.path.map((step) =>
      step.field != null ? { kind: "field", value: step.field } : { kind: "index", value: step.index }) });
    assert.deepEqual(encoded.occurrences, page.occurrences().map((row) => ({
      location: location(row.location), role: roleName.get(row.role),
    })));
    assert.deepEqual(encoded.recordAnchor, page.recordAnchor === null ? null : location(page.recordAnchor));
    bytes.fill(0);
    assert.notDeepEqual(bytes, page.canonicalBytes);
    assert.deepEqual(JSON.parse(page.canonicalBytes), encoded);
    return encoded;
  }
  const original = check(first, workspace);
  assert.equal(original.recordAnchor, null);
  for (const call of [
    () => inspect(rebuilt, source, [1, 1, 1], cursor),
    () => inspect(workspace, workspaceSubject, [1, 1, 1], cursor),
    ...[[2, 1, 1], [1, 2, 1], [1, 1, 2]].map((limits) => () => inspect(workspace, source, limits, cursor)),
  ]) mismatch(call);
  assert.deepEqual(inspect(workspace, source, [1, 1, 1], cursor).canonicalBytes,
    inspect(workspace, source, [1, 1, 1], cursor).canonicalBytes);
  const copied = first.occurrences();
  if (copied.length) {
    copied[0].location.sourceLabel = "caller mutation";
    copied[0].location.path.push({ field: "caller mutation" });
  }
  assert.deepEqual(check(first, workspace), original);

  function collect(owner, subject, limits) {
    let after;
    let terminal;
    let emptyPending = false;
    let inputsDone = false;
    let sourcesDone = false;
    const dependencies = [], occurrences = [];
    for (let pages = 0; ; pages += 1) {
      assert(pages < 4096, "the actual captured fixture must exhaust");
      const page = inspect(owner, subject, limits, after);
      const encoded = check(page, owner);
      assert(page.dependencies().length <= Math.min(limits[0] || 64, 256));
      assert(page.inspected <= Math.min(limits[1] || 64, 256));
      assert(page.occurrences().length <= Math.min(limits[2] || 64, 256));
      if (inputsDone) {
        assert.deepEqual(page.dependencies(), []);
        assert.equal(page.dependencyCopiedStringBytes, 0);
      }
      if (sourcesDone) {
        assert.deepEqual(page.occurrences(), []);
        assert.equal(page.inspected, 0);
        assert.equal(page.sourceCopiedStringBytes,
          page.recordAnchor === null ? 0 : locationCopiedStringBytes(page.recordAnchor));
      }
      inputsDone ||= page.inputsComplete;
      sourcesDone ||= page.sourcesComplete;
      dependencies.push(...encoded.dependencies);
      occurrences.push(...page.occurrences());
      if (!page.occurrences().length && !page.sourcesComplete) emptyPending = true;
      after = page.continuation();
      if (after === null) { terminal = page; break; }
    }
    assert.equal(new Set(dependencies.map((step) => step.nodeDigest)).size, dependencies.length);
    assert.equal(new Set(occurrences.map((row) => JSON.stringify(row))).size, occurrences.length);
    return { dependencies, occurrences, terminal, emptyPending };
  }
  const expected = collect(workspace, workspaceSubject, [256, 256, 256]);
  assert(expected.dependencies.length > 0);
  for (const limits of [[1, 1, 1], [1, 2, 1], [2, 1, 2], [0, 0, 0], [65535, 257, 65535]]) {
    const actual = collect(workspace, workspaceSubject, limits);
    assert.deepEqual(actual.dependencies, expected.dependencies);
    assert.deepEqual(actual.occurrences, expected.occurrences);
  }
  for (const [limits, changed] of [
    [[0, 1, 1], [64, 1, 1]], [[257, 1, 1], [65535, 1, 1]],
    [[1, 0, 1], [1, 64, 1]], [[1, 257, 1], [1, 65535, 1]],
    [[1, 1, 0], [1, 1, 64]], [[1, 1, 257], [1, 1, 65535]],
  ]) {
    const raw = inspect(workspace, workspaceSubject, limits).continuation();
    assert(raw);
    mismatch(() => inspect(workspace, workspaceSubject, changed, raw));
    check(inspect(workspace, workspaceSubject, limits, raw), workspace);
  }
  const lock = api.comparePortableWorkspaces(workspace, rebuilt).impacts(0, 256).entries
    .map((impact) => impact.subject()).find((value) => value.kind === bindings.WorkspaceSubjectKind.UniverseLock);
  const emptySelected = collect(workspace, lock, [1, 1, 1]);
  assert(emptySelected.terminal.root());
  assert.deepEqual(emptySelected.occurrences, []);
  assert(emptySelected.emptyPending);
  let unselected = api.compilePortableWorkspace(unselectedRequest(false));
  let renamed = api.compilePortableWorkspace(unselectedRequest(true));
  assert.equal(unselected.semanticIrDigest, renamed.semanticIrDigest);
  const path = [field("declarations"), index(0), field("id")];
  const declaration = subjectAt(unselected, "alpha.toml", path);
  const beta = subjectAt(unselected, "beta.toml", path);
  const absent = collect(unselected, declaration, [1, 1, 1]);
  assert.equal(absent.terminal.root(), null);
  assert.equal(absent.terminal.recordAnchor, null);
  assert.deepEqual(absent.dependencies, []);
  assert(absent.occurrences.some((row) => row.location.sourceLabel === "alpha.toml"));
  assert(absent.emptyPending);
  const retainedCursor = inspect(unselected, declaration).continuation();
  mismatch(() => inspect(renamed, declaration, [1, 1, 1], retainedCursor));
  mismatch(() => inspect(unselected, beta, [1, 1, 1], retainedCursor));
  check(inspect(unselected, declaration, [1, 1, 1], retainedCursor), unselected);
  assert(!absent.terminal.belongsTo(renamed));

  for (const bad of [-1, 0.5, 65536, 2 ** 32, NaN, Infinity, -Infinity]) {
    for (let axis = 0; axis < 3; axis += 1) {
      const limits = [1, 1, 1]; limits[axis] = bad;
      for (const after of [undefined, cursor]) assert.throws(() => inspect(workspace, source, limits, after), (error) => {
        assert.equal(error.code, "InvalidArg");
        assert.equal(error.diagnostic, undefined);
        return /integer from 0 through 65535/.test(error.message);
      });
    }
  }
  const foreignCursor = api.portableWorkspaceSubjectOccurrences(workspace, source, 1, 1).page().continuation();
  for (const bad of [true, "1", null]) {
    for (let axis = 0; axis < 3; axis += 1) {
      const limits = [1, 1, 1]; limits[axis] = bad;
      for (const after of [undefined, cursor]) assert.throws(() => inspect(workspace, source, limits, after), (error) => {
        assert(error instanceof Error);
        assert.equal(error.code, "NumberExpected");
        assert.equal(error.diagnostic, undefined);
        return true;
      });
    }
  }
  for (const call of [
    () => inspect({}, source), () => inspect(workspace, {}),
    () => bindings.PortableWorkspaceCompilation.prototype.inspectSubject.call({}, source, 1, 1, 1),
    () => bindings.PortableWorkspaceCompilation.prototype.inspectSubject.call(source, source, 1, 1, 1),
    () => inspect(workspace, source, [1, 1, 1], {}),
    () => inspect(workspace, source, [1, 1, 1], foreignCursor),
    () => inspect(workspace, source, [1, 1, 1], Object.create(bindings.PortableWorkspaceSubjectInspectionCursor.prototype)),
    () => first.belongsTo({}),
    () => api.portableWorkspaceSubjectOccurrences(workspace, source, 1, 1, cursor),
  ]) assert.throws(call);
  for (const Type of [bindings.PortableWorkspaceSubjectInspection, bindings.PortableWorkspaceSubjectInspectionCursor]) {
    assert.throws(() => new Type());
  }
  check(inspect(workspace, source, [1, 1, 1], cursor), workspace);
  // GC-only handles: prove post-drop usability, not immediate or sole-owner release.
  workspace = rebuilt = unselected = renamed = null;
  global.gc?.();
  assert.equal(first.subject().sourceId, "workspace-manifest");
  assert.equal(absent.terminal.subject().declarationId, "common");
  assert(absent.terminal.isComplete);
  assert.equal(absent.terminal.continuation(), null);
  assert.equal(JSON.parse(first.canonicalBytes).workspaceId, first.workspaceId);
  return "workspace.portable-subject-inspection";
}
