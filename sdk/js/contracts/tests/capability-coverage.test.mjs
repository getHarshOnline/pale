import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import * as contracts from "../dist/index.js";

const witnesses = JSON.parse(await readFile(
  new URL("capability-witnesses.json", import.meta.url),
  "utf8",
));

test("each selected TypeScript capability has its public callable or value witness", () => {
  assert.deepEqual(witnesses, {
    "contract.bundle-identity": [
      "SCHEMA_FAMILIES",
      "SCHEMA_MANIFEST_SHA256",
      "SCHEMA_SET_SHA256",
    ],
    "contract.document-validation": ["validate"],
    "contract.schema-lookup": ["CONTRACT_IDENTITIES"],
  });

  for (const [capability, exports] of Object.entries(witnesses)) {
    assert.ok(exports.length > 0, `${capability} has no public witness`);
    for (const name of exports) {
      assert.ok(Object.hasOwn(contracts, name), `${capability} requires the public export ${name}`);
    }
  }

  assert.ok(contracts.SCHEMA_FAMILIES.length > 0);
  assert.match(contracts.SCHEMA_MANIFEST_SHA256, /^[0-9a-f]{64}$/);
  assert.match(contracts.SCHEMA_SET_SHA256, /^[0-9a-f]{64}$/);
  assert.match(contracts.CONTRACT_IDENTITIES["diagnostic.v1"], /^[0-9a-f]{64}$/);
  assert.equal(contracts.validate("diagnostic.v1", {}).valid, false);
});
