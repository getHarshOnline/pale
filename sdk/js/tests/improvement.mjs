import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { assertIntendedUseExplanation, assertProfileEntry, assertProfilePage } from "./effective_profile.mjs";

// These are API-owned fixture inputs and expected projections, not a second
// candidate compiler or diagnostic taxonomy.
export function exerciseConfigurationImprovement(bindings, fixtureRoot, api, baseline, profile) {
  const vector = JSON.parse(fs.readFileSync(path.join(fixtureRoot, "improvement-journey.json"), "utf8"));
  const selectors = JSON.parse(fs.readFileSync(path.join(fixtureRoot, "one-shot-journey.json"), "utf8")).selectors;
  const root = path.join(fixtureRoot, "one-shot");
  const document = (label, name) => ({ label, bytes: fs.readFileSync(path.join(root, "private", name)) });
  const workspace = () => ({
    rootManifest: "pale-workspace.toml",
    documents: ["composition.toml", "pale-workspace.toml", "workspace-resolution.lock.toml", "workspace-universe.lock.toml"].map((label) => ({
      label,
      bytes: Buffer.from(fs.readFileSync(path.join(root, "workspace", label), "utf8")
        .replace('access = "mutate"', `access = "${vector.candidate_repository_access}"`)),
    })),
  });
  const request = () => ({
    projectId: selectors.project_id,
    executionWorkspaceId: selectors.execution_workspace_id,
    machineId: selectors.machine_id,
    privateInputs: {
      capabilities: document("capabilities.toml", "capabilities.toml"),
      machineBinding: {
        label: "machines/machine.toml",
        bytes: Buffer.from(fs.readFileSync(path.join(root, "private/machine-binding.toml"), "utf8")
          .replace("/captured/repository", vector.candidate_machine_root)),
      },
      distributionLock: document("distribution-lock.toml", "distribution-lock.toml"),
      lifecycleGuidance: [],
    },
  });
  const { ConfigurationImprovementPhase: Phase, ConfigurationImprovementDisposition: Disposition } = bindings;
  const consumed = (call) => assert.throws(call, (error) => {
    assert.equal(error.code, "InvalidArg");
    assert.match(error.message, /already been consumed/);
    assert.equal(error.diagnostic, undefined);
    return true;
  });
  const canonical = (value) => JSON.parse(Buffer.from(value.canonicalBytes).toString());
  const diagnostic = (call, expected) => assert.throws(call, (error) => {
    assert.deepEqual(canonical(error.diagnostic), expected);
    assert.equal(error.message, error.diagnostic.summary);
    assert.equal(error.diagnostic.effect, bindings.DiagnosticEffectCertainty.NotApplicable);
    return true;
  });
  const camel = (value) => value.replace(/_([a-z])/g, (_, char) => char.toUpperCase());
  const summary = (actual) => {
    assert.deepEqual(canonical(actual), vector.summary);
    for (const [name, expected] of Object.entries(vector.summary)) {
      const value = actual[camel(name)];
      if (!name.endsWith("_change")) {
        assert.equal(value, expected, name);
      } else if (expected.state === "unchanged") {
        assert.equal(value.kind, bindings.ConfigurationImprovementCoordinateChangeKind.Unchanged);
        assert.equal(value.digest, expected.digest);
        assert.equal(value.priorDigest, undefined);
        assert.equal(value.candidateDigest, undefined);
      } else {
        assert.equal(value.kind, bindings.ConfigurationImprovementCoordinateChangeKind.Changed);
        assert.equal(value.digest, undefined);
        assert.equal(value.priorDigest, expected.prior_digest);
        assert.equal(value.candidateDigest, expected.candidate_digest);
      }
    }
  };
  const explainBaseline = () => {
    const inspected = api.inspectConfigurationImprovement(baseline);
    assert.equal(inspected.phase, Phase.Inspected);
    assert.equal(inspected.workspaceId, baseline.workspaceId);
    assert.equal(inspected.principalUniverseDigest, baseline.principalUniverseDigest);
    const explained = api.explainConfigurationImprovement(inspected, profile);
    assert.equal(explained.phase, Phase.Explained);
    assert.equal(explained.profileDigest, profile.profileDigest);
    assert.equal(explained.instanceDigest, profile.instanceDigest);
    consumed(() => api.explainConfigurationImprovement(inspected, profile));
    consumed(() => inspected.phase);
    consumed(() => inspected.workspaceId);
    consumed(() => inspected.principalUniverseDigest);
    return explained;
  };
  const candidate = api.compilePortableWorkspace(workspace());
  const prepare = (explanation) => api.prepareConfigurationImprovementPreview(explanation, candidate, request());

  // Borrowed failure retains the exact explanation, then succeeds without
  // compiling the candidate again.
  const retry = explainBaseline();
  diagnostic(() => api.prepareConfigurationImprovementPreview(retry, candidate, {
    ...request(), projectId: "unselected",
  }), vector.diagnostics.context_mismatch);
  assert.equal(retry.phase, Phase.Explained);
  const prepared = prepare(retry);
  summary(prepared.summary());
  const retainedSummary = prepared.summary();
  const preview = api.commitConfigurationImprovementPreview(retry, prepared);
  consumed(() => api.commitConfigurationImprovementPreview(retry, prepared));
  consumed(() => prepared.summary());
  consumed(() => retry.phase);
  consumed(() => retry.profileDigest);
  consumed(() => retry.instanceDigest);
  assert.equal(preview.phase, Phase.Previewed);
  summary(preview.summary());
  assert.deepEqual(Buffer.from(retainedSummary.canonicalBytes), Buffer.from(preview.summary().canonicalBytes));

  const comparison = preview.comparisonSummary();
  assert.deepEqual(canonical(comparison), vector.comparison);
  for (const [name, expected] of Object.entries(vector.comparison)) assert.equal(comparison[camel(name)], expected);
  let offset = 0;
  let roots = 0;
  do {
    const page = preview.changeRoots(offset, 1);
    roots += page.entries.length;
    offset = page.nextOffset ?? null;
  } while (offset !== null);
  assert.equal(roots, comparison.changeRootCount);
  let impacts = 0;
  let affected = 0;
  offset = 0;
  do {
    const page = preview.impacts(offset, 2);
    for (const impact of page.entries) {
      const explanation = preview.explain(impact.subject());
      assert.notEqual(explanation.state, bindings.WorkspaceExplanationState.NotPresent);
      if (impact.state === bindings.WorkspaceImpactState.Affected) affected += 1;
    }
    impacts += page.entries.length;
    offset = page.nextOffset ?? null;
  } while (offset !== null);
  assert.equal(impacts, comparison.affectedCount + comparison.unaffectedCount);
  assert.equal(affected, comparison.affectedCount);

  const baselineProfile = preview.baselineProfile();
  const candidateProfile = preview.candidateProfile();
  assert.equal(baselineProfile.instanceDigest, vector.summary.instance_digest);
  assert.equal(candidateProfile.instanceDigest, vector.summary.candidate_instance_digest);
  for (const retained of [baselineProfile, candidateProfile]) {
    const page = retained.inspect(bindings.EffectiveProfileSection.IntendedUseCase, 0, 1);
    const value = assertProfilePage(bindings, page);
    assert(value.entries.length > 0);
    assertIntendedUseExplanation(bindings, retained, value.entries[0]);
  }
  const baselineIdentity = baselineProfile.navigateAuthoring("pale-workspace.toml", [
    bindings.AuthoringInstancePathSegment.field("workspace"),
    bindings.AuthoringInstancePathSegment.field("desired_topology"),
    bindings.AuthoringInstancePathSegment.field("projects"),
    bindings.AuthoringInstancePathSegment.index(0),
    bindings.AuthoringInstancePathSegment.field("run_policy"),
  ]).identity();
  const candidateJoin = candidateProfile.inspectEffectiveAuthoring(baselineIdentity);
  assert.deepEqual(candidateJoin.coordinates, candidateProfile.coordinates());
  assert(candidateJoin.entries.length > 0);
  const candidateDetail = candidateJoin.entries[0].detail;
  assert.equal(candidateDetail.kind, "selection");
  assert.equal(candidateDetail.selection.runPolicyId, "standard");

  // Every consuming checkpoint is callable and linear.
  const explanation = explainBaseline();
  const proposed = api.proposeConfigurationImprovement(explanation, { ...request(), workspace: workspace() });
  assert.equal(proposed.phase, Phase.Proposed);
  consumed(() => api.proposeConfigurationImprovement(explanation, { ...request(), workspace: workspace() }));
  const evaluated = api.evaluateConfigurationImprovement(proposed);
  assert.equal(evaluated.phase, Phase.Evaluated);
  assert.equal(evaluated.candidatePrincipalUniverseDigest, vector.summary.candidate_principal_universe_digest);
  assert.equal(evaluated.candidateProfileDigest, vector.summary.candidate_profile_digest);
  assert.equal(evaluated.candidateInstanceDigest, vector.summary.candidate_instance_digest);
  consumed(() => api.evaluateConfigurationImprovement(proposed));
  const diagnosed = api.diagnoseConfigurationImprovement(evaluated);
  assert.equal(diagnosed.phase, Phase.Diagnosed);
  assert.equal(diagnosed.derivationReceiptDigest, vector.summary.derivation_receipt_digest);
  consumed(() => api.diagnoseConfigurationImprovement(evaluated));
  for (const name of ["candidatePrincipalUniverseDigest", "candidateProfileDigest", "candidateInstanceDigest"]) {
    consumed(() => evaluated[name]);
  }
  const stagedPreview = api.previewConfigurationImprovement(diagnosed);
  consumed(() => api.previewConfigurationImprovement(diagnosed));
  consumed(() => diagnosed.derivationReceiptDigest);
  summary(stagedPreview.summary());
  assert.deepEqual(Buffer.from(stagedPreview.summary().canonicalBytes), Buffer.from(preview.summary().canonicalBytes));

  const invalidProposal = api.proposeConfigurationImprovement(explainBaseline(), {
    ...request(), projectId: "unselected", workspace: workspace(),
  });
  diagnostic(() => api.evaluateConfigurationImprovement(invalidProposal), vector.diagnostics.context_mismatch);
  consumed(() => invalidProposal.phase);

  // Equal coordinates do not permit splicing distinct lifecycle owners.
  const original = explainBaseline();
  const foreign = explainBaseline();
  const originalPrepared = prepare(original);
  assert.equal(original.instanceDigest, foreign.instanceDigest);
  diagnostic(() => api.commitConfigurationImprovementPreview(foreign, originalPrepared),
    vector.diagnostics.preparation_owner_mismatch);
  consumed(() => foreign.phase);
  consumed(() => originalPrepared.summary());
  assert.equal(original.phase, Phase.Explained);
  const afterMismatch = api.commitConfigurationImprovementPreview(original, prepare(original));

  // Checking both host arguments before taking either preserves remaining
  // custody if another argument has already been consumed.
  const untouched = explainBaseline();
  consumed(() => api.commitConfigurationImprovementPreview(untouched, originalPrepared));
  assert.equal(untouched.phase, Phase.Explained);
  const fourth = api.commitConfigurationImprovementPreview(untouched, prepare(untouched));
  for (const [value, finish, expected] of [
    [preview, "accept", Disposition.Accepted],
    [stagedPreview, "reject", Disposition.Rejected],
    [afterMismatch, "cancel", Disposition.Cancelled],
    [fourth, "markStale", Disposition.Stale],
  ]) {
    const terminal = value[finish]();
    assert.equal(terminal.disposition, expected);
    summary(terminal.preview());
    for (const method of ["accept", "reject", "cancel", "markStale"]) consumed(() => value[method]());
    consumed(() => value.summary());
    consumed(() => value.impacts(0, 1));
  }
  // Immutable read handles survive preview disposition but cannot recreate
  // its consuming authority.
  assert.equal(candidateProfile.instanceDigest, vector.summary.candidate_instance_digest);
  summary(retainedSummary);
  return vector.capability_id;
}

