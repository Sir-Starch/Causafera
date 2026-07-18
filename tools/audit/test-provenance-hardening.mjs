#!/usr/bin/env node
import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

import { LEGACY_BASELINE_ANALYTICS_METRICS_PATH, ROOT, canonical } from './lib/validate-capability-audit-core.mjs';
import { assertStableExecutableDigest, containsSensitiveCarrier, executableDigest } from './lib/capture-security.mjs';

const cli = path.join(ROOT, 'tools/audit/validate-capability-audit.mjs');
const capture = path.join(ROOT, 'tools/audit/capture-command.mjs');
const baseline = '26026fb3862e8d178a2e59df7a68a2901e80b123';
const baselineTree = '8507defcd090b107eaf695b1289bd42d1ebd2f32';
const runId = `provenance-hardening-${process.pid}-${crypto.randomBytes(5).toString('hex')}`;
const scratchRelative = `tools/audit/.provenance-test-${process.pid}-${crypto.randomBytes(5).toString('hex')}`;
const scratch = path.join(ROOT, scratchRelative);
const fixtureRoot = `tools/audit/fixtures/tmp/${runId}`;
const fixtureRootAbsolute = path.join(ROOT, fixtureRoot);
const failures = [];

function sha(bytes) {
  return crypto.createHash('sha256').update(bytes).digest('hex');
}

function write(relative, value, mode = 0o600) {
  const absolute = path.join(scratch, relative);
  fs.mkdirSync(path.dirname(absolute), { recursive: true });
  fs.writeFileSync(absolute, typeof value === 'string' ? value : `${JSON.stringify(value, null, 2)}\n`, { flag: 'wx', mode });
  return `${scratchRelative}/${relative}`;
}

function writeBootstrap() {
  return write('bootstrap.json', {
    run_id: runId,
    source_baseline_sha: baseline,
    source_baseline_tree_oid: baselineTree,
    audit_worktree_path: ROOT,
  });
}

const bootstrap = writeBootstrap();

function run(entry, args, env = process.env) {
  return spawnSync(process.execPath, [entry, ...args], { cwd: ROOT, encoding: 'utf8', env });
}

function expectRejected(name, result, pattern, forbiddenPattern = null) {
  if (result.status === 0 || !pattern.test(result.stderr) || (forbiddenPattern && forbiddenPattern.test(result.stderr))) {
    failures.push(`${name}: status=${result.status}; stderr=${JSON.stringify(result.stderr)}`);
  }
}

function captureArgs(suffix, adapter, sourceBaseline, argv) {
  const prefix = `${fixtureRoot}/hardening-${suffix}`;
  return [
    '--run-id', runId, '--source-baseline', sourceBaseline, '--bootstrap', bootstrap,
    '--receipt-id', `hardening-${suffix}`, '--adapter', adapter,
    '--stdout', `${prefix}.stdout`, '--stderr', `${prefix}.stderr`, '--receipt', `${prefix}.json`,
    '--', ...argv,
  ];
}

