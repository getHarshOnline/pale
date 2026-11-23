import assert from "node:assert/strict";
import test from "node:test";
import { createN8nCodeChecker } from "./n8n-code.mjs";

const checker = createN8nCodeChecker({
  linter: {
    verify(source) {
      return source.includes("invalidCode")
        ? [{ line: 2, message: "Undefined identifier" }]
        : [];
    },
  },
  format: async (code) => {
    if (code.includes("formatterFailure")) throw new Error("Cannot format Code");
    return code.trim();
  },
  recommendedRules: {},
});

function fixture(secondCode) {
  return {
    name: "Workflow",
    nodes: [
      { name: "First", type: "n8n-nodes-base.code", parameters: { jsCode: " return []; " } },
      { name: "Middle", type: "n8n-nodes-base.noOp", parameters: { jsCode: " untouched " } },
      { name: "Last", type: "n8n-nodes-base.code", parameters: { jsCode: secondCode } },
    ],
  };
}

test("a later formatter failure preserves the workflow", async () => {
  const workflow = fixture(" formatterFailure ");
  const before = structuredClone(workflow);

  await assert.rejects(checker.formatCodeNodes(workflow), /Cannot format Code/);
  assert.deepEqual(workflow, before);
});

test("a lint failure preserves the workflow and reports its source line", async () => {
  const workflow = fixture(" return invalidCode; ");
  const before = structuredClone(workflow);

  await assert.rejects(checker.formatCodeNodes(workflow), /Workflow \/ Last:1 — Undefined identifier/);
  assert.deepEqual(workflow, before);
});

test("accepted formatting updates the original Code nodes only", async () => {
  const workflow = fixture(" return [1]; ");
  const [first, middle, last] = workflow.nodes;

  assert.strictEqual(await checker.formatCodeNodes(workflow), workflow);
  assert.strictEqual(workflow.nodes[0], first);
  assert.strictEqual(workflow.nodes[2], last);
  assert.equal(first.parameters.jsCode, "return [];");
  assert.equal(last.parameters.jsCode, "return [1];");
  assert.equal(middle.parameters.jsCode, " untouched ");
});

test("a missing JavaScript body or a different Code language leaves verification open", async () => {
  const refusedParameters = [
    undefined,
    {},
    { jsCode: null },
    { language: "python", jsCode: "return [];" },
  ];
  for (const parameters of refusedParameters) {
    const workflow = fixture("return [];");
    workflow.nodes[2].parameters = parameters;
    const before = structuredClone(workflow);
    const diagnostic = /Workflow \/ Last: expected a JavaScript Code body/;

    assert.throws(() => checker.checkWorkflowCode(workflow), diagnostic);
    await assert.rejects(checker.formatCodeNodes(workflow), diagnostic);
    assert.deepEqual(workflow, before);
  }
});
