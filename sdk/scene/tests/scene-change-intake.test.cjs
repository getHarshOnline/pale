"use strict";
const assert = require("node:assert/strict");
const { performance } = require("node:perf_hooks");
const { test } = require("node:test");
const { readFileSync } = require("node:fs");
const { join, resolve } = require("node:path");
const { Pale, PortableWorkspaceCompilation, PortableWorkspaceDependencyPage,
  PortableWorkspaceSubjectInspection, WorkspaceChangeRoot, WorkspaceComparison,
  WorkspaceComparisonSubjectPage, WorkspaceContributingRootsPage, WorkspaceImpact,
  WorkspaceImpactPage } = require("@rtepl/pale");
const { WorkspaceChangeInspectionContinuation, WorkspaceChangeIntake,
  WorkspaceChangeIntakeError, WorkspaceChangeOccurrence } = require("@rtepl/pale-scene/change");
const { SceneBudget } = require("@rtepl/pale-scene");

const fixture = process.env.PALE_SCENE_TEST_ORGANIZATION_FIXTURE
  ?? resolve(__dirname, "../../js/tests/fixtures/organization");
const files = ["pale-workspace.toml", "organization.toml", "responsibility.toml",
  "resolution.lock.toml", "universe.lock.toml"];
const query = { subjects: 2, roots: 2, impacts: 2, contributingRoots: 2, dependencies: 2,
  sourceCandidates: 2, sourceOccurrences: 2 };
const limits = { queries: 1000, subjectVisits: 5000, rootVisits: 5000, proofVisits: 5000,
  sourceVisits: 5000, subjects: 2000, roots: 2000, impacts: 2000, proofSteps: 5000,
  sourceOccurrences: 2000, copiedStringBytes: 100000 };
const workspaceChangeIntake = (prior, current, comparison, selectedQuery, selectedLimits, budget) =>
  new WorkspaceChangeIntake(prior, current, comparison, selectedQuery, selectedLimits, budget);

function compile(changed = false) {
  const api = new Pale();
  const owner = api.compilePortableWorkspace({ rootManifest: files[0], documents: files.map(label => {
    let bytes = readFileSync(join(fixture, label));
    if (changed && label === "responsibility.toml") {
      bytes = Buffer.from(bytes.toString().replaceAll("evidence.alpha", "evidence.current"));
    }
    return { label, bytes };
  }) });
  return { api, owner };
}

function selected(t, queryLimits = query, intakeLimits = limits, budget) {
  const prior = compile(false).owner, current = compile(true).owner;
  const comparison = new Pale().comparePortableWorkspaces(prior, current);
  const receiptDigest = comparison.receiptDigest;
  const intake = workspaceChangeIntake(prior, current, comparison, queryLimits, intakeLimits, budget);
  t.after(() => intake.close());
  return { prior, current, receiptDigest, intake };
}

test("change queries can share the Scene aggregate budget", t => {
  const budget = new SceneBudget({ workUnits: 100000, retainedSceneBytes: 100000,
    encodedOutputBytes: 100000, deadlineMs: 10000 });
  const { intake } = selected(t, query, limits, budget);
  const subjects = intake.nextSubjects();
  assert.equal(subjects.kind, "page");
  assert(budget.charges.workUnits > 0);
  assert(budget.charges.retainedSceneBytes > 0);
  const occurrence = subjects.occurrences[0]; assert(occurrence);
  const before = budget.charges.workUnits;
  assert.equal(intake.inspect("current", occurrence).kind, "page");
  assert(budget.charges.workUnits > before);
  assert(budget.charges.retainedSceneBytes > 0);
});

function firstSubject(intake) {
  const step = intake.nextSubjects();
  assert.equal(step.kind, "page");
  assert(step.occurrences[0]);
  return step.occurrences[0];
}

