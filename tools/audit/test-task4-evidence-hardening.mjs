#!/usr/bin/env node
import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

import { ROOT, canonical } from './lib/validate-capability-audit-core.mjs';

const producer = path.join(ROOT, 'tools/audit/produce-task4-evidence.mjs');
const help = spawnSync(process.execPath, [producer, '--help'], { cwd: ROOT, encoding: 'utf8' });
assert.equal(help.status, 0, help.stderr);
assert.match(help.stdout, /^usage: node tools\/audit\/produce-task4-evidence\.mjs --run-id /);

const evidence = process.argv[2];
if (!evidence) throw new Error('usage: node tools/audit/test-task4-evidence-hardening.mjs <evidence-root>');
const cli = path.join(ROOT, 'tools/audit/validate-capability-audit.mjs');
const legacyEvidence = '.omo/evidence/audit-26026fb3862e-20260715T001000Z-e2e';
const temporary = `tools/audit/.task4-evidence-hardening-${process.pid}-${crypto.randomBytes(5).toString('hex')}`;

function relative(file) { return path.relative(ROOT, file).split(path.sep).join('/'); }
function shaBytes(bytes) { return crypto.createHash('sha256').update(bytes).digest('hex'); }
function shaJson(value) { return shaBytes(canonical(value)); }
function read(root, name) { return JSON.parse(fs.readFileSync(path.join(ROOT, root, name), 'utf8')); }
function write(root, name, value) { fs.writeFileSync(path.join(ROOT, root, name), `${JSON.stringify(value, null, 2)}\n`); }
function reseal(value) {
  const unsigned = Object.fromEntries(Object.entries(value).filter(([key]) => key !== 'receipt_sha256'));
  return { ...unsigned, receipt_sha256: shaJson(unsigned) };
}
function resignBootstrap(bootstrap) {
  const unsigned = Object.fromEntries(Object.entries(bootstrap).filter(([key]) => key !== 'runner_attestation'));
  const key = fs.readFileSync(bootstrap.runner_attestation.key_path);
  bootstrap.runner_attestation.bootstrap_hmac_sha256 = crypto.createHmac('sha256', key).update(canonical(unsigned)).digest('hex');
  return bootstrap;
}
function run(args) { return spawnSync(process.execPath, [cli, ...args], { cwd: ROOT, encoding: 'utf8' }); }
function reject(name, result, pattern) {
  assert.notEqual(result.status, 0, `${name} unexpectedly passed`);
  assert.match(result.stderr, pattern, `${name}: ${result.stderr}`);
}
function inventoryArgs(root, output = `${temporary}/inventory-output.json`) {
  const runId = read(root, 'task-1-bootstrap.json').run_id;
  return [
    'inventory', '--run-id', runId,
    '--bootstrap', `${root}/task-1-bootstrap.json`,
    '--preflight-receipt', `${root}/captures/task-01-preflight.command-receipt.json`,
    '--input', `${root}/task-4-capability-inventory.json`,
    '--candidate-manifest', `${root}/task-4-candidate-manifest.json`,
    '--blobs', `${root}/task-4-source-blobs.json`,
    '--test-list-receipt', `${root}/captures/task-04-rust-test-list.command-receipt.json`,
    '--test-results-receipt', `${root}/captures/task-04-rust-test-results.command-receipt.json`,
    '--test-reconciliation', `${root}/test-reconciliation.json`,
    '--evidence-execution-manifest', `${root}/evidence-execution-manifest.json`,
    '--out', output,
  ];
}
function copyEvidence(name) {
  const target = `${temporary}/${name}`;
  fs.cpSync(path.join(ROOT, evidence), path.join(ROOT, target), { recursive: true });
  return target;
}
function withEvidence(name, callback) {
  const target = copyEvidence(name);
  try { callback(target); } finally { fs.rmSync(path.join(ROOT, target), { recursive: true, force: true }); }
}
function lspState(root) {
  return { candidate: read(root, 'task-4-candidate-manifest.json'), lsp: read(root, 'task-4-lsp-receipts-manifest.json') };
}
function saveLspState(root, state) {
  write(root, 'task-4-lsp-receipts-manifest.json', state.lsp);
  state.candidate.extensions.receipt_manifests.lsp.sha256 = shaJson(state.lsp);
  write(root, 'task-4-candidate-manifest.json', state.candidate);
}
function setLspComplete(root, state, entry, complete) {
  const receipt = read(root, entry.receipt_path);
  receipt.complete = complete;
  const sealed = reseal(receipt);
  write(root, entry.receipt_path, sealed);
  entry.complete = complete;
  entry.receipt_sha256 = sealed.receipt_sha256;
}
function completeLspWithCandidateSymbol(root, entry, candidate) {
  const receipt = read(root, entry.receipt_path);
  const symbol = {
    name: candidate.symbol,
    kind: 'file',
    line_start: candidate.line_start,
    line_end: candidate.line_end,
    definition_character: candidate.definition_character,
    children: [],
  };
  receipt.complete = true;
  receipt.symbols = [symbol];
  receipt.ordered_result_sha256 = shaJson(receipt.symbols);
  receipt.flattened_symbols_sha256 = shaJson([Object.fromEntries(Object.entries(symbol).filter(([key]) => key !== 'children'))]);
  const sealed = reseal(receipt);
  write(root, entry.receipt_path, sealed);
  entry.complete = true;
  entry.receipt_sha256 = sealed.receipt_sha256;
  entry.flattened_symbols_sha256 = sealed.flattened_symbols_sha256;
}
function files(root) {
  const rootPath = path.join(ROOT, root);
  const listed = [];
  function visit(directory) {
    for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
      const absolute = path.join(directory, entry.name);
      if (entry.isDirectory()) visit(absolute);
      else if (entry.isFile()) listed.push({ path: path.relative(rootPath, absolute).split(path.sep).join('/'), sha256: shaBytes(fs.readFileSync(absolute)) });
    }
  }
  visit(rootPath);
  return listed.sort((left, right) => left.path.localeCompare(right.path));
}
function verifyReproducibilityManifest(root) {
  const manifestName = 'task-4-reproducibility-manifest.json';
  const manifest = read(root, manifestName);
  assert.deepEqual(Object.keys(manifest).sort(), ['collection_protocol', 'entries', 'entries_sha256', 'independent_replay_projection', 'primary_projection', 'projection_sha256', 'run_id', 'schema_version', 'source_baseline_sha', 'verdict']);
  assert.equal(manifest.schema_version, 2);
  assert.equal(manifest.run_id, path.basename(root));
  assert.equal(manifest.verdict, 'pass');
  assert.match(manifest.entries_sha256, /^[0-9a-f]{64}$/);
  assert.match(manifest.projection_sha256, /^[0-9a-f]{64}$/);
  assert.match(manifest.collection_protocol, /two independent frozen-baseline/i);
  assert.deepEqual(manifest.primary_projection, manifest.independent_replay_projection, 'independent replay projection differs from the primary collection');
  assert.equal(shaJson(manifest.primary_projection), manifest.projection_sha256, 'independent collection projection digest drifted');
  assert.ok(Array.isArray(manifest.entries) && manifest.entries.length > 20, 'reproducibility manifest must cover the full canonical evidence set');
  assert.deepEqual(manifest.entries, [...manifest.entries].sort((left, right) => left.path.localeCompare(right.path)), 'reproducibility manifest entries must be sorted');
  assert.equal(shaJson(manifest.entries), manifest.entries_sha256, 'reproducibility manifest aggregate hash drifted');
  const actual = files(root).filter((entry) => entry.path !== manifestName);
  assert.deepEqual(manifest.entries, actual, 'reproducibility manifest must hash every canonical output, not only Markdown');
  const markdown = new Map(manifest.entries.map((entry) => [entry.path, entry.sha256]));
  assert.equal(markdown.get('task-4-capability-inventory.md'), markdown.get('task-4-capability-inventory.md.second'), 'repeated inventory build output differs');
}

