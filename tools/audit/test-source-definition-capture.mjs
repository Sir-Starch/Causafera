#!/usr/bin/env node
import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

import { ROOT, canonical } from './lib/validate-capability-audit-core.mjs';

const baseline = '26026fb3862e8d178a2e59df7a68a2901e80b123';
const baselineTree = '8507defcd090b107eaf695b1289bd42d1ebd2f32';
const alternateBaseline = 'ce970ec7a84d7cb61b7cdc970ab007dec72ba569';
const capture = path.join(ROOT, 'tools/audit/capture-command.mjs');
const checker = path.join(ROOT, 'tools/audit/validate-capability-audit.mjs');
const runId = `source-definition-${process.pid}-${crypto.randomBytes(5).toString('hex')}`;
const scratchRelative = `tools/audit/.source-definition-${process.pid}-${crypto.randomBytes(5).toString('hex')}`;
const scratch = path.join(ROOT, scratchRelative);
const fixtureRoot = `tools/audit/fixtures/tmp/${runId}`;

function sha(value) {
  return crypto.createHash('sha256').update(value).digest('hex');
}

function write(relative, value) {
  const absolute = path.join(scratch, relative);
  fs.mkdirSync(path.dirname(absolute), { recursive: true });
  fs.writeFileSync(absolute, `${JSON.stringify(value, null, 2)}\n`, { flag: 'wx', mode: 0o600 });
  return `${scratchRelative}/${relative}`;
}

function run(entry, args) {
  return spawnSync(process.execPath, [entry, ...args], { cwd: ROOT, encoding: 'utf8' });
}

const bootstrap = write('bootstrap.json', {
  run_id: runId,
  source_baseline_sha: baseline,
  source_baseline_tree_oid: baselineTree,
  audit_worktree_path: ROOT,
});

function captureArgs(suffix, sourceBaseline, argv) {
  const prefix = `${fixtureRoot}/${suffix}`;
  return [
    '--run-id', runId,
    '--source-baseline', sourceBaseline,
    '--bootstrap', bootstrap,
    '--receipt-id', `source-definition-${suffix}`,
    '--adapter', 'source_definition',
    '--stdout', `${prefix}.stdout`,
    '--stderr', `${prefix}.stderr`,
    '--receipt', `${prefix}.json`,
    '--', ...argv,
  ];
}

function captureRun(suffix, argv) {
  const result = run(capture, captureArgs(suffix, baseline, argv));
  assert.equal(result.status, 0, `valid ${suffix} capture failed:\n${result.stdout}\n${result.stderr}`);
  return JSON.parse(fs.readFileSync(path.join(ROOT, `${fixtureRoot}/${suffix}.json`), 'utf8'));
}

function rejectedCapture(name, sourceBaseline, argv) {
  const result = run(capture, captureArgs(name, sourceBaseline, argv));
  assert.notEqual(result.status, 0, `${name} unexpectedly passed capture`);
  assert.match(result.stderr, /source.definition|adapter argv|trusted.*baseline|bootstrap.*baseline/i, `${name} rejected for the wrong reason: ${result.stderr}`);
}

function reseal(receipt) {
  const unsigned = Object.fromEntries(Object.entries(receipt).filter(([key]) => key !== 'receipt_sha256'));
  return { ...unsigned, receipt_sha256: sha(canonical(unsigned)) };
}

function rejectedCore(name, validReceipt, sourceBaseline, argv) {
  const receipt = structuredClone(validReceipt);
  receipt.source_baseline_sha = sourceBaseline;
  receipt.argv = argv;
  const receiptPath = write(`core-${name}.json`, reseal(receipt));
  const result = run(checker, [
    'capture', '--adapter', 'source_definition', '--run-id', runId,
    '--receipt-id', receipt.receipt_id, '--receipt', receiptPath, '--bootstrap', bootstrap,
  ]);
  assert.notEqual(result.status, 0, `${name} unexpectedly passed core receipt validation`);
  assert.match(result.stderr, /source.definition|adapter argv|trusted.*baseline|bootstrap.*baseline/i, `${name} core diagnostic changed: ${result.stderr}`);
}

try {
  const metricsPath = 'crates/ontopolis-analytics/src/metrics.rs';
  const plansPath = 'PLANS.md';
  const sourceBlob = spawnSync('git', ['cat-file', '-t', `${baseline}:${metricsPath}`], { cwd: ROOT, encoding: 'utf8' });
  assert.equal(sourceBlob.status, 0, sourceBlob.stderr);
  assert.equal(sourceBlob.stdout.trim(), 'blob', 'metrics source must be a baseline blob');

  const metrics = captureRun('baseline-metrics', ['git', 'show', `${baseline}:${metricsPath}`]);
  const plans = captureRun('baseline-plans', ['git', 'show', `${baseline}:${plansPath}`]);
  assert.equal(metrics.exit_code, 0);
  assert.equal(plans.exit_code, 0);

  const invalid = [
    ['absolute-path', baseline, ['git', 'show', `${baseline}:/etc/passwd`]],
    ['traversal-path', baseline, ['git', 'show', `${baseline}:../PLANS.md`]],
    ['dot-path', baseline, ['git', 'show', `${baseline}:./PLANS.md`]],
    ['double-slash-path', baseline, ['git', 'show', `${baseline}:crates//ontopolis-analytics/src/metrics.rs`]],
    ['empty-path', baseline, ['git', 'show', `${baseline}:`]],
    ['extra-show-argument', baseline, ['git', 'show', `${baseline}:${plansPath}`, '--stat']],
    ['option-injection', baseline, ['git', 'show', '--stat', `${baseline}:${plansPath}`]],
    ['git-status', baseline, ['git', 'status']],
    ['git-log', baseline, ['git', 'log', '--oneline']],
    ['wrong-baseline', alternateBaseline, ['git', 'show', `${alternateBaseline}:${metricsPath}`]],
  ];
  for (const [name, sourceBaseline, argv] of invalid) {
    rejectedCapture(name, sourceBaseline, argv);
    rejectedCore(name, metrics, sourceBaseline, argv);
  }

  process.stdout.write('source_definition_capture=2 valid 10 rejected capture+core pass\n');
} finally {
  fs.rmSync(scratch, { recursive: true, force: true });
  fs.rmSync(path.join(ROOT, fixtureRoot), { recursive: true, force: true });
}