function subjectCoordinate(subject) {
  return JSON.stringify([subject.kind, subject.workspaceId, subject.sourceId,
    subject.componentId, subject.moduleId, subject.ownerModuleId,
    subject.declarationNamespaceId, subject.declarationId, subject.importerModuleId,
    subject.upstreamModuleId, subject.upstreamReleaseId, subject.exportId,
    subject.sourceNamespaceId, subject.localNamespaceId,
    subject.extensionNamespaceId, subject.resourceId]);
}

function continuableInspection(intake, side = "current", matching = () => true) {
  for (let pages = 0; pages < 100; pages++) {
    const subjects = intake.nextSubjects();
    if (subjects.kind === "enumeration-exhausted") break;
    assert.equal(subjects.kind, "page");
    for (const occurrence of subjects.occurrences) {
      if (!matching(occurrence)) continue;
      const inspection = intake.inspect(side, occurrence);
      assert.equal(inspection.kind, "page");
      if (inspection.continuation) return { occurrence, inspection };
    }
  }
  assert.fail("Fixture has no matching resumable source inspection");
}

function allImpacts(intake) {
  const occurrences = [];
  for (let pages = 0; pages < 100; pages++) {
    const step = intake.nextImpacts();
    if (step.kind === "enumeration-exhausted") return occurrences;
    assert.equal(step.kind, "page");
    occurrences.push(...step.occurrences);
  }
  assert.fail("Fixture impact enumeration did not terminate");
}

test("change intake retains the exact ordered owners", t => {
  const { prior, current, intake } = selected(t);
  assert(intake instanceof WorkspaceChangeIntake);
  const ordered = new Pale().comparePortableWorkspaces(prior, current);
  assert.throws(() => workspaceChangeIntake(current, prior, ordered, query, limits),
    error => error instanceof WorkspaceChangeIntakeError && error.kind === "owner-mismatch");
  const freshPrior = compile(false).owner, freshCurrent = compile(true).owner;
  assert.equal(freshPrior.principalUniverseDigest, prior.principalUniverseDigest);
  assert.equal(freshCurrent.semanticIrDigest, current.semanticIrDigest);
  const exactPair = new Pale().comparePortableWorkspaces(prior, current);
  assert.throws(() => workspaceChangeIntake(freshPrior, freshCurrent, exactPair, query, limits),
    error => error instanceof WorkspaceChangeIntakeError && error.kind === "owner-mismatch");
  assert.throws(() => new WorkspaceChangeOccurrence(), TypeError);
});

test("change intake errors keep custody and bounds as distinct closed categories", () => {
  const custody = new WorkspaceChangeIntakeError({ kind: "foreign-occurrence" });
  assert.equal(custody.kind, "foreign-occurrence");
  assert.equal(custody.resource, undefined);
  assert.equal(custody.observed, undefined);
  assert.equal(custody.limit, undefined);
  const bounds = new WorkspaceChangeIntakeError(
    { kind: "bounds", resource: "sourceOccurrences", observed: 3, limit: 2 });
  assert.equal(bounds.kind, "bounds");
  assert.equal(bounds.resource, "sourceOccurrences");
  assert.equal(bounds.observed, 3);
  assert.equal(bounds.limit, 2);
  assert(Object.isFrozen(bounds.details));
  assert.match(bounds.message, /observed 3, limit 2/);
});

test("subject, root, and impact enumeration advance independently", t => {
  const { intake } = selected(t);
  const subjects = intake.nextSubjects(), roots = intake.nextRoots(), impacts = intake.nextImpacts();
  assert.equal(subjects.kind, "page"); assert.equal(roots.kind, "page"); assert.equal(impacts.kind, "page");
  assert(subjects.page instanceof WorkspaceComparisonSubjectPage);
  assert(impacts.page instanceof WorkspaceImpactPage);
  assert.equal(subjects.occurrences.length, subjects.page.subjects().length);
  assert.equal(roots.occurrences.length, roots.page.entries.length);
  assert.equal(impacts.occurrences.length, impacts.page.entries.length);
  assert(subjects.occurrences.every(item => item.inclusion === "subject-enumeration"));
  assert(roots.occurrences.every(item => item.inclusion === "change-root"));
  assert(impacts.occurrences.every(item => item.inclusion === "impact-partition"));
  const ids = [...subjects.occurrences, ...roots.occurrences, ...impacts.occurrences]
    .map(item => item.appearanceId);
  assert.equal(new Set(ids).size, ids.length);
  assert.deepEqual(Object.keys(subjects.occurrences[0]), []);
  assert(Object.isFrozen(subjects.occurrences[0]));
  assert.equal(intake.nextRoots().kind, "enumeration-exhausted");
  assert.equal(intake.nextSubjects().kind, "page");
  assert.equal(intake.nextImpacts().kind, "page");
});

