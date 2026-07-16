#!/usr/bin/env node
import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

import { canonical } from './lib/validate-capability-audit-core.mjs';

const root = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..', '..');
const cli = path.join(root, 'tools/audit/validate-capability-audit.mjs');
const capture = path.join(root, 'tools/audit/capture-command.mjs');
const scratchRelative = `tools/audit/.trust-test-${process.pid}-${crypto.randomBytes(6).toString('hex')}`;
const scratch = path.join(root, scratchRelative);
const baseline = '26026fb3862e8d178a2e59df7a68a2901e80b123';
const baselineTree = '8507defcd090b107eaf695b1289bd42d1ebd2f32';

function sha(value) {
  return crypto.createHash('sha256').update(value).digest('hex');
}

function write(relative, value) {
  const absolute = path.join(scratch, relative);
  fs.mkdirSync(path.dirname(absolute), { recursive: true });
  fs.writeFileSync(absolute, typeof value === 'string' ? value : `${JSON.stringify(value, null, 2)}\n`, { flag: 'wx' });
  return `${scratchRelative}/${relative}`;
}

function run(entry, args, environment = {}) {
  return spawnSync(process.execPath, [entry, ...args], {
    cwd: root,
    encoding: 'utf8',
    env: { ...process.env, ...environment },
  });
}

function invoke(args) {
  return run(cli, args);
}

function reject(args, pattern) {
  const result = invoke(args);
  assert.notEqual(result.status, 0, `unexpected acceptance: ${args.join(' ')}`);
  assert.match(result.stderr, pattern);
}

function resign(value) {
  const unsigned = Object.fromEntries(Object.entries(value).filter(([key]) => key !== 'receipt_sha256'));
  value.receipt_sha256 = sha(canonical(unsigned));
  return value;
}

function example(name) {
  return JSON.parse(fs.readFileSync(path.join(root, `tools/audit/examples/${name}`), 'utf8'));
}

function bootstrapFor(runId) {
  return write(`bootstrap-${runId}.json`, {
    run_id: runId,
    source_baseline_sha: baseline,
    source_baseline_tree_oid: baselineTree,
    audit_worktree_path: root,
  });
}

function captureRun(runId, suffix) {
  const prefix = `tools/audit/fixtures/tmp/${runId}/${suffix}`;
  const bootstrap = bootstrapFor(`${runId}-${suffix}`);
  const bootstrapObject = JSON.parse(fs.readFileSync(path.join(root, bootstrap), 'utf8'));
  bootstrapObject.run_id = runId;
  fs.writeFileSync(path.join(root, bootstrap), `${JSON.stringify(bootstrapObject, null, 2)}\n`);
  const result = run(capture, [
    '--run-id', runId,
    '--source-baseline', baseline,
    '--bootstrap', bootstrap,
    '--receipt-id', `receipt-${suffix}`,
    '--adapter', 'source_definition',
    '--stdout', `${prefix}.stdout`,
    '--stderr', `${prefix}.stderr`,
    '--receipt', `${prefix}.json`,
    '--', 'git', 'show', `${baseline}:PLANS.md`,
  ]);
  assert.equal(result.status, 0, result.stderr);
  return { prefix, receipt: JSON.parse(fs.readFileSync(path.join(root, `${prefix}.json`), 'utf8')) };
}

