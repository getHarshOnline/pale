import assert from "node:assert/strict";

// Shared by actual Node glue and the packed bundler consumer, not a mock ABI.
export function verifyConsumedArguments(bindings, portable, contract) {
  const document = () => new bindings.PortableWorkspaceDocument("unused.toml", new Uint8Array());
  const request = () => new bindings.PortableWorkspaceRequest(portable.rootManifest,
    portable.documents.map(({ label, bytes }) =>
      new bindings.PortableWorkspaceDocument(label, new Uint8Array(bytes))));
  const projection = () => new bindings.DiagnosticProjection(contract.projection.code,
    bindings.DiagnosticStage.InputAdmission, bindings.DiagnosticDisposition.InvalidInput,
    contract.projection.summary, bindings.DiagnosticRetry.AfterCorrection,
    bindings.DiagnosticEffectCertainty.NotApplicable);
  const apiProbe = (api) => {
    const identity = api.contractBundleIdentity();
    assert(identity.memberCount > 0);
    identity.free();
  };
  const compile = (api) => api.compilePortableWorkspace(request());
  const invalid = (Class, fresh, consume, kind) => {
    if (kind === "wrong-class") {
      const value = new bindings.Pale();
      return { value, release: () => value.free() };
    }
    if (kind === "empty") return { value: Object.create(Class.prototype), release() {} };
    if (kind === "freed" || kind === "consumed") {
      const value = fresh();
      if (kind === "freed") value.free();
      else consume(value);
      return { value, release() {} };
    }
    return { value: { object: {}, null: null, omitted: undefined }[kind], release() {} };
  };
  const kinds = ["object", "null", "omitted", "wrong-class", "empty", "freed", "consumed"];
  for (const [Class, fresh, call, verify] of [
    [bindings.PortableWorkspaceRequest, request,
      (api, value) => api.compilePortableWorkspace(value),
      (value) => { assert(value.workspaceId.length > 0); value.free(); }],
    [bindings.DiagnosticProjection, projection,
      (api, value) => api.projectDiagnostic(value),
      (value) => assert.equal(new TextDecoder().decode(value), contract.canonical_diagnostic)],
  ]) {
    for (const kind of kinds) {
      const api = new bindings.Pale();
      const bad = invalid(Class, fresh, (value) => verify(call(api, value)), kind);
      assert.throws(() => call(api, bad.value), Error);
      bad.release();
      apiProbe(api);
      const good = fresh();
      verify(call(api, good));
      assert.throws(() => good.free(), Error); // The existing successful call consumes it.
      // A lawful read alone cannot detect a stranded receiver borrow.
      assert.doesNotThrow(() => api.free(), `${Class.name}:${kind} receiver release`);
    }
  }

  const field = () => bindings.AuthoringPathSegment.field("workspace");
  const instance = () => bindings.AuthoringInstancePathSegment.field("workspace");
  for (const mode of ["standalone", "inspection", "navigation"]) {
    for (const later of [false, true]) for (const kind of kinds) {
      const api = new bindings.Pale();
      const receiver = mode === "standalone" ? api : compile(api);
      const make = mode === "navigation" ? instance : field;
      const call = (path) => mode === "standalone"
        ? receiver.inspectStandaloneAuthoring(bindings.StandaloneAuthoringRole.WorkspaceManifest, path)
        : mode === "inspection" ? receiver.inspectAuthoring(portable.rootManifest, path)
          : receiver.navigateAuthoring(portable.rootManifest, path);
      const prefix = later ? make() : undefined;
      const suffix = make();
      const Class = mode === "navigation" ? bindings.AuthoringInstancePathSegment : bindings.AuthoringPathSegment;
      const bad = invalid(Class, make, value => call([value]).free(), kind);
      assert.throws(() => call([...(prefix ? [prefix] : []), bad.value, suffix]), Error);
      bad.release();
      if (prefix) assert.throws(() => prefix.free(), Error);
      suffix.free(); // No conversion after the rejected element.
      call([]).free();
      if (receiver !== api) receiver.free();
      apiProbe(api); api.free();
    }
  }

  for (const later of [false, true]) for (const kind of kinds) {
    const prefix = later ? document() : undefined;
    const suffix = document();
    const bad = invalid(bindings.PortableWorkspaceDocument, document,
      value => new bindings.PortableWorkspaceRequest(portable.rootManifest, [value]).free(), kind);
    assert.throws(() => new bindings.PortableWorkspaceRequest(portable.rootManifest,
      [...(prefix ? [prefix] : []), bad.value, suffix]), Error);
    bad.release();
    if (prefix) assert.throws(() => prefix.free(), Error);
    suffix.free();
    const api = new bindings.Pale(); compile(api).free(); api.free();
  }
}