test("point, proof, and source reads require an occurrence from the same intake", t => {
  const selectedPair = selected(t), occurrence = firstSubject(selectedPair.intake);
  const point = selectedPair.intake.compare(occurrence);
  assert.equal(point.kind, "result");
  assert.equal(point.result.receiptDigest, selectedPair.receiptDigest);
  const roots = selectedPair.intake.contributingRoots(occurrence);
  assert.equal(roots.kind, "page"); assert(roots.page instanceof WorkspaceContributingRootsPage);
  const prior = selectedPair.intake.dependencies("prior", occurrence);
  const current = selectedPair.intake.dependencies("current", occurrence);
  const inspection = selectedPair.intake.inspect("current", occurrence);
  assert.equal(prior.kind, "page"); assert.equal(current.kind, "page");
  assert(prior.page instanceof PortableWorkspaceDependencyPage);
  assert(current.page instanceof PortableWorkspaceDependencyPage);
  assert.equal(inspection.kind, "page");
  assert(inspection.page instanceof PortableWorkspaceSubjectInspection);
  assert(inspection.page.belongsTo(selectedPair.current));
  const foreign = selected(t), foreignOccurrence = firstSubject(foreign.intake);
  for (const call of [
    () => selectedPair.intake.compare(foreignOccurrence),
    () => selectedPair.intake.contributingRoots(foreignOccurrence),
    () => selectedPair.intake.dependencies("current", foreignOccurrence),
    () => selectedPair.intake.inspect("current", foreignOccurrence),
    () => selectedPair.intake.compare({ appearanceId: occurrence.appearanceId,
      subject: occurrence.subject, inclusion: occurrence.inclusion }),
  ]) {
    assert.throws(call, error => error instanceof WorkspaceChangeIntakeError
      && error.kind === "foreign-occurrence");
  }
  assert.throws(() => selectedPair.intake.dependencies("later", occurrence),
    /side must be prior or current/);
  assert.throws(() => selectedPair.intake.inspect("later", occurrence),
    /side must be prior or current/);
});

test("full inspection returns exact native pages and charges their actual source and proof work", t => {
  const selectedPair = selected(t, { ...query, dependencies: 1,
    sourceCandidates: 1, sourceOccurrences: 1 });
  const occurrence = firstSubject(selectedPair.intake), before = selectedPair.intake.charges;
  const step = selectedPair.intake.inspect("current", occurrence);
  assert.equal(step.kind, "page");
  assert(step.page instanceof PortableWorkspaceSubjectInspection);
  assert(step.page.belongsTo(selectedPair.current));
  assert(!step.page.belongsTo(selectedPair.prior));
  const expected = selectedPair.current.inspectSubject(occurrence.subject, 1, 1, 1);
  assert.deepEqual(step.page.canonicalBytes, expected.canonicalBytes);
  const after = selectedPair.intake.charges;
  assert.equal(after.queries, before.queries + 1);
  assert.equal(after.proofVisits, before.proofVisits + step.page.dependencies().length);
  assert.equal(after.sourceVisits, before.sourceVisits + step.page.inspected);
  assert.equal(after.proofSteps, before.proofSteps + step.page.dependencies().length);
  assert.equal(after.sourceOccurrences, before.sourceOccurrences + step.page.occurrences().length);
  assert.equal(after.copiedStringBytes, before.copiedStringBytes
    + step.page.dependencyCopiedStringBytes + step.page.sourceCopiedStringBytes);
  const prior = selectedPair.intake.inspect("prior", occurrence);
  assert.equal(prior.kind, "page"); assert(prior.page.belongsTo(selectedPair.prior));
  assert(!prior.page.belongsTo(selectedPair.current));
});

