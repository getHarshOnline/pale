import assert from "node:assert/strict";
import fs from "node:fs";

function generation(bindings, value) {
  assert.equal(value.kind, bindings.ObservationGenerationKind.EffectiveProfile);
  return {
    kind: "effective-profile",
    profileDigest: value.profileDigest,
    instanceDigest: value.instanceDigest,
    compositionPlanDigest: value.compositionPlanDigest,
  };
}

function cursor(bindings, value) {
  assert.equal(value.revision.kind, bindings.ObservationRevisionKind.Static);
  return {
    generation: generation(bindings, value.generation),
    revision: { kind: "static" },
  };
}

function subject(bindings, value) {
  if (value.kind === bindings.ObservationSubjectKind.EffectiveProfile) {
    return { kind: "effective-profile", profileDigest: value.profileDigest };
  }
  assert.equal(value.kind, bindings.ObservationSubjectKind.ExecutionPath);
  return { kind: "execution-path", executionPathId: value.executionPathId };
}

function freshness(bindings, value) {
  if (value.kind === bindings.ObservationFreshnessKind.Fresh) {
    return { kind: "fresh" };
  }
  assert.equal(value.kind, bindings.ObservationFreshnessKind.Unknown);
  return { kind: "unknown" };
}

function fact(bindings, value) {
  assert.equal(value.category, bindings.ObservationCategory.Readiness);
  const states = new Map([
    [bindings.ObservationReadinessState.Ready, "ready"],
    [bindings.ObservationReadinessState.Unproven, "unproven"],
  ]);
  const state = states.get(value.readinessState);
  assert.notEqual(state, undefined);
  return {
    category: "readiness",
    subject: subject(bindings, value.subject),
    state,
    freshness: freshness(bindings, value.freshness),
  };
}

function completion(bindings, value) {
  const kind = value.completion === bindings.ObservationStepCompletionKind.Complete
    ? "complete"
    : value.completion === bindings.ObservationStepCompletionKind.Stopped
      ? "stopped"
      : "incomplete";
  if (kind === "complete") {
    return { kind };
  }
  assert.equal(value.stoppedBy, bindings.ObservationStepLimitKind.Returned);
  return { kind, limit: { kind: "returned" } };
}

function step(bindings, value) {
  const variants = new Map([
    [bindings.ObservationCausalStepKind.OriginReceipt, ["origin-receipt", "contractId"]],
    [bindings.ObservationCausalStepKind.EntryContract, ["entry-contract", "contractId"]],
    [bindings.ObservationCausalStepKind.ResponsibilityContract, ["responsibility-contract", "contractId"]],
    [bindings.ObservationCausalStepKind.ProviderBinding, ["provider-binding", "bindingId"]],
    [bindings.ObservationCausalStepKind.DelegationEdge, ["delegation-edge", "edgeId"]],
    [bindings.ObservationCausalStepKind.TerminationContract, ["termination-contract", "contractId"]],
    [bindings.ObservationCausalStepKind.SettlementContract, ["settlement-contract", "contractId"]],
    [bindings.ObservationCausalStepKind.ContinuityContract, ["continuity-contract", "contractId"]],
  ]);
  const selected = variants.get(value.kind);
  assert.notEqual(selected, undefined);
  return { kind: selected[0], [selected[1]]: value.referenceId };
}

function causalPath(bindings, value) {
  return {
    generation: generation(bindings, value.generation),
    subject: subject(bindings, value.subject),
    steps: value.steps.map((entry) => step(bindings, entry)),
  };
}

function pageProjection(bindings, result) {
  assert.equal(result.kind, bindings.ObservationPageResultKind.Ready);
  const page = result.page;
  return {
    kind: "ready",
    page: {
      cursor: cursor(bindings, page.cursor),
      facts: page.facts.map((entry) => fact(bindings, entry)),
      continuation: page.continuation === undefined || page.continuation === null ? null : {
        snapshot: cursor(bindings, page.continuation.snapshot),
        inspectedPosition: page.continuation.inspectedPosition,
      },
      completion: completion(bindings, page),
      inspected: page.inspected,
      copiedStringBytes: page.copiedStringBytes,
      snapshotOmittedCount: page.snapshotOmittedCount,
    },
  };
}

