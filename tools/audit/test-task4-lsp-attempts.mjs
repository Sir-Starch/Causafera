#!/usr/bin/env node
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';

const evidence = process.argv[2];
if (!evidence) throw new Error('usage: node tools/audit/test-task4-lsp-attempts.mjs <evidence-root>');

const manifest = JSON.parse(fs.readFileSync(path.join(evidence, 'task-4-lsp-receipts-manifest.json'), 'utf8'));
const bootstrap = JSON.parse(fs.readFileSync(path.join(evidence, 'task-1-bootstrap.json'), 'utf8'));
const rustLifecycle = JSON.parse(fs.readFileSync(path.join(evidence, 'task-4-rust-lsp-lifecycle.json'), 'utf8'));
assert.equal(manifest.entries.length, 132, 'LSP manifest must cover every baseline candidate');
assert.deepEqual(rustLifecycle.provider, bootstrap.rust_lsp_provider, 'unsealed direct Rust LSP provider identity');
assert.deepEqual(rustLifecycle.lifecycle, { sessions: 1, shutdown: 'confirmed', retained_processes: 0 }, 'unsealed direct Rust LSP lifecycle');
for (const entry of manifest.entries) {
  const receipt = JSON.parse(fs.readFileSync(path.join(evidence, entry.receipt_path), 'utf8'));
  const raw = JSON.parse(fs.readFileSync(path.join(evidence, receipt.raw_path), 'utf8'));
  assert.ok(Array.isArray(raw.attempts) && raw.attempts.length > 0, `missing LSP attempt chain: ${entry.path}`);
  assert.deepEqual(raw.provider, entry.path.startsWith('crates/') ? bootstrap.rust_lsp_provider : bootstrap.lsp_provider, `unsealed LSP provider identity: ${entry.path}`);
  assert.deepEqual(raw.attempts.map((attempt) => attempt.ordinal), Array.from({ length: raw.attempts.length }, (_, index) => index + 1), `non-sequential LSP attempt ordinals: ${entry.path}`);
  assert.ok(raw.attempts.every((attempt) => typeof attempt.complete === 'boolean' && Object.hasOwn(attempt, 'error') && attempt.response && typeof attempt.response === 'object'), `invalid LSP attempt diagnostic: ${entry.path}`);
}

process.stdout.write('task4_lsp_attempt_chain=132 sources two_providers pass\n');