test("inspection continuation preserves its side even when both sides retain one owner", t => {
  const raw = { ...query, dependencies: 1, sourceCandidates: 1, sourceOccurrences: 1 };
  const owner = compile(false).owner;
  const comparison = new Pale().comparePortableWorkspaces(owner, owner);
  const intake = workspaceChangeIntake(owner, owner, comparison, raw, limits);
  t.after(() => intake.close());
  const { occurrence, inspection } = continuableInspection(intake);
  const before = intake.charges;
  assert.throws(() => intake.inspect("prior", occurrence, inspection.continuation),
    error => error instanceof WorkspaceChangeIntakeError
      && error.kind === "continuation-mismatch");
  assert.deepEqual(intake.charges, before);
  const continued = intake.inspect("current", occurrence, inspection.continuation);
  assert.equal(continued.kind, "page"); assert(continued.page.belongsTo(owner));
});

test("inspection continuation preserves the exact occurrence appearance", t => {
  const raw = { ...query, dependencies: 1, sourceCandidates: 1, sourceOccurrences: 1 };
  const selectedPair = selected(t, raw);
  const impacts = new Map(allImpacts(selectedPair.intake)
    .map(occurrence => [subjectCoordinate(occurrence.subject), occurrence]));
  const { occurrence, inspection } = continuableInspection(selectedPair.intake, "current",
    candidate => impacts.has(subjectCoordinate(candidate.subject)));
  const impact = impacts.get(subjectCoordinate(occurrence.subject));
  assert(impact); assert.notEqual(impact, occurrence);
  assert.equal(occurrence.inclusion, "subject-enumeration");
  assert.equal(impact.inclusion, "impact-partition");
  const before = selectedPair.intake.charges;
  assert.throws(() => selectedPair.intake.inspect("current", impact, inspection.continuation),
    error => error instanceof WorkspaceChangeIntakeError
      && error.kind === "continuation-mismatch");
  assert.deepEqual(selectedPair.intake.charges, before);
  assert.equal(selectedPair.intake.inspect("current", occurrence,
    inspection.continuation).kind, "page");
});

test("inspection continuation preserves its intake and cannot be forged or bypassed", t => {
  const raw = { ...query, dependencies: 1, sourceCandidates: 1, sourceOccurrences: 1 };
  const selectedPair = selected(t, raw);
  const firstIntake = selectedPair.intake;
  const { occurrence, inspection } = continuableInspection(firstIntake);
  const secondComparison = new Pale().comparePortableWorkspaces(selectedPair.prior, selectedPair.current);
  const secondIntake = workspaceChangeIntake(selectedPair.prior, selectedPair.current,
    secondComparison, raw, limits);
  t.after(() => secondIntake.close());
  let corresponding;
  for (let pages = 0; pages < 100 && !corresponding; pages++) {
    const step = secondIntake.nextSubjects();
    if (step.kind === "enumeration-exhausted") break;
    assert.equal(step.kind, "page");
    corresponding = step.occurrences.find(candidate =>
      subjectCoordinate(candidate.subject) === subjectCoordinate(occurrence.subject));
  }
  assert(corresponding);
  const secondBefore = secondIntake.charges;
  assert.throws(() => secondIntake.inspect("current", corresponding, inspection.continuation),
    error => error instanceof WorkspaceChangeIntakeError
      && error.kind === "continuation-mismatch");
  assert.deepEqual(secondIntake.charges, secondBefore);

  assert.throws(() => new WorkspaceChangeInspectionContinuation(), TypeError);
  const native = inspection.page.continuation(); assert(native);
  const firstBefore = firstIntake.charges;
  for (const foreign of [native,
    Object.create(WorkspaceChangeInspectionContinuation.prototype)]) {
    assert.throws(() => firstIntake.inspect("current", occurrence, foreign),
      error => error instanceof WorkspaceChangeIntakeError
        && error.kind === "continuation-mismatch");
    assert.deepEqual(firstIntake.charges, firstBefore);
  }
  assert.equal(firstIntake.inspect("current", occurrence, inspection.continuation).kind, "page");
});

