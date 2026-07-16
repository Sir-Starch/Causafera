#!/usr/bin/env node
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
const producer = fs.readFileSync(path.join(ROOT, 'tools/audit/produce-task4-evidence.mjs'), 'utf8');

assert.doesNotMatch(producer, /mkdir\(output\);\s*startLspRuntime\(\);\s*const worktrees/, 'main must not pre-open the non-Rust OMO runtime before collectNonRustFiles owns it');
assert.match(producer, /async function collectNonRustFiles\(files, worktree, collect\) \{ startLspRuntime\(\); try \{ return await mapPool\(files, 1, collect\); \} finally \{ await stopLspRuntime\(\); \} \}/, 'non-Rust collector must own one balanced OMO runtime lifecycle');

process.stdout.write('task4_non_rust_runtime_lifecycle=pass\n');
