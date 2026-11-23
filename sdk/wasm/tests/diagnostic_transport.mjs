import assert from "node:assert/strict";

// Restore each host function before assertions or other ABI tests can use it.
function captureWithReplacement(api, target, key, replacement) {
  const original = target[key];
  try {
    target[key] = replacement;
    try {
      api.contractSchema("not-a-selected-schema");
    } catch (failure) {
      return failure;
    }
  } finally {
    target[key] = original;
  }
  assert.fail("the schema lookup must fail");
}

export function verifyDiagnosticTransport(api) {
  const hostFailure = new Error("host diagnostic transport failed");
  for (const [target, key] of [[globalThis, "Error"], [JSON, "parse"], [Reflect, "set"]]) {
    const failure = captureWithReplacement(api, target, key, () => { throw hostFailure; });
    assert.equal(failure, hostFailure);
  }

  const originalSet = Reflect.set;
  for (const rejectedProperty of ["diagnostic", "diagnosticJson", "authoringCorrections", "authoringLocation"]) {
    const thrown = captureWithReplacement(api, Reflect, "set", (target, key, value) => {
      if (key === rejectedProperty) throw hostFailure;
      return originalSet(target, key, value);
    });
    assert.equal(thrown, hostFailure);
    const failure = captureWithReplacement(api, Reflect, "set", (target, key, value) =>
      key === rejectedProperty ? false : originalSet(target, key, value));
    assert.equal(failure, "Pale could not attach diagnostic data to the host error");
  }
}
