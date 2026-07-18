#!/usr/bin/env node
import assert from 'node:assert/strict';
import fs from 'node:fs';
import { spawnSync } from 'node:child_process';

const cli = 'tools/audit/validate-capability-audit.mjs';
const evidence = process.argv[2];
if (!evidence) throw new Error('usage: node tools/audit/test-artifact-modes.mjs <evidence-root>');
const runId = evidence.split('/').filter(Boolean).at(-1);
const output = 'tools/audit/.inventory-test-output.json';
const capture = (receiptId) => `${evidence}/captures/${receiptId}.command-receipt.json`;

function invoke(mode, args, expectedStatus = 0) {
  const result = spawnSync(process.execPath, [cli, mode, ...args], { encoding: 'utf8' });
  assert.equal(result.status, expectedStatus, `${mode}: ${result.stderr}`);
  return result;
}

try {
  invoke('inventory', [
    '--run-id', runId,
    '--bootstrap', `${evidence}/task-1-bootstrap.json`,
    '--preflight-receipt', capture('task-01-preflight'),
    '--input', `${evidence}/task-4-capability-inventory.md`,
    '--candidate-manifest', `${evidence}/task-4-candidate-manifest.json`,
    '--blobs', `${evidence}/task-4-source-blobs.json`,
    '--test-list-receipt', capture('task-04-rust-test-list'),
    '--test-results-receipt', capture('task-04-rust-test-results'),
    '--test-reconciliation', `${evidence}/test-reconciliation.json`,
    '--evidence-execution-manifest', `${evidence}/evidence-execution-manifest.json`,
    '--out', output,
  ]);
  assert.equal(JSON.parse(fs.readFileSync(output, 'utf8')).verdict, 'pass');

  const artifactCases = [
    ['blobs', ['--input', `${evidence}/task-4-source-blobs.json`]],
    ['source-blobs', ['--input', `${evidence}/task-4-source-blobs.json`]],
    ['evidence-execution-manifest', ['--input', `${evidence}/evidence-execution-manifest.json`]],
    ['test-reconciliation', ['--input', `${evidence}/test-reconciliation.json`]],
    ['candidate-manifest', ['--input', `${evidence}/task-4-candidate-manifest.json`]],
    ['capability-inventory', ['--input', `${evidence}/task-4-capability-inventory.md`]],
    ['inventory-validation', ['--input', `${evidence}/task-4-inventory-validation.json`]],
    ['inventory-negative', ['--input', `${evidence}/task-4-inventory-negative.json`]],
    ['rust-test-list', ['--input', capture('task-04-rust-test-list')]],
    ['rust-test-results', ['--input', capture('task-04-rust-test-results')]],
    ['closure-manifest', ['--input', 'tools/audit/examples/closure-manifest.valid.json']],
    ['review-receipt', ['--input', 'tools/audit/examples/review-receipt.valid.json']],
  ];
  for (const family of [
    'physical-causality', 'production-bootstrap', 'cognitive-architecture',
    'spatial-architecture', 'observer-protocol', 'explanation-engine',
    'integration', 'performance', 'security',
  ]) artifactCases.push([`deep-audit-${family}`, ['--input', 'tools/audit/examples/deep-audit-fragment.valid.json']]);

  for (const [mode, args] of artifactCases) {
    invoke(mode, args);
    invoke(mode, [], 1);
  }
  invoke('inventory', ['--input', `${evidence}/task-4-capability-inventory.md`], 1);
  process.stdout.write(`artifact_modes=${artifactCases.length + 1} positive_and_negative pass\n`);
} finally {
  fs.rmSync(output, { force: true });
}
