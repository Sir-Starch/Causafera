#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import { LEGACY_BASELINE_MACHINE_DATA_MARKER, runCli } from './lib/validate-capability-audit-core.mjs';

const ROOT = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..', '..');
const DIMENSIONS = ['resolution', 'persistence', 'provenance', 'observer', 'explanation', 'determinism', 'performance', 'negative_control'];
const M2_PLUS_CAPABILITY_CLASSES = ['state', 'mutation', 'bootstrap', 'resolution'];
const M2_PLUS_LEVELS = ['M2', 'M3', 'M4', 'M5'];
const PLACEHOLDER = /(?:audited_foundation|contract_\d+|foundation|(?:ordinal|generic|placeholder))/i;
const AUDIT_DOMAINS = ['Space','Time','Matter','Energy','Pattern / Feature','Spatial geometry','Geography','Geology','Hydrology','Climate','Ecology','Biology','Physical access / perception','Cognition','Language','Mana','Causal resolution','Society','Economy','City infrastructure','Historical bootstrap','Epistemics','Practice','Isekai','Metaphysics','Simulation runtime','Explanation / analytics','Observer','UI','Optional LLM surface'];
const HELP = `usage:
  node tools/audit/build-task4-inventory.mjs verify --catalog PATH
  node tools/audit/build-task4-inventory.mjs verify --catalog PATH --inventory PATH \\
    --candidate-manifest PATH --test-reconciliation PATH --test-list-receipt PATH \\
    --test-results-receipt PATH --evidence-execution-manifest PATH --source-baseline SHA
  node tools/audit/build-task4-inventory.mjs build --catalog PATH --inventory PATH --out PATH

The verify command is read-only. The build command writes a canonical machine-data Markdown
artifact from a validated JSON capability inventory.
`;

function fail(message) { throw new Error(message); }
function canonical(value) {
  if (Array.isArray(value)) return `[${value.map(canonical).join(',')}]`;
  if (value && typeof value === 'object') return `{${Object.keys(value).sort().map((key) => `${JSON.stringify(key)}:${canonical(value[key])}`).join(',')}}`;
  return JSON.stringify(value);
}
function option(name, required = true) {
  const index = process.argv.indexOf(name);
  const value = index < 0 ? undefined : process.argv[index + 1];
  if (required && (!value || value.startsWith('--'))) fail(`missing ${name}`);
  return value;
}
function read(relative, label) {
  if (typeof relative !== 'string' || !relative) fail(`missing ${label}`);
  const absolute = path.resolve(ROOT, relative);
  if (path.relative(ROOT, absolute).startsWith('..')) fail(`${label} escapes repository`);
  return JSON.parse(fs.readFileSync(absolute, 'utf8'));
}
function output(relative) {
  if (typeof relative !== 'string' || !relative || path.isAbsolute(relative)) fail('invalid --out');
  const absolute = path.resolve(ROOT, relative);
  const parent = path.dirname(absolute);
  if (path.relative(ROOT, absolute).startsWith('..') || path.relative(ROOT, fs.realpathSync(parent)).startsWith('..')) fail('--out escapes repository');
  return absolute;
}
function own(object, fields, label) {
  if (!object || typeof object !== 'object' || Array.isArray(object) || canonical(Object.keys(object).sort()) !== canonical([...fields].sort())) fail(`invalid ${label} fields`);
}
function nonEmptyString(value, label) { if (typeof value !== 'string' || value.length === 0) fail(`invalid ${label}`); }

function validateCatalog(catalog) {
  own(catalog, ['schema_version', 'm2_plus_capability_classes', 'domains', 'capabilities'], 'capability catalog');
  if (catalog.schema_version !== 1 || !Array.isArray(catalog.domains) || !Array.isArray(catalog.capabilities)) fail('invalid capability catalog');
  if (canonical([...catalog.m2_plus_capability_classes ?? []].sort()) !== canonical([...M2_PLUS_CAPABILITY_CLASSES].sort())) fail('invalid M2 capability-class catalog');
  if (catalog.domains.length !== 30 || catalog.capabilities.length !== 61) fail('capability catalog must contain exactly 30 domains and 61 capabilities');
  if (canonical(catalog.domains.map((domain) => domain.domain)) !== canonical(AUDIT_DOMAINS)) fail('capability catalog domain order or membership mismatch');
  const ids = new Set();
  const domainIds = new Map();
  for (const domain of catalog.domains) {
    own(domain, ['domain', 'capability_ids'], 'capability catalog domain');
    nonEmptyString(domain.domain, 'capability catalog domain');
    if (!Array.isArray(domain.capability_ids) || domain.capability_ids.length === 0) fail(`empty capability catalog domain: ${domain.domain}`);
    domainIds.set(domain.domain, domain.capability_ids);
    for (const id of domain.capability_ids) {
      nonEmptyString(id, 'capability catalog ID');
      if (PLACEHOLDER.test(id) || ids.has(id)) fail(`invalid placeholder or duplicate capability ID: ${id}`);
      ids.add(id);
    }
  }
  for (const capability of catalog.capabilities) {
    own(capability, ['capability_id', 'domain', 'rationale'], 'capability catalog entry');
    nonEmptyString(capability.capability_id, 'capability catalog ID');
    nonEmptyString(capability.domain, 'capability catalog domain');
    nonEmptyString(capability.rationale, 'capability catalog rationale');
    if (!domainIds.get(capability.domain)?.includes(capability.capability_id)) fail(`catalog entry/domain mismatch: ${capability.capability_id}`);
  }
  if (ids.size !== 61 || canonical([...ids].sort()) !== canonical(catalog.capabilities.map((entry) => entry.capability_id).sort())) fail('capability catalog IDs drift');
  return ids;
}

