import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import {
  compareAppsmithExport, compareSavedApplication, compareWorkflow, contract, InvalidRetainedEvidence,
} from './compare.mjs';

const coverage = {
  n8n: 'retained-workflow-export',
  'appsmith-export': 'retained-unpublished-app-export',
  'appsmith-saved': 'saved-import-and-lint-receipt',
};

function reject(code, details = {}, exit = 2) {
  console.error(JSON.stringify({ contract, diagnostic: { code, ...details } }));
  process.exitCode = exit;
}

async function readArtifacts(expectedPath, observedPath) {
  const roles = ['expected', 'observed'];
  const files = await Promise.allSettled([readFile(expectedPath), readFile(observedPath)]);
  const unreadable = roles.filter((_, index) => files[index].status === 'rejected');
  if (unreadable.length) {
    reject('unreadable-artifact', { artifacts: unreadable });
    return;
  }

  const inputSha256 = Object.fromEntries(roles.map((role, index) =>
    [role, createHash('sha256').update(files[index].value).digest('hex')]));
  const invalid = [];
  const [expected, observed] = files.map((file, index) => {
    try {
      return JSON.parse(file.value);
    } catch (error) {
      if (!(error instanceof SyntaxError)) throw error;
      invalid.push(roles[index]);
    }
  });
  if (invalid.length) {
    reject('invalid-json', { artifacts: invalid, inputSha256 });
    return;
  }
  return { expected, observed, inputSha256 };
}

async function main() {
  const [kind, expectedPath, observedPath, ...deployments] = process.argv.slice(2);
  if (!expectedPath || !observedPath || !Object.hasOwn(coverage, kind) ||
      (kind !== 'appsmith-saved' && deployments.length) || (kind === 'appsmith-saved' && !deployments.length)) {
    reject('invalid-invocation', {
      usage: 'verify.mjs n8n|appsmith-export <expected> <observed> or appsmith-saved <expected> <observed> <accepted-deployment> [...]',
    });
    return;
  }

  const inputs = await readArtifacts(expectedPath, observedPath);
  if (!inputs) return;
  const { expected, observed, inputSha256 } = inputs;

  let findings;
  try {
    findings = kind === 'n8n' ? compareWorkflow(expected, observed) :
      kind === 'appsmith-export' ? compareAppsmithExport(expected, observed) :
      compareSavedApplication(expected, inputSha256.expected, observed, deployments);
  } catch (error) {
    if (!(error instanceof InvalidRetainedEvidence)) throw error;
    reject('invalid-retained-evidence', { inputSha256, message: error.message });
    return;
  }

  const matches = kind === 'n8n' ?
    findings.graphMatches && findings.pinnedDataEmpty &&
      ['codeDrift', 'bindingDrift', 'parameterDrift', 'behaviorDrift', 'settingsDrift']
        .every(key => findings[key].length === 0) :
    kind === 'appsmith-export' ? Object.values(findings).every(surface =>
      Object.values(surface).every(changes => changes.length === 0)) :
    findings.sourceMatches && findings.pages.every(page => !page.nativeUpdatePending);
  console.log(JSON.stringify({ contract, coverage: coverage[kind], inputSha256, matches, findings }));
  process.exitCode = matches ? 0 : 1;
}

try {
  await main();
} catch {
  reject('unexpected-verifier-failure', {}, 3);
}
