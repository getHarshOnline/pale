// Copyright © 2026 RawThoughts Enterprises Private Limited. Apache-2.0.
import assert from "node:assert/strict";

// Reuse the comparison test's actual captured-input owner and added-source case.
// No fixture generation, internal compiler API or new Node forward query is used.
export function exerciseReverseQueries(bindings, api, request, prior, selected, addedSubject, assertFact) {
  const State = bindings.WorkspaceContributingRootsState;
  const compile = (input = request()) => api.compilePortableWorkspace(input);
  const compare = (left, right) => api.comparePortableWorkspaces(left, right);
  const wireSubject = (comparison, subject) => JSON.parse(comparison.compareSubject(subject).canonicalBytes).subject;
  const nativeSubject = (subject) => Object.fromEntries([
    "kind", "workspaceId", "sourceId", "componentId", "moduleId", "ownerModuleId",
    "declarationNamespaceId", "declarationId", "importerModuleId", "upstreamModuleId",
    "upstreamReleaseId", "exportId", "sourceNamespaceId", "localNamespaceId",
    "extensionNamespaceId", "resourceId",
  ].map((key) => [key, subject[key]]));
  const nativeStep = (step) => ({ nodeDigest: step.nodeDigest, subject: nativeSubject(step.subject()),
    rule: step.rule, fact: step.fact(), provenance: step.provenance() });
  const nativeRoot = (root) => ({ rootDigest: root.rootDigest, kind: root.kind,
    subject: nativeSubject(root.subject()), prior: root.priorFact(), current: root.currentFact() });
  function rejection(call, code) {
    assert.throws(call, (error) => {
      assert(error instanceof Error);
      assert.equal(error.diagnostic.code, code);
      assert.equal(error.message, error.diagnostic.summary);
      const wire = JSON.parse(error.diagnostic.canonicalBytes);
      assert.equal(wire.code, code);
      assert.equal(wire.summary, error.message);
      assert.equal(error.diagnostic.effect, bindings.DiagnosticEffectCertainty.NotApplicable);
      return true;
    });
  }
  function unchangedBuffer(page) {
    const bytes = page.canonicalBytes;
    assert(Buffer.isBuffer(bytes));
    const expected = Buffer.from(bytes);
    bytes.fill(0);
    assert.deepEqual(page.canonicalBytes, expected);
    return JSON.parse(expected);
  }
  function invalidLimits(query, axes, after) {
    for (const bad of [-1, 0.5, 65536, 2 ** 32, NaN, Infinity, -Infinity, true, "1", null]) {
      for (let axis = 0; axis < axes; axis += 1) {
        const limits = Array(axes).fill(1); limits[axis] = bad;
        for (const cursor of [undefined, after]) assert.throws(() => query(limits, cursor), (error) => {
          assert(error instanceof Error);
          assert.equal(error.code, typeof bad === "number" ? "InvalidArg" : "NumberExpected");
          assert.equal(error.diagnostic, undefined);
          return true;
        });
      }
    }
  }

  // Test-only forward reference from the existing composed query. Never use the
  // reverse query under test to decide which nodes or contributors are expected.
  function forwardReference(owner) {
    const partition = compare(owner, owner).impacts(0, 256);
    assert.equal(partition.nextOffset, undefined);
    return partition.entries.map((impact) => {
      const selectedSubject = impact.subject();
      let after;
      let root;
      let wireRoot;
      const inputs = [];
      for (let pages = 0; ; pages += 1) {
        assert(pages < 256);
        const inspected = owner.inspectSubject(selectedSubject, 256, 256, 256, after);
        const wire = JSON.parse(inspected.canonicalBytes);
        root = inspected.root(); wireRoot = wire.root;
        inputs.push(...wire.dependencies.map(step => step.nodeDigest));
        after = inspected.continuation();
        if (after === null) break;
      }
      assert(root);
      return { subject: selectedSubject, root, wireRoot, inputs };
    }).sort((a, b) => a.root.nodeDigest < b.root.nodeDigest ? -1 : a.root.nodeDigest > b.root.nodeDigest ? 1 : 0);
  }

  const changedInput = () => ({ ...request(), documents: request().documents.map(({ label, bytes }) =>
    ({ label, bytes: Buffer.concat([bytes, Buffer.from("\n# source-only reverse-query case\n")]) })) });
  const current = compile(changedInput());
  assert.equal(prior.semanticIrDigest, current.semanticIrDigest);
  const comparison = compare(prior, current);
  const repeated = compare(prior, current);
  const reverse = compare(current, prior);
  const freshPrior = compile();
  const freshCurrent = compile(changedInput());
  assert.equal(freshCurrent.principalUniverseDigest, current.principalUniverseDigest);
  const replacedPrior = compare(freshPrior, current);
  const replacedCurrent = compare(prior, freshCurrent);
  const rootPage = comparison.changeRoots(0, 256);
  assert.equal(rootPage.nextOffset, undefined, "existing small fixture must fit the reference page");
  assert.equal(rootPage.entries.length, 3, "the existing three source documents changed");
  assert(rootPage.entries.every(root => root.subject().kind === bindings.WorkspaceSubjectKind.Source));
  const rootsByDigest = new Map(rootPage.entries.map(root => [root.rootDigest, root]));
  const currentReference = forwardReference(current);
  const currentNodes = new Map(currentReference.map(row => [row.root.nodeDigest, row]));
  function expectedContributors(kind) {
    const target = currentReference.find(row => row.subject.kind === kind);
    assert(target);
    const pending = [target.root.nodeDigest];
    const visited = new Set();
    while (pending.length > 0) {
      const digest = pending.pop();
      if (visited.has(digest)) continue;
      const node = currentNodes.get(digest);
      assert(node, "every forward input resolves in the retained current partition");
      visited.add(digest);
      assert(visited.size <= currentReference.length);
      for (const input of node.inputs) if (!visited.has(input) && !pending.includes(input)) pending.push(input);
      assert(pending.length <= currentReference.length);
    }
    const ancestorSubjects = new Set([...visited].map(digest => JSON.stringify(currentNodes.get(digest).wireRoot.subject)));
    return { subject: target.subject, roots: rootPage.entries.filter(root =>
      ancestorSubjects.has(JSON.stringify(wireSubject(comparison, root.subject())))) };
  }
  const expected = expectedContributors(bindings.WorkspaceSubjectKind.UniverseLock);
  assert.equal(expected.roots.length, 2, "universe lock depends on its source and the resolution source");
  const impacts = comparison.impacts(0, 256);
  assert.equal(impacts.nextOffset, undefined);
  const workspaceImpact = impacts.entries.find(row => row.subject().kind === bindings.WorkspaceSubjectKind.Workspace);
  assert(workspaceImpact);
  const workspaceExpected = expectedContributors(bindings.WorkspaceSubjectKind.Workspace);
  assert.equal(workspaceExpected.roots.length, 1, "semantic sealing excludes lock evidence");
  assert.equal(workspaceImpact.contributingRootCount, workspaceExpected.roots.length);
  assert.equal(comparison.compareSubject(workspaceImpact.subject()).values.comparison, bindings.WorkspaceValueComparison.Equal);
  const subject = expected.subject;
  const universeImpact = impacts.entries.find(row => row.subject().kind === bindings.WorkspaceSubjectKind.UniverseLock);
  assert(universeImpact);
  assert.equal(universeImpact.contributingRootCount, expected.roots.length);
  const otherSubject = rootPage.entries[0].subject();

  function checkRoots(page, owner, expectedState) {
    assert(page instanceof bindings.WorkspaceContributingRootsPage);
    assert(page.belongsTo(owner));
    assert.equal(page.state, expectedState);
    assert.equal(page.isComplete, page.continuation() === null);
    const wire = unchangedBuffer(page);
    assert.deepEqual(Object.keys(wire).sort(), ["receiptDigest", "priorPrincipalUniverseDigest",
      "currentPrincipalUniverseDigest", "priorSemanticIrDigest", "currentSemanticIrDigest",
      "subject", "state", "roots", "isComplete"].sort());
    const summary = page.comparison();
    for (const key of ["receiptDigest", "priorPrincipalUniverseDigest", "currentPrincipalUniverseDigest",
      "priorSemanticIrDigest", "currentSemanticIrDigest"]) {
      assert.equal(summary[key], owner[key]);
      assert.equal(wire[key], owner[key]);
    }
    assert.deepEqual(wire.subject, wireSubject(owner, page.subject()));
    assert.equal(wire.state, new Map([[State.Affected, "affected"], [State.Unaffected, "unaffected"],
      [State.NotPresent, "not_present"]]).get(expectedState));
    assert.equal(wire.isComplete, page.isComplete);
    const roots = page.roots();
    assert.deepEqual(roots.map(root => root.rootDigest), wire.roots.map(root => root.rootDigest));
    for (const [index, root] of roots.entries()) {
      const encoded = wire.roots[index];
      assert.deepEqual(encoded.subject, wireSubject(owner, root.subject()));
      assert.equal(encoded.kind, new Map([[bindings.WorkspaceChangeKind.Added, "added"],
        [bindings.WorkspaceChangeKind.Modified, "modified"], [bindings.WorkspaceChangeKind.Removed, "removed"]]).get(root.kind));
      for (const [field, fact] of [["priorFact", root.priorFact()], ["currentFact", root.currentFact()]]) {
        assert.equal(fact === null, encoded[field] === null);
        if (fact !== null) assertFact(fact, encoded[field]);
      }
    }
    return roots;
  }
  assert.deepEqual(checkRoots(comparison.contributingRoots(workspaceExpected.subject, 1), comparison,
    State.Affected).map(root => root.rootDigest), workspaceExpected.roots.map(root => root.rootDigest));
  let retainedRootTerminal;
  for (const limit of [1, 2, 0, 256, 65535]) {
    let cursor;
    const actual = [];
    for (let pages = 0; ; pages += 1) {
      assert(pages < 256);
      const page = comparison.contributingRoots(subject, limit, cursor);
      const roots = checkRoots(page, comparison, State.Affected);
      assert(roots.length <= Math.min(limit || 64, 256));
      for (const root of roots) assert.deepEqual(nativeRoot(root), nativeRoot(rootsByDigest.get(root.rootDigest)));
      actual.push(...roots.map(root => root.rootDigest));
      cursor = page.continuation();
      if (cursor === null) { retainedRootTerminal = page; break; }
    }
    assert.deepEqual(actual, expected.roots.map(root => root.rootDigest));
    assert.equal(new Set(actual).size, actual.length);
  }
  const first = comparison.contributingRoots(subject, 1);
  const cursor = first.continuation();
  assert(cursor instanceof bindings.WorkspaceContributingRootsCursor);
  assert(first.belongsTo(repeated));
  for (const incompatible of [reverse, replacedPrior, replacedCurrent]) {
    assert(!first.belongsTo(incompatible));
    rejection(() => incompatible.contributingRoots(subject, 1, cursor), "workspace_contributing_roots_continuation_mismatch");
  }
  for (const call of [() => comparison.contributingRoots(otherSubject, 1, cursor),
    () => comparison.contributingRoots(subject, 2, cursor)]) {
    rejection(call, "workspace_contributing_roots_continuation_mismatch");
  }
  const secondBytes = comparison.contributingRoots(subject, 1, cursor).canonicalBytes;
  assert.deepEqual(repeated.contributingRoots(subject, 1, cursor).canonicalBytes, secondBytes);
  assert(expected.roots.length > 1);
  const preferred = comparison.explain(subject).root().rootDigest;
  assert(expected.roots.some(root => root.rootDigest === preferred));
  assert(expected.roots.some(root => root.rootDigest !== preferred), "contributors are not a single preferred path");
  const unchanged = compare(prior, prior);
  assert.deepEqual(checkRoots(unchanged.contributingRoots(subject, 0), unchanged, State.Unaffected), []);
  const removed = compare(selected, prior);
  assert.equal(removed.compareSubject(addedSubject).values.membership, "prior-only");
  const notPresent = removed.contributingRoots(addedSubject, 1);
  assert.deepEqual(checkRoots(notPresent, removed, State.NotPresent), []);
  assert(notPresent.isComplete);
  const added = compare(prior, selected);
  assert(checkRoots(added.contributingRoots(addedSubject, 1), added, State.Affected).length > 0);
  invalidLimits((limits, after) => comparison.contributingRoots(subject, ...limits, after), 1, cursor);

  // The existing composed forward query supplies the independent reference.
  // This complete test-only oracle does not become a production reverse index.
  const self = compare(prior, prior);
  const reference = forwardReference(prior);
  assert(reference.length > 1);
  const byDigest = new Map(reference.map(row => [row.root.nodeDigest, row]));
  function checkDependents(page, owner, expectedRoot) {
    assert(page instanceof bindings.PortableWorkspaceDependentPage);
    assert(page.belongsTo(owner));
    const wire = unchangedBuffer(page);
    assert.deepEqual(Object.keys(wire).sort(), ["workspaceId", "principalUniverseDigest", "semanticIrDigest",
      "admissionEvidenceDigest", "subject", "root", "dependents", "inspected", "inputDigestComparisons", "isComplete"].sort());
    for (const key of ["workspaceId", "principalUniverseDigest", "semanticIrDigest", "admissionEvidenceDigest"]) {
      assert.equal(page[key], owner[key]); assert.equal(wire[key], owner[key]);
    }
    assert.deepEqual(wire.subject, wireSubject(self, page.subject()));
    assert.equal(page.root() === null, expectedRoot === null);
    assert.deepEqual(wire.root, expectedRoot?.wireRoot ?? null);
    if (expectedRoot) assert.deepEqual(nativeStep(page.root()), nativeStep(expectedRoot.root));
    assert.equal(page.inspected, wire.inspected);
    assert.equal(page.inputDigestComparisons, wire.inputDigestComparisons);
    assert(Number.isInteger(page.inspected) && page.inspected >= 0);
    assert(Number.isInteger(page.inputDigestComparisons) && page.inputDigestComparisons >= 0);
    assert.equal(page.isComplete, wire.isComplete);
    assert.equal(page.isComplete, page.continuation() === null);
    const nodes = page.dependents();
    assert.deepEqual(nodes.map(row => row.nodeDigest), wire.dependents.map(row => row.nodeDigest));
    for (const [index, node] of nodes.entries()) {
      const expected = byDigest.get(node.nodeDigest);
      assert(expected);
      assert.deepEqual(nativeStep(node), nativeStep(expected.root));
      assert.deepEqual(wire.dependents[index], expected.wireRoot);
    }
    return nodes;
  }
  let sawConsumer = false, sawEmptyIncomplete = false, retainedDependentTerminal;
  for (const target of reference) {
    const expected = reference.filter(node => node.inputs.includes(target.root.nodeDigest));
    sawConsumer ||= expected.length > 0;
    for (const limits of [[1, 1], [2, 1], [1, 256], [0, 0], [65535, 65535]]) {
      let after;
      let inspected = 0;
      const actual = [];
      for (let pages = 0; ; pages += 1) {
        assert(pages < 256);
        const page = prior.directDependents(target.subject, ...limits, after);
        const nodes = checkDependents(page, prior, target);
        assert(page.inspected <= Math.min(limits[0] || 64, 256));
        assert(nodes.length <= Math.min(limits[1] || 64, 256));
        const candidates = reference.slice(inspected, inspected + page.inspected);
        const matching = candidates.filter(node => node.inputs.includes(target.root.nodeDigest));
        assert.deepEqual(nodes.map(node => node.nodeDigest), matching.map(node => node.root.nodeDigest));
        const searched = candidates.filter(node => node.inputs.length > 0);
        assert(page.inputDigestComparisons >= searched.length);
        assert(page.inputDigestComparisons <= searched.reduce((sum, node) => sum + Math.ceil(Math.log2(node.inputs.length)) + 1, 0));
        inspected += page.inspected;
        assert(inspected <= reference.length);
        actual.push(...nodes.map(node => node.nodeDigest));
        sawEmptyIncomplete ||= !nodes.length && !page.isComplete;
        after = page.continuation();
        if (after === null) { retainedDependentTerminal = page; break; }
        assert(page.inspected > 0);
      }
      assert.equal(inspected, reference.length);
      assert.deepEqual(actual, expected.map(node => node.root.nodeDigest));
      assert.equal(new Set(actual).size, actual.length);
    }
  }
  assert(sawConsumer && sawEmptyIncomplete);
  const target = reference.find(node => node.subject.kind === bindings.WorkspaceSubjectKind.Workspace);
  assert(target);
  const dependentFirst = prior.directDependents(target.subject, 1, 1);
  const dependentCursor = dependentFirst.continuation();
  assert(dependentCursor instanceof bindings.PortableWorkspaceDependentCursor);
  assert(!dependentFirst.belongsTo(freshPrior));
  for (const call of [() => freshPrior.directDependents(target.subject, 1, 1, dependentCursor),
    () => prior.directDependents(otherSubject, 1, 1, dependentCursor),
    () => prior.directDependents(target.subject, 2, 1, dependentCursor),
    () => prior.directDependents(target.subject, 1, 2, dependentCursor)]) {
    rejection(call, "workspace_dependent_continuation_mismatch");
  }
  for (const [raw, different] of [[0, 64], [257, 65535]]) {
    const bound = prior.directDependents(target.subject, 1, raw).continuation();
    assert(bound);
    rejection(() => prior.directDependents(target.subject, 1, different, bound), "workspace_dependent_continuation_mismatch");
    checkDependents(prior.directDependents(target.subject, 1, raw, bound), prior, target);
  }
  const missing = prior.directDependents(addedSubject, 1, 1);
  assert.deepEqual(checkDependents(missing, prior, null), []);
  assert.equal(missing.inspected, 0);
  assert.equal(missing.inputDigestComparisons, 0);
  assert(missing.isComplete);
  invalidLimits((limits, after) => prior.directDependents(target.subject, ...limits, after), 2, dependentCursor);
  for (const call of [
    () => comparison.contributingRoots({}, 1), () => prior.directDependents({}, 1, 1),
    () => bindings.WorkspaceComparison.prototype.contributingRoots.call({}, subject, 1),
    () => bindings.PortableWorkspaceCompilation.prototype.directDependents.call({}, target.subject, 1, 1),
    () => bindings.WorkspaceComparison.prototype.contributingRoots.call(prior, subject, 1),
    () => bindings.PortableWorkspaceCompilation.prototype.directDependents.call(comparison, target.subject, 1, 1),
    () => comparison.contributingRoots(subject, 1, {}),
    () => comparison.contributingRoots(subject, 1, dependentCursor),
    () => prior.directDependents(target.subject, 1, 1, cursor),
    () => prior.directDependents(target.subject, 1, 1, {}),
    () => comparison.contributingRoots(subject, 1, Object.create(bindings.WorkspaceContributingRootsCursor.prototype)),
    () => prior.directDependents(target.subject, 1, 1, Object.create(bindings.PortableWorkspaceDependentCursor.prototype)),
    () => first.belongsTo(prior), () => dependentFirst.belongsTo(comparison),
  ]) assert.throws(call);
  for (const Type of [bindings.WorkspaceContributingRootsPage, bindings.WorkspaceContributingRootsCursor,
    bindings.PortableWorkspaceDependentPage, bindings.PortableWorkspaceDependentCursor]) assert.throws(() => new Type());
  assert.deepEqual(comparison.contributingRoots(subject, 1, cursor).canonicalBytes, secondBytes);
  const afterErrors = prior.directDependents(target.subject, 1, 1, dependentCursor).canonicalBytes;
  assert.deepEqual(prior.directDependents(target.subject, 1, 1, dependentCursor).canonicalBytes, afterErrors);
  const copiedRoots = first.roots();
  const rootBeforeMutation = nativeRoot(copiedRoots[0]);
  const copiedFact = copiedRoots[0].currentFact();
  if (copiedFact !== null) copiedFact.kind = -1;
  assert.deepEqual(nativeRoot(first.roots()[0]), rootBeforeMutation);
  copiedRoots.length = 0;
  assert.equal(first.roots().length, 1);
  const copiedNodes = retainedDependentTerminal.dependents(); copiedNodes.length = 0;
  assert.equal(retainedDependentTerminal.dependents().length,
    JSON.parse(retainedDependentTerminal.canonicalBytes).dependents.length);
  const rootBeforeCopyMutation = nativeStep(dependentFirst.root());
  dependentFirst.root().fact().kind = -1;
  dependentFirst.root().provenance().kind = -1;
  assert.deepEqual(nativeStep(dependentFirst.root()), rootBeforeCopyMutation);

  // Fresh local variables leave scope; test post-drop usability, not immediate
  // or sole-owner release. These bindings intentionally have no dispose/free API.
  const retained = (() => {
    const left = compile(), right = compile(changedInput());
    const pair = compare(left, right);
    return [pair.contributingRoots(subject, 1), pair.contributingRoots(addedSubject, 1),
      left.directDependents(target.subject, 1, 1), left.directDependents(addedSubject, 1, 1)];
  })();
  global.gc?.();
  for (const page of retained) {
    assert(page.canonicalBytes.length > 0);
    assert(page.subject() instanceof bindings.WorkspaceSubject);
  }
  assert.equal(retained[1].state, State.NotPresent);
  assert.equal(retained[3].root(), null);
  assert(retainedRootTerminal.isComplete && retainedDependentTerminal.isComplete);
  return ["workspace.portable-contributing-roots", "workspace.portable-direct-dependents"];
}
