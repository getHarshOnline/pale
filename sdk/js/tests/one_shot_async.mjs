import assert from "node:assert/strict";

// Exercise the actual scheduling/ownership boundary without a timing threshold.
export async function exerciseOneShotCompilation(bindings, api, request) {
  const copyDocument = ({ label, bytes }) => ({ label, bytes: Buffer.from(bytes) });
  const copy = {
    ...request,
    workspace: { ...request.workspace, documents: request.workspace.documents.map(copyDocument) },
    packageDocuments: request.packageDocuments.map(copyDocument),
    privateInputs: {
      capabilities: copyDocument(request.privateInputs.capabilities),
      machineBinding: copyDocument(request.privateInputs.machineBinding),
      distributionLock: copyDocument(request.privateInputs.distributionLock),
      lifecycleGuidance: request.privateInputs.lifecycleGuidance.map(copyDocument),
    },
  };
  // Neither the JS façade nor its mutable buffers need to survive enqueue.
  let owner = new bindings.Pale();
  const pending = owner.compileOneShot(copy);
  assert(pending instanceof Promise);
  assert.throws(() => api.oneShotSummary(pending), (error) => {
    assert(error instanceof Error);
    assert.equal(error.diagnostic, undefined);
    return true;
  });
  const concurrent = api.compileOneShot(request);
  owner = undefined;
  for (const document of [
    ...copy.workspace.documents, ...copy.packageDocuments,
    copy.privateInputs.capabilities, copy.privateInputs.machineBinding,
    copy.privateInputs.distributionLock, ...copy.privateInputs.lifecycleGuidance,
  ]) {
    document.bytes.fill(0);
    document.label = "changed-after-enqueue";
  }
  copy.projectId = "changed-after-enqueue";
  copy.workspace.rootManifest = "changed-after-enqueue";
  assert.equal(typeof globalThis.gc, "function", "run the ABI gate with --expose-gc");
  globalThis.gc();
  const compilation = await pending;
  assert(compilation instanceof bindings.OneShotCompilation);
  globalThis.gc();

  // Concurrent successful jobs retain independent owned requests/results.
  const other = await concurrent;
  assert.deepEqual(api.oneShotSummary(compilation).canonicalBytes, api.oneShotSummary(other).canonicalBytes);

  // The same rejected portable input reaches the same canonical API owner
  // through synchronous portable compilation and asynchronous one-shot work.
  const workspace = { ...request.workspace, rootManifest: "missing.toml" };
  let expected;
  assert.throws(() => api.compilePortableWorkspace(workspace), (error) => {
    expected = Buffer.from(error.diagnostic.canonicalBytes);
    return true;
  });
  const rejected = api.compileOneShot({ ...request, workspace });
  assert(rejected instanceof Promise);
  await assert.rejects(rejected, (error) => {
    assert(error instanceof Error);
    assert.deepEqual(Buffer.from(error.diagnostic.canonicalBytes), expected);
    assert.equal(error.message, error.diagnostic.summary);
    assert.equal(error.diagnostic.effect, bindings.DiagnosticEffectCertainty.NotApplicable);
    return true;
  });
  return compilation;
}
