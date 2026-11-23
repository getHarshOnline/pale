import assert from "node:assert/strict";
import { assertProfileEntry } from "./effective_profile.mjs";

// Uses the caller's existing compilation and canonical row oracle.
export function exerciseObservationProfile(bindings, api, workspace, profile, rebuiltProfile, foreignWorkspace, changedProfile, rows) {
  const snapshot = api.observeEffectiveProfile(profile);
  const factPage = snapshot.page(
    new bindings.ObservationFilter(),
    {
      maximumInspected: 256,
      maximumReturned: 256,
      maximumCopiedStringBytes: 2 ** 32 - 1,
    },
    bindings.ObservationCancellation.Active,
  ).page;
  assert(factPage);
  assert.equal(factPage.continuation, null);
  const subject = factPage.facts.map((fact) => fact.subject).find((value) => value.kind === bindings.ObservationSubjectKind.Machine);
  assert(subject, "fixture must expose its configured machine");
  const ready = (result) => {
    assert.equal(result.kind, bindings.ObservationProfileResultKind.Ready);
    assert.equal(result.error, null);
    assert(result.page);
    return result.page;
  };
  const refuse = (result, error) => {
    assert.equal(result.kind, bindings.ObservationProfileResultKind.Refused);
    assert.equal(result.error, error);
    assert.equal(result.page, null);
  };
  const associated = snapshot.associateProfile(profile, workspace);
  assert.equal(associated.kind, bindings.ObservationProfileAssociationResultKind.Associated);
  assert.equal(associated.error, null);
  assert(associated.association);
  assert.equal(associated.association.belongsTo(snapshot), true);
  assert.deepEqual(associated.association.coordinates, profile.coordinates());
  assert.deepEqual(associated.association.cursor, snapshot.cursor);
  assert.deepEqual(Object.keys(associated.association.coordinates).sort(), [
    "compositionPlanDigest", "executionWorkspaceId", "instanceDigest", "machineId",
    "packageSemanticIrDigest", "profileDigest", "projectId", "selectedTopologyDigest",
    "workspaceAdmissionEvidenceDigest", "workspaceId", "workspaceSemanticIrDigest",
  ]);
  const full = ready(snapshot.inspectProfile(profile, workspace, subject, 256, 256));
  assert.equal(full.mapping, bindings.ObservationProfileMapping.PrimaryRows);
  assert(full.entries.length > 0);
  assert.equal(full.continuation, null);
  assert.equal(full.coordinates.instanceDigest, profile.instanceDigest);
  assert.equal(full.coordinates.profileDigest, profile.profileDigest);
  for (const entry of full.entries) {
    assertProfileEntry(bindings, entry, rows.get(entry.explanationHandle));
    assert.equal(profile.explain(entry.explanationHandle).handle, entry.explanationHandle);
  }
  const first = ready(snapshot.inspectProfile(profile, workspace, subject, 1, 1));
  assert(first.continuation, "raw-row limit must leave a continuation");
  const handles = first.entries.map((entry) => entry.explanationHandle);
  let inspected = first.inspected;
  let page = first;
  let calls = 1;
  while (page.continuation !== null) {
    assert(calls++ <= full.inspected + 1, "continuation must advance");
    page = ready(snapshot.inspectProfile(profile, workspace, subject, 1, 1, page.continuation));
    assert(page.inspected <= 1);
    assert(page.entries.length <= 1);
    inspected += page.inspected;
    handles.push(...page.entries.map((entry) => entry.explanationHandle));
  }
  assert.equal(inspected, full.inspected);
  assert.deepEqual(handles, full.entries.map((entry) => entry.explanationHandle));
  assert.deepEqual(
    ready(snapshot.inspectProfile(rebuiltProfile, workspace, subject, 256, 256)).entries.map((entry) => entry.explanationHandle),
    handles,
  );
  const errors = bindings.ObservationProfileError;
  for (const [visited, returned] of [[0, 1], [1, 0], [257, 1], [1, 257]]) {
    refuse(snapshot.inspectProfile(profile, workspace, subject, visited, returned), errors.InvalidLimit);
  }
  // napi_get_value_uint32 can truncate fractions or wrap modulo 2^32.
  // The existing exact numeric decoder must reject before SDK admission.
  for (const value of [-1, -4294967295, 1.5, NaN, Infinity, -Infinity, 65537, 4294967297]) {
    refuse(snapshot.inspectProfile(profile, workspace, subject, value, 1), errors.InvalidLimit);
    refuse(snapshot.inspectProfile(profile, workspace, subject, 1, value), errors.InvalidLimit);
  }
  refuse(snapshot.inspectProfile(profile, foreignWorkspace, subject, 1, 1), errors.SourceMismatch);
  refuse(snapshot.inspectProfile(changedProfile, workspace, subject, 1, 1), errors.CoordinateMismatch);
  const foreignAssociation = snapshot.associateProfile(profile, foreignWorkspace);
  assert.equal(foreignAssociation.kind, bindings.ObservationProfileAssociationResultKind.Refused);
  assert.equal(foreignAssociation.error, errors.SourceMismatch);
  assert.equal(foreignAssociation.association, null);
  const changedAssociation = snapshot.associateProfile(changedProfile, workspace);
  assert.equal(changedAssociation.kind, bindings.ObservationProfileAssociationResultKind.Refused);
  assert.equal(changedAssociation.error, errors.CoordinateMismatch);
  assert.equal(changedAssociation.association, null);
  refuse(snapshot.inspectProfile(profile, workspace, subject, 2, 1, first.continuation), errors.ContinuationMismatch);
  const secondSnapshot = api.observeEffectiveProfile(profile);
  assert.deepEqual(secondSnapshot.cursor, snapshot.cursor);
  assert.equal(associated.association.belongsTo(secondSnapshot), false);
  refuse(secondSnapshot.inspectProfile(profile, workspace, subject, 1, 1, first.continuation), errors.ContinuationMismatch);
  const whole = factPage.facts.map((fact) => fact.subject).find((value) => value.kind === bindings.ObservationSubjectKind.EffectiveProfile);
  assert(whole);
  const unmapped = ready(snapshot.inspectProfile(profile, workspace, whole, 256, 256));
  assert.equal(unmapped.mapping, bindings.ObservationProfileMapping.NoPrimaryRows);
  assert.deepEqual(unmapped.entries, []);
  refuse(snapshot.inspectProfile(profile, workspace, whole, 1, 1, first.continuation), errors.ContinuationMismatch);
  return "observation.profile-inspection";
}