function commandReceipt(value, adapter, label) {
  if (value?.adapter !== adapter || value.exit_code !== 0 || !Array.isArray(value.argv)) fail(`invalid ${label} receipt`);
  return value;
}
function exactTestReceipt(value, label, requiredFlag) {
  commandReceipt(value, requiredFlag ? 'exact_test' : 'cargo_test_list', label);
  if (value.argv[0] !== 'cargo' || value.argv[1] !== 'test' || value.argv.includes('--list') !== !requiredFlag || (requiredFlag && !value.argv.includes('--exact'))) fail(`invalid ${label} command`);
}
function tuple(capability, lifecycleBoundary) {
  return {
    domain: capability.domain,
    capability_class: capability.capability_class,
    state_path: capability.authoritative_state?.path ?? null,
    mutation_owner: capability.mutation_owner?.symbol ?? capability.mutation_owner?.path ?? null,
    lifecycle_boundary: lifecycleBoundary,
  };
}
function isModuleWide(candidate, context, candidates) {
  const kind = String(context.endpoint_declaration?.kind ?? '');
  if (!/module/i.test(kind)) return false;
  return candidates.some((nested) => nested.path === candidate.path && nested.candidate_key !== candidate.candidate_key
    && candidate.line_start <= nested.line_start && candidate.line_end >= nested.line_end);
}

