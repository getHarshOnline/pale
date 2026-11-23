import assert from "node:assert/strict";
import fs from "node:fs";

/** Exercise the actual portable WASM boundary, including consumed handles. */
export function verifyAuthoringOccurrencePages(bindings, fixtureRoot) {
  const fixture = JSON.parse(fs.readFileSync(`${fixtureRoot}/portable-journey.json`, "utf8"));
  const api = new bindings.Pale();
  const compile = () => api.compilePortableWorkspace(new bindings.PortableWorkspaceRequest(
    fixture.root_manifest,
    fixture.document_labels.map(label => new bindings.PortableWorkspaceDocument(
      label, fs.readFileSync(`${fixtureRoot}/portable/${label}`),
    )),
  ));
  const workspace = compile();
  let workspaceReleased = false;
  const other = compile();
  const field = name => bindings.AuthoringInstancePathSegment.field(name);
  const navigation = workspace.navigateAuthoring(
    fixture.root_manifest, [field("workspace"), field("workspace_id")],
  );
  const identity = navigation.identity();
  assert.ok(identity instanceof bindings.AuthoringIdentity);
  const sourceNavigation = workspace.navigateAuthoring(
    fixture.root_manifest, [field("workspace"), field("resolution_lock")],
  );
  const sourceIdentity = sourceNavigation.identity();
  assert.ok(sourceIdentity instanceof bindings.AuthoringIdentity);
  assert.notEqual(sourceIdentity.kind(), identity.kind());

  const describeLocation = location => {
    const path = location.path().map(segment => {
      try {
        return segment.kind() === bindings.AuthoringInstancePathSegmentKind.Field
          ? { field: segment.fieldValue } : { index: segment.indexValue };
      } finally { segment.free(); }
    });
    return { sourceLabel: location.sourceLabel, path };
  };
  const rows = page => page.occurrences().map(occurrence => {
    const location = occurrence.location();
    try {
      return { ...describeLocation(location), role: occurrence.role() };
    } finally { location.free(); occurrence.free(); }
  });
  const exactPath = path => path.map(segment => "field" in segment
    ? bindings.AuthoringInstancePathSegment.field(segment.field)
    : bindings.AuthoringInstancePathSegment.index(Number(segment.index)));
  const rejectedContinuation = error => {
    assert.ok(error instanceof Error);
    assert.equal(error.diagnostic.code, "authoring_continuation_mismatch");
    assert.equal(error.message, error.diagnostic.summary);
    assert.deepEqual(JSON.parse(error.diagnosticJson), error.diagnostic);
    assert.equal(error.authoringLocation, undefined);
    assert.deepEqual(error.authoringCorrections, []);
    return true;
  };

  const walk = (inspected, returned) => {
    const collected = [];
    let cursor;
    let pages = 0;
    let sparse = false;
    do {
      const page = workspace.authoringOccurrences(identity, inspected, returned, cursor);
      // The previous handle has been consumed. The next is a new owned handle.
      cursor = undefined;
      try {
        const current = rows(page);
        assert.ok(page.inspected <= Math.min(inspected || 64, 256));
        assert.ok(current.length <= Math.min(returned || 64, 256));
        collected.push(...current);
        sparse ||= current.length === 0 && !page.isComplete;
        cursor = page.continuation();
        assert.equal(cursor === undefined, page.isComplete);
        assert.ok(++pages < 2048, "the small fixture must reach a proven terminal page");
      } finally { page.free(); }
    } while (cursor !== undefined);
    return { collected, sparse };
  };

  try {
    // Generated classes have private TypeScript constructors. Empty JavaScript
    // wrappers must still be unusable and must never turn into absent cursors.
    assert.throws(() => new bindings.AuthoringOccurrence().location());
    assert.throws(() => new bindings.AuthoringOccurrencePage().occurrences());
    for (const [index, invalid] of [null, {}, "cursor", 0, new bindings.AuthoringOccurrencePageCursor()].entries()) {
      assert.throws(() => workspace.authoringOccurrences(identity, 1, 1, invalid), error => {
        assert.ok(error instanceof Error);
        assert.match(error.message, /unconsumed authoring occurrence cursor/);
        assert.equal(Object.hasOwn(error, "diagnostic"), false);
        return true;
      }, `invalid cursor case ${index} must not restart the scan`);
    }
    const first = workspace.authoringOccurrences(identity, 1, 1);
    try {
      assert.equal(first.isComplete, false);
      // The generated identity check runs before Rust consumes the cursor.
      const identityRejected = first.continuation();
      assert.throws(() => workspace.authoringOccurrences({}, 1, 1, identityRejected));
      const afterIdentityRejection = workspace.authoringOccurrences(identity, 1, 1, identityRejected);
      afterIdentityRejection.free();
      assert.throws(() => other.authoringOccurrences(identity, 1, 1, first.continuation()),
        rejectedContinuation);
      assert.throws(() => workspace.authoringOccurrences(identity, 1, 2, first.continuation()),
        rejectedContinuation);
      assert.throws(() => workspace.authoringOccurrences(identity, 2, 1, first.continuation()),
        rejectedContinuation);
      const mismatched = first.continuation();
      assert.throws(() => workspace.authoringOccurrences(sourceIdentity, 1, 1, mismatched),
        rejectedContinuation);
      assert.throws(() => workspace.authoringOccurrences(identity, 1, 1, mismatched));
      const consumed = first.continuation();
      const resumed = workspace.authoringOccurrences(identity, 1, 1, consumed);
      resumed.free();
      assert.throws(() => workspace.authoringOccurrences(identity, 1, 1, consumed));
      const released = first.continuation();
      released.free();
      assert.throws(() => workspace.authoringOccurrences(identity, 1, 1, released));

      let coerced = false;
      const invalid = [undefined, null, true, false, "1", 1n, {},
        { valueOf() { coerced = true; return 1; } },
        -1, 0.5, NaN, Infinity, -Infinity, 65536, 2 ** 32];
      for (const value of invalid) {
        for (const limits of [[value, 1], [1, value]]) {
          const rejectLimit = error => {
            assert.ok(error instanceof Error);
            assert.match(error.message, /integer from 0 through 65535/);
            assert.equal(Object.hasOwn(error, "diagnostic"), false);
            return true;
          };
          assert.throws(() => workspace.authoringOccurrences(identity, ...limits), rejectLimit);
          const invalidated = first.continuation();
          assert.throws(() => workspace.authoringOccurrences(identity, ...limits, invalidated), rejectLimit);
          assert.throws(() => workspace.authoringOccurrences(identity, 1, 1, invalidated));
        }
      }
      assert.equal(coerced, false);
    } finally { first.free(); }

    for (const [requested, different] of [[0, 64], [257, 65535]]) {
      const page = workspace.authoringOccurrences(identity, requested, 1);
      try {
        assert.equal(page.isComplete, false);
        assert.throws(() => workspace.authoringOccurrences(identity, different, 1, page.continuation()),
          rejectedContinuation);
      } finally { page.free(); }
    }

    const small = walk(1, 1);
    assert.equal(small.sparse, true);
    assert.ok(small.collected.length > 1);
    for (const limits of [[0, 0], [64, 64], [256, 256], [65535, 65535]]) {
      assert.deepEqual(walk(...limits).collected, small.collected);
    }
    for (const occurrence of small.collected) {
      const resolved = workspace.navigateAuthoring(occurrence.sourceLabel, exactPath(occurrence.path));
      const resolvedIdentity = resolved.identity();
      try {
        assert.equal(resolved.occurrenceRole(), occurrence.role);
        assert.equal(resolvedIdentity.kind(), identity.kind());
        assert.equal(resolvedIdentity.value, identity.value);
        assert.equal(resolvedIdentity.authoredValue, identity.authoredValue);
      } finally { resolvedIdentity.free(); resolved.free(); }
    }
    // Navigation can report Projection as a reference. Preserve its actual role.
    const references = navigation.references().map(location => {
      try {
        const description = describeLocation(location);
        const resolved = workspace.navigateAuthoring(description.sourceLabel, exactPath(description.path));
        try { return { ...description, role: resolved.occurrenceRole() }; }
        finally { resolved.free(); }
      } finally { location.free(); }
    });
    assert.equal(navigation.referencesTruncated, false);
    const referenceRows = small.collected.filter(row =>
      row.role === bindings.AuthoringOccurrenceRole.Reference
      || row.role === bindings.AuthoringOccurrenceRole.Projection);
    assert.deepEqual(referenceRows, references);
    const definition = navigation.definition();
    try {
      assert.deepEqual(small.collected.filter(row => row.role === bindings.AuthoringOccurrenceRole.Definition),
        [{ ...describeLocation(definition), role: bindings.AuthoringOccurrenceRole.Definition }]);
    } finally { definition.free(); }
    const rowKey = row => JSON.stringify(row, (_key, value) => typeof value === "bigint" ? value.toString() : value);
    assert.equal(new Set(small.collected.map(rowKey)).size, small.collected.length);

    const pending = workspace.authoringOccurrences(identity, 1, 1);
    const retained = workspace.authoringOccurrences(identity, 65535, 65535);
    const before = rows(retained);
    assert.ok(before.length > 1);
    workspace.free();
    workspaceReleased = true;
    assert.deepEqual(rows(retained), before);
    const cursor = pending.continuation();
    const sibling = pending.continuation();
    pending.free();
    retained.free();
    cursor.free();
    sibling.free();
  } finally {
    if (!workspaceReleased) workspace.free();
    other.free(); identity.free(); navigation.free(); sourceIdentity.free(); sourceNavigation.free(); api.free();
  }
  return "workspace.portable-authoring-occurrences";
}
