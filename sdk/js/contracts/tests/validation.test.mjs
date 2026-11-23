import assert from "node:assert/strict";
import test from "node:test";
import {
  AUTHORED_INPUT_FAMILIES,
  CandidateValidationError,
  CONTRACT_IDENTITIES,
  SCHEMA_FAMILIES,
  SCHEMA_MANIFEST_SHA256,
  SCHEMA_SET_SHA256,
  buildAuthoredInput,
  validate,
} from "../dist/index.js";

test("validator inventory covers every authored family", () => {
  assert.ok(SCHEMA_FAMILIES.length > 0);
  assert.ok(AUTHORED_INPUT_FAMILIES.every((family) => SCHEMA_FAMILIES.includes(family)));
});

test("validation rejects an invalid authored record", () => {
  assert.equal(validate("pale-workspace.v1", {}).valid, false);
});

test("resource paths remain distinct from Package identities", () => {
  const selection = (packageId, resourceId) => ({
    capability_bindings: {},
    semantic: { doctrine: [{ selection: "exact", resource: {
      package_id: packageId, resource_id: resourceId,
    } }] },
  });
  for (const path of ["motion", "motion/time-models", "web/css", "tools/v1.check"]) {
    assert.equal(validate("capabilities.v1", selection("example.design", path)).valid, true, path);
  }
  for (const path of ["/motion", "motion/", "motion//time", "../motion",
    "motion/../time", "motion/./time", "motion\\time", "motion/%2e%2e",
    "motion/time#heading", "motion/time?mode=1", "motion/Time", "motion/éclair"]) {
    assert.equal(validate("capabilities.v1", selection("example.design", path)).valid, false, path);
  }
  assert.equal(validate("capabilities.v1", selection("example/design", "motion")).valid, false);
});

test("the generated boundary consumes the current diagnostic contract", () => {
  const diagnostic = {
    schema: "pale/diagnostic.v1",
    schema_sha256: CONTRACT_IDENTITIES["diagnostic.v1"],
    schema_version: 1,
    code: "resident_recovery_required",
    stage: "resident_recovery",
    severity: "error",
    disposition: "blocked",
    summary: "resident recovery evidence must be reconciled",
    retry: "reconcile_first",
    effect: "not_applicable",
    truncation: {
      shortened_fields: 0,
      omitted_causes: 0,
      omitted_notes: 0,
      omitted_help: 0,
      omitted_facts: 0,
    },
  };

  const admitted = validate("diagnostic.v1", diagnostic);
  assert.equal(admitted.valid, true);
  if (admitted.valid) assert.equal(admitted.value.stage, "resident_recovery");

  assert.equal(
    validate("diagnostic.v1", { ...diagnostic, schema_version: 0x1_0000_0000 }).valid,
    false,
  );
});

test("builder reports schema failure", () => {
  assert.throws(() => buildAuthoredInput("pale-workspace.v1", {}), CandidateValidationError);
});

test("non-authored family cannot enter builder", () => {
  for (const family of [
    "diagnostic.v1",
    "local-admin-command.v1",
    "deployment-control-store-genesis-receipt.v1",
  ]) {
    assert.ok(SCHEMA_FAMILIES.includes(family));
    assert.throws(() => buildAuthoredInput(family, {}), /not an authored-input family/);
  }
});

test("builder owns and recursively freezes the exact canonicalized tree", () => {
  const source = {
    distribution: {
      compiler: "pale-surface@1",
      schema_manifest_sha256: SCHEMA_MANIFEST_SHA256,
      schema_set_sha256: SCHEMA_SET_SHA256,
    },
    package: [
      {
        id: "example.engine",
        version: "1.0.0",
        content_digest: "00a8332abc43aa6fe5b7f503bd4e6035a4658dc3898a9712a2aeba5422540875",
      },
    ],
  };
  const candidate = buildAuthoredInput("distribution-lock.v1", source);
  source.package[0].id = "mutated.after.build";

  assert.equal(candidate.value.package?.[0]?.id, "example.engine");
  assert.equal(JSON.parse(candidate.canonicalJson).package[0].id, "example.engine");
  assert.ok(Object.isFrozen(candidate.value));
  assert.ok(Object.isFrozen(candidate.value.distribution));
  assert.ok(Object.isFrozen(candidate.value.package));
  assert.ok(Object.isFrozen(candidate.value.package?.[0]));
  assert.throws(() => {
    candidate.value.distribution.compiler = "mutated";
  }, TypeError);
});

test("paired model/native tool names survive authoring and reject an empty callable name", () => {
  const tool = { name: "Bash", model_name: "exec_command", observability: "pre_tool_observable" };
  const source = { capability_bindings: { inspect: {
    approval: "none", effect_class: "read", implementation: {
      kind: "host_native", provider_tools: [{
        provider_binding: { provider_binding_id: "synthetic-terminal",
          provider_binding_identity: { provider_binding_sha256: "a".repeat(64) } },
        tools: [tool],
      }],
    },
  } } };
  const candidate = buildAuthoredInput("capabilities.v1", source);
  const retained = JSON.parse(candidate.canonicalJson)
    .capability_bindings.inspect.implementation.provider_tools[0].tools[0];
  assert.equal(retained.name, "Bash");
  assert.equal(retained.model_name, "exec_command");
  tool.model_name = "";
  assert.equal(validate("capabilities.v1", source).valid, false);
});