test("empty incomplete source pages remain resumable native results", t => {
  const selectedPair = selected(t, { ...query, dependencies: 1,
    sourceCandidates: 1, sourceOccurrences: 1 });
  const occurrence = firstSubject(selectedPair.intake);
  const first = selectedPair.intake.inspect("current", occurrence);
  assert.equal(first.kind, "page"); const cursor = first.continuation; assert(cursor);
  const second = selectedPair.intake.inspect("current", occurrence, cursor);
  assert.equal(second.kind, "page");
  assert.deepEqual(second.page.occurrences(), []);
  assert.equal(second.page.sourcesComplete, false);
  assert(second.continuation);
});

test("a source page beyond the remaining retained-byte limit is not published", t => {
  const raw = { ...query, subjects: 1, dependencies: 1,
    sourceCandidates: 1, sourceOccurrences: 1 };
  const selectedPair = selected(t, raw, { ...limits, copiedStringBytes: 3 });
  const occurrence = firstSubject(selectedPair.intake), before = selectedPair.intake.charges;
  assert.equal(before.copiedStringBytes, 2);
  const expected = selectedPair.current.inspectSubject(occurrence.subject, 1, 1, 1);
  assert(expected.dependencyCopiedStringBytes + expected.sourceCopiedStringBytes > 1);
  assert.deepEqual(selectedPair.intake.inspect("current", occurrence),
    { kind: "resource-stop", resource: "copiedStringBytes" });
  const after = selectedPair.intake.charges;
  assert.equal(after.queries, before.queries + 1);
  assert.equal(after.proofVisits, before.proofVisits + raw.dependencies);
  assert.equal(after.sourceVisits, before.sourceVisits + raw.sourceCandidates);
  assert.equal(after.proofSteps, before.proofSteps + raw.dependencies);
  assert.equal(after.sourceOccurrences, before.sourceOccurrences + raw.sourceOccurrences);
  assert.equal(after.copiedStringBytes, before.copiedStringBytes);
  assert.equal(selectedPair.intake.compare(occurrence).kind, "result");
});

test("native cursor failure keeps its reservation and publishes no new appearance", t => {
  const selectedPair = selected(t, { ...query, dependencies: 1 });
  let occurrence, foreignCursor;
  for (let pages = 0; pages < 20 && !foreignCursor; pages++) {
    const step = selectedPair.intake.nextSubjects();
    assert.equal(step.kind, "page");
    for (const candidate of step.occurrences) {
      const native = selectedPair.current.directDependencies(candidate.subject, 1);
      const cursor = native.continuation();
      if (cursor) { occurrence = candidate; foreignCursor = cursor; break; }
    }
  }
  assert(occurrence); assert(foreignCursor);
  const before = selectedPair.intake.charges;
  assert.throws(() => selectedPair.intake.dependencies("prior", occurrence, foreignCursor));
  const after = selectedPair.intake.charges;
  assert.equal(after.queries, before.queries + 1);
  assert.equal(after.proofVisits, before.proofVisits + 1);
  assert.equal(after.proofSteps, before.proofSteps + 1);
  assert.equal(selectedPair.intake.compare(occurrence).kind, "result");
});

