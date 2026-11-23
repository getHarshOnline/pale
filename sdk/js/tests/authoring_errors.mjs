// Copyright © 2026 RawThoughts Enterprises Private Limited. Apache-2.0.
import assert from "node:assert/strict";
import fs from "node:fs";
import { createRequire } from "node:module";
import path from "node:path";
import { fileURLToPath } from "node:url";

// These local fixtures preserve the compiler/LSP's qualified rejected inputs.
const fixture = new URL("./fixtures/authoring/visibility-widening/", import.meta.url);

function rejected(operation) {
  try { operation(); } catch (error) {
    assert(error instanceof Error);
    return error;
  }
  assert.fail("the canonical request must be rejected");
}

export function exerciseAuthoringErrors(bindings, api) {
  const documents = fs.readdirSync(fixture).sort().map((label) => ({
    label, bytes: fs.readFileSync(new URL(label, fixture)),
  }));
  const request = { rootManifest: "pale-workspace.toml", documents };
  let error = rejected(() => api.compilePortableWorkspace(request));
  assert.equal(error.diagnostic.code, "workspace_semantics_rejected");
  assert(Array.isArray(error.authoringCorrections), "API correction sidecar must survive the error boundary");
  assert.equal(error.authoringCorrections.length, 1);
  const correction = error.authoringCorrections[0];
  assert(correction instanceof bindings.AuthoringCorrection);
  assert.equal(correction.kind, bindings.AuthoringCorrectionKind.NarrowImportVisibility);
  assert.equal(correction.sourceLabel, "consumer-module.toml");
  const expectedPath = [{ field: "module" }, { field: "imports" }, { index: 0 }, { field: "visibility_ceiling" }];
  assert.deepEqual(correction.path, expectedPath);
  assert.equal(correction.expectedSource, '"public"');
  assert.equal(correction.replacementSource, '"protected"');
  assert.equal(correction.proofSubject.kind, bindings.AuthoringIdentityKind.Export);
  assert.equal(correction.proofSubject.value, "candidate::public-api");
  assert.equal(correction.proofSubject.authoredValue, "public-api");
  assert.equal(correction.diagnosticIdentity, error.diagnostic.identity);
  assert.deepEqual(error.authoringLocation, { sourceLabel: correction.sourceLabel, path: expectedPath });
  const canonical = Buffer.from(error.diagnostic.canonicalBytes);
  assert.equal(JSON.parse(canonical).code, error.diagnostic.code);
  assert(!canonical.toString().includes(fileURLToPath(fixture)));
  assert(!Object.hasOwn(JSON.parse(canonical), "authoring_corrections"));
  error = undefined;
  global.gc?.();
  const pathCopy = correction.path;
  pathCopy.length = 0;
  assert.deepEqual(correction.path, expectedPath);
  assert.equal(correction.expectedSource, '"public"');
  assert.throws(() => new bindings.AuthoringCorrection());

  const invalidDocuments = documents.map((document) => document.label === request.rootManifest
    ? { ...document, bytes: Buffer.from(document.bytes.toString().replace("schema_version = 1", "schema_version = 2")) }
    : document);
  const invalid = rejected(() => api.compilePortableWorkspace({ ...request, documents: invalidDocuments }));
  assert.equal(invalid.diagnostic.code, "workspace_contract_rejected");
  assert.deepEqual(invalid.authoringCorrections, []);
  assert.deepEqual(invalid.authoringLocation, {
    sourceLabel: request.rootManifest, path: [{ field: "workspace" }, { field: "schema_version" }],
  });
  const unrelated = rejected(() => api.contractSchema("not-a-selected-schema"));
  assert.deepEqual(unrelated.authoringCorrections, []);
  // Option::None is explicitly written as null, not an omitted property.
  assert(Object.hasOwn(unrelated, "authoringLocation"));
  assert.equal(unrelated.authoringLocation, null);
  assert.deepEqual(fs.readdirSync(fixture).sort().map((label) => fs.readFileSync(new URL(label, fixture))), documents.map(({ bytes }) => bytes));
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const bindings = createRequire(import.meta.url)(path.resolve(process.argv[2]));
  exerciseAuthoringErrors(bindings, new bindings.Pale());
  console.log("authoring error sidecars: correction, location-only, absent, retained copy PASS");
}
