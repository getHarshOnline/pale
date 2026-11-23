import assert from "node:assert/strict";
import crypto from "node:crypto";
import fs from "node:fs";
import { createRequire } from "node:module";
import path from "node:path";

const SMOKE_SAMPLES = 5;
const CHARACTERIZATION_SAMPLES = 201;
const MAX_COORDINATE_BYTES = 512;

function coordinate(name, smoke, fallback) {
  const value = process.env[name] ?? (smoke ? fallback : undefined);
  assert(value, `${name} is required for a characterized run`);
  assert(Buffer.byteLength(value) <= MAX_COORDINATE_BYTES, `${name} exceeds 512 bytes`);
  assert(!/[\t\r\n]/u.test(value), `${name} must be one TSV-safe line`);
  return value;
}

function percentile(sorted, rank) {
  return sorted[Math.ceil((rank / 100) * sorted.length) - 1];
}

function distribution(observations) {
  const sorted = [...observations].sort((left, right) => left - right);
  return {
    p50: percentile(sorted, 50),
    p95: percentile(sorted, 95),
    p99: sorted.length >= 101 ? percentile(sorted, 99) : "",
    max: sorted.at(-1),
  };
}

function measure(samples, operation) {
  for (let index = 0; index < 3; index += 1) operation();
  const observations = [];
  for (let index = 0; index < samples; index += 1) {
    const started = process.hrtime.bigint();
    operation();
    observations.push(Number(process.hrtime.bigint() - started));
  }
  return distribution(observations);
}

function fixtureDigest(fixtureRoot, portable) {
  const members = [
    "bundle-identity.json",
    "contract-diagnostic.json",
    "portable-journey.json",
    ...portable.document_labels.map((label) => `portable/${label}`),
  ];
  const digest = crypto.createHash("sha256");
  digest.update("pale.cross-leaf-ffi-benchmark-fixture.v1\0");
  for (const member of members) {
    digest.update(member);
    digest.update("\0");
    digest.update(fs.readFileSync(path.join(fixtureRoot, member)));
    digest.update("\0");
  }
  return digest.digest("hex");
}

function portableRequest(bindings, rootManifest, documents) {
  return new bindings.PortableWorkspaceRequest(
    rootManifest,
    documents.map(({ label, bytes }) => new bindings.PortableWorkspaceDocument(label, bytes)),
  );
}

function compilePortable(api, bindings, portable, documents) {
  const request = portableRequest(bindings, portable.root_manifest, documents);
  return api.compilePortableWorkspace(request);
}

function takeImpactPage(comparison) {
  const page = comparison.impacts(0, 1);
  const nextOffset = page.nextOffset;
  const entries = page.intoEntries();
  return { nextOffset, entries };
}

function proveLaws(api, bindings, fixture, contract, portable, documents) {
  const identity = api.contractBundleIdentity();
  assert.equal(identity.memberCount, fixture.member_count);
  assert.equal(identity.schemaManifestSha256, fixture.schema_manifest_sha256);
  assert.equal(identity.schemaSetSha256, fixture.schema_set_sha256);
  identity.free();

  const diagnosticBytes = new TextEncoder().encode(contract.canonical_diagnostic);
  assert.deepEqual(Buffer.from(api.admitDiagnostic(diagnosticBytes)), Buffer.from(diagnosticBytes));
  const validated = api.validateContractDocument(contract.schema.registry_id, diagnosticBytes);
  assert.equal(validated.schemaId, contract.schema.registry_id);
  assert.equal(validated.schemaSha256, contract.schema.schema_sha256);
  assert.deepEqual(Buffer.from(validated.canonicalBytes), Buffer.from(diagnosticBytes));
  validated.free();

  const prior = compilePortable(api, bindings, portable, documents);
  const current = compilePortable(api, bindings, portable, documents);
  const actualCompilation = {
    workspace_id: prior.workspaceId,
    principal_universe_digest: prior.principalUniverseDigest,
    semantic_ir_digest: prior.semanticIrDigest,
    admission_evidence_digest: prior.admissionEvidenceDigest,
    source_document_count: prior.sourceDocumentCount,
    derivation_source_count: prior.derivationSourceCount,
    derivation_node_count: prior.derivationNodeCount,
    admission_evidence_count: prior.admissionEvidenceCount,
    adopted_release_count: prior.adoptedReleaseCount,
    package_release_count: prior.packageReleaseCount,
    normative_pack_release_count: prior.normativePackReleaseCount,
  };
  assert.deepEqual(actualCompilation, portable.compilation);

  const comparison = api.comparePortableWorkspaces(prior, current);
  const { nextOffset, entries } = takeImpactPage(comparison);
  assert.equal(nextOffset, portable.first_impact.next_offset);
  assert.equal(entries.length, 1);
  const [impact] = entries;
  const subject = impact.subject();
  assert.equal(subject.kind(), bindings.WorkspaceSubjectKind.Source);
  assert.equal(subject.sourceId, portable.first_impact.source_id);
  assert.equal(impact.state(), bindings.WorkspaceImpactState.Unaffected);
  assert.equal(impact.contributingRootCount, portable.first_impact.contributing_root_count);
  subject.free();
  impact.free();
  return { comparison, current, diagnosticBytes, prior };
}

