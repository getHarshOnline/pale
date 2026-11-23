import assert from "node:assert/strict";
import test from "node:test";
import { canonicalize } from "../dist/index.js";
test("RFC 8785 number serialization vector", () => assert.equal(canonicalize([333333333.33333329, 1e30, 4.5, 2e-3, 1e-27]), "[333333333.3333333,1e+30,4.5,0.002,1e-27]"));
test("RFC 8785 property ordering vector", () => assert.equal(canonicalize({ z: 1, a: 2, "€": 3 }), '{"a":2,"z":1,"€":3}'));
test("non-finite values are rejected", () => assert.throws(() => canonicalize(Number.NaN), /non-finite/));
test("RFC 8785 escapes control characters without escaping valid Unicode", () => assert.equal(canonicalize("€$\u000f\nA'B\"\\\"/"), '"€$\\u000f\\nA\'B\\"\\\\\\"/"'));
test("I-JSON invalid Unicode and sparse arrays are rejected", () => {
  assert.throws(() => canonicalize("\ud800"), /lone Unicode surrogate/);
  assert.throws(() => canonicalize(new Array(2)), /sparse arrays/);
});
