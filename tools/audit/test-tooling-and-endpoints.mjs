#!/usr/bin/env node
import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

import { ROOT, canonical } from './lib/validate-capability-audit-core.mjs';

const evidence = process.argv[2];
if (!evidence) throw new Error('usage: node tools/audit/test-tooling-and-endpoints.mjs <evidence-root>');
const runId = path.basename(evidence);
const sourceBaseline = '26026fb3862e8d178a2e59df7a68a2901e80b123';
const endpointCli = 'tools/audit/verify-mapping-endpoints.mjs';
const toolingCli = 'tools/audit/build-tooling-blobs.mjs';
const lspReceipts = fs.readdirSync(path.join(evidence, 'task-4-endpoint'))
  .filter((name) => name.endsWith('.references.receipt.json'))
  .map((name) => `${evidence}/task-4-endpoint/${name}`).join(',');

function invoke(cli, args) {
  return spawnSync(process.execPath, [cli, ...args], { cwd: ROOT, encoding: 'utf8' });
}
function digest(value) { return crypto.createHash('sha256').update(canonical(value)).digest('hex'); }
function reseal(value) {
  const unsigned = Object.fromEntries(Object.entries(value).filter(([key]) => key !== 'receipt_sha256'));
  return { ...unsigned, receipt_sha256: digest(unsigned) };
}
function endpointArgs(candidate, reconciliation) {
  return [
    '--candidate-manifest', candidate,
    '--test-reconciliation', reconciliation,
    '--graph-manifest', `${evidence}/task-4-graph-receipts-manifest.json`,
    '--module-receipt', `${evidence}/task-4-endpoint/module-declarations.receipt.json`,
    '--lsp-reference-receipts', lspReceipts,
  ];
}

