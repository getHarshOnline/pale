import { dependencyClosure } from './dependency-closure.mjs';

// Statically check a supplied n8n workflow with caller-selected tool versions.
const codeNodeType = "n8n-nodes-base.code";
const defaultGlobals = [
  "$", "$json", "$input", "$execution", "$workflow", "$prevNode", "$runIndex",
  "$itemIndex", "$node", "$items", "$env", "$vars", "$getWorkflowStaticData",
  "$evaluateExpression", "$now", "$today", "DateTime", "require", "Buffer",
  "TextEncoder", "TextDecoder", "URL", "URLSearchParams", "console",
  "structuredClone",
];
const syntax = { ecmaVersion: 2024, sourceType: "script" };

// The runtime wraps the Code body so that both return and await are legal.
function wrap(code) {
  return `void async function () {\n${code}\n};`;
}

function javaScriptBody(workflowName, node) {
  const { language, jsCode } = node.parameters ?? {};
  if ((language !== undefined && language !== "javaScript") || typeof jsCode !== "string") {
    throw new Error(`${workflowName} / ${node.name}: expected a JavaScript Code body.`);
  }
  return jsCode;
}

export function createN8nCodeChecker({
  linter,
  recommendedRules,
  format,
  globalNames = defaultGlobals,
}) {
  const globals = Object.fromEntries(globalNames.map((name) => [name, "readonly"]));
  const referencesByCode = new Map();

  function lintCodeNode(code) {
    return linter.verify(wrap(code), {
      languageOptions: { ...syntax, globals },
      rules: {
        ...recommendedRules,
        "no-unused-vars": ["error", { ignoreRestSiblings: true, argsIgnorePattern: "^_" }],
      },
    }).map((message) => ({ ...message, line: Math.max(1, message.line - 1) }));
  }

  function freeCodeReferences(code) {
    if (referencesByCode.has(code)) return referencesByCode.get(code);
    const messages = linter.verify(wrap(code), { languageOptions: syntax, rules: {} });
    const parseError = messages.find((message) => message.fatal);
    if (parseError) {
      throw new Error(`Cannot analyse generated Code: ${parseError.message}`);
    }

    const references = new Set(
      linter.getSourceCode().scopeManager.globalScope.through
        .map((reference) => reference.identifier.name),
    );
    referencesByCode.set(code, references);
    return references;
  }

  function resolveCodeHelpers(names, roots, referenceSource) {
    const available = [...names];
    const known = new Set(available);

    return dependencyClosure(roots, name => {
      if (!known.has(name)) throw new Error(`Unknown helper: ${name}`);
      const source = referenceSource(name);
      if (source === undefined) return [];
      const references = freeCodeReferences(source);
      return available.filter(candidate => references.has(candidate));
    }, { depthFirst: true });
  }

  function findingsForNode(workflowName, nodeName, code) {
    return lintCodeNode(code).map((finding) => ({
      workflow: workflowName,
      node: nodeName,
      ...finding,
    }));
  }

  function checkWorkflowCode(workflow) {
    return workflow.nodes.flatMap((node) =>
      node.type === codeNodeType
        ? findingsForNode(workflow.name, node.name, javaScriptBody(workflow.name, node))
        : [],
    );
  }

  async function formatCodeNodes(workflow, options) {
    const prepared = [];
    for (const node of workflow.nodes) {
      if (node.type === codeNodeType) {
        const body = javaScriptBody(workflow.name, node);
        prepared.push({ node, code: await format(body, options) });
      }
    }
    const findings = prepared.flatMap(({ node, code }) =>
      findingsForNode(workflow.name, node.name, code),
    );
    if (findings.length) {
      const details = findings.map(({ node, line, message }) =>
        `${workflow.name} / ${node}:${line} — ${message}`,
      );
      throw new Error(`Code checks failed before export:\n${details.join("\n")}`);
    }
    for (const { node, code } of prepared) {
      node.parameters.jsCode = code;
    }
    return workflow;
  }

  return { lintCodeNode, freeCodeReferences, resolveCodeHelpers, checkWorkflowCode, formatCodeNodes };
}