function renderReport(configuration, fixtureSha256, cases) {
  const header = [
    "source_coordinate",
    "fixture_sha256",
    "run_coordinate",
    "evidence_class",
    "leaf",
    "operation",
    "law_gate_passed",
    "request_bytes",
    "response_bytes",
    "copy_lower_bound_bytes",
    "samples",
    "p50_ns",
    "p95_ns",
    "p99_ns",
    "max_ns",
    "hardware",
    "runtime",
    "arch",
    "proof_ceiling",
    "release_evidence",
  ].join("\t");
  const rows = cases.map((entry) => {
    const result = measure(configuration.samples, entry.run);
    return [
      configuration.source,
      fixtureSha256,
      configuration.run,
      configuration.smoke ? "smoke_validation" : "local_characterization",
      "wasm",
      entry.id,
      "true",
      entry.requestBytes,
      entry.responseBytes,
      entry.requestBytes + entry.responseBytes,
      configuration.samples,
      result.p50,
      result.p95,
      result.p99,
      result.max,
      configuration.hardware,
      process.version,
      `${process.platform}-${process.arch}`,
      "local_leaf_latency_and_payload_lower_bound_only",
      "false",
    ].join("\t");
  });
  const payload = `${header}\n${rows.join("\n")}\n`;
  const seal = crypto.createHash("sha256").update(payload).digest("hex");
  return `${payload}# pale-native-ffi-benchmark-seal\trows=${rows.length}\tpayload_sha256=${seal}\tsource_coordinate=${configuration.source}\trun_coordinate=${configuration.run}\n`;
}

const arguments_ = process.argv.slice(2);
const smokeIndex = arguments_.indexOf("--smoke");
const smoke = smokeIndex !== -1;
if (smoke) arguments_.splice(smokeIndex, 1);
assert.equal(arguments_.length, 2, "usage: native_ffi.mjs MODULE FIXTURES [--smoke]");
const [moduleArgument, fixtureArgument] = arguments_;
const fixtureRoot = path.resolve(fixtureArgument);
const modulePath = path.resolve(moduleArgument);
const packageMarker = path.join(path.dirname(modulePath), "package.json");
const temporaryPackageMarker = !fs.existsSync(packageMarker);
if (temporaryPackageMarker) {
  fs.writeFileSync(packageMarker, '{\n  "type": "commonjs"\n}\n');
}
let bindings;
try {
  bindings = createRequire(import.meta.url)(modulePath);
} finally {
  if (temporaryPackageMarker) fs.rmSync(packageMarker);
}
const api = new bindings.Pale();
const fixture = JSON.parse(fs.readFileSync(path.join(fixtureRoot, "bundle-identity.json")));
const contract = JSON.parse(fs.readFileSync(path.join(fixtureRoot, "contract-diagnostic.json")));
const portable = JSON.parse(fs.readFileSync(path.join(fixtureRoot, "portable-journey.json")));
const documents = portable.document_labels.map((label) => ({
  label,
  bytes: fs.readFileSync(path.join(fixtureRoot, "portable", label)),
}));
const { comparison, current, diagnosticBytes, prior } = proveLaws(
  api,
  bindings,
  fixture,
  contract,
  portable,
  documents,
);

const compilationRequestBytes = Buffer.byteLength(portable.root_manifest)
  + documents.reduce(
    (total, document) => total + Buffer.byteLength(document.label) + document.bytes.length,
    0,
  );
const compilationResponseBytes = Buffer.byteLength(portable.compilation.workspace_id)
  + (3 * 64)
  + (7 * 4);
const impactResponseBytes = Buffer.byteLength(portable.first_impact.source_id)
  + Buffer.byteLength(portable.first_impact.state)
  + 8;
const cases = [
  {
    id: "bundle_identity",
    requestBytes: 0,
    responseBytes: 4 + (2 * 64),
    run: () => {
      const identity = api.contractBundleIdentity();
      const count = identity.memberCount;
      identity.free();
      return count;
    },
  },
  {
    id: "diagnostic_admission",
    requestBytes: diagnosticBytes.length,
    responseBytes: diagnosticBytes.length,
    run: () => api.admitDiagnostic(diagnosticBytes).length,
  },
  {
    id: "contract_validation",
    requestBytes: Buffer.byteLength(contract.schema.registry_id) + diagnosticBytes.length,
    responseBytes: diagnosticBytes.length,
    run: () => {
      const validated = api.validateContractDocument(contract.schema.registry_id, diagnosticBytes);
      const length = validated.canonicalBytes.length;
      validated.free();
      return length;
    },
  },
  {
    id: "portable_compilation",
    requestBytes: compilationRequestBytes,
    responseBytes: compilationResponseBytes,
    run: () => {
      const compilation = compilePortable(api, bindings, portable, documents);
      const count = compilation.sourceDocumentCount;
      compilation.free();
      return count;
    },
  },
  {
    id: "retained_impact_page",
    requestBytes: 14,
    responseBytes: impactResponseBytes,
    run: () => {
      const { entries } = takeImpactPage(comparison);
      for (const entry of entries) entry.free();
      return entries.length;
    },
  },
];
const configuration = {
  smoke,
  samples: smoke ? SMOKE_SAMPLES : CHARACTERIZATION_SAMPLES,
  source: coordinate(
    "PALE_FFI_BENCH_SOURCE_COORDINATE",
    smoke,
    "smoke-unsealed-worktree",
  ),
  run: coordinate("PALE_FFI_BENCH_RUN_COORDINATE", smoke, "smoke-validation"),
  hardware: coordinate(
    "PALE_FFI_BENCH_HARDWARE",
    smoke,
    "unspecified-smoke-host",
  ),
};
process.stdout.write(renderReport(configuration, fixtureDigest(fixtureRoot, portable), cases));
comparison.free();
current.free();
prior.free();
api.free();