try {
  const missing = resign(example('command-receipt.valid.json'));
  missing.run_id = 'trust-missing-sidecar';
  missing.source_baseline_sha = baseline;
  missing.argv = ['git', 'show', `${baseline}:PLANS.md`];
  const gitHash = sha(fs.readFileSync(fs.realpathSync('/usr/bin/git')));
  missing.deterministic_projection.mode = `operation=show;outcome=pass;executable=git;executable_sha256=${gitHash};evidence_class=source_definition;state_anchor=baseline`;
  missing.deterministic_projection.summary = `exit=${missing.exit_code};signal=none`;
  missing.projection_sha256 = sha(canonical(missing.deterministic_projection));
  missing.stdout_path = `${scratchRelative}/missing.stdout`;
  missing.stderr_path = `${scratchRelative}/missing.stderr`;
  const missingPath = write('missing-receipt.json', resign(missing));
  reject(['capture', '--adapter', missing.adapter, '--run-id', missing.run_id, '--receipt-id', missing.receipt_id, '--receipt', missingPath, '--bootstrap', bootstrapFor(missing.run_id)], /missing stdout|sidecar/i);

  const malformed = example('capability-audit-input.valid.json');
  malformed.capabilities[0].carriers = 42;
  const malformedPath = write('arbitrary-name.json', malformed);
  reject(['capability-inventory', '--input', malformedPath], /carriers.*array/i);

  const inconsistent = example('capability-audit.valid.json');
  inconsistent.capabilities[0].derived_maturity = 'M5';
  const inconsistentPath = write('not-prefixed-invalid.json', inconsistent);
  reject(['audit', '--audit', inconsistentPath], /derived|M5|evidence/i);

  const nonexistentEvidence = example('capability-audit-input.valid.json');
  nonexistentEvidence.capabilities[0].levels.M1.evidence_ids = ['does-not-exist'];
  reject(['capability-inventory', '--input', write('nonexistent-evidence.json', nonexistentEvidence)], /unknown evidence/i);

  const invalidM2 = example('capability-audit-input.valid.json');
  invalidM2.capabilities[0].levels.M2 = { status: 'satisfied', evidence_ids: ['e-001'], rationale: 'fabricated' };
  invalidM2.capabilities[0].target_maturity = 'M2';
  reject(['capability-inventory', '--input', write('inconsistent-m2.json', invalidM2)], /cannot satisfy M2|production evidence/i);

  const orderedNegative = invoke([
    'inventory', '--fixture-mode', 'true', '--run-id', 'fixture-run',
    '--inventory', 'tools/audit/fixtures/invalid-manifest-and-domain.md',
    '--candidate-manifest', 'tools/audit/fixtures/fixture-candidate-manifest.valid.json',
    '--blobs', 'tools/audit/fixtures/fixture-blobs.json',
    '--test-list-receipt', 'tools/audit/fixtures/fixture-test-list.command-receipt.valid.json',
    '--test-results-receipt', 'tools/audit/fixtures/fixture-test-results.command-receipt.valid.json',
    '--test-reconciliation', 'tools/audit/fixtures/fixture-test-reconciliation.valid.json',
    '--evidence-execution-manifest', 'tools/audit/fixtures/fixture-evidence-execution.valid.json',
    '--out', `${scratchRelative}/must-not-exist.json`,
  ]);
  assert.equal(orderedNegative.status, 1);
  assert.equal(orderedNegative.stdout, '');
  assert.equal(orderedNegative.stderr, 'missing domain: Hydrology\nunmapped source\n');

  const help = invoke(['--help']);
  assert.equal(help.status, 0, help.stderr);
  assert.match(help.stdout, /Usage:/);

  const unknownAdapter = run(capture, [
    '--run-id', 'trust-test',
    '--source-baseline', '26026fb3862e8d178a2e59df7a68a2901e80b123',
    '--receipt-id', 'unknown-adapter',
    '--adapter', 'not_registered',
    '--stdout', `${scratchRelative}/unknown.stdout`,
    '--stderr', `${scratchRelative}/unknown.stderr`,
    '--receipt', `${scratchRelative}/unknown.json`,
    '--', process.execPath, '-e', 'process.stdout.write("ok\\n")',
  ]);
  assert.notEqual(unknownAdapter.status, 0, 'unregistered capture adapter was accepted');

  const runId = `trust-${process.pid}-${crypto.randomBytes(4).toString('hex')}`;
  const first = captureRun(runId, 'attempt-0001');
  const second = captureRun(runId, 'attempt-0002');
  assert.deepEqual(first.receipt.deterministic_projection, second.receipt.deterministic_projection, 'capture projection is not deterministic');
  const homeFirst = captureRun(runId, 'home-0001');
  const homeSecond = captureRun(runId, 'home-0002');
  assert.deepEqual(homeFirst.receipt.deterministic_projection, homeSecond.receipt.deterministic_projection, 'nondeterministic raw output leaked into projection');
  const runtimeBootstrap = bootstrapFor(`${runId}-validation`);
  const runtimeBootstrapObject = JSON.parse(fs.readFileSync(path.join(root, runtimeBootstrap), 'utf8'));
  runtimeBootstrapObject.run_id = runId;
  fs.writeFileSync(path.join(root, runtimeBootstrap), `${JSON.stringify(runtimeBootstrapObject, null, 2)}\n`);
  assert.equal(invoke(['capture', '--adapter', first.receipt.adapter, '--run-id', runId, '--receipt-id', first.receipt.receipt_id, '--receipt', `${first.prefix}.json`, '--bootstrap', runtimeBootstrap]).status, 0, 'attempt-0001 no longer validates after retry');

  const execution = example('evidence-execution-manifest.valid.json');
  execution.run_id = runId;
  execution.source_baseline_sha = first.receipt.source_baseline_sha;
  execution.claims[0] = {
    capability_id: 'cap-space-001',
    binding_id: 'bind-space-001',
    adapter: first.receipt.adapter,
    argv: first.receipt.argv,
    baseline_target: 'tools/audit/examples/capability-audit.valid.json',
    facets: ['primitive_boundary'],
    receipt_id: first.receipt.receipt_id,
    receipt_path: `${first.prefix}.json`,
    receipt_sha256: first.receipt.receipt_sha256,
  };
  resign(execution);
  const executionPath = write('execution.json', execution);
  assert.equal(invoke(['evidence-execution-manifest', '--input', executionPath]).status, 0, 'valid execution receipt binding was rejected');
  const unrelated = structuredClone(execution);
  unrelated.claims[0].receipt_id = 'fabricated-unrelated-receipt';
  resign(unrelated);
  reject(['evidence-execution-manifest', '--input', write('unrelated-execution.json', unrelated)], /receipt identity mismatch/i);
  const staleClaimHash = structuredClone(execution);
  staleClaimHash.claims[0].receipt_sha256 = '0'.repeat(64);
  resign(staleClaimHash);
  reject(['evidence-execution-manifest', '--input', write('stale-claim-hash.json', staleClaimHash)], /receipt_sha256 mismatch/i);

  const swapped = structuredClone(second.receipt);
  [swapped.stdout_path, swapped.stderr_path] = [swapped.stderr_path, swapped.stdout_path];
  [swapped.stdout_sha256, swapped.stderr_sha256] = [swapped.stderr_sha256, swapped.stdout_sha256];
  swapped.deterministic_projection.summary = `exit=${swapped.exit_code};signal=none`;
  swapped.projection_sha256 = sha(canonical(swapped.deterministic_projection));
  resign(swapped);
  const swappedPath = write('swapped-receipt.json', swapped);
  const intent = resign({
    schema_version: 1,
    run_id: second.receipt.run_id,
    source_baseline_sha: second.receipt.source_baseline_sha,
    attempt_id: second.receipt.attempt_id,
    receipt_id: second.receipt.receipt_id,
    phase: second.receipt.phase,
    adapter: second.receipt.adapter,
    argv: second.receipt.argv,
    cwd: second.receipt.cwd,
    environment: second.receipt.environment,
    tool_versions: second.receipt.tool_versions,
    stdout_path: second.receipt.stdout_path,
    stderr_path: second.receipt.stderr_path,
    receipt_path: swappedPath,
    workload: second.receipt.workload,
  });
  const intentPath = write('swapped-intent.json', intent);
  reject([
    'execute-intent', '--phase', intent.phase, '--attempt-id', String(intent.attempt_id),
    '--source-baseline', intent.source_baseline_sha, '--adapter', intent.adapter,
    '--run-id', intent.run_id, '--receipt-id', intent.receipt_id, '--intent', intentPath,
    '--stdout', intent.stdout_path, '--stderr', intent.stderr_path, '--receipt', swappedPath,
  ], /does not match prepared intent: stdout_path/i);

  const modifiedReceipt = structuredClone(first.receipt);
  fs.appendFileSync(path.join(root, modifiedReceipt.stdout_path), 'tamper');
  reject(['capture', '--adapter', modifiedReceipt.adapter, '--run-id', runId, '--receipt-id', modifiedReceipt.receipt_id, '--receipt', `${first.prefix}.json`, '--bootstrap', runtimeBootstrap], /stdout sidecar.*hash/i);

  const sameSidecar = structuredClone(second.receipt);
  sameSidecar.stderr_path = sameSidecar.stdout_path;
  sameSidecar.stderr_sha256 = sameSidecar.stdout_sha256;
  sameSidecar.receipt_id = 'same-sidecar';
  resign(sameSidecar);
  const samePath = write('same-sidecar.json', sameSidecar);
  reject(['capture', '--adapter', sameSidecar.adapter, '--run-id', runId, '--receipt-id', sameSidecar.receipt_id, '--receipt', samePath, '--bootstrap', runtimeBootstrap], /distinct/i);

  const symlinkTarget = write('symlink-target.txt', 'bytes');
  const symlinkAbsolute = path.join(scratch, 'stdout-link');
  fs.symlinkSync(path.join(root, symlinkTarget), symlinkAbsolute);
  const symlinkReceipt = structuredClone(second.receipt);
  symlinkReceipt.stdout_path = `${scratchRelative}/stdout-link`;
  symlinkReceipt.stdout_sha256 = sha('bytes');
  symlinkReceipt.receipt_id = 'symlink-sidecar';
  resign(symlinkReceipt);
  const symlinkReceiptPath = write('symlink-receipt.json', symlinkReceipt);
  reject(['capture', '--adapter', symlinkReceipt.adapter, '--run-id', runId, '--receipt-id', symlinkReceipt.receipt_id, '--receipt', symlinkReceiptPath, '--bootstrap', runtimeBootstrap], /symlink/i);

  reject(['evidence-execution-manifest', '--input', executionPath], /stdout sidecar.*hash/i);

  const outside = run(capture, [
    '--run-id', runId, '--source-baseline', first.receipt.source_baseline_sha,
    '--receipt-id', 'outside', '--adapter', 'source_definition',
    '--stdout', 'tools/audit/outside.stdout', '--stderr', 'tools/audit/outside.stderr', '--receipt', 'tools/audit/outside.json',
    '--', 'node', '-e', '',
  ]);
  assert.notEqual(outside.status, 0, 'capture accepted output outside run root');

  const secret = run(capture, [
    '--run-id', runId, '--source-baseline', first.receipt.source_baseline_sha,
    '--receipt-id', 'secret', '--adapter', 'source_definition',
    '--stdout', `tools/audit/fixtures/tmp/${runId}/secret.stdout`, '--stderr', `tools/audit/fixtures/tmp/${runId}/secret.stderr`, '--receipt', `tools/audit/fixtures/tmp/${runId}/secret.json`,
    '--', 'node', '-e', '', '--token=do-not-copy',
  ]);
  assert.notEqual(secret.status, 0, 'capture accepted secret-prone argv');

  process.stdout.write('trust_boundaries pass\n');
} finally {
  for (const entry of fs.readdirSync(path.join(root, 'tools/audit/fixtures/tmp'), { withFileTypes: true })) {
    if (entry.isDirectory() && entry.name.startsWith('trust-')) fs.rmSync(path.join(root, 'tools/audit/fixtures/tmp', entry.name), { recursive: true, force: true });
  }
  fs.rmSync(scratch, { recursive: true, force: true });
}
