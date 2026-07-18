#!/usr/bin/env node
import assert from 'node:assert/strict';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const result = spawnSync(process.execPath, [path.join(ROOT, 'tools/audit/produce-task4-evidence.mjs'), '--lsp-child-environment-smoke'], { cwd: ROOT, encoding: 'utf8', env: { ...process.env, TASK4_SENTINEL_SECRET: 'must-not-reach-lsp-child' } });

assert.equal(result.status, 0, result.stderr || result.stdout);
assert.equal(result.stdout, 'lsp_child_environment=pass\n');