try {
  assert.equal(fs.existsSync(fixtureRootAbsolute), false, 'clean-baseline regression requires an absent provenance fixture root');
  assert.equal(containsSensitiveCarrier('Bearer SYNTHETIC_SENTINEL'), true, 'bare bearer carrier was not detected');
  const wrongBaseline = run(capture, captureArgs('wrong-baseline', 'source_definition', 'ce970ec7a84d7cb61b7cdc970ab007dec72ba569', [
    'git', 'show', 'ce970ec7a84d7cb61b7cdc970ab007dec72ba569:tools/audit/capture-command.mjs',
  ]));
  expectRejected('wrong trusted baseline', wrongBaseline, /trusted.*baseline|bootstrap.*baseline/i);

  const wrongAdapter = run(capture, captureArgs('wrong-adapter', 'exact_test', baseline, [
    'node', 'tools/audit/validate-capability-audit.mjs', 'schema', '--fixture-mode', 'true',
  ]));
  expectRejected('adapter argv mismatch', wrongAdapter, /adapter.*argv|argv.*policy/i);

  const unsafeDirectory = fs.mkdtempSync(path.join(os.tmpdir(), 'causafera-untrusted-exec-'));
  fs.chmodSync(unsafeDirectory, 0o755);
  const unsafeExecutable = path.join(unsafeDirectory, 'git');
  fs.writeFileSync(unsafeExecutable, '#!/bin/sh\nexec /usr/bin/git "$@"\n', { mode: 0o755 });
  const unsafeEnv = { ...process.env, PATH: `${unsafeDirectory}:${process.env.PATH}` };
  const invalidSourceDefinition = run(capture, captureArgs('invalid-source-definition-argv', 'source_definition', baseline, ['untrusted-audit-command']), unsafeEnv);
  expectRejected('source-definition argv is rejected before executable trust', invalidSourceDefinition, /invalid source-definition argv/i, /writable|executable path/i);

  const unsafe = run(capture, captureArgs('unsafe-executable', 'source_definition', baseline, [
    'git', 'show', `${baseline}:${LEGACY_BASELINE_ANALYTICS_METRICS_PATH}`,
  ]), unsafeEnv);
  expectRejected('policy-valid argv reaches untrusted executable boundary', unsafe, /capture executable path is group\/world writable/i, /invalid source-definition argv/i);
  fs.rmSync(unsafeDirectory, { recursive: true, force: true });

  const headerSecret = run(capture, captureArgs('secret-header', 'source_definition', baseline, [
    'node', '-e', 'process.stdout.write("safe\\n")', 'Authorization: Basic SYNTHETIC_SENTINEL',
  ]));
  expectRejected('authorization header secret', headerSecret, /secret|credential/i);

  const outputSecret = run(capture, captureArgs('secret-output', 'source_definition', baseline, [
    'node', '-e', 'process.stdout.write("Bearer SYNTHETIC_SENTINEL")',
  ]));
  expectRejected('captured output secret', outputSecret, /secret|credential|adapter.*argv/i);
  if (fs.existsSync(path.join(ROOT, `${fixtureRoot}/hardening-secret-output.stdout`))) failures.push('captured output secret: secret stdout sidecar persisted');

  const mutableExecutable = write('mutable-executable.sh', '#!/bin/sh\nprintf "# changed\\n" >> "$0"\nprintf stable\\n\n', 0o755);
  const mutableDigest = executableDigest(path.join(ROOT, mutableExecutable));
  fs.appendFileSync(path.join(ROOT, mutableExecutable), '# independently changed\n');
  assert.throws(() => assertStableExecutableDigest(path.join(ROOT, mutableExecutable), mutableDigest), /executable changed.*digest/i);
  const mutable = run(capture, captureArgs('mutable-executable', 'source_definition', baseline, [mutableExecutable]));
  expectRejected('mutable executable digest', mutable, /executable.*(changed|digest|stable)|mutation|adapter.*argv/i);

  const materialize = run(cli, [
    'materialize-audit', '--bootstrap', bootstrap,
    '--input', 'tools/audit/examples/capability-audit-input.valid.json',
    '--evidence-manifest', 'tools/audit/examples/evidence-execution-manifest.valid.json',
    '--output', 'tools/audit/examples/capability-audit.valid.json',
  ]);
  expectRejected('materialized wrong trusted identity', materialize, /trusted.*(run|baseline)|bootstrap.*(run|baseline)/i);

  const trackedPath = 'tools/audit/examples/capability-audit.valid.json';
  const trackedOid = spawnSync('git', ['rev-parse', `${baseline}:${trackedPath}`], { cwd: ROOT, encoding: 'utf8' }).stdout.trim();
  const bundle = {
    schema_version: 1, run_id: runId, source_baseline_sha: baseline,
    entries: [{
      logical_path: trackedPath, tracked_path: trackedPath, tracked_blob_oid: trackedOid,
      byte_sha256: '0'.repeat(64), wrapper_receipt_sha256: '0'.repeat(64),
      original_receipt_sha256: '0'.repeat(64), raw_byte_hashes: ['0'.repeat(64)], kind: 'json',
    }],
  };
  bundle.receipt_sha256 = sha(canonical(bundle));
  const bundleResult = run(cli, ['bundle', '--bootstrap', bootstrap, '--bundle', write('self-reported-bundle.json', bundle)]);
  expectRejected('self-reported bundle hashes', bundleResult, /bundle.*(byte|hash|oid|receipt)|tracked.*mismatch/i);

  const aggregate = run(cli, [
    'aggregate', '--fixture-mode', 'true',
    '--candidates', 'tools/audit/examples/candidate-manifest.valid.json',
    '--evidence-manifest', 'tools/audit/examples/evidence-execution-manifest.valid.json',
    '--audit', 'tools/audit/examples/capability-audit.valid.json',
    '--test-reconciliation', 'tools/audit/examples/test-reconciliation.valid.json',
    '--bundle', 'tools/audit/examples/bundle-manifest.valid.json', '--deep-audits', '',
  ]);
  expectRejected('empty deep audit contract', aggregate, /four.*deep.audit|required deep.audit|deep.audits.*empty/i);

  const pnpmMismatch = run(capture, captureArgs('pnpm-mismatch', 'pnpm_build', baseline, ['pnpm', 'lint']));
  expectRejected('pnpm adapter argv mismatch', pnpmMismatch, /adapter.*argv|argv.*policy/i);

  assert.deepEqual(failures, [], `trust/provenance bypasses accepted:\n${failures.join('\n')}`);
  process.stdout.write('provenance_hardening=12 pass\n');
} finally {
  fs.rmSync(scratch, { recursive: true, force: true });
  fs.rmSync(fixtureRootAbsolute, { recursive: true, force: true });
  assert.equal(fs.existsSync(fixtureRootAbsolute), false, 'provenance fixture root cleanup did not remove the missing-root regression fixture');
}