test("captured SDK methods ignore later shadowing", t => {
  const selectedPair = selected(t);
  const replacements = [
    [WorkspaceComparison.prototype, "subjects", { value: () => assert.fail("shadowed subjects") }],
    [WorkspaceComparison.prototype, "changeRoots", { value: () => assert.fail("shadowed roots") }],
    [WorkspaceComparison.prototype, "impacts", { value: () => assert.fail("shadowed impacts") }],
    [WorkspaceComparison.prototype, "compareSubject", { value: () => assert.fail("shadowed compare") }],
    [WorkspaceComparison.prototype, "contributingRoots", { value: () => assert.fail("shadowed contributors") }],
    [PortableWorkspaceCompilation.prototype, "directDependencies", { value: () => assert.fail("shadowed dependencies") }],
    [PortableWorkspaceCompilation.prototype, "inspectSubject", { value: () => assert.fail("shadowed inspection") }],
    [WorkspaceComparisonSubjectPage.prototype, "subjects", { value: () => assert.fail("shadowed page") }],
    [WorkspaceComparisonSubjectPage.prototype, "belongsTo", { value: () => false }],
    [WorkspaceChangeRoot.prototype, "subject", { value: () => assert.fail("shadowed root subject") }],
    [WorkspaceImpact.prototype, "subject", { value: () => assert.fail("shadowed impact subject") }],
    [WorkspaceContributingRootsPage.prototype, "roots", { value: () => assert.fail("shadowed page roots") }],
    [PortableWorkspaceDependencyPage.prototype, "dependencies", { value: () => assert.fail("shadowed proof") }],
    [PortableWorkspaceSubjectInspection.prototype, "belongsTo", { value: () => false }],
    [PortableWorkspaceSubjectInspection.prototype, "dependencies", { value: () => assert.fail("shadowed inspection dependencies") }],
    [PortableWorkspaceSubjectInspection.prototype, "occurrences", { value: () => assert.fail("shadowed inspection sources") }],
    [PortableWorkspaceSubjectInspection.prototype, "inspected", { get: () => assert.fail("shadowed inspected") }],
    [PortableWorkspaceSubjectInspection.prototype, "dependencyCopiedStringBytes", { get: () => assert.fail("shadowed dependency bytes") }],
    [PortableWorkspaceSubjectInspection.prototype, "sourceCopiedStringBytes", { get: () => assert.fail("shadowed source bytes") }],
  ];
  const originals = replacements.map(([target, name]) => [target, name,
    Object.getOwnPropertyDescriptor(target, name)]);
  for (const [target, name, descriptor] of replacements) {
    Object.defineProperty(target, name, { ...descriptor, configurable: true });
  }
  try {
    const subject = firstSubject(selectedPair.intake);
    assert.equal(selectedPair.intake.nextRoots().kind, "page");
    assert.equal(selectedPair.intake.nextImpacts().kind, "page");
    assert.equal(selectedPair.intake.compare(subject).kind, "result");
    assert.equal(selectedPair.intake.contributingRoots(subject).kind, "page");
    assert.equal(selectedPair.intake.dependencies("current", subject).kind, "page");
    assert.equal(selectedPair.intake.inspect("current", subject).kind, "page");
  } finally {
    for (const [target, name, descriptor] of originals) Object.defineProperty(target, name, descriptor);
  }
});

test("cancellation and resource stops do not advance enumeration", t => {
  const { intake } = selected(t);
  const controller = new AbortController(); controller.abort();
  const before = intake.charges;
  assert.deepEqual(intake.nextSubjects(controller.signal), { kind: "stopped", reason: "cancelled" });
  assert.deepEqual(intake.nextRoots(controller.signal), { kind: "stopped", reason: "cancelled" });
  assert.deepEqual(intake.nextImpacts(controller.signal), { kind: "stopped", reason: "cancelled" });
  assert.deepEqual(intake.charges, before);
  const occurrence = firstSubject(intake), foreignCursor = compile().owner
    .inspectSubject(occurrence.subject, 1, 1, 1).continuation();
  assert(foreignCursor);
  const afterOccurrence = intake.charges;
  assert.deepEqual(intake.inspect("current", occurrence, foreignCursor, controller.signal),
    { kind: "stopped", reason: "cancelled" });
  assert.deepEqual(intake.charges, afterOccurrence);
  let cancellationReads = 0;
  const signal = new Proxy(new AbortController().signal, { get(target, property) {
    if (property === "aborted") return ++cancellationReads > 1;
    const value = Reflect.get(target, property, target);
    return typeof value === "function" ? value.bind(target) : value;
  } });
  assert.equal(intake.nextSubjects(signal).kind, "page");
  assert.deepEqual(intake.nextRoots(signal), { kind: "stopped", reason: "cancelled" });

  const limited = selected(t, query, { ...limits, copiedStringBytes: 1 }).intake;
  const stopped = limited.nextSubjects();
  assert.deepEqual(stopped, { kind: "resource-stop", resource: "copiedStringBytes" });
  assert.deepEqual(limited.charges, { queries: 0, subjectVisits: 0, rootVisits: 0,
    proofVisits: 0, sourceVisits: 0, subjects: 0, roots: 0, impacts: 0,
    proofSteps: 0, sourceOccurrences: 0, copiedStringBytes: 0 });
});