function verifyTask4(catalog, inventory, candidates, reconciliation, testList, testResults, execution, sourceBaseline) {
  if (inventory.source_baseline_sha !== sourceBaseline || candidates.source_baseline_sha !== sourceBaseline || reconciliation.source_baseline_sha !== sourceBaseline || execution.source_baseline_sha !== sourceBaseline) fail('Task 4 source baseline mismatch');
  const catalogIds = validateCatalog(catalog);
  if (!Array.isArray(inventory.capabilities) || inventory.capabilities.length !== 61) fail('Task 4 inventory must contain exactly 61 capabilities');
  const inventoryCapabilities = new Map(inventory.capabilities.map((capability) => [capability.capability_id, capability]));
  if (inventoryCapabilities.size !== 61 || canonical([...inventoryCapabilities.keys()].sort()) !== canonical([...catalogIds].sort())) fail('Task 4 inventory/catalog IDs drift');
  for (const capability of inventory.capabilities) {
    const claimsM2Plus = M2_PLUS_LEVELS.some((level) => capability.target_maturity === level || capability.levels?.[level]?.status === 'satisfied');
    if (claimsM2Plus && !catalog.m2_plus_capability_classes.includes(capability.capability_class)) fail(`disallowed M2 capability class: ${capability.capability_class}`);
  }
  if (!Array.isArray(candidates.graph_queries)) fail('missing graph queries');
  const dataFlows = candidates.graph_queries.filter((query) => query.edge_type === 'DATA_FLOWS');
  if (dataFlows.length > 1 || candidates.graph_queries.length !== (dataFlows.length === 1 ? 23 : 19)) fail('graph query cardinality must be exactly 19, or 23 with DATA_FLOWS');
  if (new Set(candidates.graph_queries.map((query) => query.query_id)).size !== candidates.graph_queries.length) fail('duplicate Task 4 graph query ID');
  for (const query of candidates.graph_queries) {
    own(query, ['query_id', 'edge_type', 'query', 'result_count'], 'Task 4 graph query');
    if (typeof query.query !== 'string' || !query.query.trim() || /baseline source review slot/i.test(query.query) || !Number.isInteger(query.result_count) || query.result_count < 0) fail(`invalid Task 4 graph query identity: ${query.query_id}`);
  }
  const semanticReview = candidates.extensions?.semantic_review;
  own(semanticReview, ['receipt_id', 'path', 'sha256'], 'Task 4 semantic review reference');
  nonEmptyString(semanticReview.receipt_id, 'Task 4 semantic review receipt ID');
  nonEmptyString(semanticReview.path, 'Task 4 semantic review path');
  if (!/^[0-9a-f]{64}$/.test(semanticReview.sha256)) fail('invalid Task 4 semantic review digest');
  if (dataFlows.length === 0) {
    const absence = candidates.extensions?.receipt_manifests?.data_flows_absence;
    own(absence, ['receipt_id', 'path', 'sha256'], 'DATA_FLOWS absence receipt');
    nonEmptyString(absence.receipt_id, 'DATA_FLOWS absence receipt ID');
    nonEmptyString(absence.path, 'DATA_FLOWS absence receipt path');
    if (!/^[0-9a-f]{64}$/.test(absence.sha256)) fail('invalid DATA_FLOWS absence receipt digest');
  }
  exactTestReceipt(testList, 'test-list', false);
  exactTestReceipt(testResults, 'test-results', true);
  const candidateByKey = new Map(candidates.candidates.map((candidate) => [candidate.candidate_key, candidate]));
  const contexts = candidates.extensions?.binding_contexts;
  if (!Array.isArray(contexts)) fail('missing Task 4 binding contexts');
  const contextById = new Map();
  for (const context of contexts) {
    own(context, ['binding_id', 'candidate_key', 'grouping', 'endpoint_declaration'], 'Task 4 binding context');
    if (contextById.has(context.binding_id)) fail(`duplicate Task 4 binding context: ${context.binding_id}`);
    own(context.grouping, ['domain', 'capability_class', 'state_path', 'mutation_owner', 'lifecycle_boundary'], `Task 4 binding tuple ${context.binding_id}`);
    contextById.set(context.binding_id, context);
  }
  const bindingById = new Map();
  for (const candidate of candidates.candidates) for (const binding of candidate.bindings ?? []) {
    own(binding, ['binding_id', 'capability_id', 'role', 'lifecycle_boundary'], 'Task 4 binding');
    if (!['primary', 'shared', 'carrier', 'support'].includes(binding.role)) fail(`invalid Task 4 binding role: ${binding.binding_id}`);
    const capability = inventoryCapabilities.get(binding.capability_id);
    const context = contextById.get(binding.binding_id);
    if (!capability || !context || context.candidate_key !== candidate.candidate_key || bindingById.has(binding.binding_id)) fail(`invalid Task 4 binding: ${binding.binding_id}`);
    nonEmptyString(binding.lifecycle_boundary, `Task 4 binding lifecycle boundary ${binding.binding_id}`);
    if (canonical(context.grouping) !== canonical(tuple(capability, binding.lifecycle_boundary))) fail(`Task 4 binding tuple mismatch: ${binding.binding_id}`);
    if (isModuleWide(candidate, context, candidates.candidates)) fail(`module-wide catch-all binding: ${binding.binding_id}`);
    bindingById.set(binding.binding_id, { ...binding, candidate_key: candidate.candidate_key });
  }
  for (const context of contexts) if (!bindingById.has(context.binding_id)) {
    fail(`orphan Task 4 binding context: ${context.binding_id}`);
  }
  const testsById = new Map(reconciliation.tests.map((test) => [test.test_id, test]));
  const mappingsByBinding = new Map();
  for (const mapping of reconciliation.mappings) {
    if (!bindingById.has(mapping.binding_id) || !candidateByKey.has(mapping.candidate_key) || !testsById.has(mapping.test_id)) fail(`invalid Task 4 test mapping: ${mapping.binding_id}`);
    const entries = mappingsByBinding.get(mapping.binding_id) ?? [];
    entries.push(mapping);
    mappingsByBinding.set(mapping.binding_id, entries);
  }
  const claimsByBinding = new Map();
  for (const claim of execution.claims) {
    const binding = bindingById.get(claim.binding_id);
    if (!binding || binding.capability_id !== claim.capability_id) fail(`invalid Task 4 evidence binding: ${claim.binding_id}`);
    const entries = claimsByBinding.get(claim.binding_id) ?? [];
    entries.push(claim);
    claimsByBinding.set(claim.binding_id, entries);
  }
  for (const capability of inventory.capabilities) {
    if (capability.levels?.M1?.status !== 'satisfied') continue;
    if (capability.levels?.M0?.status !== 'satisfied') fail(`M1 requires M0: ${capability.capability_id}`);
    if (!Array.isArray(capability.levels.M0.evidence_ids) || capability.levels.M0.evidence_ids.length === 0 || !Array.isArray(capability.levels.M1.evidence_ids) || capability.levels.M1.evidence_ids.length === 0) fail(`M1 requires admitted M0/M1 evidence: ${capability.capability_id}`);
    const determinism = capability.semantic_profile?.determinism;
    if (!determinism || determinism.status !== 'present' || !Array.isArray(determinism.evidence_ids) || determinism.evidence_ids.length === 0) fail(`M1 requires determinism contract: ${capability.capability_id}`);
    const bindings = [...bindingById.values()].filter((binding) => binding.capability_id === capability.capability_id);
    if (bindings.length === 0) fail(`M1 lacks mapped definitions: ${capability.capability_id}`);
    for (const binding of bindings) {
      const candidate = candidateByKey.get(binding.candidate_key);
      if (candidate.exclusion_id !== null || !candidate.symbol || candidate.line_end < candidate.line_start) fail(`M1 lacks baseline definition: ${capability.capability_id}`);
      const mappedTests = (mappingsByBinding.get(binding.binding_id) ?? []).filter((mapping) => {
        const test = testsById.get(mapping.test_id);
        return test.eligibility === 'exact_test' && test.discovered_receipt_id === testList.receipt_id;
      });
      if (mappedTests.length === 0) fail(`M1 lacks successful baseline-discovered exact test: ${capability.capability_id}`);
      const failureCases = candidate.failure_cases ?? [];
      const facets = new Set((claimsByBinding.get(binding.binding_id) ?? []).flatMap((claim) => claim.facets ?? []));
      for (const failureCase of failureCases) if (!facets.has(`failure_case:${failureCase}`)) fail(`M1 lacks applicable failure-case receipt: ${capability.capability_id}/${failureCase}`);
    }
  }
}

