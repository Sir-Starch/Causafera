#!/usr/bin/env node
import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

const root = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..', '..');
const cli = path.join(root, 'tools/audit/build-task4-inventory.mjs');
const catalog = JSON.parse(fs.readFileSync(path.join(root, 'tools/audit/capability-catalog.json'), 'utf8'));
const temporary = fs.mkdtempSync(path.join(root, 'tools/audit/.task4-contract-'));
const baseline = '26026fb3862e8d178a2e59df7a68a2901e80b123';
const dimensions = ['resolution', 'persistence', 'provenance', 'observer', 'explanation', 'determinism', 'performance', 'negative_control'];

function clone(value) { return structuredClone(value); }
function write(name, value) { const file = path.join(temporary, name); fs.writeFileSync(file, `${JSON.stringify(value, null, 2)}\n`); return path.relative(root, file); }
function run(args) { return spawnSync(process.execPath, [cli, ...args], { cwd: root, encoding: 'utf8' }); }
function reject(name, args, expression) { const result = run(args); assert.notEqual(result.status, 0, `${name} accepted`); assert.match(result.stderr, expression, `${name}: ${result.stderr}`); }

function semantic(status = 'missing') {
  return Object.fromEntries(dimensions.map((field) => [field, { status: field === 'determinism' ? status : 'missing', evidence_ids: field === 'determinism' && status === 'present' ? ['determinism-evidence'] : [], rationale: `${field} ${status}` }]));
}
function capability(entry, active) {
  return {
    capability_id: entry.capability_id, domain: entry.domain, capability_class: 'state',
    authoritative_state: { path: active ? 'crates/core/src/lib.rs' : null, symbol: active ? 'State' : null, rationale: 'state' },
    mutation_owner: { path: active ? 'crates/core/src/lib.rs' : null, symbol: active ? 'commit' : null, rationale: 'owner' },
    levels: { M0: { status: active ? 'satisfied' : 'missing', evidence_ids: active ? ['m0-evidence'] : [] }, M1: { status: active ? 'satisfied' : 'missing', evidence_ids: active ? ['m1-evidence'] : [] } },
    semantic_profile: semantic(active ? 'present' : 'missing'),
  };
}
function model() {
  const active = catalog.capabilities[0];
  const inventory = {
    source_baseline_sha: baseline,
    domains: catalog.domains.map((domain) => ({ domain: domain.domain, capability_ids: [...domain.capability_ids] })),
    capabilities: catalog.capabilities.map((entry) => capability(entry, entry.capability_id === active.capability_id)),
  };
  const candidate = { candidate_key: 'candidate-1', bindings: [{ binding_id: 'binding-1', capability_id: active.capability_id, role: 'primary', lifecycle_boundary: 'commit' }], exclusion_id: null, symbol: 'State', path: 'crates/core/src/lib.rs', line_start: 1, line_end: 8, failure_cases: ['failure-1'] };
  const candidates = {
    source_baseline_sha: baseline,
    graph_queries: Array.from({ length: 19 }, (_, index) => ({ query_id: `q-${index}`, edge_type: 'GIT_SYMBOL_FALLBACK', query: `baseline git symbol fallback ${index}`, result_count: 1 })),
    candidates: [candidate],
    extensions: { receipt_manifests: { graph: { receipt_id: 'graph-manifest', path: 'graph.json', sha256: '0'.repeat(64) }, lsp: { receipt_id: 'lsp-manifest', path: 'lsp.json', sha256: '0'.repeat(64) }, data_flows_absence: { receipt_id: 'absence-1', path: 'absence.json', sha256: '0'.repeat(64) } }, semantic_review: { receipt_id: 'semantic-review', path: 'semantic.json', sha256: '0'.repeat(64) }, binding_contexts: [{
      binding_id: 'binding-1', candidate_key: 'candidate-1',
      grouping: { domain: active.domain, capability_class: 'state', state_path: 'crates/core/src/lib.rs', mutation_owner: 'commit', lifecycle_boundary: 'commit' },
      endpoint_declaration: { qualified_name: 'State', line_start: 1, line_end: 8, kind: 'graph_node' },
    }] },
  };
  const reconciliation = { source_baseline_sha: baseline, tests: [{ test_id: 'test-1', eligibility: 'exact_test', discovered_receipt_id: 'list-1' }], mappings: [{ binding_id: 'binding-1', candidate_key: 'candidate-1', test_id: 'test-1' }] };
  const testList = { adapter: 'cargo_test_list', exit_code: 0, receipt_id: 'list-1', argv: ['cargo', 'test', '--list'] };
  const testResults = { adapter: 'exact_test', exit_code: 0, receipt_id: 'results-1', argv: ['cargo', 'test', 'example', '--exact'] };
  const execution = { source_baseline_sha: baseline, claims: [{ binding_id: 'binding-1', capability_id: active.capability_id, facets: ['failure_case:failure-1'] }] };
  return { inventory, candidates, reconciliation, testList, testResults, execution };
}
function args(data, catalogPath = 'tools/audit/capability-catalog.json') {
  return ['verify', '--catalog', catalogPath, '--inventory', write('inventory.json', data.inventory), '--candidate-manifest', write('candidates.json', data.candidates), '--test-reconciliation', write('tests.json', data.reconciliation), '--test-list-receipt', write('list.json', data.testList), '--test-results-receipt', write('results.json', data.testResults), '--evidence-execution-manifest', write('execution.json', data.execution), '--source-baseline', baseline];
}

