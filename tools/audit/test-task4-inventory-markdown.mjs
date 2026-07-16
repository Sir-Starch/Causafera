#!/usr/bin/env node
import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

import { ROOT, parseMachineData, readArtifact } from './lib/validate-capability-audit-core.mjs';

const builder = path.join(ROOT, 'tools/audit/build-task4-inventory.mjs');
const checker = path.join(ROOT, 'tools/audit/validate-capability-audit.mjs');
const catalog = JSON.parse(fs.readFileSync(path.join(ROOT, 'tools/audit/capability-catalog.json'), 'utf8'));
const template = JSON.parse(fs.readFileSync(path.join(ROOT, 'tools/audit/examples/capability-audit-input.valid.json'), 'utf8'));
const temporary = fs.mkdtempSync(path.join(ROOT, 'tools/audit/.task4-markdown-'));
const runId = `task4-markdown-${process.pid}-${crypto.randomBytes(5).toString('hex')}`;
const baseline = '26026fb3862e8d178a2e59df7a68a2901e80b123';

function clone(value) { return structuredClone(value); }
function run(entry, args) { return spawnSync(process.execPath, [entry, ...args], { cwd: ROOT, encoding: 'utf8' }); }

function inventoryInput() {
  const evidence = clone(template.evidence[0]);
  evidence.run_id = runId;
  evidence.source_baseline_sha = baseline;
  const capabilityTemplate = template.capabilities[0];
  return {
    schema_version: 1,
    run_id: runId,
    source_baseline_sha: baseline,
    domains: catalog.domains.map((domain) => ({ domain: domain.domain, capability_ids: [...domain.capability_ids] })),
    capabilities: catalog.capabilities.map((entry) => {
      const capability = clone(capabilityTemplate);
      capability.capability_id = entry.capability_id;
      capability.domain = entry.domain;
      return capability;
    }),
    evidence: [evidence],
    todo_bindings: clone(template.todo_bindings),
    source_reconciliation: clone(template.source_reconciliation),
    extensions: {},
  };
}

try {
  const inputPath = path.join(temporary, 'inventory.json');
  const markdownPath = path.join(temporary, 'inventory.md');
  const bareMarkdownPath = path.join(temporary, 'bare-inventory.md');
  const input = inventoryInput();
  fs.writeFileSync(inputPath, `${JSON.stringify(input, null, 2)}\n`);
  fs.writeFileSync(bareMarkdownPath, `${JSON.stringify(input, null, 2)}\n`);

  const build = run(builder, ['build', '--catalog', 'tools/audit/capability-catalog.json', '--inventory', path.relative(ROOT, inputPath), '--out', path.relative(ROOT, markdownPath)]);
  assert.equal(build.status, 0, build.stderr);
  const markdown = fs.readFileSync(markdownPath, 'utf8');
  assert.match(markdown, /^<!-- ontopolis-machine-data:capability-audit-input:v1 -->\n```json\n/m);
  assert.match(markdown, /\n```\n<!-- \/ontopolis-machine-data -->\n$/);
  const parsed = parseMachineData(markdown);
  assert.ok(parsed, 'generated inventory must expose machine data');
  assert.equal(parsed.domains.length, 30);
  assert.equal(parsed.capabilities.length, 61);
  assert.equal(readArtifact(path.relative(ROOT, markdownPath)).value.capabilities.length, 61);

  const capabilityInventory = run(checker, ['capability-inventory', '--run-id', runId, '--input', path.relative(ROOT, markdownPath)]);
  assert.equal(capabilityInventory.status, 0, capabilityInventory.stderr);

  const bareMarkdown = run(checker, ['capability-inventory', '--run-id', runId, '--input', path.relative(ROOT, bareMarkdownPath)]);
  assert.notEqual(bareMarkdown.status, 0, 'bare JSON with an .md extension must be rejected');
  assert.match(bareMarkdown.stderr, /bare JSON Markdown artifact requires ontopolis-machine-data envelope/);
  process.stdout.write('task4_inventory_markdown=30 domains 61 capabilities roundtrip pass\n');
} finally {
  fs.rmSync(temporary, { recursive: true, force: true });
}