function buildTask4Inventory(catalog, inventoryPath, outPath) {
  const inventory = read(inventoryPath, 'inventory');
  runCli(['capability-inventory', '--run-id', inventory.run_id, '--input', inventoryPath]);
  const catalogIds = validateCatalog(catalog);
  const inventoryIds = new Set(inventory.capabilities.map((capability) => capability.capability_id));
  if (inventoryIds.size !== 61 || canonical([...inventoryIds].sort()) !== canonical([...catalogIds].sort())) fail('Task 4 inventory/catalog IDs drift');
  const domains = new Map(inventory.domains.map((domain) => [domain.domain, domain.capability_ids]));
  for (const domain of catalog.domains) if (canonical(domains.get(domain.domain)) !== canonical(domain.capability_ids)) fail(`Task 4 inventory/catalog domain drift: ${domain.domain}`);
  const artifact = `<!-- ${LEGACY_BASELINE_MACHINE_DATA_MARKER}:capability-audit-input:v1 -->\n\`\`\`json\n${JSON.stringify(inventory, null, 2)}\n\`\`\`\n<!-- /${LEGACY_BASELINE_MACHINE_DATA_MARKER} -->\n`;
  fs.writeFileSync(output(outPath), artifact, { encoding: 'utf8', flag: 'wx' });
}

try {
  const command = process.argv[2];
  if (command === '--help' || command === '-h') process.stdout.write(HELP);
  else if (command === 'build') {
    buildTask4Inventory(read(option('--catalog'), 'catalog'), option('--inventory'), option('--out'));
    process.stdout.write('task4_inventory_markdown=30 domains 61 capabilities pass\n');
  } else if (command === 'verify') {
    const catalog = read(option('--catalog'), 'catalog');
    validateCatalog(catalog);
    const required = ['--inventory', '--candidate-manifest', '--test-reconciliation', '--test-list-receipt', '--test-results-receipt', '--evidence-execution-manifest', '--source-baseline'];
    const complete = required.every((flag) => process.argv.includes(flag));
    if (!complete) process.stdout.write('catalog_domains=30 catalog_capabilities=61 pass\n');
    else {
      verifyTask4(catalog, read(option('--inventory'), 'inventory'), read(option('--candidate-manifest'), 'candidate manifest'), read(option('--test-reconciliation'), 'test reconciliation'), read(option('--test-list-receipt'), 'test-list receipt'), read(option('--test-results-receipt'), 'test-results receipt'), read(option('--evidence-execution-manifest'), 'evidence execution manifest'), option('--source-baseline'));
      process.stdout.write('task4_inventory_contract=61 concrete capabilities pass\n');
    }
  } else fail(`unknown command: ${command ?? '<none>'}`);
} catch (error) {
  process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
  process.exitCode = 1;
}
