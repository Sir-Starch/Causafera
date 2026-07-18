#!/usr/bin/env node
import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

import { ROOT } from './lib/validate-capability-audit-core.mjs';

const capture = path.join(ROOT, 'tools/audit/capture-command.mjs');
const baseline = '26026fb3862e8d178a2e59df7a68a2901e80b123';
const baselineTree = '8507defcd090b107eaf695b1289bd42d1ebd2f32';
const runId = `capture-cargo-${process.pid}-${crypto.randomBytes(5).toString('hex')}`;
const scratch = fs.mkdtempSync(path.join(ROOT, 'tools/audit/.capture-cargo-dispatch-'));
const baselineWorktreeRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'causafera-audit-baseline-'));
const baselineWorktree = path.join(baselineWorktreeRoot, 'worktree');
const fixtureRoot = `tools/audit/fixtures/tmp/${runId}`;

function relative(absolute) {
  return path.relative(ROOT, absolute).split(path.sep).join('/');
}

function writeBootstrap() {
  const bootstrap = path.join(scratch, 'bootstrap.json');
  const worktreeAdd = spawnSync('git', ['worktree', 'add', '--detach', baselineWorktree, baseline], { cwd: ROOT, encoding: 'utf8' });
  assert.equal(worktreeAdd.status, 0, `failed to create frozen baseline worktree: ${worktreeAdd.stderr}`);
  fs.writeFileSync(bootstrap, `${JSON.stringify({
    run_id: runId,
    source_baseline_sha: baseline,
    source_baseline_tree_oid: baselineTree,
    audit_worktree_path: ROOT,
    original_worktree_path: baselineWorktree,
  }, null, 2)}\n`, { flag: 'wx', mode: 0o600 });
  return relative(bootstrap);
}

const bootstrap = writeBootstrap();

function runCapture(suffix, argv, environment = process.env) {
  const prefix = `${fixtureRoot}/${suffix}`;
  return spawnSync(process.execPath, [capture,
    '--run-id', runId,
    '--source-baseline', baseline,
    '--bootstrap', bootstrap,
    '--receipt-id', `capture-cargo-${suffix}`,
    '--adapter', 'cargo_test_list',
    '--stdout', `${prefix}.stdout`,
    '--stderr', `${prefix}.stderr`,
    '--receipt', `${prefix}.json`,
    '--', ...argv,
  ], { cwd: ROOT, encoding: 'utf8', env: environment });
}

function reject(name, result, expression) {
  assert.notEqual(result.status, 0, `${name} unexpectedly passed`);
  assert.match(`${result.stdout}\n${result.stderr}`, expression, `${name} diagnostic changed`);
}

try {
  const cargo = runCapture('canonical-list', ['cargo', 'test', '-p', 'ontopolis-core', '--lib', '--', '--list']);
  assert.equal(cargo.status, 0, `canonical cargo capture failed:\n${cargo.stdout}\n${cargo.stderr}`);
  const receipt = JSON.parse(fs.readFileSync(path.join(ROOT, `${fixtureRoot}/canonical-list.json`), 'utf8'));
  assert.equal(receipt.exit_code, 0, 'canonical cargo capture receipt recorded a failure');
  assert.equal(receipt.cwd, '$FROZEN_BASELINE', 'cargo receipt must record frozen baseline execution');
  assert.match(fs.readFileSync(path.join(ROOT, `${fixtureRoot}/canonical-list.stdout`), 'utf8'), /tests, 0 benchmarks/);

  const unsafeDirectory = fs.mkdtempSync(path.join(os.tmpdir(), 'causafera-unsafe-cargo-path-'));
  fs.chmodSync(unsafeDirectory, 0o770);
  try {
    const unsafePath = runCapture('unsafe-path', ['cargo', 'test', '--list'], { ...process.env, PATH: unsafeDirectory });
    reject('unsafe PATH cargo', unsafePath, /trusted PATH|executable not found/i);
  } finally {
    fs.rmSync(unsafeDirectory, { recursive: true, force: true });
  }

  const mutable = path.join(scratch, 'cargo');
  try {
    fs.writeFileSync(mutable, '#!/bin/sh\nprintf changed >> "$0"\nprintf safe\\n\n', { mode: 0o755 });
    const mutableResult = runCapture('mutable-executable', [relative(mutable), 'test', '--list']);
    reject('mutable cargo executable', mutableResult, /executable changed during execution|digest is not stable/i);

    const unsafeTargetDirectory = fs.mkdtempSync(path.join(os.tmpdir(), 'causafera-unsafe-cargo-target-'));
    fs.chmodSync(unsafeTargetDirectory, 0o770);
    try {
      const unsafeTarget = path.join(unsafeTargetDirectory, 'cargo-target');
      fs.writeFileSync(unsafeTarget, '#!/bin/sh\nprintf unsafe\\n\n', { mode: 0o755 });
      fs.unlinkSync(mutable);
      fs.symlinkSync(unsafeTarget, mutable);
      const symlinkResult = runCapture('symlink-target', [relative(mutable), 'test', '--list']);
      reject('unsafe symlink target', symlinkResult, /group\/world writable|trusted PATH|executable/i);
    } finally {
      fs.rmSync(unsafeTargetDirectory, { recursive: true, force: true });
    }
  } finally {
    fs.rmSync(mutable, { force: true });
  }

  process.stdout.write('capture_cargo_dispatch=4 pass\n');
} finally {
  fs.rmSync(scratch, { recursive: true, force: true });
  fs.rmSync(path.join(ROOT, fixtureRoot), { recursive: true, force: true });
  spawnSync('git', ['worktree', 'remove', '--force', baselineWorktree], { cwd: ROOT, encoding: 'utf8' });
  fs.rmSync(baselineWorktreeRoot, { recursive: true, force: true });
}
