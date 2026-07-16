#!/usr/bin/env node
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { runCli } from './lib/validate-capability-audit-core.mjs';

const root = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..', '..');
const inputPath = path.join(root, 'tools/audit/examples/capability-audit-input.valid.json');
const originalRead = fs.readFileSync;
const seed = JSON.parse(originalRead(inputPath, 'utf8'));
const catalog = JSON.parse(originalRead(path.join(root, 'tools/audit/capability-catalog.json'), 'utf8'));
const capabilities = [];

for (const domain of catalog.domains) {
  for (const capabilityId of domain.capability_ids) {
    const capability = structuredClone(seed.capabilities[0]);
    capability.capability_id = capabilityId;
    capability.domain = domain.domain;
    capability.capability_class = 'validation';
    capability.target_maturity = 'M2';
    capability.semantic_profile.target = 'M2';
    for (const level of ['M0', 'M1', 'M2']) {
      capability.levels[level] = {
        status: 'satisfied',
        evidence_ids: ['e-001'],
        rationale: 'synthetic class-eligibility negative'
      };
    }
    capabilities.push(capability);
  }
}

const input = structuredClone(seed);
input.capabilities = capabilities;
input.domains = catalog.domains;
input.evidence[0].adapter = 'production_composition';
input.evidence[0].facets = ['production_root_qn', 'mutation_owner_qn', 'path'];
fs.readFileSync = function patchedRead(file, ...args) {
  if (String(file) === inputPath) return JSON.stringify(input);
  return originalRead.call(this, file, ...args);
};

try {
  assert.throws(
    () => runCli(['capability-inventory', '--input', 'tools/audit/examples/capability-audit-input.valid.json', '--run-id', input.run_id]),
    /disallowed M2 capability class: validation/
  );
  process.stdout.write('m2_class_eligibility=30 domains 61 capabilities reject pass\n');
} finally {
  fs.readFileSync = originalRead;
}