const temporary = fs.mkdtempSync(path.join(evidence, '.trust-test-'));
try {
  const candidatePath = `${evidence}/task-4-candidate-manifest.json`;
  const reconciliationPath = `${evidence}/test-reconciliation.json`;
  const happy = invoke(endpointCli, endpointArgs(candidatePath, reconciliationPath));
  assert.equal(happy.status, 0, happy.stderr);
  assert.match(happy.stdout, /^mapping_endpoints=\d+ pass$/m);

  const candidates = JSON.parse(fs.readFileSync(candidatePath, 'utf8'));
  const reconciliation = JSON.parse(fs.readFileSync(reconciliationPath, 'utf8'));
  for (const candidate of candidates.candidates) for (const binding of candidate.bindings) {
    assert.equal(typeof binding, 'object', 'endpoint consumers require object bindings');
    assert.equal(typeof binding.binding_id, 'string');
    assert.equal(typeof binding.role, 'string');
    assert.equal(typeof binding.lifecycle_boundary, 'string');
  }
  const mapped = reconciliation.mappings[0];
  const mappedCandidateKey = mapped.candidate_key;
  const mappedBindingId = mapped.binding_id;
  const oldCandidate = candidates.candidates.find((item) => item.candidate_key === mapped.candidate_key);
  assert.ok(oldCandidate);
  const oldKey = oldCandidate.candidate_key;
  const oldBinding = oldCandidate.bindings.find((binding) => binding.binding_id === mapped.binding_id);
  const symbol = `${oldCandidate.symbol}_wrong_symbol`;
  const candidateKey = digest({ path: oldCandidate.path, symbol, line_start: oldCandidate.line_start, line_end: oldCandidate.line_end, blob_oid: oldCandidate.blob_oid });
  assert.ok(oldBinding && typeof oldBinding === 'object');
  const context = candidates.extensions.binding_contexts.find((item) => item.binding_id === oldBinding.binding_id);
  assert.ok(context);
  const bindingId = digest({ candidate_key: candidateKey, capability_id: oldBinding.capability_id });
  Object.assign(oldCandidate, { candidate_key: candidateKey, symbol, graph_receipt_ids: ['graph-node-struct'], bindings: [{ ...oldBinding, binding_id: bindingId }] });
  Object.assign(context, {
    binding_id: bindingId,
    candidate_key: candidateKey,
    endpoint_declaration: { qualified_name: symbol, line_start: oldCandidate.line_start, line_end: oldCandidate.line_end, kind: 'graph_node' },
  });
  for (const mapping of reconciliation.mappings) if (mapping.candidate_key === oldKey) Object.assign(mapping, { candidate_key: candidateKey, binding_id: bindingId });
  const wrongCandidate = `${temporary}/same-file-wrong-symbol.candidate.json`;
  const wrongReconciliation = `${temporary}/same-file-wrong-symbol.reconciliation.json`;
  fs.writeFileSync(wrongCandidate, `${JSON.stringify(candidates, null, 2)}\n`);
  fs.writeFileSync(wrongReconciliation, `${JSON.stringify(reseal(reconciliation), null, 2)}\n`);
  const wrong = invoke(endpointCli, endpointArgs(wrongCandidate, wrongReconciliation));
  assert.equal(wrong.status, 1, wrong.stdout);
  assert.match(wrong.stderr, /^(?:endpoint containment failed|candidate is not a genuine declaration):/m);

  const unknownReconciliation = JSON.parse(fs.readFileSync(reconciliationPath, 'utf8'));
  unknownReconciliation.mappings[0].binding_id = 'unknown-binding';
  const unknownPath = `${temporary}/unknown-binding.reconciliation.json`;
  fs.writeFileSync(unknownPath, `${JSON.stringify(reseal(unknownReconciliation), null, 2)}\n`);
  const unknown = invoke(endpointCli, endpointArgs(candidatePath, unknownPath));
  assert.equal(unknown.status, 1, unknown.stdout);
  assert.match(unknown.stderr, /^mapping binding mismatch:/m);

  const staleCandidates = JSON.parse(fs.readFileSync(candidatePath, 'utf8'));
  const staleCandidate = staleCandidates.candidates.find((candidate) => candidate.candidate_key === mappedCandidateKey);
  const staleBinding = staleCandidate.bindings.find((binding) => binding.binding_id === mappedBindingId);
  const staleContext = staleCandidates.extensions.binding_contexts.find((item) => item.binding_id === staleBinding.binding_id);
  const staleId = `${staleBinding.binding_id}-stale`;
  staleBinding.binding_id = staleId;
  staleContext.binding_id = staleId;
  const staleCandidatePath = `${temporary}/stale-binding.candidate.json`;
  fs.writeFileSync(staleCandidatePath, `${JSON.stringify(staleCandidates, null, 2)}\n`);
  const stale = invoke(endpointCli, endpointArgs(staleCandidatePath, reconciliationPath));
  assert.equal(stale.status, 1, stale.stdout);
  assert.match(stale.stderr, /^mapping binding mismatch:/m);

  const malformedCandidates = JSON.parse(fs.readFileSync(candidatePath, 'utf8'));
  const malformedCandidate = malformedCandidates.candidates.find((candidate) => candidate.bindings.length > 0);
  malformedCandidate.bindings[0] = malformedCandidate.bindings[0].binding_id;
  const malformedCandidatePath = `${temporary}/malformed-binding.candidate.json`;
  fs.writeFileSync(malformedCandidatePath, `${JSON.stringify(malformedCandidates, null, 2)}\n`);
  const malformed = invoke(endpointCli, endpointArgs(malformedCandidatePath, reconciliationPath));
  assert.equal(malformed.status, 1, malformed.stdout);
  assert.match(malformed.stderr, /candidate(?:-manifest)?\.candidates.*bindings|candidate binding/);

  const toolingCommit = spawnSync('git', ['rev-parse', 'HEAD'], { cwd: ROOT, encoding: 'utf8' }).stdout.trim();
  const toolingManifest = `${temporary}/tooling-blobs.json`;
  const build = invoke(toolingCli, ['build', '--run-id', runId, '--source-baseline', sourceBaseline, '--tooling-commit', toolingCommit, '--out', toolingManifest]);
  if (build.status === 0) {
    const verify = invoke(toolingCli, ['verify', '--input', toolingManifest, '--run-id', runId, '--source-baseline', sourceBaseline, '--tooling-commit', toolingCommit]);
    assert.equal(verify.status, 0, verify.stderr);
    const tampered = JSON.parse(fs.readFileSync(toolingManifest, 'utf8'));
    tampered.entries[0].blob_oid = '1'.repeat(40);
    const tamperedPath = `${temporary}/tooling-blobs.tampered.json`;
    fs.writeFileSync(tamperedPath, `${JSON.stringify(reseal(tampered), null, 2)}\n`);
    const rejected = invoke(toolingCli, ['verify', '--input', tamperedPath]);
    assert.equal(rejected.status, 1);
    assert.match(rejected.stderr, /entries do not match immutable commit tree/);
  } else {
    assert.match(build.stderr, /tooling (?:worktree (?:differs|scope has)|tree is empty)/);
  }

  const recursive = invoke(toolingCli, ['build', '--run-id', runId, '--source-baseline', sourceBaseline, '--tooling-commit', toolingCommit, '--out', 'tools/audit/.recursive-output.json']);
  assert.equal(recursive.status, 1);
  assert.match(recursive.stderr, /output must be outside tools\/audit/);
  process.stdout.write('tooling_and_endpoint_trust content_driven pass\n');
} finally {
  fs.rmSync(temporary, { recursive: true, force: true });
}
