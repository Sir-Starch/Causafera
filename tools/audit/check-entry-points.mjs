#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';

const ROOT = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..', '..');
const REQUIRED = [
  'tools/audit/build-task4-inventory.mjs',
  'tools/audit/build-tooling-blobs.mjs',
  'tools/audit/capture-command.mjs',
  'tools/audit/lib/capture-security.mjs',
  'tools/audit/lib/validate-capability-audit-core.mjs',
  'tools/audit/produce-task4-evidence.mjs',
  'tools/audit/run-source-tests.mjs',
  'tools/audit/test-artifact-modes.mjs',
  'tools/audit/test-capture-cargo-dispatch.mjs',
  'tools/audit/test-m2-class-eligibility.mjs',
  'tools/audit/test-provenance-hardening.mjs',
  'tools/audit/test-source-definition-capture.mjs',
  'tools/audit/test-task4-direct-rust-carrier-location.mjs',
  'tools/audit/test-task4-direct-rust-null-references.mjs',
  'tools/audit/test-task4-direct-rust-provider.mjs',
  'tools/audit/test-task4-direct-rust-reference-anchor.mjs',
  'tools/audit/test-task4-direct-rust-symbol-normalization.mjs',
  'tools/audit/test-task4-evidence-hardening.mjs',
  'tools/audit/test-task4-inventory-contract.mjs',
  'tools/audit/test-task4-inventory-markdown.mjs',
  'tools/audit/test-task4-lsp-attempts.mjs',
  'tools/audit/test-task4-lsp-child-environment.mjs',
  'tools/audit/test-task4-non-rust-runtime.mjs',
  'tools/audit/test-tooling-and-endpoints.mjs',
  'tools/audit/test-trust-boundaries.mjs',
  'tools/audit/validate-capability-audit.mjs',
  'tools/audit/verify-mapping-endpoints.mjs',
];

const trackedTests = new Set(REQUIRED.filter((file) => path.basename(file).startsWith('test-')));
if (trackedTests.size === 0) throw new Error('audit test entry-point list is empty');

for (const file of REQUIRED) {
  const absolute = path.join(ROOT, file);
  let stat;
  try {
    stat = fs.lstatSync(absolute);
  } catch {
    throw new Error(`required audit entry point is missing: ${file}`);
  }
  if (stat.isSymbolicLink() || !stat.isFile()) throw new Error(`required audit entry point is not a regular file: ${file}`);
  if (stat.size === 0 || fs.readFileSync(absolute, 'utf8').trim().length === 0) throw new Error(`required audit entry point is empty: ${file}`);
}

process.stdout.write(`audit_entry_points=${REQUIRED.length} audit_tests=${trackedTests.size} nonempty_regular_files pass\n`);