try {
  fs.mkdirSync(path.join(ROOT, temporary), { recursive: true });
  const goodPreflight = run(['preflight', '--run-id', path.basename(evidence), '--bootstrap', `${evidence}/task-1-bootstrap.json`]);
  assert.equal(goodPreflight.status, 0, goodPreflight.stderr);
  const goodInventory = run(inventoryArgs(evidence));
  assert.equal(goodInventory.status, 0, goodInventory.stderr);
  verifyReproducibilityManifest(evidence);

  withEvidence('bootstrap-cleanliness', (root) => {
    const bootstrap = read(root, 'task-1-bootstrap.json');
    bootstrap.cleanliness.original_status_porcelain = [' M fabricated'];
    write(root, 'task-1-bootstrap.json', bootstrap);
    reject('fabricated bootstrap cleanliness', run(['preflight', '--run-id', path.basename(evidence), '--bootstrap', `${root}/task-1-bootstrap.json`]), /invalid preflight cleanliness|runner signature/);
  });
  withEvidence('bootstrap-ancestry', (root) => {
    const bootstrap = read(root, 'task-1-bootstrap.json');
    bootstrap.ancestry.audit_worktree_descends_from_baseline = false;
    write(root, 'task-1-bootstrap.json', bootstrap);
    reject('fabricated bootstrap ancestry', run(['preflight', '--run-id', path.basename(evidence), '--bootstrap', `${root}/task-1-bootstrap.json`]), /invalid preflight ancestry|runner signature/);
  });
  withEvidence('audit-worktree-dirty-at-capture', (root) => {
    const bootstrap = read(root, 'task-1-bootstrap.json');
    bootstrap.cleanliness.audit_status_porcelain_before_evidence = [' M tools/audit/capture-command.mjs'];
    write(root, 'task-1-bootstrap.json', resignBootstrap(bootstrap));
    reject('signed audit worktree drift', run(['preflight', '--run-id', bootstrap.run_id, '--bootstrap', `${root}/task-1-bootstrap.json`]), /invalid preflight cleanliness/);
  });
  withEvidence('recorded-audit-tree-mismatch', (root) => {
    const bootstrap = read(root, 'task-1-bootstrap.json');
    bootstrap.audit_tree_oid = '0'.repeat(40);
    write(root, 'task-1-bootstrap.json', resignBootstrap(bootstrap));
    reject('signed recorded audit tree mismatch', run(['preflight', '--run-id', bootstrap.run_id, '--bootstrap', `${root}/task-1-bootstrap.json`]), /recorded audit execution tree mismatch/);
  });
  withEvidence('bootstrap-run-id', (root) => {
    reject('fabricated bootstrap run ID', run(['preflight', '--run-id', 'forged-run-id', '--bootstrap', `${root}/task-1-bootstrap.json`]), /preflight run_id: invocation mismatch/);
  });
  withEvidence('fabricated-graph', (root) => {
    const graph = read(root, 'task-4-graph-receipts-manifest.json');
    const receipt = read(root, graph.slots[0].receipt_path);
    fs.appendFileSync(path.join(ROOT, root, receipt.raw_path), '\n{"fabricated":true}\n');
    reject('fabricated graph raw result', run(inventoryArgs(root)), /Task 4 graph raw hash mismatch/);
  });
  withEvidence('fabricated-lsp', (root) => {
    const { lsp } = lspState(root);
    const receipt = read(root, lsp.entries[0].receipt_path);
    fs.appendFileSync(path.join(ROOT, root, receipt.raw_path), '\n{"fabricated":true}\n');
    reject('fabricated LSP raw result', run(inventoryArgs(root)), /Task 4 LSP raw hash mismatch/);
  });
  withEvidence('forged-data-flows', (root) => {
    const candidate = read(root, 'task-4-candidate-manifest.json');
    const reference = candidate.extensions.receipt_manifests.data_flows_absence;
    const absence = read(root, reference.path);
    absence.edge_type = 'CALLS';
    write(root, reference.path, absence);
    reference.sha256 = shaJson(absence);
    write(root, 'task-4-candidate-manifest.json', candidate);
    reject('forged DATA_FLOWS absence', run(inventoryArgs(root)), /DATA_FLOWS absence receipt linkage mismatch/);
  });
  withEvidence('forged-graph-schema', (root) => {
    const candidate = read(root, 'task-4-candidate-manifest.json');
    const absence = read(root, candidate.extensions.receipt_manifests.data_flows_absence.path);
    const schema = read(root, absence.graph_schema_path);
    schema.edge_types.push({ type: 'DATA_FLOWS', count: 1, properties: [] });
    write(root, absence.graph_schema_path, schema);
    reject('forged captured graph schema', run(inventoryArgs(root)), /graph schema raw hash mismatch|does not prove DATA_FLOWS absence/);
  });
  withEvidence('missing-preflight', (root) => {
    const args = inventoryArgs(root);
    const index = args.indexOf('--preflight-receipt');
    args[index + 1] = `${root}/missing-preflight.json`;
    reject('missing preflight receipt', run(args), /missing preflight receipt/);
  });
  withEvidence('forged-preflight', (root) => {
    const receipt = read(root, 'captures/task-01-preflight.command-receipt.json');
    receipt.exit_code = 1;
    const sealed = reseal(receipt);
    write(root, 'captures/task-01-preflight.command-receipt.json', sealed);
    reject('resealed preflight without runner key', run(inventoryArgs(root)), /invalid preflight receipt|runner signature/);
  });
  withEvidence('forged-runner-attestation', (root) => {
    const receipt = read(root, 'captures/task-04-rust-test-list.command-receipt.json');
    receipt.stdout_sha256 = '0'.repeat(64);
    const sealed = reseal(receipt);
    write(root, 'captures/task-04-rust-test-list.command-receipt.json', sealed);
    reject('resealed receipt without runner key', run(inventoryArgs(root)), /runner signature|stdout sidecar/);
  });
  withEvidence('forged-integration-target', (root) => {
    const reconciliation = read(root, 'test-reconciliation.json');
    reconciliation.targets[0].path = 'crates/ontopolis-types/src/coords.rs';
    reconciliation.targets[0].blob_oid = '0'.repeat(40);
    write(root, 'test-reconciliation.json', reseal(reconciliation));
    reject('forged coords integration target', run(inventoryArgs(root)), /target metadata|test metadata/);
  });
  withEvidence('missing-data-flows', (root) => {
    const candidate = read(root, 'task-4-candidate-manifest.json');
    fs.rmSync(path.join(ROOT, root, candidate.extensions.receipt_manifests.data_flows_absence.path));
    reject('missing DATA_FLOWS absence', run(inventoryArgs(root)), /missing Task 4 DATA_FLOWS absence receipt/);
  });
  withEvidence('stale-lsp-blob', (root) => {
    const state = lspState(root);
    state.lsp.entries[0].worktree_blob_oid = '0'.repeat(40);
    saveLspState(root, state);
    reject('stale LSP blob', run(inventoryArgs(root)), /stale Task 4 LSP blob binding/);
  });
  withEvidence('stale-lsp-range', (root) => {
    const candidate = read(root, 'task-4-candidate-manifest.json');
    const mapped = candidate.candidates.find((entry) => entry.bindings.length > 0);
    mapped.line_start = 999999;
    mapped.line_end = 999999;
    write(root, 'task-4-candidate-manifest.json', candidate);
    reject('stale LSP range', run(inventoryArgs(root)), /Task 4 candidate LSP range is not covered/);
  });
  withEvidence('stale-binding', (root) => {
    const candidate = read(root, 'task-4-candidate-manifest.json');
    candidate.candidates.find((entry) => entry.bindings.length > 0).bindings[0].binding_id = 'stale-binding';
    write(root, 'task-4-candidate-manifest.json', candidate);
    reject('stale binding', run(inventoryArgs(root)), /candidate binding context mismatch/);
  });
  for (const expectedIncomplete of [26, 28]) withEvidence(`lsp-${expectedIncomplete}-incomplete`, (root) => {
    const state = lspState(root);
    const entry = state.lsp.entries.find((item) => item.complete === (expectedIncomplete === 28));
    setLspComplete(root, state, entry, expectedIncomplete !== 28);
    saveLspState(root, state);
    reject(`${expectedIncomplete} incomplete LSP sources`, run(inventoryArgs(root)), /Task 4 LSP provider aggregate must be 105 complete and 27 incomplete files/);
  });
  withEvidence('missing-lsp-source', (root) => {
    const state = lspState(root);
    state.lsp.entries.pop();
    saveLspState(root, state);
    reject('missing LSP source', run(inventoryArgs(root)), /Task 4 LSP manifest must cover exactly 132 candidate files/);
  });
  withEvidence('duplicate-lsp-source', (root) => {
    const state = lspState(root);
    state.lsp.entries.push(structuredClone(state.lsp.entries[0]));
    saveLspState(root, state);
    reject('duplicate LSP source', run(inventoryArgs(root)), /Task 4 LSP manifest must cover exactly 132 candidate files/);
  });
  withEvidence('incomplete-lsp-m1', (root) => {
    const state = lspState(root);
    const execution = read(root, 'evidence-execution-manifest.json');
    const claimed = new Set(execution.claims.map((claim) => claim.binding_id));
    const candidate = state.candidate.candidates.find((entry) => entry.bindings.some((binding) => !claimed.has(binding.binding_id)));
    const binding = candidate.bindings.find((item) => !claimed.has(item.binding_id));
    const incomplete = state.lsp.entries.find((entry) => !entry.complete && entry.path !== candidate.path);
    const candidateEntry = state.lsp.entries.find((entry) => entry.path === candidate.path);
    const incompleteCandidate = state.candidate.candidates.find((entry) => entry.path === incomplete?.path);
    assert.ok(incomplete && candidateEntry && incompleteCandidate, 'fixture must contain a complete non-claimed source and an incomplete source');
    setLspComplete(root, state, candidateEntry, false);
    completeLspWithCandidateSymbol(root, incomplete, incompleteCandidate);
    saveLspState(root, state);
    const inventory = read(root, 'task-4-capability-inventory.json');
    const capability = inventory.capabilities.find((entry) => entry.capability_id === binding.capability_id);
    capability.target_maturity = 'M1';
    for (const level of ['M0', 'M1']) capability.levels[level] = { status: 'satisfied', evidence_ids: ['e-source-definition'], rationale: 'negative M1 LSP provenance control' };
    capability.semantic_profile.determinism = { status: 'present', evidence_ids: ['e-source-definition'], rationale: 'negative M1 LSP provenance control' };
    write(root, 'task-4-capability-inventory.json', inventory);
    reject('incomplete LSP source promoted to M1', run(inventoryArgs(root)), /incomplete Task 4 LSP file cannot support M1 maturity/);
  });
  withEvidence('empty-carriers', (root) => {
    const inventory = read(root, 'task-4-capability-inventory.json');
    inventory.capabilities[0].carriers = [];
    write(root, 'task-4-capability-inventory.json', inventory);
    reject('empty capability carriers', run(inventoryArgs(root)), /carriers must be non-empty/);
  });

  const legacy = run([
    'inventory', '--run-id', path.basename(legacyEvidence),
    '--input', `${legacyEvidence}/task-4-capability-inventory.json`,
    '--candidate-manifest', `${legacyEvidence}/task-4-candidate-manifest.json`,
    '--blobs', `${legacyEvidence}/task-4-source-blobs.json`,
    '--test-list-receipt', `${legacyEvidence}/rust-test-list.command-receipt.json`,
    '--test-results-receipt', `${legacyEvidence}/rust-test-results.command-receipt.json`,
    '--test-reconciliation', `${legacyEvidence}/test-reconciliation.json`,
    '--evidence-execution-manifest', `${legacyEvidence}/evidence-execution-manifest.json`,
    '--out', `${temporary}/legacy-output.json`,
  ]);
  reject('legacy synthetic 001000 evidence', legacy, /missing --bootstrap|invalid candidate semantic review|Task 4/);
  process.stdout.write('task4_evidence_hardening=19 fail_closed_controls pass\n');
} finally {
  fs.rmSync(path.join(ROOT, temporary), { recursive: true, force: true });
}
