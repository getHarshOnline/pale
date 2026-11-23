// Copyright © 2026 RawThoughts Enterprises Private Limited. Apache-2.0.
import assert from "node:assert/strict";

// Existing actual fixture owners are supplied by subject_comparison.mjs.
export function exerciseForwardDiscovery(bindings, api, request, prior, selected, addedSubject, assertFact) {
  const compare = (left, right) => api.comparePortableWorkspaces(left, right);
  const compile = () => api.compilePortableWorkspace(request());
  const self = compare(prior, prior);
  const wireSubject = (subject) => JSON.parse(self.compareSubject(subject).canonicalBytes).subject;
  const identity = (subject) => JSON.stringify(wireSubject(subject));
  const step = (value) => ({ nodeDigest: value.nodeDigest, subject: identity(value.subject()),
    rule: value.rule, fact: value.fact(), provenance: value.provenance() });
  function captured(page) {
    const bytes = page.canonicalBytes;
    assert(Buffer.isBuffer(bytes)); const copy = Buffer.from(bytes); bytes.fill(0);
    assert.deepEqual(page.canonicalBytes, copy); return JSON.parse(copy);
  }
  function mismatch(call, code) {
    assert.throws(call, error => {
      assert(error instanceof Error); assert.equal(error.diagnostic.code, code);
      const wire = JSON.parse(error.diagnostic.canonicalBytes);
      assert.equal(wire.code, code); assert.equal(wire.summary, error.message);
      assert.equal(error.diagnostic.effect, bindings.DiagnosticEffectCertainty.NotApplicable); return true;
    });
  }
  function malformed(query, after) {
    for (const limit of [-1, 0.5, 65536, 2 ** 32, NaN, Infinity, -Infinity, true, "1", null]) {
      for (const cursor of [undefined, after]) assert.throws(() => query(limit, cursor), error => {
        assert(error instanceof Error); assert.equal(error.diagnostic, undefined);
        assert.equal(error.code, typeof limit === "number" ? "InvalidArg" : "NumberExpected"); return true;
      });
    }
  }
  function selectedSubjects(owner) {
    const page = compare(owner, owner).impacts(0, 256);
    assert.equal(page.nextOffset, undefined, "existing small fixture fits the independent reference");
    return page.entries.map(row => row.subject());
  }
  const priorSubjects = selectedSubjects(prior), currentSubjects = selectedSubjects(selected);
  const lock = priorSubjects.find(subject => subject.kind === bindings.WorkspaceSubjectKind.UniverseLock);
  const leaf = priorSubjects.find(subject => subject.kind === bindings.WorkspaceSubjectKind.Source);
  assert(lock && leaf);

  // Existing composed inspection is an independent direct-input reference.
  function inputsOf(owner, subject) {
    const dependencies = [], encoded = []; let after, root, wireRoot;
    for (let pages = 0; ; pages += 1) {
      assert(pages < 256);
      const page = owner.inspectSubject(subject, 256, 256, 256, after);
      const wire = JSON.parse(page.canonicalBytes); root = page.root(); wireRoot = wire.root;
      dependencies.push(...page.dependencies()); encoded.push(...wire.dependencies);
      after = page.continuation(); if (after === null) break;
    }
    return { root, wireRoot, dependencies, encoded };
  }
  function checkInputs(page, expected) {
    assert(page instanceof bindings.PortableWorkspaceDependencyPage);
    const wire = captured(page);
    assert.deepEqual(Object.keys(wire).sort(), ["principalUniverseDigest", "semanticIrDigest", "subject", "root", "dependencies", "copiedStringBytes", "isComplete"].sort());
    assert.equal(page.principalUniverseDigest, prior.principalUniverseDigest);
    assert.equal(page.semanticIrDigest, prior.semanticIrDigest);
    assert.equal(wire.principalUniverseDigest, page.principalUniverseDigest);
    assert.equal(wire.semanticIrDigest, page.semanticIrDigest);
    assert.deepEqual(wire.subject, wireSubject(page.subject()));
    assert.equal(page.root() === null, expected.root === null); assert.deepEqual(wire.root, expected.wireRoot);
    if (expected.root !== null) assert.deepEqual(step(page.root()), step(expected.root));
    const rows = page.dependencies(); assert.equal(rows.length, wire.dependencies.length);
    for (const [index, row] of rows.entries()) {
      assertFact(row.fact(), wire.dependencies[index].fact);
      assert.equal(row.nodeDigest, wire.dependencies[index].nodeDigest);
    }
    assert(Number.isInteger(page.copiedStringBytes) && page.copiedStringBytes >= 0);
    assert.equal(page.copiedStringBytes, wire.copiedStringBytes);
    assert.equal(wire.isComplete, page.isComplete); assert.equal(page.isComplete, page.continuation() === null);
    assert.equal("belongsTo" in page, false); assert.equal("workspaceId" in page, false);
    assert.equal("admissionEvidenceDigest" in page, false);
    return { rows, wire };
  }
  let terminalInputs;
  for (const subject of [...priorSubjects, addedSubject]) {
    const expected = inputsOf(prior, subject);
    for (const limit of [1, 2, 0, 256, 65535]) {
      let after; const actual = [], encoded = [];
      for (let pages = 0; ; pages += 1) {
        assert(pages < 256);
        const page = prior.directDependencies(subject, limit, after), checked = checkInputs(page, expected);
        assert(checked.rows.length <= Math.min(limit || 64, 256));
        actual.push(...checked.rows.map(step)); encoded.push(...checked.wire.dependencies);
        after = page.continuation(); if (after === null) { terminalInputs = page; break; }
      }
      assert.deepEqual(actual, expected.dependencies.map(step)); assert.deepEqual(encoded, expected.encoded);
      assert.equal(new Set(actual.map(row => row.nodeDigest)).size, actual.length);
    }
  }
  const expectedLock = inputsOf(prior, lock); assert.equal(expectedLock.dependencies.length, 2);
  const accountedInputs = prior.directDependencies(lock, 256);
  const accountedInputBytes = Buffer.from(accountedInputs.canonicalBytes);
  const accountedInputWire = JSON.parse(accountedInputBytes);
  assert(accountedInputs.dependencies().length > 0);
  assert(accountedInputs.copiedStringBytes > 0);
  assert.equal(accountedInputs.copiedStringBytes, accountedInputWire.copiedStringBytes);
  const repeatedAccountedInputs = prior.directDependencies(lock, 256);
  assert.equal(repeatedAccountedInputs.copiedStringBytes, accountedInputs.copiedStringBytes);
  assert.deepEqual(repeatedAccountedInputs.canonicalBytes, accountedInputBytes);
  const first = prior.directDependencies(lock, 1), cursor = first.continuation();
  assert(first.copiedStringBytes > 0);
  assert(cursor instanceof bindings.PortableWorkspaceDependencyCursor);
  const nextBytes = prior.directDependencies(lock, 1, cursor).canonicalBytes;
  const alias = prior; assert.deepEqual(alias.directDependencies(lock, 1, cursor).canonicalBytes, nextBytes);
  const fresh = compile(); assert.equal(fresh.principalUniverseDigest, prior.principalUniverseDigest);
  for (const call of [() => fresh.directDependencies(lock, 1, cursor),
    () => prior.directDependencies(leaf, 1, cursor), () => prior.directDependencies(lock, 2, cursor)]) {
    mismatch(call, "workspace_dependency_continuation_mismatch");
  }
  const inputless = prior.directDependencies(leaf, 1);
  assert(inputless.root() !== null && inputless.isComplete); assert.deepEqual(inputless.dependencies(), []);
  const missing = prior.directDependencies(addedSubject, 1);
  assert.equal(missing.root(), null); assert(missing.isComplete); assert.deepEqual(missing.dependencies(), []);
  malformed((limit, after) => prior.directDependencies(lock, limit, after), cursor);

  // Union membership comes from both self-impact partitions, not the new query.
  const forward = compare(prior, selected), reverse = compare(selected, prior), repeated = compare(prior, selected);
  const expectedUnion = new Set([...priorSubjects, ...currentSubjects].map(identity));
  const sideOrders = [priorSubjects.map(identity), currentSubjects.map(identity)];
  function checkUnion(page, owner) {
    assert(page instanceof bindings.WorkspaceComparisonSubjectPage); assert(page.belongsTo(owner));
    const wire = captured(page), summary = page.comparison();
    assert.deepEqual(Object.keys(wire).sort(), ["receiptDigest", "priorPrincipalUniverseDigest", "currentPrincipalUniverseDigest",
      "priorSemanticIrDigest", "currentSemanticIrDigest", "subjects", "inspected", "keyComparisons", "subjectIdentityBytes", "isComplete"].sort());
    for (const key of ["receiptDigest", "priorPrincipalUniverseDigest", "currentPrincipalUniverseDigest", "priorSemanticIrDigest", "currentSemanticIrDigest"]) {
      assert.equal(summary[key], owner[key]); assert.equal(wire[key], owner[key]);
    }
    const subjects = page.subjects(); assert.deepEqual(subjects.map(wireSubject), wire.subjects);
    assert(Number.isInteger(page.subjectIdentityBytes) && page.subjectIdentityBytes >= 0);
    assert.equal(page.subjectIdentityBytes, wire.subjectIdentityBytes);
    for (const subject of subjects) assert.notEqual(owner.compareSubject(subject).values.membership, "absent");
    assert.equal(page.inspected, wire.inspected); assert.equal(page.keyComparisons, wire.keyComparisons);
    assert(Number.isInteger(page.inspected) && page.inspected >= subjects.length && page.inspected <= 2 * subjects.length + 2);
    assert(Number.isInteger(page.keyComparisons) && page.keyComparisons >= 0 && page.keyComparisons <= subjects.length);
    assert.equal(page.isComplete, wire.isComplete); assert.equal(page.isComplete, page.continuation() === null);
    return subjects.map(identity);
  }
  let canonicalOrder, terminalUnion;
  for (const owner of [forward, reverse]) for (const limit of [1, 2, 0, 256, 65535]) {
    let after; const actual = [];
    for (let pages = 0; ; pages += 1) {
      assert(pages < 256); const page = owner.subjects(limit, after);
      const keys = checkUnion(page, owner); assert(keys.length <= Math.min(limit || 64, 256)); actual.push(...keys);
      after = page.continuation(); if (after === null) { terminalUnion = page; break; }
    }
    assert.equal(new Set(actual).size, actual.length); assert.deepEqual(new Set(actual), expectedUnion);
    if (canonicalOrder === undefined) canonicalOrder = actual; else assert.deepEqual(actual, canonicalOrder);
    for (const order of sideOrders) { const membership = new Set(order); assert.deepEqual(actual.filter(key => membership.has(key)), order); }
  }
  assert.equal(reverse.compareSubject(addedSubject).values.membership, "prior-only");
  const currentImpactKeys = new Set(reverse.impacts(0, 256).entries.map(row => identity(row.subject())));
  assert(!currentImpactKeys.has(identity(addedSubject))); assert(canonicalOrder.includes(identity(addedSubject)));
  const unionFirst = forward.subjects(1), unionCursor = unionFirst.continuation();
  assert(unionFirst.subjectIdentityBytes > 0);
  assert(unionCursor instanceof bindings.WorkspaceComparisonSubjectCursor);
  assert(unionFirst.belongsTo(repeated));
  const unionNext = forward.subjects(1, unionCursor).canonicalBytes;
  assert.deepEqual(repeated.subjects(1, unionCursor).canonicalBytes, unionNext);
  const freshPair = compare(fresh, selected);
  for (const owner of [reverse, freshPair]) { assert(!unionFirst.belongsTo(owner)); mismatch(() => owner.subjects(1, unionCursor), "workspace_comparison_subject_continuation_mismatch"); }
  mismatch(() => forward.subjects(2, unionCursor), "workspace_comparison_subject_continuation_mismatch");
  malformed((limit, after) => forward.subjects(limit, after), unionCursor);

  const dependencyMethod = bindings.PortableWorkspaceCompilation.prototype.directDependencies;
  const unionMethod = bindings.WorkspaceComparison.prototype.subjects;
  for (const call of [() => dependencyMethod.call({}, lock, 1), () => dependencyMethod.call(forward, lock, 1),
    () => unionMethod.call({}, 1), () => unionMethod.call(prior, 1), () => prior.directDependencies({}, 1),
    () => prior.directDependencies(lock, 1, {}), () => forward.subjects(1, {}),
    () => prior.directDependencies(lock, 1, unionCursor), () => forward.subjects(1, cursor),
    () => prior.directDependencies(lock, 1, Object.create(bindings.PortableWorkspaceDependencyCursor.prototype)),
    () => forward.subjects(1, Object.create(bindings.WorkspaceComparisonSubjectCursor.prototype)),
    () => unionFirst.belongsTo(prior)]) assert.throws(call);
  for (const Type of [bindings.PortableWorkspaceDependencyPage, bindings.PortableWorkspaceDependencyCursor,
    bindings.WorkspaceComparisonSubjectPage, bindings.WorkspaceComparisonSubjectCursor]) assert.throws(() => new Type());
  assert.deepEqual(prior.directDependencies(lock, 1, cursor).canonicalBytes, nextBytes);
  assert.deepEqual(forward.subjects(1, unionCursor).canonicalBytes, unionNext);
  const rowBefore = step(first.dependencies()[0]); first.dependencies()[0].fact().kind = -1;
  assert.deepEqual(step(first.dependencies()[0]), rowBefore);
  const copied = first.dependencies(); copied.length = 0; assert.equal(first.dependencies().length, 1);
  const unionCopied = unionFirst.subjects(); unionCopied.length = 0; assert.equal(unionFirst.subjects().length, 1);

  // Usability after local variables leave scope is not proof of GC timing or
  // graph retention for the deliberately detached terminal dependency page.
  const retained = (() => {
    const workspace = compile(), pair = compare(workspace, workspace);
    return [workspace.directDependencies(lock, 1), workspace.directDependencies(addedSubject, 1),
      pair.subjects(1), pair.subjects(256)];
  })();
  global.gc?.(); for (const page of retained) assert(page.canonicalBytes.length > 0);
  assert.equal(retained[1].root(), null); assert(retained[1].isComplete && retained[3].isComplete);
  assert(retained[2].subjects().length > 0 && retained[3].comparison().receiptDigest.length > 0);
  assert(terminalInputs.isComplete && terminalUnion.isComplete);
  return ["workspace.portable-direct-dependencies", "workspace.portable-comparison-subjects"];
}
