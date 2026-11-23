// Copyright © 2026 RawThoughts Enterprises Private Limited. Apache-2.0.
import assert from "node:assert/strict";
import { assertProfileEntry } from "./effective_profile.mjs";

const invalidLimits = [-1, 0.5, 65536, 2 ** 32, NaN, Infinity, -Infinity];
const locationKey = (location) => JSON.stringify(location);

function mismatch(operation) {
  assert.throws(operation, (error) => {
    assert(error instanceof Error);
    assert.equal(error.diagnostic.code, "authoring_continuation_mismatch");
    assert.equal(error.message, error.diagnostic.summary);
    const diagnostic = JSON.parse(Buffer.from(error.diagnostic.canonicalBytes).toString());
    assert.equal(diagnostic.code, error.diagnostic.code);
    assert.equal(diagnostic.summary, error.diagnostic.summary);
    assert.equal(diagnostic.source, undefined);
    return true;
  });
}

function invalidLimit(operation) {
  assert.throws(operation, (error) => {
    assert(error instanceof Error);
    assert.equal(error.code, "InvalidArg");
    assert.match(error.message, /integer from 0 through 65535/);
    assert.equal(error.diagnostic, undefined, "carrier misuse is not an API rejection");
    return true;
  });
}

function occurrences(bindings, read, navigation, navigate) {
  const resolve = (location) => navigate(location.sourceLabel, location.path.map((segment) => {
    assert.notEqual(segment.field != null, segment.index != null);
    return segment.field != null
      ? bindings.AuthoringInstancePathSegment.field(segment.field)
      : bindings.AuthoringInstancePathSegment.index(segment.index);
  }));
  const display = (identity) => ({
    kind: identity.kind, value: identity.value, authoredValue: identity.authoredValue,
  });
  const rows = [];
  let after;
  let pageCount = 0;
  let emptyIncomplete = false;
  do {
    assert(++pageCount < 4096, "fixture scan must terminate");
    const page = read(1, 1, after);
    assert(page instanceof bindings.AuthoringOccurrencePage);
    assert(page.inspected <= 1);
    const copied = page.occurrences();
    assert(copied.length <= 1);
    const expected = page.occurrences();
    const expectedBytes = expected.reduce((pageBytes, row) => pageBytes
      + Buffer.byteLength(row.location.sourceLabel)
      + row.location.path.reduce((pathBytes, segment) => pathBytes
        + (segment.field == null ? 0 : Buffer.byteLength(segment.field)), 0), 0);
    assert.equal(page.copiedStringBytes, expectedBytes);
    rows.push(...expected);
    if (copied.length) {
      copied[0].location.sourceLabel = "changed copy";
      copied[0].location.path.push({ field: "changed copy" });
      assert.deepEqual(page.occurrences(), expected);
    }
    after = page.continuation();
    assert.equal(page.isComplete, after === null);
    if (after !== null) {
      assert(after instanceof bindings.AuthoringOccurrencePageCursor);
      if (!expected.length) emptyIncomplete = true;
    }
  } while (after !== null);
  for (const row of rows) {
    const resolved = resolve(row.location);
    assert.deepEqual(display(resolved.identity()), display(navigation.identity()));
    assert.equal(row.role, resolved.occurrenceRole);
  }
  const keysForRole = (entries, role) => entries
    .filter((row) => row.role === role).map((row) => locationKey(row.location)).sort();
  const definition = navigation.definition();
  assert(definition, "these fixture identities must have a real definition");
  assert.equal(resolve(definition).occurrenceRole, bindings.AuthoringOccurrenceRole.Definition);
  assert.deepEqual(keysForRole(rows, bindings.AuthoringOccurrenceRole.Definition), [locationKey(definition)]);
  assert.equal(navigation.referencesTruncated, false);
  // Navigation includes content-bearing projections among its references.
  // Resolve their exact roles; neither list may substitute for the other.
  const references = navigation.references().map((location) => ({
    location, role: resolve(location).occurrenceRole,
  }));
  for (const role of [bindings.AuthoringOccurrenceRole.Reference,
    bindings.AuthoringOccurrenceRole.Projection]) {
    assert.deepEqual(keysForRole(rows, role), keysForRole(references, role));
  }
  assert.equal(new Set(rows.map((row) => `${row.role}:${locationKey(row.location)}`)).size, rows.length);
  assert.deepEqual(read(0, 0).occurrences(), read(64, 64).occurrences());
  assert(read(65535, 65535).inspected <= 256);
  assert(read(65535, 65535).occurrences().length <= 256);
  return { rows, emptyIncomplete };
}