try {
  const happy = run(args(model()));
  assert.equal(happy.status, 0, happy.stderr);
  assert.match(happy.stdout, /task4_inventory_contract=61 concrete capabilities pass/);

  const bareProfile = model();
  bareProfile.inventory.capabilities[0].semantic_profile.determinism = 'present';
  reject('bare semantic status', args(bareProfile), /determinism contract/);

  const missingExact = model();
  missingExact.reconciliation.tests[0].discovered_receipt_id = 'other-list';
  reject('M1 exact test', args(missingExact), /baseline-discovered exact test/);

  const wrongGraph = model();
  wrongGraph.candidates.graph_queries.push({ query_id: 'q-20', edge_type: 'CALLS', query: 'MATCH (n) RETURN n', result_count: 0 });
  reject('graph cardinality', args(wrongGraph), /graph query cardinality/);

  for (const count of [18, 20, 22]) {
    const malformedCardinality = model();
    malformedCardinality.candidates.graph_queries = Array.from({ length: count }, (_, index) => ({
      query_id: `malformed-q-${index}`,
      edge_type: 'GIT_SYMBOL_FALLBACK',
      query: `baseline git symbol fallback malformed ${index}`,
      result_count: 1,
    }));
    reject(`graph cardinality ${count}`, args(malformedCardinality), /graph query cardinality/);
  }

  const duplicateGraph = model();
  duplicateGraph.candidates.graph_queries[1].query_id = duplicateGraph.candidates.graph_queries[0].query_id;
  reject('duplicate graph query ID', args(duplicateGraph), /duplicate Task 4 graph query ID/);

  const absentDataFlowsReceipt = model();
  delete absentDataFlowsReceipt.candidates.extensions.receipt_manifests.data_flows_absence;
  reject('missing DATA_FLOWS absence receipt', args(absentDataFlowsReceipt), /DATA_FLOWS absence receipt/);

  const forgedDataFlowsReceipt = model();
  forgedDataFlowsReceipt.candidates.extensions.receipt_manifests.data_flows_absence.sha256 = 'forged';
  reject('forged DATA_FLOWS absence receipt digest', args(forgedDataFlowsReceipt), /invalid DATA_FLOWS absence receipt digest/);

  const dataFlowsPresent = model();
  dataFlowsPresent.candidates.graph_queries = Array.from({ length: 23 }, (_, index) => ({
    query_id: `data-flows-q-${index}`,
    edge_type: index === 0 ? 'DATA_FLOWS' : 'GIT_SYMBOL_FALLBACK',
    query: `baseline concrete graph query ${index}`,
    result_count: 0,
  }));
  const dataFlowsHappy = run(args(dataFlowsPresent));
  assert.equal(dataFlowsHappy.status, 0, dataFlowsHappy.stderr);

  const syntheticGraph = model();
  syntheticGraph.candidates.graph_queries[0].query = 'baseline source review slot 1';
  reject('synthetic graph query', args(syntheticGraph), /invalid Task 4 graph query identity/);

  const catchAll = model();
  catchAll.candidates.candidates.push({ candidate_key: 'nested', bindings: [], exclusion_id: null, symbol: 'Nested', path: 'crates/core/src/lib.rs', line_start: 2, line_end: 3, failure_cases: [] });
  catchAll.candidates.extensions.binding_contexts[0].endpoint_declaration.kind = 'graph_module';
  reject('module-wide catch-all', args(catchAll), /module-wide catch-all/);

  const tupleMismatch = model();
  tupleMismatch.candidates.extensions.binding_contexts[0].grouping.lifecycle_boundary = 'wrong-boundary';
  reject('binding tuple', args(tupleMismatch), /binding tuple mismatch/);

  const missingBindingField = model();
  delete missingBindingField.candidates.candidates[0].bindings[0].role;
  reject('missing binding field', args(missingBindingField), /invalid Task 4 binding fields/);

  const bindingMismatch = model();
  bindingMismatch.candidates.candidates[0].bindings[0].capability_id = catalog.capabilities[1].capability_id;
  reject('mismatched binding capability', args(bindingMismatch), /binding tuple mismatch/);

  const disallowedM2Class = model();
  disallowedM2Class.inventory.capabilities[0].capability_class = 'validation';
  disallowedM2Class.inventory.capabilities[0].target_maturity = 'M2';
  disallowedM2Class.inventory.capabilities[0].levels.M2 = { status: 'satisfied', evidence_ids: ['m2-evidence'] };
  reject('disallowed M2 class', args(disallowedM2Class), /disallowed M2 capability class/);

  const placeholderCatalog = clone(catalog);
  placeholderCatalog.domains[0].capability_ids[0] = 'space.audited_foundation';
  placeholderCatalog.capabilities[0].capability_id = 'space.audited_foundation';
  reject('placeholder catalog', ['verify', '--catalog', write('placeholder-catalog.json', placeholderCatalog)], /placeholder/);

  const sixtyCatalog = clone(catalog);
  const removed = sixtyCatalog.capabilities.pop();
  sixtyCatalog.domains.find((domain) => domain.domain === removed.domain).capability_ids.pop();
  reject('60 capability catalog', ['verify', '--catalog', write('60-catalog.json', sixtyCatalog)], /exactly 30 domains and 61 capabilities/);

  const sixtyTwoCatalog = clone(catalog);
  sixtyTwoCatalog.domains[0].capability_ids.push('space.extra_concrete_capability');
  sixtyTwoCatalog.capabilities.push({ capability_id: 'space.extra_concrete_capability', domain: 'Space', rationale: 'negative count regression' });
  reject('62 capability catalog', ['verify', '--catalog', write('62-catalog.json', sixtyTwoCatalog)], /exactly 30 domains and 61 capabilities/);

  const duplicateDomainCatalog = clone(catalog);
  duplicateDomainCatalog.domains[1].domain = duplicateDomainCatalog.domains[0].domain;
  for (const entry of duplicateDomainCatalog.capabilities) if (entry.domain === 'Time') entry.domain = 'Space';
  reject('duplicate catalog domain', ['verify', '--catalog', write('duplicate-domain-catalog.json', duplicateDomainCatalog)], /domain order or membership mismatch/);

  const missingDomainCatalog = clone(catalog);
  missingDomainCatalog.domains[1].domain = 'Missing bounded domain';
  for (const entry of missingDomainCatalog.capabilities) if (entry.domain === 'Time') entry.domain = 'Missing bounded domain';
  reject('missing required catalog domain', ['verify', '--catalog', write('missing-domain-catalog.json', missingDomainCatalog)], /domain order or membership mismatch/);
  process.stdout.write('task4_inventory_contract=18 boundaries pass\n');
} finally {
  fs.rmSync(temporary, { recursive: true, force: true });
}
