import assert from "node:assert/strict";

// Compare a copied native value to fields already produced by the SDK's
// canonical serializer. This test helper neither constructs profile meaning
// nor supplies fixture data. Its closed enum map only translates host spelling.
export function assertProfileEntry(bindings, actual, expected) {
  assert(actual instanceof bindings.EffectiveProfileEntry);
  assert(actual.detail, "profile entry must expose its discriminated detail");
  assert.equal("kind" in actual, false, "no redundant outer kind alias");
  const { explanationHandle, ...detail } = expected;
  assert.equal(actual.explanationHandle, explanationHandle);
  const fieldFamilies = {
    resourceKind: "EffectiveProfileResourceKind", axis: "EffectiveProfileModeAxis",
    accessCeiling: "EffectiveProfileRepositoryAccess", exposure: "OperationExposure",
    nativeRealization: "EffectiveProfileNativeRealization", realization: "EffectiveProfileNativeRealization",
    authorityCeiling: "EffectiveProfileEffectClass", effectCeiling: "EffectiveProfileEffectClass",
    permittedEffects: "EffectiveProfileEffectClass", forbiddenEffects: "EffectiveProfileEffectClass",
    effectClass: "EffectiveProfileEffectClass", provider: "EffectiveProfileProviderKind",
    narrowingReason: "NarrowingReason", policyBlock: "PolicyBlock",
  };
  const enumValue = (family, name) => {
    assert(family, `missing enum family for ${name}`);
    const member = name.split("-").map((part) => part[0].toUpperCase() + part.slice(1)).join("");
    const value = bindings[family][member];
    assert.notEqual(value, undefined, `${family}.${member}`);
    return value;
  };
  function compare(value, record, field) {
    if (Array.isArray(record)) {
      assert(Array.isArray(value), field);
      assert.equal(value.length, record.length, field);
      record.forEach((item, index) => compare(value[index], item, field));
    } else if (record !== null && typeof record === "object") {
      assert(value !== null && typeof value === "object", field);
      if (field === "participant" || field === "responsibilities" || (field === "selection" && actual.detail.kind === "selection")) {
        assert.equal(typeof value.kind, "string", `${field} must expose a literal discriminant`);
        assert.equal(value.constructor, Object, `${field} must be a copied variant payload`);
      }
      if (value.constructor === Object) {
        assert.deepEqual(Object.keys(value).sort(), Object.keys(record).sort(), field);
      }
      for (const [key, expectedValue] of Object.entries(record)) {
        if (key === "kind") {
          assert.equal(value.kind, expectedValue);
          continue;
        }
        const enumAxis = key === "reason" ? (record.kind === "narrowed" ? "narrowingReason" : "policyBlock") : key;
        compare(value[key], expectedValue, enumAxis);
      }
    } else if (typeof record === "string" && fieldFamilies[field]) {
      assert.equal(value, enumValue(fieldFamilies[field], record), field);
    } else {
      assert.deepEqual(value, record, field);
    }
  }
  compare(actual.detail, detail, "entry detail");
}

export function assertProfilePage(bindings, page) {
  assert(Array.isArray(page.entries), "profile page must expose complete typed entries");
  const canonical = JSON.parse(Buffer.from(page.canonicalBytes));
  assert.equal(page.entryCount, canonical.entries.length);
  assert.equal(page.entries.length, canonical.entries.length);
  assert.equal(page.offset, canonical.offset);
  assert.equal(page.nextOffset ?? null, canonical.nextOffset);
  page.entries.forEach((entry, index) => assertProfileEntry(bindings, entry, canonical.entries[index]));
  return canonical;
}

// All callers use the canonical one-shot corpus. This checks its actual first
// intended-use row, not coverage of every possible SDK evidence variant.
export function assertIntendedUseExplanation(bindings, profile, row) {
  const result = profile.explain(row.explanationHandle);
  assert.equal(result.handle, row.explanationHandle);
  assert(result.evidence, "retained explanation must expose complete typed evidence");
  assert.equal(Object.hasOwn(result, "evidenceKind"), false);
  assert.equal(result.evidence.constructor, Object);
  const evidence = result.evidence;
  if (row.kind === "declared-selection") {
    assert.equal(evidence.kind, "declared-selection");
    const selection = evidence.selection;
    assert.equal(typeof selection.kind, "string", "declared selection must be a literal-discriminated value");
    assert.equal(row.selection.kind, "resource");
    assert.equal(selection.kind, "resource");
    assert.equal(row.selection.resourceKind, "role");
    assert.equal(selection.resourceKind, bindings.EffectiveProfileResourceKind.Role);
    assert.equal(row.selection.selector.kind, "exact");
    assert.equal(selection.selector.kind, "exact");
    assert.equal(selection.selector.resourceId, row.selection.selector.resourceId);
    assert.deepEqual(Object.keys(selection).sort(), ["kind", "resourceKind", "selector"]);
    assert.deepEqual(Object.keys(selection.selector).sort(), ["kind", "resourceId"]);
    for (const name of ["EffectiveProfileDeclaredSelection", "EffectiveProfileDeclaredSelectionKind",
      "EffectiveProfileDeclaredSelector", "EffectiveProfileDeclaredSelectorKind"]) {
      assert.equal(name in bindings, false, "unreleased shell or kind alias remains");
    }
    assert.deepEqual(Object.keys(evidence).sort(), ["kind", "selection"]);
  } else {
    assert.equal(row.kind, "responsibility");
    assert.equal(evidence.kind, "responsibility");
    assert.equal(evidence.contractId, row.contractId);
    assert.equal(evidence.executionPathId, row.executionPathId);
    assert.equal(evidence.reasonSteps.length, row.reasonSteps.length);
    evidence.reasonSteps.forEach((reason, index) => {
      const expected = row.reasonSteps[index];
      assert.equal(reason.constructor, Object);
      assert.deepEqual(reason, expected);
    });
    assert.deepEqual(Object.keys(evidence).sort(), ["contractId", "executionPathId", "kind", "reasonSteps"]);
  }
  return result;
}