// Actual compiled fixtures are supplied by the main ABI journey. No identity
// is reconstructed from its display properties and no page triggers a compile.
export function exerciseAuthoringPages(bindings, api, {
  workspace, rebuiltWorkspace, portableNavigation, effective, rebuiltEffective,
  effectiveWorkspace, effectiveIdentity, profileRows,
}) {
  assert(Number.isSafeInteger(bindings.MAX_AUTHORING_OCCURRENCE_PAGE_COPIED_STRING_BYTES)
    && bindings.MAX_AUTHORING_OCCURRENCE_PAGE_COPIED_STRING_BYTES > 0);
  const copiedStringBytes = Object.getOwnPropertyDescriptor(
    bindings.AuthoringOccurrencePage.prototype, "copiedStringBytes",
  )?.get;
  assert.equal(typeof copiedStringBytes, "function");
  for (const fabricated of [{}, Object.create(bindings.AuthoringOccurrencePage.prototype)]) {
    assert.throws(
      () => copiedStringBytes.call(fabricated),
      "the copied-string counter rejects fabricated native page wrappers",
    );
  }
  const portableIdentity = portableNavigation.identity();
  const portable = (inspected, returned, after) => api.portableWorkspaceAuthoringOccurrences(
    workspace, portableIdentity, inspected, returned, after,
  );
  const portableResult = occurrences(bindings, portable, portableNavigation,
    (source, path) => api.navigatePortableWorkspaceAuthoring(workspace, source, path));
  assert(portableResult.rows.length > 0);
  const portableCursor = portable(1, 0).continuation();
  assert(portableCursor instanceof bindings.AuthoringOccurrencePageCursor);
  mismatch(() => api.portableWorkspaceAuthoringOccurrences(
    rebuiltWorkspace, portableIdentity, 1, 0, portableCursor,
  ));
  mismatch(() => portable(1, 64, portableCursor));
  mismatch(() => portable(64, 0, portableCursor));

  const field = (name) => bindings.AuthoringInstancePathSegment.field(name);
  const repositoryNavigation = effective.navigateAuthoring("pale-workspace.toml", [
    field("workspace"), field("desired_topology"), field("repositories"),
    bindings.AuthoringInstancePathSegment.index(0), field("id"),
  ]);
  const repositoryIdentity = repositoryNavigation.identity();
  const profile = (inspected, returned, after) => effective.authoringOccurrences(
    repositoryIdentity, inspected, returned, after,
  );
  const effectiveResult = occurrences(bindings, profile, repositoryNavigation,
    (source, path) => effective.navigateAuthoring(source, path));
  assert(effectiveResult.emptyIncomplete, "a sparse scan must not equate an empty page with completion");
  const profileCursor = profile(1, 0).continuation();
  assert(profileCursor instanceof bindings.AuthoringOccurrencePageCursor);
  mismatch(() => rebuiltEffective.authoringOccurrences(repositoryIdentity, 1, 0, profileCursor));
  mismatch(() => effective.authoringOccurrences(effectiveIdentity, 1, 0, profileCursor));
  mismatch(() => profile(1, 64, profileCursor));
  mismatch(() => profile(64, 0, profileCursor));
  mismatch(() => api.portableWorkspaceAuthoringOccurrences(
    effectiveWorkspace, repositoryIdentity, 1, 0, profileCursor,
  ));
  mismatch(() => effective.authoringOccurrences(portableIdentity, 1, 0, portableCursor));
  for (const read of [portable, profile]) {
    const defaultScanCursor = read(0, 1).continuation();
    assert(defaultScanCursor instanceof bindings.AuthoringOccurrencePageCursor);
    mismatch(() => read(64, 1, defaultScanCursor));
    const cappedScanCursor = read(257, 1).continuation();
    assert(cappedScanCursor instanceof bindings.AuthoringOccurrencePageCursor);
    mismatch(() => read(65535, 1, cappedScanCursor));
  }

  const reference = effective.inspectEffectiveAuthoring(repositoryIdentity);
  assert.equal(reference.truncated, false);
  assert(reference.entries.length > 1, "fixture must exercise a real continuation");
  let association = effective.inspectEffectiveAuthoringPage(repositoryIdentity, 1);
  const associationCursor = association.continuation();
  assert(associationCursor instanceof bindings.EffectiveAuthoringContinuation);
  const handles = [];
  do {
    assert(association instanceof bindings.EffectiveAuthoringPage);
    assert.deepEqual(association.coordinates, reference.coordinates);
    const coordinateCopy = association.coordinates;
    coordinateCopy.instanceDigest = "changed copy";
    assert.deepEqual(association.coordinates, reference.coordinates);
    const rows = association.entries;
    assert(rows.length <= 1);
    for (const row of rows) {
      handles.push(row.explanationHandle);
      assertProfileEntry(bindings, row, profileRows.get(row.explanationHandle));
      assert.equal(effective.explain(row.explanationHandle).handle, row.explanationHandle);
    }
    rows.length = 0;
    assert.equal(association.entries.length, 1);
    const after = association.continuation();
    association = after === null ? null
      : effective.inspectEffectiveAuthoringPage(repositoryIdentity, 1, after);
  } while (association !== null);
  assert.deepEqual(handles, reference.entries.map((row) => row.explanationHandle));
  mismatch(() => rebuiltEffective.inspectEffectiveAuthoringPage(repositoryIdentity, 1, associationCursor));
  mismatch(() => effective.inspectEffectiveAuthoringPage(effectiveIdentity, 1, associationCursor));
  mismatch(() => effective.inspectEffectiveAuthoringPage(repositoryIdentity, 2, associationCursor));
  assert.deepEqual(
    effective.inspectEffectiveAuthoringPage(repositoryIdentity, 0).entries.map((row) => row.explanationHandle),
    effective.inspectEffectiveAuthoringPage(repositoryIdentity, 64).entries.map((row) => row.explanationHandle),
  );
  assert(effective.inspectEffectiveAuthoringPage(repositoryIdentity, 65535).entries.length <= 256);

  for (const invalid of invalidLimits) {
    for (const [read, cursor] of [[portable, portableCursor], [profile, profileCursor]]) {
      for (const after of [undefined, cursor]) {
        invalidLimit(() => read(invalid, 0, after));
        invalidLimit(() => read(1, invalid, after));
      }
    }
    for (const after of [undefined, associationCursor]) {
      invalidLimit(() => effective.inspectEffectiveAuthoringPage(repositoryIdentity, invalid, after));
    }
  }

  // JS aliases and independently returned cursor wrappers share the SDK owner.
  const alias = effective;
  let retainedPage = profile(1, 0);
  const firstCopy = retainedPage.continuation();
  const secondCopy = retainedPage.continuation();
  retainedPage = null;
  globalThis.gc?.();
  assert.deepEqual(
    alias.authoringOccurrences(repositoryIdentity, 1, 0, firstCopy).occurrences(),
    alias.authoringOccurrences(repositoryIdentity, 1, 0, secondCopy).occurrences(),
  );
  return [
    "workspace.portable-authoring-occurrences",
    "workspace.effective-profile-authoring-occurrences",
    "workspace.effective-profile-authoring-pages",
  ];
}
