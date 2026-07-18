#!/usr/bin/env node
import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { execFileSync, spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const baseline = execFileSync('git', ['rev-parse', 'main'], { cwd: ROOT, encoding: 'utf8' }).trim();
const tree = execFileSync('git', ['rev-parse', `${baseline}^{tree}`], { cwd: ROOT, encoding: 'utf8' }).trim();
const head = execFileSync('git', ['rev-parse', 'HEAD'], { cwd: ROOT, encoding: 'utf8' }).trim();
const auditTree = execFileSync('git', ['rev-parse', 'HEAD^{tree}'], { cwd: ROOT, encoding: 'utf8' }).trim();
const mainWorktreeRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'causafera-main-worktree-'));
const main = path.join(mainWorktreeRoot, 'worktree');
const worktreeAdd = spawnSync('git', ['worktree', 'add', '--detach', main, baseline], { cwd: ROOT, encoding: 'utf8' });
assert.equal(worktreeAdd.status, 0, `failed to create hermetic main worktree: ${worktreeAdd.stderr}`);
const providerPath = '0.0.0';
const lspPath = path.join(ROOT, 'tools/audit/produce-task4-evidence.mjs');
const sha256 = (value) => crypto.createHash('sha256').update(value).digest('hex');
const canonical = (value) => Array.isArray(value) ? `[${value.map(canonical).join(',')}]` : value && typeof value === 'object' ? `{${Object.keys(value).sort().map((key) => `${JSON.stringify(key)}:${canonical(value[key])}`).join(',')}}` : JSON.stringify(value);
const runId = `task4-direct-rust-contract-${process.pid}`;
const trustDir = path.join(ROOT, '.omo', 'audit-trust');
const keyPath = path.join(trustDir, `${runId}.key`);
const key = crypto.randomBytes(32);
const bootstrap = {
  run_id: runId, source_baseline_sha: baseline, source_baseline_tree_oid: tree,
  original_worktree_path: main, audit_worktree_path: ROOT, audit_head_sha: head, audit_tree_oid: auditTree,
  lsp_provider: { version: providerPath, path: lspPath, sha256: sha256(fs.readFileSync(lspPath)) },
  tool_versions: { git: 'test', node: 'test', cargo: 'test', pnpm: 'test' },
  graph_status: { status: 'ready', index_sha: baseline },
  cleanliness: { original_status_porcelain: [], original_staged_paths: [], relevant_staged_paths: [], audit_status_porcelain_before_evidence: [], audit_staged_paths: [] },
  inventories: { worktrees: [main, ROOT] }, ancestry: { source_baseline_is_original_head: true, audit_worktree_descends_from_baseline: true },
  target_preimages: [{ path: 'PLANS.md', blob_oid: execFileSync('git', ['rev-parse', `${baseline}:PLANS.md`], { cwd: ROOT, encoding: 'utf8' }).trim() }],
};
fs.mkdirSync(trustDir, { recursive: true });
fs.writeFileSync(keyPath, key, { mode: 0o600 });
bootstrap.runner_attestation = { version: 1, key_path: keyPath, key_sha256: sha256(key), bootstrap_hmac_sha256: crypto.createHmac('sha256', key).update(canonical(bootstrap)).digest('hex') };
const bootstrapPath = path.join(ROOT, '.omo', `${runId}.bootstrap.json`);
fs.writeFileSync(bootstrapPath, `${JSON.stringify(bootstrap)}\n`);
try {
  const result = spawnSync(process.execPath, [path.join(ROOT, 'tools/audit/validate-capability-audit.mjs'), 'preflight', '--run-id', runId, '--bootstrap', path.relative(ROOT, bootstrapPath)], { cwd: ROOT, encoding: 'utf8' });
  assert.notEqual(result.status, 0, 'missing rust_lsp_provider contract unexpectedly passed preflight');
  assert.match(`${result.stderr}${result.stdout}`, /rust LSP provider|rust_lsp_provider/, 'preflight rejected the bootstrap for the wrong reason');
  process.stdout.write('task4_direct_rust_provider_contract=pass\n');
} finally {
  fs.rmSync(bootstrapPath, { force: true });
  fs.rmSync(keyPath, { force: true });
  spawnSync('git', ['worktree', 'remove', '--force', main], { cwd: ROOT, encoding: 'utf8' });
  fs.rmSync(mainWorktreeRoot, { recursive: true, force: true });
}
