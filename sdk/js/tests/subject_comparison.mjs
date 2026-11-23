import assert from "node:assert/strict";
import { exerciseReverseQueries } from "./reverse_queries.mjs";
import { exerciseForwardDiscovery } from "./forward_discovery.mjs";

// Compare actual compiler results, not fabricated handles or an emulated graph.
export function exerciseSubjectComparison(bindings, api, request) {
  const compile = (input) => api.compilePortableWorkspace(input);
  const original = request();
  const root = original.documents.find(({ label }) => label === original.rootManifest);
  assert(root);
  const replaceRoot = (bytes, label = root.label) => ({
    rootManifest: label,
    documents: original.documents.map((document) => document === root ? { label, bytes } : document),
  });
  const prior = compile(original);
  const commented = compile(replaceRoot(Buffer.concat([root.bytes, Buffer.from("\n# Source-only review change.\n")])));
  const comparison = api.comparePortableWorkspaces(prior, commented);
  const summary = comparison.summary();
  assert.equal(summary.priorPrincipalUniverseDigest, prior.principalUniverseDigest);
  assert.equal(summary.currentPrincipalUniverseDigest, commented.principalUniverseDigest);
  assert.equal(summary.priorSemanticIrDigest, prior.semanticIrDigest);
  assert.equal(summary.currentSemanticIrDigest, commented.semanticIrDigest);
  assert.equal(summary.receiptDigest, comparison.receiptDigest);
  assert.equal(summary.priorSemanticIrDigest, summary.currentSemanticIrDigest);

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
  function assertFact(fact, wire) {
    const { kind, ...payload } = wire;
    assert.equal(fact.kind, factKinds[kind]);
    const actual = Object.fromEntries(Object.entries(fact)
      .filter(([name, value]) => name !== "kind" && value != null));
    assert.deepEqual(actual, payload);
  }
  function answer(owner, subject, membership, expectedComparison) {
    const result = owner.compareSubject(subject);
    const wire = JSON.parse(result.canonicalBytes);
    assert.equal(result.receiptDigest, owner.receiptDigest);
    assert.equal(wire.receiptDigest, result.receiptDigest);
    const copiedSubject = result.subject();
    for (const name of ["kind", "workspaceId", "sourceId", "componentId", "moduleId",
      "ownerModuleId", "declarationNamespaceId", "declarationId", "importerModuleId",
      "upstreamModuleId", "upstreamReleaseId", "exportId", "sourceNamespaceId",
      "localNamespaceId", "extensionNamespaceId", "resourceId"]) {
      assert.equal(copiedSubject[name], subject[name]);
    }
    const values = result.values;
    assert.equal(values.membership, membership);
    assert.equal(wire.values.membership, membership.replaceAll("-", "_"));
    const fields = ["membership"];
    if (["prior-only", "shared"].includes(membership)) {
      fields.push("prior");
      assertFact(values.prior, wire.values.priorFact);
    } else {
      assert(!Object.hasOwn(values, "prior"));
      assert(!Object.hasOwn(wire.values, "priorFact"));
    }
    if (["current-only", "shared"].includes(membership)) {
      fields.push("current");
      assertFact(values.current, wire.values.currentFact);
    } else {
      assert(!Object.hasOwn(values, "current"));
      assert(!Object.hasOwn(wire.values, "currentFact"));
    }
    if (membership === "shared") {
      fields.push("comparison");
      assert.equal(values.comparison, expectedComparison);
      assert.equal(wire.values.comparison,
        expectedComparison === bindings.WorkspaceValueComparison.Equal ? "equal" : "changed");
    } else {
      assert(!Object.hasOwn(values, "comparison"));
      assert(!Object.hasOwn(wire.values, "comparison"));
    }
    assert.deepEqual(Object.keys(values).sort(), fields.sort());
    const bytes = Buffer.from(result.canonicalBytes);
    assert(Buffer.isBuffer(result.canonicalBytes));
    assert.deepEqual(owner.compareSubject(copiedSubject).canonicalBytes, bytes);
    result.canonicalBytes.fill(0);
    values.membership = "caller-modified";
    if (values.prior) values.prior.sourceDigest = "caller-modified";
    assert.deepEqual(result.canonicalBytes, bytes);
    assert.equal(result.values.membership, membership);
    if (wire.values.priorFact) assertFact(result.values.prior, wire.values.priorFact);
    return result;
  }
  const roots = comparison.changeRoots(0, 256);
  assert.equal(roots.nextOffset, undefined);
  assert.equal(roots.entries.length, 1);
  const rootBytes = Buffer.from(roots.canonicalBytes);
  const rootWire = JSON.parse(rootBytes);
  assert.equal(rootWire.entries.length, roots.entries.length);
  assert.equal(rootWire.nextOffset, null);
  roots.canonicalBytes.fill(0);
  assert.deepEqual(roots.canonicalBytes, rootBytes);
  assert.deepEqual(comparison.changeRoots(0, 256).canonicalBytes, rootBytes);
  const detachedRoots = roots.entries;
  const rootNextOffset = roots.nextOffset;
  detachedRoots.length = 0;
  assert.equal(roots.entries.length, 1);
  assert.equal(roots.nextOffset, rootNextOffset);
  assert.deepEqual(roots.canonicalBytes, rootBytes);
  assert.throws(() => new bindings.WorkspaceChangeRootPage());
  const sourceRoot = roots.entries[0];
  const source = sourceRoot.subject();
  const changed = answer(comparison, source, "shared", bindings.WorkspaceValueComparison.Changed);
  assert.deepEqual(changed.values.prior, sourceRoot.priorFact());
  assert.deepEqual(changed.values.current, sourceRoot.currentFact());
  assert.notEqual(changed.values.prior.sourceDigest, changed.values.current.sourceDigest);
  assert.equal(JSON.parse(changed.canonicalBytes).subject.sourceId, source.sourceId);

  const impacts = comparison.impacts(0, 256);
  assert.equal(impacts.nextOffset, undefined);
  const workspaceImpact = impacts.entries.find((impact) => impact.subject().kind === bindings.WorkspaceSubjectKind.Workspace);
  assert(workspaceImpact);
  assert.equal(workspaceImpact.state, bindings.WorkspaceImpactState.Affected);
  const equal = answer(comparison, workspaceImpact.subject(), "shared", bindings.WorkspaceValueComparison.Equal);
  assert.deepEqual(equal.values.prior, equal.values.current);
  assert.equal(equal.values.current.workspaceSemanticIrDigest, commented.semanticIrDigest);
  assert.equal(comparison.explain(equal.subject()).state, bindings.WorkspaceExplanationState.Affected);
  assert.equal(JSON.parse(equal.canonicalBytes).subject.workspaceId, prior.workspaceId);

  // These are captured fixture bytes only. Selecting the empty declaration set
  // adds source custody, without provider admission or a native action.
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
  const rootWithComposition = root.bytes.toString().replace(
    "experience_compositions = []", 'experience_compositions = ["composition.toml"]');
  assert.notEqual(rootWithComposition, root.bytes.toString());
  const selectedInput = replaceRoot(Buffer.from(rootWithComposition));
  selectedInput.documents.push({ label: "composition.toml", bytes: composition });
  const selected = compile(selectedInput);
  const forward = api.comparePortableWorkspaces(prior, selected);
  const reverse = api.comparePortableWorkspaces(selected, prior);
  const addedRoot = forward.changeRoots(0, 256).entries.find((entry) => entry.kind === bindings.WorkspaceChangeKind.Added);
  assert(addedRoot);
  const addedSubject = addedRoot.subject();
  const added = answer(forward, addedSubject, "current-only");
  const removed = answer(reverse, addedSubject, "prior-only");
  assert.deepEqual(added.values.current, removed.values.prior);
  assert.equal(added.values.current.sourcePath, "composition.toml");
  const absent = answer(comparison, addedSubject, "absent");
  assert.deepEqual(JSON.parse(absent.canonicalBytes).values, { membership: "absent" });

  const renamed = compile(replaceRoot(root.bytes, "renamed.toml"));
  const renamedPair = api.comparePortableWorkspaces(prior, renamed);
  const renamedValue = answer(renamedPair, source, "shared", bindings.WorkspaceValueComparison.Changed).values;
  assert.equal(renamedValue.prior.sourceDigest, renamedValue.current.sourceDigest);
  assert.equal(renamedValue.prior.byteLen, renamedValue.current.byteLen);
  assert.equal(renamedValue.prior.sourcePath, root.label);
  assert.equal(renamedValue.current.sourcePath, "renamed.toml");
  assert.throws(() => comparison.compareSubject({}));
  assert.throws(() => comparison.compareSubject(prior));
  assert.throws(() => new bindings.WorkspaceSubjectComparison());
  return ["workspace.portable-subject-comparison",
    ...exerciseReverseQueries(bindings, api, request, prior, selected, addedSubject, assertFact),
    ...exerciseForwardDiscovery(bindings, api, request, prior, selected, addedSubject, assertFact)];
}