test("each aggregate resource stops before its native call", t => {
  for (const resource of ["queries", "subjectVisits", "rootVisits", "proofVisits", "sourceVisits",
    "subjects", "roots", "impacts", "proofSteps", "sourceOccurrences", "copiedStringBytes"]) {
    const resourceLimits = { ...limits, [resource]: 1 };
    const intake = selected(t, query, resourceLimits).intake;
    let occurrence;
    if (resource === "queries") assert.equal(intake.nextRoots().kind, "page");
    if (["proofVisits", "proofSteps", "sourceVisits", "sourceOccurrences"].includes(resource)) {
      occurrence = firstSubject(intake);
    }
    const before = intake.charges;
    const step = resource === "queries" ? intake.nextImpacts()
      : resource === "rootVisits" || resource === "roots" ? intake.nextRoots()
      : resource === "impacts" ? intake.nextImpacts()
      : ["proofVisits", "proofSteps"].includes(resource) ? intake.dependencies("current", occurrence)
      : resource === "sourceVisits" || resource === "sourceOccurrences"
        ? intake.inspect("current", occurrence)
      : intake.nextSubjects();
    assert.deepEqual(step, { kind: "resource-stop", resource }, resource);
    assert.deepEqual(intake.charges, before, resource);
  }
});

test("close releases intake membership but keeps returned SDK handles readable", t => {
  const { intake } = selected(t), step = intake.nextSubjects();
  assert.equal(step.kind, "page"); const occurrence = step.occurrences[0];
  assert(occurrence); const subject = occurrence.subject;
  intake.close();
  assert.equal(intake.nextSubjects().kind, "closed");
  assert.equal(intake.nextRoots().kind, "closed");
  assert.equal(intake.nextImpacts().kind, "closed");
  assert.equal(intake.compare(occurrence).kind, "closed");
  assert.equal(intake.inspect("current", occurrence).kind, "closed");
  assert.equal(occurrence.subject, subject);
  assert.equal(typeof subject.kind, "number");
});

test("bounded change intake remains linear on the representative fixture", t => {
  const started = performance.now();
  let subjectCount = 0, rootCount = 0, impactCount = 0;
  for (let run = 0; run < 50; run++) {
    const { intake } = selected(t, { subjects: 256, roots: 256, impacts: 256,
      contributingRoots: 256, dependencies: 256, sourceCandidates: 256,
      sourceOccurrences: 256 });
    for (let count = 0; count < 100; count++) {
      const step = intake.nextSubjects();
      if (step.kind === "enumeration-exhausted") break;
      assert.equal(step.kind, "page"); subjectCount += step.occurrences.length;
    }
    for (let count = 0; count < 100; count++) {
      const step = intake.nextRoots();
      if (step.kind === "enumeration-exhausted") break;
      assert.equal(step.kind, "page"); rootCount += step.occurrences.length;
    }
    for (let count = 0; count < 100; count++) {
      const step = intake.nextImpacts();
      if (step.kind === "enumeration-exhausted") break;
      assert.equal(step.kind, "page"); impactCount += step.occurrences.length;
    }
  }
  assert.equal(subjectCount, 550); assert.equal(rootCount, 50); assert.equal(impactCount, 550);
  assert(performance.now() - started < 5000, "Representative change intake exceeded five seconds");
});
