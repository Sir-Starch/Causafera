#!/usr/bin/env node
import { spawnSync } from 'node:child_process';

const SELF_CONTAINED_TESTS = [
  'tools/audit/test-capture-cargo-dispatch.mjs',
  'tools/audit/test-m2-class-eligibility.mjs',
  'tools/audit/test-provenance-hardening.mjs',
  'tools/audit/test-source-definition-capture.mjs',
  'tools/audit/test-task4-direct-rust-carrier-location.mjs',
  'tools/audit/test-task4-direct-rust-null-references.mjs',
  'tools/audit/test-task4-direct-rust-provider.mjs',
  'tools/audit/test-task4-direct-rust-reference-anchor.mjs',
  'tools/audit/test-task4-direct-rust-symbol-normalization.mjs',
  'tools/audit/test-task4-inventory-contract.mjs',
  'tools/audit/test-task4-inventory-markdown.mjs',
  'tools/audit/test-task4-lsp-child-environment.mjs',
  'tools/audit/test-task4-non-rust-runtime.mjs',
  'tools/audit/test-trust-boundaries.mjs',
];

if (SELF_CONTAINED_TESTS.length === 0) throw new Error('source-only audit test list is empty');
const result = spawnSync(process.execPath, ['--test', ...SELF_CONTAINED_TESTS], { stdio: 'inherit' });
process.exitCode = Number.isInteger(result.status) ? result.status : 1;