function lensProjection(bindings, result) {
  assert.equal(result.kind, bindings.ObservationLensDispositionKind.Ready);
  const lens = result.lens;
  return {
    kind: "ready",
    lens: {
      cursor: cursor(bindings, lens.cursor),
      nodes: lens.nodes.map((node) => ({
        subject: subject(bindings, node.subject),
        facts: node.facts.map((entry) => fact(bindings, entry)),
      })),
      causalPaths: lens.causalPaths.map((entry) => causalPath(bindings, entry)),
      continuation: lens.continuation === undefined || lens.continuation === null ? null : {
        snapshot: cursor(bindings, lens.continuation.snapshot),
        inspectedPosition: lens.continuation.inspectedPosition,
      },
      completion: completion(bindings, lens),
      inspectedSubjects: lens.inspectedSubjects,
      copiedStringBytes: lens.copiedStringBytes,
      causalPathLookups: lens.causalPathLookups,
      causalSteps: lens.causalSteps,
      snapshotOmittedCount: lens.snapshotOmittedCount,
    },
  };
}

export function exerciseObservationConformance(bindings, snapshot, fixtureRoot) {
  const expected = JSON.parse(fs.readFileSync(`${fixtureRoot}/observation-journey.json`, "utf8"));
  const pageLimits = {
    maximumInspected: 16,
    maximumReturned: 1,
    maximumCopiedStringBytes: 4096,
  };
  const filter = new bindings.ObservationFilter();
  filter.selectCategory(bindings.ObservationCategory.Readiness);
  const page = snapshot.page(filter, pageLimits, bindings.ObservationCancellation.Active);
  const mismatch = snapshot.page(
    filter,
    { ...pageLimits, maximumReturned: 2 },
    bindings.ObservationCancellation.Active,
    page.page.continuation,
  );
  assert.equal(mismatch.kind, bindings.ObservationPageResultKind.Resynchronize);
  assert.equal(
    mismatch.resynchronization.reason,
    bindings.ObservationResynchronizationReason.ContinuationMismatch,
  );

  const discovery = snapshot.page(
    new bindings.ObservationFilter(),
    { maximumInspected: 256, maximumReturned: 256, maximumCopiedStringBytes: 2 ** 32 - 1 },
    bindings.ObservationCancellation.Active,
  );
  const executionPath = discovery.page.facts
    .map((entry) => entry.subject)
    .find((entry) => entry?.kind === bindings.ObservationSubjectKind.ExecutionPath);
  assert.notEqual(executionPath, undefined);
  const path = snapshot.causalPath(executionPath);
  assert.equal(path.kind, bindings.ObservationCausalResultKind.Found);

  const lensFilter = new bindings.ObservationFilter();
  lensFilter.selectSubject(executionPath);
  const lensLimits = {
    maximumInspected: 256,
    maximumNodes: 1,
    maximumCopiedStringBytes: 4096,
    maximumCausalPathLookups: 1,
    maximumCausalSteps: 256,
  };
  const lens = snapshot.visualLens(
    lensFilter,
    lensLimits,
    true,
    bindings.ObservationCancellation.Active,
  );
  const actual = {
    snapshot: {
      cursor: cursor(bindings, snapshot.cursor),
      factCount: snapshot.factCount,
      omittedFactCount: snapshot.omittedFactCount,
    },
    pageLimits,
    page: pageProjection(bindings, page),
    continuationMismatch: {
      kind: "resynchronize",
      resynchronization: {
        reason: { kind: "continuation-mismatch" },
        current: cursor(bindings, mismatch.resynchronization.current),
      },
    },
    causalPath: { kind: "ready", path: causalPath(bindings, path.path) },
    lensLimits,
    lens: lensProjection(bindings, lens),
  };
  assert.deepEqual(actual, expected);
}