// The native local route must return the same retained review owner as the
// source-first stages. These filesystem helpers only capture the fixture bytes.
export async function exerciseLocalConfigurationImprovement(bindings, fixtureRoot, api, createWorkspace, runtime) {
  const baseline = createWorkspace(fixtureRoot, runtime);
  const candidate = createWorkspace(fixtureRoot, runtime);
  const selectors = JSON.parse(fs.readFileSync(path.join(fixtureRoot, "one-shot-journey.json"), "utf8")).selectors;
  const request = {
    baselineWorkspaceManifest: baseline.manifest,
    candidateWorkspaceManifest: candidate.manifest,
    projectId: selectors.project_id,
    executionWorkspaceId: selectors.execution_workspace_id,
    machineId: selectors.machine_id,
  };
  const documents = (root) => ({
    rootManifest: "pale-workspace.toml",
    documents: ["composition.toml", "pale-workspace.toml", "workspace-resolution.lock.toml", "workspace-universe.lock.toml"]
      .map((label) => ({ label, bytes: fs.readFileSync(path.join(root, label)) })),
  });
  const privateInputs = (root) => {
    const document = (label) => ({ label, bytes: fs.readFileSync(path.join(root, label)) });
    return {
      capabilities: document("capabilities.toml"),
      machineBinding: document("machines/machine.toml"),
      distributionLock: document("distribution-lock.toml"),
      lifecycleGuidance: [],
    };
  };
  const snapshot = (root, prefix = "") => fs.readdirSync(path.join(root, prefix), { withFileTypes: true })
    .sort((left, right) => left.name.localeCompare(right.name))
    .flatMap((entry) => {
      const label = path.join(prefix, entry.name);
      return entry.isDirectory()
        ? [[`${label}/`, null], ...snapshot(root, label)]
        : [[label, fs.readFileSync(path.join(root, label)).toString("base64")]];
    });
  try {
    const candidateManifest = candidate.manifest;
    const original = fs.readFileSync(candidateManifest, "utf8");
    assert(original.includes('access = "mutate"'));
    fs.writeFileSync(candidateManifest, original.replace('access = "mutate"', 'access = "read"'));
    fs.rmSync(path.join(candidate.root, "distribution/packages"), { recursive: true });
    const before = [snapshot(baseline.root), snapshot(candidate.root)];

    const portable = api.compilePortableWorkspace(documents(baseline.root));
    const captured = {
      projectId: request.projectId, executionWorkspaceId: request.executionWorkspaceId,
      machineId: request.machineId, privateInputs: privateInputs(baseline.root),
      packageDocuments: ["package.toml", "role.md", "roles.toml"].map((name) => ({
        label: `engine/${name}`, bytes: fs.readFileSync(path.join(baseline.root, "distribution/packages/engine", name)),
      })),
    };
    const profile = api.compileEffectiveProfile(portable, captured);
    const explanation = api.explainConfigurationImprovement(api.inspectConfigurationImprovement(portable), profile);
    const proposal = api.proposeConfigurationImprovement(explanation, {
      projectId: request.projectId, executionWorkspaceId: request.executionWorkspaceId,
      machineId: request.machineId, workspace: documents(candidate.root),
      privateInputs: privateInputs(candidate.root),
    });
    const direct = api.previewConfigurationImprovement(api.diagnoseConfigurationImprovement(
      api.evaluateConfigurationImprovement(proposal),
    ));

    const owned = { ...request };
    let owner = new bindings.Pale();
    const pending = owner.previewLocalConfigurationImprovement(owned);
    assert(pending instanceof Promise);
    for (const field of Object.keys(owned)) owned[field] = "changed-after-enqueue";
    owner = undefined;
    globalThis.gc();
    const preview = await pending;
    assert(preview instanceof bindings.ConfigurationImprovementPreview);
    const summary = Buffer.from(preview.summary().canonicalBytes);
    assert.deepEqual(summary, Buffer.from(direct.summary().canonicalBytes));
    assert.deepEqual(Buffer.from(preview.comparisonSummary().canonicalBytes), Buffer.from(direct.comparisonSummary().canonicalBytes));
    assert(preview.impacts(0, 2).entries.length > 0);
    const impact = preview.impacts(0, 1).entries[0];
    assert.notEqual(preview.explain(impact.subject()).state, bindings.WorkspaceExplanationState.NotPresent);
    for (const retained of [preview.baselineProfile(), preview.candidateProfile()]) {
      const page = retained.inspect(bindings.EffectiveProfileSection.IntendedUseCase, 0, 1);
      const canonicalPage = assertProfilePage(bindings, page);
      assertIntendedUseExplanation(bindings, retained, canonicalPage.entries[0]);
    }
    const copied = (() => {
      const retained = preview.candidateProfile();
      const page = retained.inspect(bindings.EffectiveProfileSection.IntendedUseCase, 0, 1);
      return { row: page.entries[0], expected: assertProfilePage(bindings, page).entries[0] };
    })();
    globalThis.gc();
    assertProfileEntry(bindings, copied.row, copied.expected);
    for (const [finish, disposition] of [
      ["accept", bindings.ConfigurationImprovementDisposition.Accepted],
      ["reject", bindings.ConfigurationImprovementDisposition.Rejected],
      ["cancel", bindings.ConfigurationImprovementDisposition.Cancelled],
    ]) {
      const value = await api.previewLocalConfigurationImprovement(request);
      assert.equal(value[finish]().disposition, disposition);
      assert.throws(() => value.summary(), /already been consumed/);
    }
    direct.cancel();

    for (const bad of [
      { ...request, baselineWorkspaceManifest: "relative-private-marker" },
      { ...request, candidateWorkspaceManifest: "relative-private-marker" },
      { ...request, candidateWorkspaceManifest: path.join(candidate.root, "missing-private-marker.toml") },
      { ...request, candidateWorkspaceManifest: `${candidate.root}/../private-marker/pale-workspace.toml` },
      { ...request, projectId: "private-selector-marker" },
    ]) {
      const failingManifest = bad.baselineWorkspaceManifest !== request.baselineWorkspaceManifest
        ? bad.baselineWorkspaceManifest : bad.candidateWorkspaceManifest;
      let expectedDiagnostic;
      assert.throws(() => api.compileLocalOneShot({
        workspaceManifest: failingManifest, projectId: bad.projectId,
        executionWorkspaceId: bad.executionWorkspaceId, machineId: bad.machineId,
      }), (error) => { expectedDiagnostic = Buffer.from(error.diagnostic.canonicalBytes); return true; });
      await assert.rejects(api.previewLocalConfigurationImprovement(bad), (error) => {
        assert(error instanceof Error);
        assert.equal(error.message, error.diagnostic.summary);
        assert.equal(error.diagnostic.code, "local_workspace_rejected");
        assert.deepEqual(Buffer.from(error.diagnostic.canonicalBytes), expectedDiagnostic);
        assert.equal(error.diagnostic.effect, bindings.DiagnosticEffectCertainty.NotApplicable);
        const bytes = Buffer.from(error.diagnostic.canonicalBytes).toString();
        for (const marker of [baseline.root, candidate.root, "private-marker", "private-selector-marker"]) assert(!bytes.includes(marker));
        return true;
      });
    }
    assert.deepEqual([snapshot(baseline.root), snapshot(candidate.root)], before);

    // Files changed after preview are not silently rebound to its retained owner.
    fs.writeFileSync(candidateManifest, original);
    const afterEdit = [snapshot(baseline.root), snapshot(candidate.root)];
    const fresh = await api.previewLocalConfigurationImprovement(request);
    assert.notDeepEqual(Buffer.from(fresh.summary().canonicalBytes), summary);
    assert.deepEqual(Buffer.from(preview.summary().canonicalBytes), summary);
    assert.equal(preview.markStale().disposition, bindings.ConfigurationImprovementDisposition.Stale);
    assert.throws(() => preview.markStale(), /already been consumed/);
    fresh.reject();
    assert.deepEqual([snapshot(baseline.root), snapshot(candidate.root)], afterEdit);
  } finally {
    fs.rmSync(baseline.root, { recursive: true, force: true });
    fs.rmSync(candidate.root, { recursive: true, force: true });
  }
  return "local.configuration-improvement-preview";
}
