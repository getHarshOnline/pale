import assert from "node:assert/strict";
import fs from "node:fs";

// Exercise actual compiled inputs and the generated WebAssembly handle boundary.
export function verifySubjectComparison(bindings, fixtureRoot) {
  const owned = new Set();
  const keep = (value) => { owned.add(value); return value; };
  const release = (value) => { assert(owned.delete(value)); value.free(); };
  const entries = (page) => page.intoEntries().map(keep); // intoEntries consumes the page.
  const api = keep(new bindings.Pale());
  const fixture = JSON.parse(fs.readFileSync(`${fixtureRoot}/portable-journey.json`, "utf8"));
  const documents = fixture.document_labels.map((label) => ({ label, bytes: fs.readFileSync(`${fixtureRoot}/portable/${label}`) }));
  const root = documents.find(({ label }) => label === fixture.root_manifest);
  assert(root);
  const replaceRoot = (bytes, label = root.label) => ({ rootManifest: label,
    documents: documents.map((document) => document === root ? { label, bytes } : document) });
  const compile = (input) => keep(api.compilePortableWorkspace(new bindings.PortableWorkspaceRequest(
    input.rootManifest,
    input.documents.map(({ label, bytes }) => new bindings.PortableWorkspaceDocument(label, bytes)),
  )));
  const compare = (prior, current) => {
    const pair = keep(api.comparePortableWorkspaces(prior, current));
    assert.equal(pair.priorPrincipalUniverseDigest, prior.principalUniverseDigest);
    assert.equal(pair.currentPrincipalUniverseDigest, current.principalUniverseDigest);
    assert.equal(pair.priorSemanticIrDigest, prior.semanticIrDigest);
    assert.equal(pair.currentSemanticIrDigest, current.semanticIrDigest);
    return pair;
  };
  const factKinds = {
    source_content: bindings.WorkspaceFactKind.SourceContent,
    component: bindings.WorkspaceFactKind.Component,
    module: bindings.WorkspaceFactKind.Module,
    declaration: bindings.WorkspaceFactKind.Declaration,
    import_surface: bindings.WorkspaceFactKind.ImportSurface,
    extension: bindings.WorkspaceFactKind.Extension,
    effective_policy: bindings.WorkspaceFactKind.EffectivePolicy,
    responsibility_closure: bindings.WorkspaceFactKind.ResponsibilityClosure,
    organizational_translation_closure: bindings.WorkspaceFactKind.OrganizationalTranslationClosure,
    experience_composition: bindings.WorkspaceFactKind.ExperienceComposition,
    resource: bindings.WorkspaceFactKind.Resource,
    resolution_lock: bindings.WorkspaceFactKind.ResolutionLock,
    universe_lock: bindings.WorkspaceFactKind.UniverseLock,
    semantic_ir: bindings.WorkspaceFactKind.SemanticIr,
  };
  const factFields = ["sourcePath", "sourceDigest", "byteLen", "componentDigest", "moduleDigest",
    "valueDigest", "surfaceDigest", "closureDigest", "resolutionDigest", "universeDigest", "workspaceSemanticIrDigest"];
  function factData(fact) {
    try {
      return { kind: fact.kind(), ...Object.fromEntries(factFields.map((field) => [field, fact[field]])) };
    } finally { fact.free(); }
  }
  function assertFact(fact, wire) {
    assert(fact instanceof bindings.WorkspaceFact);
    const actual = factData(fact);
    assert.equal(actual.kind, factKinds[wire.kind]);
    for (const field of factFields) assert.equal(actual[field], wire[field], `${wire.kind}.${field}`);
  }
  function answer(pair, subject, membership, equality) {
    const fields = ["sourceId", "componentId", "moduleId", "ownerModuleId", "declarationNamespaceId",
      "declarationId", "importerModuleId", "upstreamModuleId", "upstreamReleaseId", "exportId",
      "sourceNamespaceId", "localNamespaceId", "extensionNamespaceId", "resourceId", "workspaceId"];
    const expectedSubject = { kind: subject.kind(), ...Object.fromEntries(fields.map(field => [field, subject[field]])) };
    assert(owned.delete(subject)); // Successful conversion consumes this handle.
    const result = keep(pair.compareSubject(subject));
    assert.throws(() => subject.kind());
    const bytes = result.canonicalBytes();
    assert(bytes instanceof Uint8Array);
    const wire = JSON.parse(new TextDecoder().decode(bytes));
    assert.equal(result.receiptDigest, pair.receiptDigest);
    assert.equal(wire.receiptDigest, result.receiptDigest);
    assert.equal(result.membership(), bindings.WorkspaceSubjectMembership[membership]);
    assert.equal(wire.values.membership, { Absent: "absent", PriorOnly: "prior_only", CurrentOnly: "current_only", Shared: "shared" }[membership]);
    const copied = keep(result.subject());
    assert.equal(copied.kind(), expectedSubject.kind);
    for (const field of fields) {
      assert.equal(copied[field], expectedSubject[field]);
    }
    assert(owned.delete(copied));
    const repeated = keep(pair.compareSubject(copied));
    assert.throws(() => copied.kind());
    assert.deepEqual(repeated.canonicalBytes(), bytes);
    release(repeated);
    const expectedFields = ["membership"];
    if (["PriorOnly", "Shared"].includes(membership)) {
      expectedFields.push("priorFact"); assertFact(result.priorFact(), wire.values.priorFact);
    } else { assert.equal(result.priorFact(), undefined); }
    if (["CurrentOnly", "Shared"].includes(membership)) {
      expectedFields.push("currentFact"); assertFact(result.currentFact(), wire.values.currentFact);
    } else { assert.equal(result.currentFact(), undefined); }
    if (membership === "Shared") {
      expectedFields.push("comparison");
      assert.equal(result.comparison(), bindings.WorkspaceValueComparison[equality]);
      assert.equal(wire.values.comparison, equality.toLowerCase());
    } else { assert.equal(result.comparison(), undefined); }
    assert.deepEqual(Object.keys(wire.values).sort(), expectedFields.sort());
    result.canonicalBytes().fill(0);
    assert.deepEqual(result.canonicalBytes(), bytes);
    return result;
  }
  try {
    const prior = compile(replaceRoot(root.bytes));
    const commented = compile(replaceRoot(Buffer.concat([root.bytes, Buffer.from("\n# Source-only review change.\n")])));
    const pair = compare(prior, commented);
    assert.equal(prior.semanticIrDigest, commented.semanticIrDigest);
    const rootsPage = pair.changeRoots(0, 256);
    assert.equal(rootsPage.nextOffset, undefined);
    const roots = entries(rootsPage);
    assert.equal(roots.length, 1);
    const source = keep(roots[0].subject());
    const changed = answer(pair, keep(roots[0].subject()), "Shared", "Changed");
    assert.deepEqual(factData(changed.priorFact()), factData(roots[0].priorFact()));
    assert.deepEqual(factData(changed.currentFact()), factData(roots[0].currentFact()));
    assert.notEqual(factData(changed.priorFact()).sourceDigest, factData(changed.currentFact()).sourceDigest);
    assert.equal(JSON.parse(Buffer.from(changed.canonicalBytes())).subject.sourceId, source.sourceId);

    const impacts = entries(pair.impacts(0, 256));
    const workspaceImpact = impacts.find((impact) => {
      const subject = impact.subject();
      try { return subject.kind() === bindings.WorkspaceSubjectKind.Workspace; }
      finally { subject.free(); }
    });
    assert(workspaceImpact);
    assert.equal(workspaceImpact.state(), bindings.WorkspaceImpactState.Affected);
    const workspaceSubject = keep(workspaceImpact.subject());
    const equal = answer(pair, keep(workspaceImpact.subject()), "Shared", "Equal");
    assert.deepEqual(factData(equal.priorFact()), factData(equal.currentFact()));
    assert.equal(factData(equal.currentFact()).workspaceSemanticIrDigest, commented.semanticIrDigest);
    const explanation = keep(pair.explain(workspaceSubject));
    assert.equal(explanation.state(), bindings.WorkspaceExplanationState.Affected);
    const explanationBytes = explanation.canonicalBytes();
    const explanationWire = JSON.parse(Buffer.from(explanationBytes));
    assert.equal(explanationWire.state, "affected");
    assert.equal(explanationWire.subject.workspaceId, prior.workspaceId);
    assert(explanationWire.causal);
    const explanationRootValue = explanation.root();
    assert(explanationRootValue);
    const explanationRoot = keep(explanationRootValue);
    assert.equal(explanationWire.causal.root.rootDigest, explanationRoot.rootDigest);
    const explanationSteps = explanation.steps().map(keep);
    assert.deepEqual(
      explanationWire.causal.steps.map((step) => step.nodeDigest),
      explanationSteps.map((step) => step.nodeDigest),
    );
    assert.equal(explanationWire.causal.frontier, {
      [bindings.WorkspaceExplanationFrontier.Predecessor]: "predecessor",
      [bindings.WorkspaceExplanationFrontier.Current]: "current",
    }[explanation.frontier()]);
    for (const step of explanationSteps) release(step);
    release(explanationRoot);
    const repeatedExplanation = keep(pair.explain(workspaceSubject));
    assert.deepEqual(repeatedExplanation.canonicalBytes(), explanationBytes);
    release(repeatedExplanation);
    const retainedExplanationBytes = explanation.canonicalBytes();
    explanationBytes.fill(0);
    assert.deepEqual(explanation.canonicalBytes(), retainedExplanationBytes);
    assert.equal(JSON.parse(Buffer.from(equal.canonicalBytes())).subject.workspaceId, prior.workspaceId);

    // Captured empty declarations add source custody, not native readiness.
    const composition = Buffer.from(`schema = "pale/workspace-composition-source.v1"
schema_version = 1
[declarations]
responsibility_contracts = []
participant_provider_manifests = []
provider_bindings = []
entry_contracts = []
request_translations = []
result_translations = []
termination_contracts = []
settlement_contracts = []
continuity_contracts = []
edge_contracts = []
selected_experience_paths = []
`);
    const selectedText = root.bytes.toString().replace("experience_compositions = []", 'experience_compositions = ["composition.toml"]');
    assert.notEqual(selectedText, root.bytes.toString());
    const selectedInput = replaceRoot(Buffer.from(selectedText));
    selectedInput.documents.push({ label: "composition.toml", bytes: composition });
    const selected = compile(selectedInput);
    const forward = compare(prior, selected);
    const reverse = compare(selected, prior);
    const addedRoot = entries(forward.changeRoots(0, 256)).find((entry) => entry.kind() === bindings.WorkspaceChangeKind.Added);
    assert(addedRoot);
    const addedSubject = keep(addedRoot.subject());
    const added = answer(forward, keep(addedRoot.subject()), "CurrentOnly");
    const removed = answer(reverse, keep(addedRoot.subject()), "PriorOnly");
    assert.deepEqual(factData(added.currentFact()), factData(removed.priorFact()));
    assert.equal(factData(added.currentFact()).sourcePath, "composition.toml");
    answer(pair, keep(addedRoot.subject()), "Absent");
    const absentExplanation = keep(pair.explain(addedSubject));
    assert.equal(absentExplanation.state(), bindings.WorkspaceExplanationState.NotPresent);

    const renamed = compile(replaceRoot(root.bytes, "renamed.toml"));
    const renamedPair = compare(prior, renamed);
    const renamedResult = answer(renamedPair, keep(roots[0].subject()), "Shared", "Changed");
    const renamedPrior = factData(renamedResult.priorFact());
    const renamedCurrent = factData(renamedResult.currentFact());
    assert.equal(renamedPrior.sourceDigest, renamedCurrent.sourceDigest);
    assert.equal(renamedPrior.byteLen, renamedCurrent.byteLen);
    assert.equal(renamedPrior.sourcePath, root.label);
    assert.equal(renamedCurrent.sourcePath, "renamed.toml");

    const freed = keep(changed.subject()); release(freed);
    const consumed = changed.subject();
    const firstUse = keep(pair.compareSubject(consumed)); release(firstUse);
    for (const invalid of [{}, null, undefined, prior, new bindings.WorkspaceSubject(), freed, consumed]) {
      const receiver = compare(prior, commented);
      assert.throws(() => receiver.compareSubject(invalid));
      const recovered = keep(receiver.compareSubject(changed.subject()));
      assert.deepEqual(recovered.canonicalBytes(), changed.canonicalBytes());
      release(recovered);
      release(receiver); // Every rejection must leave its receiver freeable.
    }
    const emptyResult = new bindings.WorkspaceSubjectComparison();
    assert.throws(() => emptyResult.membership());

    const retainedFact = changed.currentFact();
    const retainedBytes = changed.canonicalBytes();
    release(pair); release(prior); release(commented); release(api);
    assert.deepEqual(explanation.canonicalBytes(), retainedExplanationBytes);
    assert.deepEqual(changed.canonicalBytes(), retainedBytes);
    const copiedAfterParent = keep(changed.subject());
    assert.equal(copiedAfterParent.sourceId, source.sourceId);
    release(changed);
    assert.equal(factData(retainedFact).sourcePath, root.label);
    assert.throws(() => changed.canonicalBytes());
    return "workspace.portable-subject-comparison";
  } finally {
    for (const value of [...owned].reverse()) release(value);
  }
}
