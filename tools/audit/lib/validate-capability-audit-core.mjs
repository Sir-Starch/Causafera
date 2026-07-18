import fs from "fs";
import path from "path";
import crypto from "crypto";
import { execFileSync } from "child_process";

export const ROOT = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..', '..', '..');
export const AUDIT = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..');
export const ZERO_SHA = '0'.repeat(64);
export const ZERO_OID = '0'.repeat(40);

// LEGACY BASELINE 26026fb COMPATIBILITY
// These identifiers describe the immutable historical baseline commit 26026fb.
// They are not active Causafera package or protocol names.
// They must remain exactly as defined here to successfully validate historical receipts and fixtures.
export const LEGACY_BASELINE_MACHINE_DATA_MARKER = 'ontopolis-machine-data';
export const LEGACY_BASELINE_TYPES_PACKAGE = 'ontopolis-types';
export const LEGACY_BASELINE_COORDS_PATH = 'crates/ontopolis-types/tests/coords.rs';

const REPO_ROOT = ROOT;
const M2_PLUS_LEVELS = ['M2', 'M3', 'M4', 'M5'];
const M2_PLUS_CAPABILITY_CLASSES = ['state', 'mutation', 'bootstrap', 'resolution'];

const FORMAT_VALIDATORS = new Map([
  ['integer', (value) => Number.isInteger(value)],
  ['positive integer', (value) => Number.isInteger(value) && value > 0],
  ['positive-integer', (value) => Number.isInteger(value) && value > 0],
  ['run-id', (value) => typeof value === 'string' && value.length > 0 && !/[\/\s]/.test(value)],
  ['attempt-id', (value) => typeof value === 'string' && /^\d+$/.test(value)],
  ['repo-path', (value) => typeof value === 'string' && isRepoRelativePath(value)],
  ['git-sha1', (value) => typeof value === 'string' && /^[0-9a-f]{40}$/.test(value)],
  ['git-oid', (value) => typeof value === 'string' && /^[0-9a-f]{40}$/.test(value)],
  ['sha256', (value) => typeof value === 'string' && /^[0-9a-f]{64}$/.test(value)],
  ['timestamp', (value) => typeof value === 'string' && /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$/.test(value)],
  ['timestamp-utc', (value) => typeof value === 'string' && /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$/.test(value)],
]);


export const STABLE_MODES = [
  'capture',
  'prepare-intent',
  'execute-intent',
  'preflight',
  'schema',
  'inventory',
  'deep-audit',
  'bundle',
  'materialize-audit',
  'audit',
  'execplan',
  'sequencing',
  'selected-plan',
  'governance',
  'links',
  'plan-index',
  'scope',
  'apply-closure',
  'closure',
  'aggregate',
  'finalize',
  'build-attestation',
  'attest',
  'recover',
  'ingest-tool-response',
];

export const INGEST_OPERATIONS = [
  'index_status',
  'graph_schema',
  'query_graph',
  'trace_path',
  'lsp_symbols',
  'lsp_references',
  'schema_absence',
];

export const ADAPTER_EXTRAS = [
  'audit_checker',
  'benchmark_diagnostic',
  'cargo_metadata',
  'cargo_test_list',
  'confirmed_violation',
  'counterfactual_test',
  'cross_domain_test',
  'documentation_contract',
  'exact_test',
  'explanation_metric',
  'git_baseline',
  'git_diff_check',
  'git_mutation',
  'git_preflight',
  'git_scope',
  'git_tree',
  'graph_call_path',
  'independent_review',
  'observer_projection',
  'persistence_test',
  'pnpm_build',
  'pnpm_install',
  'pnpm_lint',
  'pnpm_typecheck',
  'production_composition',
  'replay_test',
  'representative_benchmark',
  'rust_ci',
];

export const SUPPORT_FILES = [
  'tools/audit/fixtures/fixture-blobs.json',
  'tools/audit/fixtures/fixture-candidate-manifest.valid.json',
  'tools/audit/fixtures/fixture-evidence-execution.valid.json',
  'tools/audit/fixtures/fixture-graph-receipts.valid.json',
  'tools/audit/fixtures/fixture-test-list.command-receipt.valid.json',
  'tools/audit/fixtures/fixture-test-reconciliation.valid.json',
  'tools/audit/fixtures/fixture-test-results.command-receipt.valid.json',
  'tools/audit/fixtures/fixture-trace-receipts.valid.json',
  'tools/audit/fixtures/fixture-writes-receipts.valid.json',
];

export const INVALID_FILES = [
  'tools/audit/fixtures/invalid-attestation-omission.json',
  'tools/audit/fixtures/invalid-backlog-bio-dependency.md',
  'tools/audit/fixtures/invalid-broken-link.md',
  'tools/audit/fixtures/invalid-closure-preimage.json',
  'tools/audit/fixtures/invalid-closure-wildcard.json',
  'tools/audit/fixtures/invalid-counter-only-m2.md',
  'tools/audit/fixtures/invalid-diagnostic-m5.md',
  'tools/audit/fixtures/invalid-disallowed-m2-class.json',
  'tools/audit/fixtures/invalid-digest-distance-m4.md',
  'tools/audit/fixtures/invalid-doctest-m1.json',
  'tools/audit/fixtures/invalid-duplicate-plan-index.md',
  'tools/audit/fixtures/invalid-evidence-free-m3.md',
  'tools/audit/fixtures/invalid-manifest-and-domain.md',
  'tools/audit/fixtures/invalid-missing-persistence.md',
  'tools/audit/fixtures/invalid-schema-drift.json',
  'tools/audit/fixtures/invalid-selected-plan-missing-negative-control.md',
  'tools/audit/fixtures/invalid-selection-and-cycle.md',
  'tools/audit/fixtures/invalid-semantic-profile-string.json',
  'tools/audit/fixtures/invalid-test-reconciliation-incomplete.json',
  'tools/audit/fixtures/invalid-wrapper-substitution.json',
  'tools/audit/fixtures/invalid-zero-actor-proof.md',
];
const MODE_REQUIREMENTS = new Map([
  ['capture', ['--adapter', '--run-id', '--receipt-id', '--receipt']],
  ['prepare-intent', ['--phase', '--attempt-id', '--source-baseline', '--adapter', '--run-id', '--receipt-id', '--intent', '--stdout', '--stderr', '--receipt']],
  ['execute-intent', ['--phase', '--attempt-id', '--source-baseline', '--adapter', '--run-id', '--receipt-id', '--intent', '--stdout', '--stderr', '--receipt']],
  ['execplan', ['--run-id', '--receipt-id']],
  ['scope', ['--run-id', '--allowlist']],
  ['apply-closure', ['--run-id', '--closure', '--authorization']],
  ['ingest-tool-response', ['--operation', '--response']],
]);


const CANONICAL_SCHEMA = 'tools/audit/schema-contracts.json';
const CANONICAL_ADAPTER = 'tools/audit/adapter-contracts.json';
const CANONICAL_FIXTURE_MANIFEST = 'tools/audit/fixture-manifest.json';
const CANONICAL_SCHEMA_DIR = 'tools/audit/schemas';
const ACTIVE_PLAN = 'plans/detailed-development-maturity-audit.md';
const EXECPLAN_HEADINGS = ['Goal','Context','Relevant invariants','Ontology domains affected','Causal carriers affected','Relevant documents','Current state','Proposed architecture','Primitive vs emergent review','Non-goals','Implementation stages','Verification','Benchmark plan','Determinism impact','Memory impact','Observer impact','Explanation impact','Persistence impact','Cross-domain effects','Risks','Documentation changes','TODO changes','Decision log','Progress'];
const AUDIT_DOMAINS = ['Space','Time','Matter','Energy','Pattern / Feature','Spatial geometry','Geography','Geology','Hydrology','Climate','Ecology','Biology','Physical access / perception','Cognition','Language','Mana','Causal resolution','Society','Economy','City infrastructure','Historical bootstrap','Epistemics','Practice','Isekai','Metaphysics','Simulation runtime','Explanation / analytics','Observer','UI','Optional LLM surface'];

function die(message, code = 1) {
  const err = new Error(message);
  err.exitCode = code;
  throw err;
}

export function canonical(value) {
  if (value === null || typeof value === 'boolean') return JSON.stringify(value);
  if (typeof value === 'number') return Number.isFinite(value) ? JSON.stringify(value) : die('non-finite number');
  if (typeof value === 'string') return JSON.stringify(value);
  if (Array.isArray(value)) return '[' + value.map(canonical).join(',') + ']';
  if (typeof value === 'object') {
    return '{' + Object.keys(value).sort().map((k) => JSON.stringify(k) + ':' + canonical(value[k])).join(',') + '}';
  }
  die('unsupported value type');
}

export function sha256Json(value) {
  return crypto.createHash('sha256').update(canonical(value)).digest('hex');
}

export function sameKeys(actual, expected) {
  const a = [...actual].sort();
  const e = [...expected].sort();
  return a.length === e.length && a.every((v, i) => v === e[i]);
}

export function assertSameKeys(obj, expected, label) {
  if (!sameKeys(Object.keys(obj), expected)) die(`invalid ${label}: expected keys ${expected.join(',')}`);
}

export function isPlainObject(value) {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

export function parseMachineData(text) {
  const regex = new RegExp(`<!--\\s*${LEGACY_BASELINE_MACHINE_DATA_MARKER}:[^>]+-->\\s*\`\`\`json\\n([\\s\\S]*?)\\n\`\`\`\\s*<!--\\s*\\/${LEGACY_BASELINE_MACHINE_DATA_MARKER}\\s*-->`);
  const match = text.match(regex);
  return match ? JSON.parse(match[1]) : null;
}
export function resolveRepoArtifact(candidate, { baseDir = REPO_ROOT, label = 'path', mustExist = true, regularFile = true } = {}) {
  if (typeof candidate !== 'string' || candidate.length === 0) die(`invalid ${label}: empty path`);
  const [pathPart] = candidate.split('#', 1);
  if (!pathPart) die(`invalid ${label}: empty path`);
  if (/^[a-zA-Z][a-zA-Z0-9+.-]*:/.test(pathPart)) die(`invalid ${label}: external URL not allowed`);
  if (path.isAbsolute(pathPart)) die(`invalid ${label}: absolute path not allowed`);
  if (!isRepoRelativePath(pathPart)) die(`invalid ${label}: unsafe path segments`);
  const base = path.resolve(baseDir);
  const relBase = path.relative(REPO_ROOT, base);
  if (base !== REPO_ROOT && (path.isAbsolute(relBase) || relBase.startsWith('..'))) die(`invalid ${label}: base outside repo root`);
  const abs = path.resolve(base, pathPart);
  const rel = path.relative(REPO_ROOT, abs);
  if (path.isAbsolute(rel) || rel.startsWith('..')) die(`invalid ${label}: path outside repo root`);
  const parts = rel.split(path.sep).filter(Boolean);
  let current = REPO_ROOT;
  for (const part of parts) {
    current = path.join(current, part);
    if (!fs.existsSync(current)) {
      if (mustExist) die(`missing ${label}`);
      return abs;
    }
    const stat = fs.lstatSync(current);
    if (stat.isSymbolicLink()) die(`invalid ${label}: symlink not allowed`);
  }
  if (mustExist && regularFile) {
    const stat = fs.statSync(abs);
    if (!stat.isFile()) die(`invalid ${label}: expected regular file`);
  }
  return abs;
}

function readArtifactResolved(filePath) {
  const text = fs.readFileSync(filePath, 'utf8');
  if (filePath.endsWith('.json')) return { text, value: JSON.parse(text) };
  const value = parseMachineData(text);
  if (value === null && /^\s*[{[]/.test(text)) die(`bare JSON Markdown artifact requires ${LEGACY_BASELINE_MACHINE_DATA_MARKER} envelope`);
  return { text, value: value ?? {}, machine: value !== null };
}

export function readText(filePath, options = {}) {
  return fs.readFileSync(resolveRepoArtifact(filePath, options), 'utf8');
}

export function readJson(filePath, options = {}) {
  return JSON.parse(readText(filePath, options));
}

export function readArtifact(filePath, options = {}) {
  return readArtifactResolved(resolveRepoArtifact(filePath, options));
}

export function isRepoRelativePath(value) {
  if (typeof value !== 'string' || value.length === 0) return false;
  if (path.isAbsolute(value) || value.includes('\\')) return false;
  return !value.split('/').some((part) => part === '' || part === '.' || part === '..');
}

export function assertSourceDefinitionArgv(argv, sourceBaseline) {
  if (!Array.isArray(argv) || argv.length !== 3 || argv[0] !== 'git' || argv[1] !== 'show') {
    die('invalid source-definition argv');
  }
  if (typeof sourceBaseline !== 'string' || !/^[0-9a-f]{40}$/.test(sourceBaseline)) {
    die('invalid source-definition baseline');
  }
  const target = argv[2];
  const prefix = `${sourceBaseline}:`;
  if (typeof target !== 'string' || !target.startsWith(prefix)) die('invalid source-definition target');
  const sourcePath = target.slice(prefix.length);
  if (sourcePath.normalize('NFC') !== sourcePath || !isRepoRelativePath(sourcePath)) {
    die('invalid source-definition path');
  }
  let commit;
  let objectType;
  try {
    commit = execFileSync('git', ['rev-parse', `${sourceBaseline}^{commit}`], { cwd: REPO_ROOT, encoding: 'utf8' }).trim();
    objectType = execFileSync('git', ['cat-file', '-t', `${sourceBaseline}:${sourcePath}`], { cwd: REPO_ROOT, encoding: 'utf8' }).trim();
  } catch {
    die('invalid source-definition baseline source blob');
  }
  if (commit !== sourceBaseline || objectType !== 'blob') die('invalid source-definition baseline source blob');
  return sourcePath;
}

export function pathValues(root, expr) {
  if (expr === 'root') return [root];
  const parts = expr.split('.');
  let values = [root];
  for (const part of parts) {
    const next = [];
    if (part === '*') {
      for (const value of values) {
        if (!isPlainObject(value)) continue;
        next.push(...Object.values(value));
      }
      values = next;
      continue;
    }
    const isArray = part.endsWith('[]');
    const key = isArray ? part.slice(0, -2) : part;
    for (const value of values) {
      if (!isPlainObject(value) || !(key in value)) continue;
      const child = value[key];
      if (isArray) {
        if (Array.isArray(child)) next.push(...child);
      } else {
        next.push(child);
      }
    }
    values = next;
  }
  return values;
}

function hasDeclaredEmptyArray(root, expr) {
  if (!expr.endsWith('[]')) return false;
  const arrays = pathValues(root, expr.slice(0, -2));
  return arrays.some((value) => Array.isArray(value) && value.length === 0);
}

function validateFormatValue(value, formatName, label) {
  const validator = FORMAT_VALIDATORS.get(formatName);
  if (!validator) die(`unknown format: ${formatName}`);
  if (!validator(value)) die(`invalid ${label}: expected ${formatName}`);
}

function implicitFormatForField(fieldName, value) {
  if (value === null || value === undefined) return null;
  if (fieldName === 'schema_version' || fieldName === 'line_start' || fieldName === 'line_end' || fieldName === 'definition_character' || fieldName === 'result_count' || fieldName === 'sample_count' || fieldName === 'exit_code' || fieldName === 'total' || fieldName === 'returned' || fieldName === 'limit' || fieldName === 'duration') return 'integer';
  if (fieldName === 'attempt_id') return 'attempt-id';
  if (fieldName === 'run_id') return 'run-id';
  if (fieldName === 'timestamp_utc') return 'timestamp-utc';
  if (fieldName === 'path' || fieldName.endsWith('_path')) return 'repo-path';
  if (fieldName === 'source_baseline_sha' || (fieldName.endsWith('_sha') && !fieldName.endsWith('_sha256'))) return 'git-sha1';
  if (fieldName === 'blob_oid' || fieldName === 'tree_oid' || fieldName.endsWith('_oid')) return 'git-oid';
  if (fieldName.endsWith('_sha256')) return 'sha256';
  if (fieldName.endsWith('_oids') && Array.isArray(value)) return 'git-oid[]';
  return null;
}

function validateImplicitFieldShape(fieldName, value, label) {
  const formatName = implicitFormatForField(fieldName, value);
  if (!formatName) return;
  if (formatName === 'git-oid[]') {
    for (const item of value) validateFormatValue(item, 'git-oid', `${label}[]`);
    return;
  }
  validateFormatValue(value, formatName, label);
}

function validateRootExact(obj, expectedFields, label) {
  assertSameKeys(obj, expectedFields, label);
}

function validateArrayItems(arr, fields, label) {
  if (!Array.isArray(arr)) die(`invalid ${label}: expected array`);
  for (const item of arr) {
    if (!isPlainObject(item)) die(`invalid ${label}: expected object`);
    assertSameKeys(item, fields, label);
  }
}

const ARRAY_FIELDS = new Set([
  'allowlist', 'argv', 'bounded_inputs', 'candidate_keys', 'capabilities', 'capability_ids',
  'carriers', 'checks', 'claims', 'commands', 'dependencies', 'domains', 'entries', 'evidence_ids',
  'facets', 'failure_cases', 'files', 'gaps', 'graph_queries', 'graph_receipt_ids', 'input_hashes',
  'lsp_receipt_ids', 'mappings', 'metrics', 'negative_controls', 'nested_records', 'raw_byte_hashes',
  'required_facets', 'reviewed_artifact_oids', 'targets', 'tests', 'todo_bindings', 'tree_files',
  'validation_commands', 'validation_envelope',
]);

function validateDeclaredTypes(value, label) {
  if (Array.isArray(value)) {
    value.forEach((item, index) => validateDeclaredTypes(item, `${label}[${index}]`));
    return;
  }
  if (!isPlainObject(value)) return;
  for (const [field, child] of Object.entries(value)) {
    if (ARRAY_FIELDS.has(field) && !Array.isArray(child)) die(`invalid ${label}.${field}: expected array`);
    if ((field === 'extensions' || field === 'environment' || field === 'tool_versions' || field === 'workload' || field === 'levels' || field === 'semantic_profile') && !isPlainObject(child)) die(`invalid ${label}.${field}: expected object`);
    validateDeclaredTypes(child, `${label}.${field}`);
  }
}

function validateNested(spec, obj, label) {
  if (!spec || !Array.isArray(spec.root_fields) || !Array.isArray(spec.nested_records)) die(`invalid ${label}: invalid schema spec`);
  validateRootExact(obj, spec.root_fields, label);
  validateDeclaredTypes(obj, label);
  const declaredPaths = new Set(['root', ...spec.nested_records.map((record) => record.path)]);
  for (const pathExpr of Object.keys(spec.required_fields || {})) {
    if (!declaredPaths.has(pathExpr)) die(`invalid ${label}: undeclared required path ${pathExpr}`);
  }
  for (const [pathExpr, fields] of Object.entries(spec.required_fields || {})) {
    const nullable = new Set(spec.nullable_fields?.[pathExpr] || []);
    if (pathExpr === 'root') {
      for (const field of fields) {
        if (!Object.prototype.hasOwnProperty.call(obj, field)) die(`invalid ${label}: missing required field ${field}`);
        if (obj[field] === null && !nullable.has(field)) die(`invalid ${label}: null not allowed for ${field}`);
        validateImplicitFieldShape(field, obj[field], `${label}.${field}`);
      }
      continue;
    }
    const values = pathValues(obj, pathExpr);
    if (values.length === 0 && !hasDeclaredEmptyArray(obj, pathExpr)) die(`invalid ${label}.${pathExpr}: missing required path`);
    for (const value of values) {
      if (!isPlainObject(value)) die(`invalid ${label}.${pathExpr}: expected object`);
      for (const field of fields) {
        if (!Object.prototype.hasOwnProperty.call(value, field)) die(`invalid ${label}.${pathExpr}: missing required field ${field}`);
        if (value[field] === null && !nullable.has(field)) die(`invalid ${label}.${pathExpr}: null not allowed for ${field}`);
        validateImplicitFieldShape(field, value[field], `${label}.${pathExpr}.${field}`);
      }
    }
  }
  for (const record of spec.nested_records) {
    if (record.kind === 'array') {
      if (!record.path.endsWith('[]')) die(`invalid ${label}: array record path must end in []`);
      for (const container of pathValues(obj, record.path.slice(0, -2))) if (!Array.isArray(container)) die(`invalid ${label}.${record.path}: expected array`);
    } else if (record.kind !== 'object') {
      die(`invalid ${label}: unknown nested record kind ${record.kind}`);
    }
    const values = pathValues(obj, record.path);
    if (values.length === 0 && !hasDeclaredEmptyArray(obj, record.path)) die(`invalid ${label}.${record.path}: missing required path`);
    for (const value of values) {
      if (!isPlainObject(value)) die(`invalid ${label}.${record.path}: expected object`);
      assertSameKeys(value, record.fields, `${label}.${record.path}`);
      for (const field of record.fields) validateImplicitFieldShape(field, value[field], `${label}.${record.path}.${field}`);
    }
  }
  for (const [pathExpr, fields] of Object.entries(spec.nullable_fields || {})) {
    if (pathExpr === 'root') {
      for (const [field, value] of Object.entries(obj)) {
        if (value === null && !fields.includes(field)) die(`invalid ${label}: null not allowed for ${field}`);
      }
      continue;
    }
    const values = pathValues(obj, pathExpr);
    for (const value of values) {
      if (!isPlainObject(value)) continue;
      for (const [field, child] of Object.entries(value)) {
        if (child === null && !fields.includes(field)) die(`invalid ${label}.${pathExpr}: null not allowed for ${field}`);
      }
    }
  }
  for (const [pathExpr, allowed] of Object.entries(spec.enums || {})) {
    for (const value of pathValues(obj, pathExpr)) {
      if (!allowed.includes(value)) die(`invalid ${label}.${pathExpr}: invalid enum value`);
    }
  }
  for (const [pathExpr, formatName] of Object.entries(spec.formats || {})) {
    if (!FORMAT_VALIDATORS.has(formatName)) die(`unknown format: ${formatName}`);
    for (const value of pathValues(obj, pathExpr)) {
      if (value === null) continue;
      validateFormatValue(value, formatName, `${label}.${pathExpr}`);
    }
  }
  if ('receipt_sha256' in obj) {
    const expected = sha256Json(Object.fromEntries(Object.entries(obj).filter(([k]) => k !== 'receipt_sha256')));
    if (obj.receipt_sha256 !== expected) die('invalid receipt_sha256');
  }
}

export function loadContracts(schemaPath = CANONICAL_SCHEMA, adapterPath = CANONICAL_ADAPTER) {
  const schemaContracts = readJson(schemaPath);
  const adapterContracts = readJson(adapterPath);
  if (schemaContracts.schema_version !== 1 || adapterContracts.schema_version !== 1) die('invalid schema_version');
  const schemas = new Map(schemaContracts.schemas.map((s) => [s.name, s]));
  const adapters = new Map(adapterContracts.adapters.map((a) => [a.name, a]));
  return { schemaContracts, adapterContracts, schemas, adapters };
}

function validateSchemaRecord(record) {
  const fields = ['additional_properties', 'enums', 'formats', 'name', 'nested_records', 'nullable_fields', 'required_fields', 'root_fields'];
  if (record.constraints !== undefined) fields.push('constraints');
  assertSameKeys(record, fields, `schema record ${record.name}`);
  if (record.additional_properties !== false) die(`invalid schema record ${record.name}: additional_properties`);
  if (record.constraints !== undefined && !isPlainObject(record.constraints)) die(`invalid schema record ${record.name}: constraints`);
  if (!Array.isArray(record.root_fields) || !Array.isArray(record.nested_records)) die(`invalid schema record ${record.name}`);
  const declaredPaths = new Set(['root', ...record.nested_records.map((nested) => nested.path)]);
  for (const [pathExpr, fields] of Object.entries(record.required_fields || {})) {
    if (!declaredPaths.has(pathExpr)) die(`invalid schema record ${record.name}: undeclared required path ${pathExpr}`);
    if (!Array.isArray(fields)) die(`invalid schema record ${record.name}: invalid required_fields ${pathExpr}`);
  }
  for (const pathExpr of Object.keys(record.nullable_fields || {})) {
    const nullableBase = pathExpr.includes('.') ? pathExpr.slice(0, pathExpr.lastIndexOf('.')) : pathExpr;
    if (!declaredPaths.has(pathExpr) && !declaredPaths.has(nullableBase)) die(`invalid schema record ${record.name}: undeclared nullable path ${pathExpr}`);
  }
  for (const formatName of Object.values(record.formats || {})) {
    if (!FORMAT_VALIDATORS.has(formatName)) die(`unknown format: ${formatName}`);
  }
}

function validateAdapterRecord(record, schemas) {
  assertSameKeys(record, ['allowed_levels', 'argv_grammar', 'evidence_class', 'name', 'projection_schema', 'replay_policy', 'required_facets', 'state_anchor_policy'], `adapter record ${record.name}`);
  if (!Array.isArray(record.allowed_levels) || !Array.isArray(record.required_facets)) die(`invalid adapter record ${record.name}`);
  const levelPrefix = ['M0', 'M1', 'M2', 'M3', 'M4', 'M5'].slice(0, record.allowed_levels.length);
  if (canonical(record.allowed_levels) !== canonical(levelPrefix)) die(`invalid adapter record ${record.name}: allowed_levels`);
  if (record.argv_grammar !== `capture --adapter ${record.name}`) die(`invalid adapter record ${record.name}: argv_grammar`);
  if (!schemas.has(record.projection_schema)) die(`invalid adapter record ${record.name}: projection_schema`);
  if (record.replay_policy !== 'deterministic-projection') die(`unknown replay policy: ${record.replay_policy}`);
  if (!['baseline', 'diff', 'graph', 'receipt', 'scope', 'tree'].includes(record.state_anchor_policy)) die(`unknown state anchor policy: ${record.state_anchor_policy}`);
  if (!['benchmark_diagnostic', 'confirmed_violation', 'documentation_contract', 'executed_test', 'explanation_metric', 'observer_projection', 'operational_check', 'persistence_roundtrip', 'production_call_path', 'regression_check', 'representative_benchmark', 'source_definition'].includes(record.evidence_class)) die(`unknown evidence class: ${record.evidence_class}`);
  if (record.required_facets.some((facet) => typeof facet !== 'string' || facet.length === 0) || new Set(record.required_facets).size !== record.required_facets.length) die(`invalid adapter record ${record.name}: required_facets`);
}

export function validateContractFiles(schemaPath = CANONICAL_SCHEMA, adapterPath = CANONICAL_ADAPTER) {
  const schemaContracts = readJson(schemaPath);
  const adapterContracts = readJson(adapterPath);
  if (canonical(schemaContracts) !== canonical(readJson(CANONICAL_SCHEMA))) die('contract drift: schema-contracts');
  if (canonical(adapterContracts) !== canonical(readJson(CANONICAL_ADAPTER))) die('contract drift: adapter-contracts');
  const schemas = new Map(schemaContracts.schemas.map((s) => [s.name, s]));
  const adapters = new Map(adapterContracts.adapters.map((a) => [a.name, a]));
  if (schemaContracts.schemas.length !== 24) die('invalid schema count');
  if (adapterContracts.adapters.length !== 29) die('invalid adapter count');
  const schemaNames = schemaContracts.schemas.map((s) => s.name);
  const adapterNames = adapterContracts.adapters.map((a) => a.name);
  if (new Set(schemaNames).size !== 24 || new Set(adapterNames).size !== 29) die('duplicate contract names');
  if (schemaNames.join(',') !== [...schemaNames].sort().join(',')) die('schema names drift');
  if (adapterNames.join(',') !== [...adapterNames].sort().join(',')) die('adapter names drift');
  for (const s of schemaContracts.schemas) validateSchemaRecord(s);
  for (const a of adapterContracts.adapters) validateAdapterRecord(a, schemas);
  const expectedAdapterNames = ['source_definition', ...ADAPTER_EXTRAS].sort();
  if (canonical(adapterNames) !== canonical(expectedAdapterNames)) die('adapter/code drift');
  try {
    validateAdapterRecord({ ...adapterContracts.adapters[0], replay_policy: 'unknown-policy' }, schemas);
    die('invalid adapter policy unexpectedly accepted');
  } catch (error) {
    if (!String(error?.message ?? error).includes('unknown replay policy')) throw error;
  }
  const schemaDir = resolveRepoArtifact(CANONICAL_SCHEMA_DIR, { label: 'schema directory', regularFile: false });
  const schemaFiles = fs.readdirSync(schemaDir).filter((name) => name.endsWith('.json')).sort();
  const expectedSchemaFiles = schemaNames.map((name) => `${name}.json`).sort();
  if (canonical(schemaFiles) !== canonical(expectedSchemaFiles)) die('schema/code drift: schema file set');
  for (const schema of schemaContracts.schemas) {
    const schemaFile = readJson(`${CANONICAL_SCHEMA_DIR}/${schema.name}.json`);
    if (canonical(schemaFile) !== canonical(schema)) die(`schema/code drift: ${schema.name}`);
  }
  return { schemaContracts, adapterContracts, schemas, adapters };
}

export function expectedFixturePaths() {
  const { schemas } = loadContracts();
  const canonicalSchemas = [...schemas.keys()].sort();
  const schemaExamples = canonicalSchemas.map((name) => `tools/audit/examples/${name}.valid.json`);
  const adapterExamples = ADAPTER_EXTRAS.map((name) => `tools/audit/examples/command-receipt.${name}.valid.json`);
  return [...schemaExamples, ...adapterExamples, ...SUPPORT_FILES, ...INVALID_FILES].sort();
}

export function validateFixtureManifest() {
  const manifest = readJson(CANONICAL_FIXTURE_MANIFEST);
  const expected = expectedFixturePaths();
  if (manifest.schema_version !== 1) die('invalid fixture manifest schema_version');
  if (!Array.isArray(manifest.files)) die('invalid fixture manifest: files');
  if (JSON.stringify([...new Set(manifest.files)].sort()) !== JSON.stringify(expected)) die('fixture manifest drift');
  if (manifest.files.length !== expected.length) die('fixture manifest drift');
  return manifest;
}

export function validateSchemaExample(name, obj, spec) {
  validateNested(spec, obj, name);
  if (name === 'fixture-manifest') {
    const manifest = validateFixtureManifest();
    if (JSON.stringify(obj.files) !== JSON.stringify(manifest.files)) die('fixture manifest drift');
  }
  return true;
}

function requireObject(value, label) {
  if (!isPlainObject(value)) die(`invalid ${label}: expected object`);
}

function requireString(value, label) {
  if (typeof value !== 'string' || !value) die(`invalid ${label}`);
}

function requireStringArray(value, label) {
  if (!Array.isArray(value) || value.some((v) => typeof v !== 'string' || v.length === 0)) die(`invalid ${label}: expected string array`);
}

function requireNonEmptyArray(value, label) {
  if (!Array.isArray(value) || value.length === 0) die(`invalid ${label}: expected non-empty array`);
}

function hasPathMarker(text, needle) {
  return text.includes(needle);
}

function capabilityById(obj, id) {
  return obj.capabilities?.find((cap) => cap.capability_id === id) ?? null;
}

function capabilityHasProductionEvidence(capability, evidence) {
  const ids = new Set(capability.levels?.M2?.evidence_ids ?? []);
  return evidence.some((item) => ids.has(item.evidence_id) && !['source_definition', 'confirmed_violation'].includes(item.adapter));
}

function validateM2PlusCapabilityClass(capability, schema) {
  const declared = schema?.constraints?.m2_plus_capability_classes;
  if (!Array.isArray(declared) || canonical([...declared].sort()) !== canonical([...M2_PLUS_CAPABILITY_CLASSES].sort())) {
    die('invalid M2 capability-class contract');
  }
  const claimsM2Plus = M2_PLUS_LEVELS.includes(capability.target_maturity)
    || M2_PLUS_LEVELS.some((level) => capability.levels?.[level]?.status === 'satisfied');
  if (claimsM2Plus && !declared.includes(capability.capability_class)) {
    die(`disallowed M2 capability class: ${capability.capability_class}`);
  }
}

const MATURITY_LEVELS = ['M0', 'M1', 'M2', 'M3', 'M4', 'M5'];

function validateSemanticProfile(capability) {
  const profile = capability.semantic_profile;
  requireObject(profile, `capability ${capability.capability_id} semantic_profile`);
  for (const field of ['production_reachability', 'target']) {
    requireString(profile[field], `capability ${capability.capability_id} semantic_profile.${field}`);
  }
  for (const field of ['gaps', 'dependencies']) {
    requireStringArray(profile[field], `capability ${capability.capability_id} semantic_profile.${field}`);
  }
  for (const field of ['resolution', 'persistence', 'provenance', 'observer', 'explanation', 'determinism', 'performance', 'negative_control']) {
    const dimension = profile[field];
    requireObject(dimension, `capability ${capability.capability_id} semantic_profile.${field}`);
    requireString(dimension.status, `capability ${capability.capability_id} semantic_profile.${field}.status`);
    requireStringArray(dimension.evidence_ids, `capability ${capability.capability_id} semantic_profile.${field}.evidence_ids`);
    requireString(dimension.rationale, `capability ${capability.capability_id} semantic_profile.${field}.rationale`);
  }
}

function validateCapabilitySemantics(obj, { derived }) {
  const { adapters } = loadContracts();
  requireNonEmptyArray(obj.capabilities, 'capabilities');
  requireNonEmptyArray(obj.domains, 'domains');
  requireNonEmptyArray(obj.evidence, 'evidence');
  requireNonEmptyArray(obj.todo_bindings, 'todo_bindings');
  unique(obj.capabilities.map((capability) => capability.capability_id), 'capability IDs');
  unique(obj.domains.map((domain) => domain.domain), 'domain names');
  unique(obj.evidence.map((item) => item.evidence_id), 'evidence IDs');
  unique(obj.todo_bindings.map((binding) => binding.todo_id), 'TODO binding IDs');
  const capabilities = new Map(obj.capabilities.map((capability) => [capability.capability_id, capability]));
  const evidence = new Map(obj.evidence.map((item) => [item.evidence_id, item]));
  const referencedCapabilities = new Set();
  for (const domain of obj.domains) {
    requireString(domain.domain, 'domains[].domain');
    requireStringArray(domain.capability_ids, `domain ${domain.domain} capability_ids`);
    if (domain.capability_ids.length === 0) die(`domain ${domain.domain} must reference at least one capability`);
    unique(domain.capability_ids, `domain ${domain.domain} capability_ids`);
    for (const id of domain.capability_ids) {
      const capability = capabilities.get(id);
      if (!capability || capability.domain !== domain.domain) die('manifest/domain mismatch');
      if (referencedCapabilities.has(id)) die(`capability appears in multiple domains: ${id}`);
      referencedCapabilities.add(id);
    }
  }
  for (const capability of obj.capabilities) {
    requireString(capability.capability_id, 'capabilities[].capability_id');
    requireString(capability.domain, `capability ${capability.capability_id} domain`);
    requireStringArray(capability.carriers, `capability ${capability.capability_id} carriers`);
    if (capability.carriers.length === 0) die(`capability ${capability.capability_id} carriers must be non-empty`);
    unique(capability.carriers, `capability ${capability.capability_id} carriers`);
    validateSemanticProfile(capability);
    requireObject(capability.representative_workload, `capability ${capability.capability_id} representative_workload`);
    for (const field of ['bounded_inputs', 'metrics', 'validation_envelope']) {
      requireStringArray(capability.representative_workload[field], `capability ${capability.capability_id} representative_workload.${field}`);
      if (capability.representative_workload.status === 'present' && capability.representative_workload[field].length === 0) die(`capability ${capability.capability_id} present workload requires ${field}`);
    }
    if (!referencedCapabilities.has(capability.capability_id)) die(`capability missing from domain manifest: ${capability.capability_id}`);
    requireObject(capability.levels, `capability ${capability.capability_id} levels`);
    assertSameKeys(capability.levels, MATURITY_LEVELS, `capability ${capability.capability_id} levels`);
    let encounteredUnsatisfied = false;
    const satisfiedEvidence = [];
    let highestSatisfied = null;
    for (const level of MATURITY_LEVELS) {
      const record = capability.levels[level];
      requireObject(record, `capability ${capability.capability_id} ${level}`);
      requireStringArray(record.evidence_ids, `capability ${capability.capability_id} ${level} evidence_ids`);
      unique(record.evidence_ids, `capability ${capability.capability_id} ${level} evidence_ids`);
      for (const id of record.evidence_ids) if (!evidence.has(id)) die(`capability ${capability.capability_id} references unknown evidence: ${id}`);
      for (const id of record.evidence_ids) {
        const adapter = adapters.get(evidence.get(id).adapter);
        if (!adapter) die(`evidence ${id} has unknown adapter: ${evidence.get(id).adapter}`);
        if (record.status === 'satisfied' && !adapter.allowed_levels.includes(level)) die(`evidence ${id} adapter ${adapter.name} cannot satisfy ${level}`);
      }
      if (record.status === 'satisfied') {
        if (encounteredUnsatisfied) die(`capability ${capability.capability_id} has non-contiguous satisfied levels`);
        if (record.evidence_ids.length === 0) die(`capability ${capability.capability_id} ${level} satisfied without evidence`);
        highestSatisfied = level;
        satisfiedEvidence.push(...record.evidence_ids);
      } else {
        encounteredUnsatisfied = true;
        if (record.status === 'missing' && record.evidence_ids.length !== 0) die(`capability ${capability.capability_id} ${level} missing with evidence`);
        if (record.status === 'not_applicable' && record.evidence_ids.length !== 0) die(`capability ${capability.capability_id} ${level} not_applicable with evidence`);
      }
    }
    if (derived) {
      const expectedMaturity = highestSatisfied ?? 'deferred';
      if (capability.derived_maturity !== expectedMaturity) die(`capability ${capability.capability_id} derived_maturity mismatch: expected ${expectedMaturity}`);
      const expectedEvidence = [...new Set(satisfiedEvidence)].sort();
      if (canonical([...capability.derived_level_evidence_ids].sort()) !== canonical(expectedEvidence)) die(`capability ${capability.capability_id} derived evidence mismatch`);
    }
  }
  for (const item of obj.evidence) {
    if (item.run_id !== obj.run_id) die(`evidence ${item.evidence_id} run_id mismatch`);
    if (item.source_baseline_sha !== obj.source_baseline_sha) die(`evidence ${item.evidence_id} source_baseline_sha mismatch`);
    requireStringArray(item.facets, `evidence ${item.evidence_id} facets`);
    const adapter = adapters.get(item.adapter);
    if (!adapter) die(`evidence ${item.evidence_id} has unknown adapter: ${item.adapter}`);
    for (const required of adapter.required_facets) if (!item.facets.some((facet) => facet === required || facet.startsWith(`${required}:`))) die(`evidence ${item.evidence_id} missing facet: ${required}`);
    requireObject(item.workload, `evidence ${item.evidence_id} workload`);
    requireStringArray(item.workload.bounded_inputs, `evidence ${item.evidence_id} workload bounded_inputs`);
  }
  if (obj.source_reconciliation.status === 'present' && obj.source_reconciliation.evidence_ids.length === 0) die('present source reconciliation requires evidence');
  for (const id of obj.source_reconciliation.evidence_ids) if (!evidence.has(id)) die(`source reconciliation references unknown evidence: ${id}`);
}

function validateCapabilityAuditInputSemantic(obj, text) {
  requireObject(obj, 'capability-audit-input');
  const schema = loadContracts().schemas.get('capability-audit-input');
  validateNested(schema, obj, 'capability-audit-input');
  if (!Array.isArray(obj.capabilities)) die('invalid capability-audit-input: capabilities');
  if (!Array.isArray(obj.domains)) die('invalid capability-audit-input: domains');
  if (!Array.isArray(obj.evidence)) die('invalid capability-audit-input: evidence');
  if (!Array.isArray(obj.todo_bindings)) die('invalid capability-audit-input: todo_bindings');
  if (obj.schema_version !== 1) die('invalid capability-audit-input: schema_version');
  validateCapabilitySemantics(obj, { derived: false });
  const caps = new Map(obj.capabilities.map((cap) => [cap.capability_id, cap]));
  for (const domain of obj.domains) {
    requireObject(domain, 'domains[]');
    requireString(domain.domain, 'domains[].domain');
    requireStringArray(domain.capability_ids, 'domains[].capability_ids');
    for (const id of domain.capability_ids) {
      const cap = caps.get(id);
      if (!cap || cap.domain !== domain.domain) die('manifest/domain mismatch');
    }
  }
  for (const cap of obj.capabilities) {
    requireObject(cap, 'capabilities[]');
    validateM2PlusCapabilityClass(cap, schema);
    if (cap.target_maturity === 'M2' && !capabilityHasProductionEvidence(cap, obj.evidence)) {
      die('counterfactual-only evidence cannot satisfy M2');
    }
    if (cap.target_maturity === 'M3' && (!Array.isArray(obj.evidence) || obj.evidence.length === 0)) {
      die('missing evidence for M3');
    }
    if (cap.target_maturity === 'M4') {
      const workload = cap.representative_workload ?? {};
      if (hasPathMarker(workload.name ?? '', 'digest-distance') || hasPathMarker(JSON.stringify(workload), 'digest-distance')) {
        die('digest-byte distance is not M4 evidence');
      }
    }
    if (cap.target_maturity === 'M5') {
      const workload = cap.representative_workload ?? {};
      if (cap.capability_class === 'validation' || workload.name === 'diagnostic' || hasPathMarker(JSON.stringify(workload), 'diagnostic-only')) {
        die('diagnostic-only benchmark cannot satisfy M5');
      }
    }
  }
  if (!Array.isArray(obj.evidence) || obj.evidence.length === 0) {
    for (const cap of obj.capabilities) {
      if (cap.target_maturity === 'M3') die('missing evidence for M3');
    }
  }
  if (hasPathMarker(text, 'benchmark-diagnostic')) die('diagnostic-only benchmark cannot satisfy M5');
  return true;
}

function validateCapabilityAuditSemantic(obj, text = '') {
  requireObject(obj, 'capability-audit');
  const schema = loadContracts().schemas.get('capability-audit');
  validateNested(schema, obj, 'capability-audit');
  if (obj.schema_version !== 1) die('invalid capability-audit: schema_version');
  if (!Array.isArray(obj.capabilities) || !Array.isArray(obj.domains) || !Array.isArray(obj.evidence) || !Array.isArray(obj.todo_bindings)) {
    die('invalid capability-audit');
  }
  validateCapabilitySemantics(obj, { derived: true });
  const caps = new Map(obj.capabilities.map((cap) => [cap.capability_id, cap]));
  for (const domain of obj.domains) {
    for (const id of domain.capability_ids ?? []) {
      const cap = caps.get(id);
      if (!cap || cap.domain !== domain.domain) die('manifest/domain mismatch');
    }
  }
  for (const cap of obj.capabilities) {
    validateM2PlusCapabilityClass(cap, schema);
    if (cap.target_maturity === 'M2' && !capabilityHasProductionEvidence(cap, obj.evidence)) die('counterfactual-only evidence cannot satisfy M2');
    if (cap.target_maturity === 'M3' && (obj.evidence?.length ?? 0) === 0) die('missing evidence for M3');
    if (cap.target_maturity === 'M4') {
      const workload = cap.representative_workload ?? {};
      const evidenceText = JSON.stringify(obj.evidence ?? []);
      if (String(workload.name ?? '').includes('digest-distance') || evidenceText.includes('digest-distance')) die('digest-byte distance is not M4 evidence');
    }
    if (cap.target_maturity === 'M5') {
      const workload = cap.representative_workload ?? {};
      const evidenceText = JSON.stringify(obj.evidence ?? []);
      if (String(workload.name ?? '').includes('diagnostic') || evidenceText.includes('benchmark-diagnostic') || evidenceText.includes('diagnostic-only') || evidenceText.includes('benchmark_diagnostic')) die('diagnostic-only benchmark cannot satisfy M5');
    }
  }
  unsupportedM5(obj);
  return true;
}

function validateSourceBlobs(obj) {
  validateNested({ root_fields: ['schema_version','run_id','source_baseline_sha','tree_oid','entries','receipt_sha256'], nested_records: [{ path: 'entries[]', kind: 'array', fields: ['logical_path','tracked_path','tracked_blob_oid','byte_sha256','kind'] }] }, obj, 'source-blobs');
  return true;
}

function validateCommandReceiptLike(obj, expectedAdapter) {
  validateNested(loadContracts().schemas.get('command-receipt'), obj, 'command-receipt');
  if (obj.adapter !== expectedAdapter) die(`invalid command-receipt adapter: ${expectedAdapter}`);
  if (obj.deterministic_projection?.adapter !== expectedAdapter) die('invalid command-receipt deterministic_projection adapter');
  commandReceipt(obj, expectedAdapter, { runtimeSidecars: false });
  return true;
}

function validateCommandIntent(obj) {
  validateNested(loadContracts().schemas.get('command-intent'), obj, 'command-intent');
  return true;
}

function validateReceiptAttemptPhase(obj, label) {
  if (obj.phase === 'collection') {
    if (obj.attempt_id !== null) die(`${label} collection attempt_id must be null`);
    return;
  }
  if (typeof obj.attempt_id !== 'string' || !/^(?:000[1-9]|00[1-9]\d|0[1-9]\d{2}|[1-9]\d{3,})$/.test(obj.attempt_id)) {
    die(`${label} attempt phase requires attempt_id 0001+`);
  }
}

function validateGraphReceipt(obj) {
  validateNested(loadContracts().schemas.get('graph-receipt'), obj, 'graph-receipt');
  validateReceiptAttemptPhase(obj, 'graph-receipt');
  return true;
}

function validateLspReceipt(obj) {
  validateNested(loadContracts().schemas.get('lsp-receipt'), obj, 'lsp-receipt');
  validateReceiptAttemptPhase(obj, 'lsp-receipt');
  return true;
}

function validateCandidateManifest(obj) {
  validateNested(loadContracts().schemas.get('candidate-manifest'), obj, 'candidate-manifest');
  return true;
}

function validateEvidenceExecutionManifest(obj) {
  validateNested(loadContracts().schemas.get('evidence-execution-manifest'), obj, 'evidence-execution-manifest');
  return true;
}

function validateTestReconciliation(obj) {
  validateNested(loadContracts().schemas.get('test-reconciliation'), obj, 'test-reconciliation');
  if (!Array.isArray(obj.mappings) || obj.mappings.length === 0) die('incomplete test reconciliation');
  const mappingIds = new Set(obj.mappings.map((m) => m.test_id));
  const tests = new Map((obj.tests ?? []).map((test) => [test.test_id, test]));
  for (const mapping of obj.mappings) if (tests.get(mapping.test_id)?.eligibility === 'discovery_only') die('doctest cannot satisfy M1');
  if (mappingIds.size === 0) die('incomplete test reconciliation');
  return true;
}

function validateClosureManifest(obj) {
  validateNested(loadContracts().schemas.get('closure-manifest'), obj, 'closure-manifest');
  for (const entry of obj.entries ?? []) {
    if (String(entry.path).includes('*') || String(entry.replacement_path).includes('*')) die('wildcard path not allowed');
    if (!/^[0-9a-f]{40}$/.test(entry.preimage_oid)) die('invalid closure preimage');
  }
  return true;
}

function validateBundleWrapper(obj) {
  validateNested(loadContracts().schemas.get('bundle-wrapper'), obj, 'bundle-wrapper');
  const projection = obj.deterministic_projection ?? {};
  if (projection.portable !== true) die('invalid wrapper substitution');
  if (!Array.isArray(projection.logical_paths) || !projection.logical_paths.includes('tools/audit/examples/capability-audit.valid.json')) die('invalid wrapper substitution');
  if (obj.portable_payload_sha256 === obj.original_payload_sha256) die('invalid wrapper substitution');
  return true;
}

function validateAttestationManifest(obj) {
  validateNested(loadContracts().schemas.get('attestation-manifest'), obj, 'attestation-manifest');
  return true;
}

function validateReviewReceipt(obj) {
  validateNested(loadContracts().schemas.get('review-receipt'), obj, 'review-receipt');
  requireNonEmptyArray(obj.reviewed_artifact_oids, 'reviewed_artifact_oids');
  return true;
}

function validateSelectedPlan(text) {
  const obj = parseMachineData(text) ?? JSON.parse(text);
  requireObject(obj, 'selected-plan');
  if (!Array.isArray(obj.tests) || !Array.isArray(obj.non_goals)) die('invalid selected-plan');
  if (!obj.non_goals.some((v) => typeof v === 'string' && v.trim() === 'negative control')) die('missing negative control');
  return true;
}

function validateGovernance(text) {
  const obj = parseMachineData(text) ?? JSON.parse(text);
  requireObject(obj, 'governance');
  if (obj.backlog_item === 'TODO-BIO-003' && Array.isArray(obj.depends_on) && obj.depends_on.some((v) => v === 'TODO-SIM-001' || v === 'TODO-RUNTIME-001')) {
    die('invalid bio dependency');
  }
  return true;
}

function validateLinks(text, filePath) {
  const linkPattern = /\[([^\]]+)\]\(([^)]+)\)/g;
  let match;
  while ((match = linkPattern.exec(text))) {
    const target = match[2].trim();
    if (!target || target.startsWith('#')) continue;
    if (/^[a-zA-Z][a-zA-Z0-9+.-]*:/.test(target)) continue;
    const [pathPart] = target.split('#', 1);
    const resolved = resolveRepoArtifact(pathPart, { baseDir: path.dirname(filePath), label: 'link target', mustExist: false, regularFile: false });
    if (!fs.existsSync(resolved)) die('broken link');
    if (!fs.statSync(resolved).isFile()) die('broken link');
  }
  return true;
}

function validatePlanIndex(text) {
  const items = [...text.matchAll(/^[-*]\s+(.+)$/gm)].map((m) => m[1].trim());
  if (new Set(items).size !== items.length) die('duplicate plan index');
  return true;
}

function pick(args, names, fallback, label) { const value = names.map((name) => args[name]).find((item) => typeof item === 'string') ?? fallback; if (!value) die(`missing ${names[0]} for ${label}`); return value; }
function csv(value) { return typeof value === 'string' ? value.split(',').map((item) => item.trim()).filter(Boolean) : []; }
function own(obj, fields, label) { requireObject(obj, label); for (const field of fields) if (!Object.prototype.hasOwnProperty.call(obj, field)) die(`invalid ${label}: missing required field ${field}`); }
function unique(values, label) { if (new Set(values).size !== values.length) die(`invalid ${label}: duplicate values`); }
function sortedUnique(values, label) { if (!Array.isArray(values)) die(`invalid ${label}: expected array`); unique(values, label); if (canonical(values) !== canonical([...values].sort())) die(`invalid ${label}: values must be sorted`); }
function same(actual, expected, label) { if (expected !== undefined && String(actual) !== String(expected)) die(`invalid ${label}: invocation mismatch`); }
function sameIdentity(objects, label) { if (new Set(objects.map((obj) => obj.run_id)).size !== 1) die(`invalid ${label}: run_id mismatch`); if (new Set(objects.map((obj) => obj.source_baseline_sha)).size !== 1) die(`invalid ${label}: source_baseline_sha mismatch`); }
function fileSha(file) { return crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex'); }

function runnerAttestationPayload(bootstrap) {
  return Object.fromEntries(Object.entries(bootstrap).filter(([key]) => key !== 'runner_attestation'));
}

function runnerAttestationKey(bootstrap, { required = false } = {}) {
  const attestation = bootstrap.runner_attestation;
  if (attestation === undefined) {
    if (required) die('trusted bootstrap runner attestation is required');
    return null;
  }
  own(attestation, ['version', 'key_path', 'key_sha256', 'bootstrap_hmac_sha256'], 'trusted bootstrap runner attestation');
  if (attestation.version !== 1 || !path.isAbsolute(attestation.key_path) || !/^[0-9a-f]{64}$/.test(attestation.key_sha256) || !/^[0-9a-f]{64}$/.test(attestation.bootstrap_hmac_sha256)) die('invalid trusted bootstrap runner attestation');
  const trustedDirectory = path.join(REPO_ROOT, '.omo', 'audit-trust') + path.sep;
  const resolved = fs.realpathSync(attestation.key_path);
  if (!resolved.startsWith(trustedDirectory) || resolved.includes(`${path.sep}evidence${path.sep}`)) die('invalid trusted bootstrap runner key path');
  const stat = fs.statSync(resolved);
  if (!stat.isFile() || (stat.mode & 0o077) !== 0) die('invalid trusted bootstrap runner key permissions');
  const key = fs.readFileSync(resolved);
  if (fileSha(resolved) !== attestation.key_sha256) die('invalid trusted bootstrap runner key digest');
  const expected = crypto.createHmac('sha256', key).update(canonical(runnerAttestationPayload(bootstrap))).digest('hex');
  if (expected !== attestation.bootstrap_hmac_sha256) die('invalid trusted bootstrap runner signature');
  return key;
}

function receiptAttestationPayload(receipt) {
  return {
    run_id: receipt.run_id,
    source_baseline_sha: receipt.source_baseline_sha,
    receipt_id: receipt.receipt_id,
    adapter: receipt.adapter,
    argv: receipt.argv,
    cwd: receipt.cwd,
    exit_code: receipt.exit_code,
    stdout_sha256: receipt.stdout_sha256,
    stderr_sha256: receipt.stderr_sha256,
  };
}

function assertReceiptRunnerAttestation(bootstrap, receipt, { required = false } = {}) {
  if (bootstrap === null) {
    if (required) die('trusted bootstrap runner attestation is required');
    return;
  }
  const key = runnerAttestationKey(bootstrap, { required });
  if (key === null) return;
  const signature = /(?:^|;)runner_hmac_sha256=([0-9a-f]{64})(?:;|$)/.exec(receipt.deterministic_projection?.mode ?? '')?.[1];
  if (!signature) die('invalid command-receipt runner attestation');
  const expected = crypto.createHmac('sha256', key).update(canonical(receiptAttestationPayload(receipt))).digest('hex');
  if (signature !== expected) die('invalid command-receipt runner signature');
}

function assertFrozenBaselineReceipt(bootstrap, receipt, label) {
  if (receipt.cwd !== '$FROZEN_BASELINE') die(`${label} did not execute in the frozen baseline worktree`);
  if (typeof bootstrap.original_worktree_path !== 'string' || !path.isAbsolute(bootstrap.original_worktree_path)) die(`${label} has no frozen baseline worktree`);
  const head = execFileSync('git', ['-C', bootstrap.original_worktree_path, 'rev-parse', 'HEAD'], { encoding: 'utf8' }).trim();
  const tree = execFileSync('git', ['-C', bootstrap.original_worktree_path, 'rev-parse', 'HEAD^{tree}'], { encoding: 'utf8' }).trim();
  const status = execFileSync('git', ['-C', bootstrap.original_worktree_path, 'status', '--porcelain=v1'], { encoding: 'utf8' }).trim();
  if (head !== bootstrap.source_baseline_sha || tree !== bootstrap.source_baseline_tree_oid || status !== '') die(`${label} frozen baseline worktree drifted`);
}

function trustedBootstrap(args, objects = [], { requireRunnerAttestation = false } = {}) {
  if (args['--fixture-mode'] === true || args['--fixture-mode'] === 'true') return null;
  const bootstrapPath = args['--bootstrap'];
  if (typeof bootstrapPath !== 'string') die('trusted bootstrap is required');
  const bootstrap = readJson(bootstrapPath, { label: 'trusted bootstrap' });
  own(bootstrap, ['run_id','source_baseline_sha','source_baseline_tree_oid','audit_worktree_path'], 'trusted bootstrap');
  for (const object of objects) sameIdentity([bootstrap, object], 'trusted bootstrap');
  let commit;
  let tree;
  try {
    commit = execFileSync('git', ['rev-parse', `${bootstrap.source_baseline_sha}^{commit}`], { cwd: REPO_ROOT, encoding: 'utf8' }).trim();
    tree = execFileSync('git', ['rev-parse', `${bootstrap.source_baseline_sha}^{tree}`], { cwd: REPO_ROOT, encoding: 'utf8' }).trim();
  } catch { die('trusted bootstrap baseline is not a commit'); }
  if (commit !== bootstrap.source_baseline_sha || tree !== bootstrap.source_baseline_tree_oid) die('trusted bootstrap baseline/tree mismatch');
  if (fs.realpathSync(bootstrap.audit_worktree_path) !== fs.realpathSync(REPO_ROOT)) die('trusted bootstrap worktree mismatch');
  runnerAttestationKey(bootstrap, { required: requireRunnerAttestation });
  return bootstrap;
}

function assertReceiptArgvPolicy(obj) {
  const command = path.basename(obj.argv[0]);
  const isNode = obj.argv[0] === 'node' || command.startsWith('node');
  const isChecker = isNode && obj.argv[1]?.endsWith('tools/audit/validate-capability-audit.mjs');
  let accepted = false;
  if (obj.adapter === 'source_definition') {
    assertSourceDefinitionArgv(obj.argv, obj.source_baseline_sha);
    accepted = true;
  }
  else if (obj.adapter === 'git_baseline') accepted = command === 'git' && obj.argv[1] === 'rev-parse' && obj.argv.slice(2).includes(`${obj.source_baseline_sha}^{commit}`) && obj.argv.slice(2).includes(`${obj.source_baseline_sha}^{tree}`);
  else if (obj.adapter === 'exact_test') accepted = command === 'cargo' && obj.argv[1] === 'test' && obj.argv.includes('--exact');
  else if (obj.adapter === 'cargo_test_list') accepted = command === 'cargo' && obj.argv[1] === 'test' && obj.argv.includes('--list');
  else if (obj.adapter === 'audit_checker') accepted = isChecker;
  else if (obj.adapter === 'git_preflight') accepted = isChecker && obj.argv[2] === 'preflight';
  else if (obj.adapter === 'git_diff_check') accepted = command === 'git' && obj.argv[1] === 'diff' && obj.argv.includes('--check');
  else if (obj.adapter === 'git_scope') accepted = isChecker && obj.argv[2] === 'scope';
  else if (obj.adapter === 'rust_ci') accepted = command === 'cargo' && obj.argv[1] === 'run' && obj.argv.includes('xtask') && obj.argv.includes('ci');
  else if (obj.adapter === 'pnpm_install') accepted = command === 'pnpm' && obj.argv[1] === 'install';
  else if (obj.adapter === 'pnpm_lint') accepted = command === 'pnpm' && obj.argv[1] === 'lint';
  else if (obj.adapter === 'pnpm_typecheck') accepted = command === 'pnpm' && obj.argv[1] === 'typecheck';
  else if (obj.adapter === 'pnpm_build') accepted = command === 'pnpm' && obj.argv[1] === 'build';
  else if (obj.adapter === 'cargo_metadata') accepted = command === 'cargo' && obj.argv[1] === 'metadata';
  else if (obj.adapter.endsWith('_test') || ['benchmark_diagnostic','representative_benchmark'].includes(obj.adapter)) accepted = command === 'cargo' && obj.argv[1] === 'test';
  else if (['documentation_contract','confirmed_violation','explanation_metric','observer_projection','production_composition'].includes(obj.adapter)) accepted = isChecker || (command === 'cargo' && ['test','run'].includes(obj.argv[1]));
  if (!accepted) die(`invalid command-receipt adapter argv policy: ${obj.adapter}`);
}

function verifyReceiptExecutable(obj) {
  const command = path.basename(obj.argv[0]);
  let executable;
  if (command === 'node' || command.startsWith('node')) executable = fs.realpathSync(process.execPath);
  else {
    const systemPath = path.join('/usr/bin', command);
    if (!fs.existsSync(systemPath)) die(`invalid command-receipt executable provenance: ${command}`);
    executable = fs.realpathSync(systemPath);
  }
  const stat = fs.statSync(executable);
  const parent = fs.statSync(path.dirname(executable));
  if (!stat.isFile() || (stat.mode & 0o022) !== 0 || (parent.mode & 0o022) !== 0) die(`invalid command-receipt untrusted executable: ${command}`);
  const recorded = /(?:^|;)executable_sha256=([a-f0-9]{64})(?:;|$)/.exec(obj.deterministic_projection.mode)?.[1];
  if (!recorded || recorded !== fileSha(executable)) die(`invalid command-receipt executable digest: ${command}`);
}

function commandReceipt(obj, adapter, { runtimeSidecars = true } = {}) {
  validateNested(loadContracts().schemas.get('command-receipt'), obj, 'command-receipt');
  if (adapter !== undefined && obj.adapter !== adapter) die('invalid command-receipt: adapter mismatch');
  const registered = loadContracts().adapters.get(obj.adapter);
  if (!registered) die(`invalid command-receipt: unknown adapter ${obj.adapter}`);
  requireNonEmptyArray(obj.argv, 'command-receipt.argv');
  if (!Number.isInteger(obj.exit_code)) die('invalid command-receipt: exit_code');
  if (obj.deterministic_projection.adapter !== obj.adapter) die('invalid command-receipt: projection adapter mismatch');
  if (obj.projection_sha256 !== sha256Json(obj.deterministic_projection)) die('invalid command-receipt: projection_sha256');
  if (runtimeSidecars) {
    assertReceiptArgvPolicy(obj);
    verifyReceiptExecutable(obj);
    if (obj.stdout_path === obj.stderr_path) die('invalid command-receipt: stdout and stderr sidecars must be distinct');
    const stdout = resolveRepoArtifact(obj.stdout_path, { label: 'stdout sidecar' });
    const stderr = resolveRepoArtifact(obj.stderr_path, { label: 'stderr sidecar' });
    if (stdout === stderr) die('invalid command-receipt: stdout and stderr sidecars must be distinct');
    if (fileSha(stdout) !== obj.stdout_sha256) die('invalid stdout sidecar: byte hash mismatch');
    if (fileSha(stderr) !== obj.stderr_sha256) die('invalid stderr sidecar: byte hash mismatch');
    const truth = /^exit=(-?\d+);signal=([^;]+)$/.exec(obj.deterministic_projection.summary);
    if (!truth || Number(truth[1]) !== obj.exit_code) die('invalid command-receipt: projection execution truth mismatch');
  }
}
function captureMode(args) { const obj = readJson(args['--receipt'], { label: 'capture receipt' }); const bootstrap = trustedBootstrap(args, [obj]); commandReceipt(obj, String(args['--adapter'])); assertReceiptRunnerAttestation(bootstrap, obj, { required: bootstrap !== null && (bootstrap.runner_attestation !== undefined || bootstrap.original_worktree_path !== undefined) }); same(obj.run_id, args['--run-id'], 'capture run_id'); same(obj.receipt_id, args['--receipt-id'], 'capture receipt_id'); return 'pass\n'; }
function intentInvocation(obj, args) { validateCommandIntent(obj); requireNonEmptyArray(obj.argv, 'command-intent.argv'); for (const [field, flag] of [['phase','--phase'],['attempt_id','--attempt-id'],['source_baseline_sha','--source-baseline'],['adapter','--adapter'],['run_id','--run-id'],['receipt_id','--receipt-id'],['stdout_path','--stdout'],['stderr_path','--stderr'],['receipt_path','--receipt']]) same(obj[field], args[flag], `intent ${field}`); }
function prepareIntentMode(args) { intentInvocation(readJson(args['--intent'], { label: 'command intent' }), args); return 'pass\n'; }
function executeIntentMode(args) { const intent = readJson(args['--intent'], { label: 'command intent' }); const output = readJson(args['--receipt'], { label: 'execution receipt' }); intentInvocation(intent, args); commandReceipt(output, intent.adapter); for (const field of ['schema_version','run_id','source_baseline_sha','attempt_id','receipt_id','phase','adapter','argv','cwd','environment','tool_versions','stdout_path','stderr_path','workload']) if (canonical(intent[field]) !== canonical(output[field])) die(`execution receipt does not match prepared intent: ${field}`); return 'pass\n'; }
function validateDirectRustLspProvenance(obj) {
  own(obj.rust_lsp_provider, ['name','version','path','sha256','transport','root_path'], 'preflight rust LSP provider');
  const provider = obj.rust_lsp_provider;
  if (provider.name !== 'rust-analyzer' || provider.transport !== 'stdio' || typeof provider.version !== 'string' || !provider.version || !path.isAbsolute(provider.path) || !path.isAbsolute(provider.root_path) || provider.root_path !== obj.original_worktree_path || !/^[0-9a-f]{64}$/.test(provider.sha256) || !fs.existsSync(provider.path) || fileSha(provider.path) !== provider.sha256) die('invalid preflight rust LSP provider');
  own(obj.rust_lsp_lifecycle, ['sessions','shutdown','retained_processes'], 'preflight rust LSP lifecycle');
  const lifecycle = obj.rust_lsp_lifecycle;
  if (lifecycle.sessions !== 1 || lifecycle.shutdown !== 'pending' || lifecycle.retained_processes !== null) die('invalid preflight rust LSP lifecycle');
}
function preflightMode(args) { const obj = readJson(pick(args, ['--bootstrap'], null, 'preflight'), { label: 'preflight bootstrap' }); own(obj, ['run_id','source_baseline_sha','source_baseline_tree_oid','original_worktree_path','audit_worktree_path','audit_head_sha','audit_tree_oid','lsp_provider','rust_lsp_provider','rust_lsp_lifecycle','tool_versions','graph_status','cleanliness','inventories','ancestry','target_preimages','runner_attestation'], 'preflight bootstrap'); same(obj.run_id, args['--run-id'], 'preflight run_id'); validateFormatValue(obj.source_baseline_sha, 'git-sha1', 'preflight baseline'); validateFormatValue(obj.source_baseline_tree_oid, 'git-oid', 'preflight tree'); assertFrozenBaselineReceipt(obj, { cwd: '$FROZEN_BASELINE' }, 'preflight'); const worktreeList = execFileSync('git', ['worktree', 'list', '--porcelain'], { encoding: 'utf8' }).trim(); if (!worktreeList.includes(obj.audit_worktree_path)) die('preflight: audit worktree not found'); if (!path.isAbsolute(obj.original_worktree_path) || !path.isAbsolute(obj.audit_worktree_path)) die('invalid preflight worktree path'); let recordedAuditTree; try { recordedAuditTree = execFileSync('git', ['rev-parse', `${obj.audit_head_sha}^{tree}`], { cwd: REPO_ROOT, encoding: 'utf8' }).trim(); } catch { die('preflight: recorded audit execution commit is unavailable'); } if (recordedAuditTree !== obj.audit_tree_oid) die('preflight: recorded audit execution tree mismatch'); runnerAttestationKey(obj, { required: true }); if (obj.graph_status?.status !== 'ready' || obj.graph_status.index_sha !== obj.source_baseline_sha) die('invalid preflight graph status'); own(obj.lsp_provider, ['version','path','sha256'], 'preflight LSP provider'); if (!/^\d+\.\d+\.\d+$/.test(obj.lsp_provider.version) || !path.isAbsolute(obj.lsp_provider.path) || !/^[0-9a-f]{64}$/.test(obj.lsp_provider.sha256) || !fs.existsSync(obj.lsp_provider.path) || fileSha(obj.lsp_provider.path) !== obj.lsp_provider.sha256) die('invalid preflight LSP provider'); validateDirectRustLspProvenance(obj); for (const tool of ['git','node','cargo','pnpm']) requireString(obj.tool_versions?.[tool], `preflight tool ${tool}`); for (const field of ['original_status_porcelain','original_staged_paths','relevant_staged_paths','audit_status_porcelain_before_evidence','audit_staged_paths']) if (!Array.isArray(obj.cleanliness?.[field])) die(`invalid preflight cleanliness: ${field}`); if (obj.cleanliness.original_status_porcelain.length || obj.cleanliness.original_staged_paths.length || obj.cleanliness.relevant_staged_paths.length || obj.cleanliness.audit_status_porcelain_before_evidence.length || obj.cleanliness.audit_staged_paths.length) die('invalid preflight cleanliness'); if (Object.values(obj.ancestry).some((value) => value !== true)) die('invalid preflight ancestry'); requireNonEmptyArray(obj.inventories.worktrees, 'preflight worktrees'); requireNonEmptyArray(obj.target_preimages, 'preflight target_preimages'); unique(obj.target_preimages.map((entry) => entry.path), 'preflight target preimages'); for (const entry of obj.target_preimages) { own(entry, ['path','blob_oid'], 'preflight target preimage'); if (!isRepoRelativePath(entry.path)) die('invalid preflight target path'); if (entry.blob_oid !== null) validateFormatValue(entry.blob_oid, 'git-oid', 'preflight target oid'); } if (args['--receipt']) { const rec = readJson(args['--receipt'], { label: 'preflight receipt' }); own(rec, ['run_id','source_baseline_sha','adapter','exit_code','deterministic_projection','projection_sha256','receipt_sha256'], 'preflight receipt'); if (rec.adapter !== 'git_preflight' || rec.exit_code !== 0) die('invalid preflight receipt'); sameIdentity([obj, rec], 'preflight'); if (rec.projection_sha256 !== sha256Json(rec.deterministic_projection)) die('invalid preflight projection_sha256'); if (rec.receipt_sha256 !== sha256Json(Object.fromEntries(Object.entries(rec).filter(([key]) => key !== 'receipt_sha256')))) die('invalid preflight receipt_sha256'); assertReceiptRunnerAttestation(obj, rec, { required: true }); } return 'pass\n'; }
function deepAuditMode(args) { const fragment = readArtifact(pick(args, ['--fragment','--deep-audit','--audit'], 'tools/audit/examples/deep-audit-fragment.valid.json', 'deep-audit')).value; const candidates = readJson(pick(args, ['--candidates'], 'tools/audit/examples/candidate-manifest.valid.json', 'deep-audit')); const audit = readJson(pick(args, ['--capability-audit'], 'tools/audit/examples/capability-audit.valid.json', 'deep-audit')); validateNested(loadContracts().schemas.get('deep-audit-fragment'), fragment, 'deep-audit-fragment'); validateCandidateManifest(candidates); validateCapabilityAuditSemantic(audit); sameIdentity([fragment,candidates,audit], 'deep-audit'); for (const field of ['qualified_roots','capability_ids','candidate_keys','evidence_ids','findings']) requireNonEmptyArray(fragment[field], `deep-audit ${field}`); requireString(fragment.family, 'deep-audit family'); requireString(fragment.stop_condition, 'deep-audit stop condition'); const knownCandidates = new Set(candidates.candidates.map((item) => item.candidate_key)), knownCapabilities = new Set(audit.capabilities.map((item) => item.capability_id)), knownEvidence = new Set(audit.evidence.map((item) => item.evidence_id)); for (const key of fragment.candidate_keys) if (!knownCandidates.has(key)) die(`deep-audit unknown candidate: ${key}`); for (const id of fragment.capability_ids) if (!knownCapabilities.has(id)) die(`deep-audit unknown capability: ${id}`); for (const id of fragment.evidence_ids) if (!knownEvidence.has(id)) die(`deep-audit unknown evidence: ${id}`); return 'pass\n'; }
function bundleMode(args) {
  const obj = readJson(pick(args, ['--bundle','--manifest'], 'tools/audit/examples/bundle-manifest.valid.json', 'bundle'));
  trustedBootstrap(args, [obj]);
  validateNested(loadContracts().schemas.get('bundle-manifest'), obj, 'bundle-manifest');
  requireNonEmptyArray(obj.entries, 'bundle entries');
  sortedUnique(obj.entries.map((entry) => entry.logical_path), 'bundle logical paths');
  unique(obj.entries.map((entry) => entry.tracked_path), 'bundle tracked paths');
  for (const entry of obj.entries) {
    if (!isRepoRelativePath(entry.logical_path) || !isRepoRelativePath(entry.tracked_path)) die('invalid bundle path');
    validateFormatValue(entry.tracked_blob_oid, 'git-oid', 'bundle tracked oid');
    for (const field of ['byte_sha256','wrapper_receipt_sha256','original_receipt_sha256']) validateFormatValue(entry[field], 'sha256', `bundle ${field}`);
    sortedUnique(entry.raw_byte_hashes, 'bundle raw hashes');
    for (const hash of entry.raw_byte_hashes) validateFormatValue(hash, 'sha256', 'bundle raw hash');
    requireString(entry.kind, 'bundle kind');
    const tracked = resolveRepoArtifact(entry.tracked_path, { label: 'bundle tracked path' });
    const actualByteHash = fileSha(tracked);
    const actualBlob = execFileSync('git', ['hash-object', entry.tracked_path], { cwd: REPO_ROOT, encoding: 'utf8' }).trim();
    if (actualByteHash !== entry.byte_sha256 || actualBlob !== entry.tracked_blob_oid) die(`bundle tracked byte/hash/oid mismatch: ${entry.tracked_path}`);
  }
  return 'pass\n';
}
function thirty(obj, label) { if (!Array.isArray(obj.domains) || obj.domains.length !== 30) die(`${label} must contain exactly 30 domains`); if (canonical(obj.domains.map((item) => item.domain)) !== canonical(AUDIT_DOMAINS)) die(`${label} domain order or membership mismatch`); }
function unsupportedM5(audit) { const evidence = new Map(audit.evidence.map((item) => [item.evidence_id,item])); for (const cap of audit.capabilities) if (cap.derived_maturity === 'M5') { const adapters = new Set((cap.derived_level_evidence_ids ?? []).map((id) => evidence.get(id)?.adapter)); for (const needed of ['replay_test','persistence_test','representative_benchmark']) if (!adapters.has(needed)) die(`unsupported M5 capability ${cap.capability_id}: missing ${needed}`); if (!adapters.has('counterfactual_test') && !adapters.has('confirmed_violation')) die(`unsupported M5 capability ${cap.capability_id}: missing negative control`); } }
function materializeAuditMode(args) {
  const input = readJson(pick(args, ['--input','--audit-input'], 'tools/audit/examples/capability-audit-input.valid.json', 'materialize-audit'));
  const execution = readJson(pick(args, ['--evidence-manifest','--execution'], 'tools/audit/examples/evidence-execution-manifest.valid.json', 'materialize-audit'));
  const output = readJson(pick(args, ['--output','--audit'], 'tools/audit/examples/capability-audit.valid.json', 'materialize-audit'));
  trustedBootstrap(args, [input, execution, output]);
  validateCapabilityAuditInputSemantic(input, canonical(input));
  validateEvidenceManifestSemantic(execution);
  const expected = structuredClone(input);
  expected.capabilities = expected.capabilities.map((capability) => {
    const admitted = [];
    let maturity = null;
    for (const level of ['M0','M1','M2','M3','M4','M5']) {
      const record = capability.levels[level];
      if (record.status !== 'satisfied') break;
      maturity = level;
      admitted.push(...record.evidence_ids);
    }
    return { ...capability, derived_maturity: maturity, derived_level_evidence_ids: [...new Set(admitted)].sort() };
  });
  if (canonical(output) !== canonical(expected)) die('materialize-audit caller output does not match internally derived audit');
  validateCapabilityAuditSemantic(output);
  thirty(input, 'capability audit input');
  if (input.source_reconciliation.status !== 'present') die('materialize-audit has unmapped source candidates');
  const inputIds = new Set(input.capabilities.map((item) => item.capability_id));
  for (const claim of execution.claims) if (!inputIds.has(claim.capability_id)) die(`materialize-audit unknown execution capability: ${claim.capability_id}`);
  unsupportedM5(output);
  if (args['--bundle']) bundleMode({ ...args, '--bundle': args['--bundle'] });
  return 'pass\n';
}
function headings(text) { return [...text.matchAll(/^##\s+(.+?)\s*$/gm)].map((match) => match[1].trim()); }
function section(text, heading) { const marker = `## ${heading}`, start = text.indexOf(marker); if (start < 0) return ''; const body = text.indexOf('\n', start + marker.length); if (body < 0) return ''; const end = text.indexOf('\n## ', body + 1); return text.slice(body + 1, end < 0 ? text.length : end); }
function nativeHeadings(text, exact = true) { const actual = headings(text); if (exact && canonical(actual) !== canonical(EXECPLAN_HEADINGS)) die('invalid ExecPlan headings or order'); let cursor = -1; for (const expected of EXECPLAN_HEADINGS) { cursor = actual.indexOf(expected, cursor + 1); if (cursor < 0) die(`missing native heading: ${expected}`); } }
function execplanMode(args) { const text = readText(pick(args, ['--plan'], ACTIVE_PLAN, 'execplan')); nativeHeadings(text); if (!/^\*\*Status:\*\*\s+Active\s*$/m.test(text)) die('invalid ExecPlan status: expected Active'); if (!/^\*\*Source baseline:\*\*\s+`[0-9a-f]{40}`\s*$/m.test(text)) die('missing ExecPlan source baseline SHA'); same(text.match(/^\*\*Run ID:\*\*\s+`([^`]+)`/m)?.[1], args['--run-id'], 'ExecPlan run_id'); if (!/does not implement product behavior|No `?\.rs`?.*artifact|No \.rs/si.test(text) || /```(?:rust|typescript|tsx|proto)\b/i.test(text)) die('ExecPlan contains product code implementation'); requireString(String(args['--receipt-id']), 'execplan receipt_id'); return 'pass\n'; }
function hasCycle(nodes, edges) { const graph = new Map(nodes.map((node) => [node.node_id,[]])); for (const edge of edges) if (edge.edge_type === 'requires') graph.get(edge.from)?.push(edge.to); const active = new Set(), done = new Set(); function visit(id) { if (active.has(id)) return true; if (done.has(id)) return false; active.add(id); for (const next of graph.get(id) ?? []) if (visit(next)) return true; active.delete(id); done.add(id); return false; } return nodes.some((node) => visit(node.node_id)); }
function sequencingMode(args) { const obj = readArtifact(pick(args, ['--sequencing','--input','--audit'], 'tools/audit/examples/sequencing.valid.json', 'sequencing')).value; if (Array.isArray(obj.nodes) && Array.isArray(obj.edges)) { const ids = new Set(obj.nodes.map((node) => node.node_id)); if (obj.edges.every((edge) => ids.has(edge.from) && ids.has(edge.to)) && hasCycle(obj.nodes,obj.edges)) die('selection cycle'); } validateNested(loadContracts().schemas.get('sequencing'), obj, 'sequencing'); requireNonEmptyArray(obj.nodes, 'sequencing nodes'); requireNonEmptyArray(obj.candidates, 'sequencing candidates'); const nodeIds = obj.nodes.map((item) => item.node_id), candidateIds = obj.candidates.map((item) => item.candidate_id); unique(nodeIds, 'sequencing node IDs'); unique(candidateIds, 'sequencing candidate IDs'); const known = new Set(nodeIds), statuses = new Map(obj.nodes.map((item) => [item.node_id,item.status])); for (const edge of obj.edges) { if (!known.has(edge.from) || !known.has(edge.to)) die(`sequencing edge references unknown node: ${edge.from}->${edge.to}`); if (edge.edge_type === 'requires' && ['ready','selected'].includes(statuses.get(edge.from)) && !['ready','selected'].includes(statuses.get(edge.to))) die(`sequencing readiness missing prerequisite: ${edge.from} requires ${edge.to}`); } if (obj.extensions.requires_cycles !== undefined && obj.extensions.requires_cycles !== 0) die('requires_cycles must be 0'); const selection = obj.selection, candidates = new Set(candidateIds); if (!['sim','runtime','prerequisite-remediation'].includes(selection.mode)) die('invalid selection mode'); if (!candidates.has(selection.selected_candidate_id)) die('selection references unknown candidate'); unique(selection.ready_candidate_ids, 'ready candidate IDs'); for (const id of selection.ready_candidate_ids) if (!candidates.has(id)) die(`unknown ready candidate: ${id}`); if (selection.mode === 'sim' && !/SIM/i.test(selection.selected_candidate_id)) die('sim selection must select simulation candidate'); if (selection.mode === 'runtime' && !/RUNTIME/i.test(selection.selected_candidate_id)) die('runtime selection must select runtime candidate'); if (selection.mode === 'prerequisite-remediation') { requireNonEmptyArray(selection.minimal_remediation_node_ids, 'minimal_remediation_node_ids'); unique(selection.minimal_remediation_node_ids, 'minimal remediation'); for (const id of selection.minimal_remediation_node_ids) if (!known.has(id)) die(`invalid minimal remediation node: ${id}`); } requireString(selection.tie_break_serialization, 'tie-break serialization'); try { JSON.parse(selection.tie_break_serialization); } catch { die('invalid tie-break serialization'); } return 'pass\n'; }
function selectedPlanMode(args) { const artifact = readArtifact(pick(args, ['--plan','--selected-plan','--audit'], null, 'selected-plan')); if (artifact.machine) validateSelectedPlan(artifact.text); nativeHeadings(artifact.text, false); if (!/^\*\*Status:\*\*\s+Draft\s*$/m.test(artifact.text)) die('selected plan must be Draft in its file'); const mode = artifact.machine ? artifact.value.mode : artifact.text.match(/^\*\*Selection mode:\*\*\s+`?(sim|runtime|prerequisite-remediation)`?/m)?.[1]; if (!['sim','runtime','prerequisite-remediation'].includes(mode)) die('selected plan must have exactly one valid selection mode'); if (/\b(?:TBD|choose one|implementation choice unresolved|to be decided)\b/i.test(artifact.text)) die('selected plan has unresolved implementation choice'); const answers = artifact.machine && Array.isArray(artifact.value.detailed_development_answers) ? artifact.value.detailed_development_answers : [...artifact.text.matchAll(/^\s*(10|[1-9])\.\s+\S.+$/gm)]; if (answers.length < 10) die('selected plan must answer all ten Detailed Development questions'); const verify = section(artifact.text, 'Verification').toLowerCase(); for (const gate of ['failure','control','replay','save/resume','benchmark']) if (!verify.includes(gate)) die(`selected plan missing executable ${gate} gate`); const nonGoals = section(artifact.text, 'Non-goals').toLowerCase(); for (const item of ['semantic','ui','llm']) if (!nonGoals.includes(item)) die(`selected plan must exclude ${item} scope`); return 'pass\n'; }

function todos(text) { const map = new Map(), matches = [...text.matchAll(/^##\s+(TODO-[A-Z0-9-]+):\s+(.+)$/gm)]; for (let i = 0; i < matches.length; i++) { const match = matches[i], end = i + 1 < matches.length ? matches[i + 1].index : text.length, body = text.slice(match.index,end), fields = {}; for (const item of body.matchAll(/^\*\*([^*]+):\*\*\s*(.*)$/gm)) fields[item[1]] = item[2]; map.set(match[1], { heading: match[0], fields }); } return map; }
function matrixRows(text) { const rows = []; for (const match of text.matchAll(/^\|\s*([^|]+?)\s*\|\s*([^|]+?)\s*\|\s*([^|]+?)\s*\|/gm)) { const domain = match[1].trim(); if (domain !== 'Domain' && !/^-+$/.test(domain)) rows.push({ domain, status: match[3].trim() }); } return rows; }
function governanceMode(args) { if (args['--governance']) validateGovernance(readText(args['--governance'])); const artifact = readArtifact(pick(args, ['--audit'], null, 'governance')), matrix = matrixRows(readText(pick(args, ['--matrix'], null, 'governance'))), backlogText = readText(pick(args, ['--backlog'], null, 'governance')), selectedPath = pick(args, ['--selected-plan'], null, 'governance'), selected = readText(selectedPath), registry = readText(pick(args, ['--plan-index'], 'PLANS.md', 'governance')); validateCapabilityAuditSemantic(artifact.value, artifact.text); thirty(artifact.value, 'governance audit'); if (matrix.length !== 30 || canonical(matrix.map((row) => row.domain)) !== canonical(AUDIT_DOMAINS)) die('governance matrix must retain exactly 30 domains'); const statuses = artifact.value.extensions?.domain_statuses; if (isPlainObject(statuses)) for (const row of matrix) if (statuses[row.domain] !== row.status) die(`governance status mismatch: ${row.domain}`); if (!/^\*\*Status:\*\*\s+Draft\s*$/m.test(selected)) die('selected plan must be Draft in its file'); if (/\bPhase\s+27\b|final\s+phase\s+(?:number|is|:)\s*\d+/i.test(selected)) die('governance must not reserve a final phase number'); const selectedRel = path.relative(REPO_ROOT, resolveRepoArtifact(selectedPath)).split(path.sep).join('/'), active = registry.match(/^## Active Plans\s*$([\s\S]*?)(?=^##\s+)/m)?.[1] ?? '', draft = registry.match(/^## Draft Plans\s*$([\s\S]*?)(?=^##\s+)/m)?.[1] ?? ''; if (active.includes(selectedRel) || !draft.includes(selectedRel)) die('selected plan must be Draft in plan index only'); const backlog = todos(backlogText); for (const binding of artifact.value.todo_bindings) { const todo = backlog.get(binding.todo_id); if (!todo) die(`governance missing TODO binding: ${binding.todo_id}`); if (todo.heading !== binding.heading || todo.fields.Goal !== binding.goal || todo.fields['Acceptance Criteria'] !== binding.acceptance || todo.fields.Dependencies !== binding.dependency_range) die(`governance pinned backlog clause drift: ${binding.todo_id}`); } for (const [id,todo] of backlog) for (const dependency of todo.fields.Dependencies?.match(/TODO-[A-Z0-9-]+/g) ?? []) if (!backlog.has(dependency)) die(`governance unresolved dependency: ${id} -> ${dependency}`); const bio = backlog.get('TODO-BIO-003'); if (!bio || bio.fields.Status !== 'Proposed') die('TODO-BIO-003 must remain blocked as Proposed'); const deps = new Set(bio.fields.Dependencies?.match(/TODO-[A-Z0-9-]+/g) ?? []); for (const dep of ['TODO-DEPTH-001','TODO-SIM-001']) if (!deps.has(dep)) die(`TODO-BIO-003 missing blocking dependency: ${dep}`); return 'pass\n'; }
function markdownLinks(text, source) { const matches = [...text.matchAll(/\[([^\]]+)\]\(([^)]+)\)/g)]; if ((text.match(/\]\(/g) ?? []).length !== matches.length) die('invalid markdown link syntax'); for (const match of matches) { const raw = match[2].trim().replace(/^<|>$/g,''); if (!raw) die('invalid markdown link syntax'); if (/^[a-zA-Z][a-zA-Z0-9+.-]*:/.test(raw)) continue; const [part,anchor] = raw.split('#',2); let target = source; if (part) { target = part.startsWith('/') ? resolveRepoArtifact(part.slice(1), { mustExist: false }) : resolveRepoArtifact(part, { baseDir: path.dirname(source), mustExist: false }); if (!fs.existsSync(target) || !fs.statSync(target).isFile()) die(`broken link: ${raw}`); } if (anchor && target.endsWith('.md')) { const anchors = headings(fs.readFileSync(target,'utf8')).map((heading) => heading.toLowerCase().replace(/[^a-z0-9\s-]/g,'').trim().replace(/\s+/g,'-')); if (!anchors.includes(anchor.toLowerCase())) die(`broken link anchor: ${raw}`); } } }
function linksMode(args) { const paths = csv(args['--paths'] ?? args['--path']); if (!paths.length) die('missing --paths for links'); unique(paths, 'link paths'); for (const item of paths) { const source = resolveRepoArtifact(item, { label: 'link source' }); markdownLinks(fs.readFileSync(source,'utf8'), source); } return 'pass\n'; }
function planIndexMode(args) { const text = readText(pick(args, ['--index','--plan-index'], 'PLANS.md', 'plan-index')); validatePlanIndex(text); const refs = [...text.matchAll(/`(plans\/[^`]+\.md)`/g)].map((match) => match[1]); unique(refs, 'plan index references'); for (const ref of refs) resolveRepoArtifact(ref); if (refs.filter((ref) => ref === ACTIVE_PLAN).length !== 1) die('active plan must appear exactly once'); const active = text.match(/^## Active Plans\s*$([\s\S]*?)(?=^##\s+)/m)?.[1] ?? ''; if (!active.includes(ACTIVE_PLAN)) die('active plan is not registered under Active Plans'); const draft = text.match(/^## Draft Plans\s*$([\s\S]*?)(?=^##\s+)/m)?.[1] ?? ''; for (const ref of [...draft.matchAll(/`(plans\/[^`]+\.md)`/g)].map((match) => match[1])) nativeHeadings(readText(ref), false); const actual = fs.readdirSync(path.join(REPO_ROOT,'plans')).filter((name) => name.endsWith('.md')).map((name) => `plans/${name}`).sort(), missing = actual.filter((item) => !refs.includes(item)); if (missing.length) die(`orphaned plan references: ${missing.join(',')}`); return 'pass\n'; }
function gitLines(args) { const out = execFileSync('git', args, { cwd: REPO_ROOT, encoding: 'utf8' }).trim(); return out ? out.split('\n').filter(Boolean).sort() : []; }
function scopeState(base) { const status = gitLines(['status','--porcelain=v1','--untracked-files=all']); return { committed: base ? gitLines(['diff','--name-only',`${base}..HEAD`]) : [], cached: gitLines(['diff','--cached','--name-only']), unstaged: gitLines(['diff','--name-only']), untracked: status.filter((line) => line.startsWith('?? ')).map((line) => line.slice(3)).sort(), ignored: gitLines(['status','--porcelain=v1','--ignored=matching','--untracked-files=normal']).filter((line) => line.startsWith('!! ')).map((line) => line.slice(3)).sort() }; }
function scopeMode(args) { const allowlist = csv(args['--allowlist']); if (!allowlist.length) die('scope allowlist must not be empty'); for (const item of allowlist) if (!isRepoRelativePath(item)) die(`invalid scope allowlist path: ${item}`); let base = args['--source-baseline']; if (args['--preflight']) { const pre = readJson(args['--preflight']); same(pre.run_id,args['--run-id'],'scope run_id'); base ??= pre.source_baseline_sha; if (pre.cleanliness?.original_status_porcelain?.length) die('scope preflight was not clean'); } if (base) validateFormatValue(base,'git-sha1','scope baseline'); const actual = scopeState(base), changed = [...new Set([...actual.committed,...actual.cached,...actual.unstaged,...actual.untracked])], permitted = (item) => allowlist.some((prefix) => item === prefix || item.startsWith(`${prefix}/`)), unexpected = changed.filter((item) => !permitted(item)); if (unexpected.length) die(`unexpected scope modifications: ${unexpected.join(',')}`); if (args['--expected-index-sha'] && execFileSync('git',['write-tree'],{cwd:REPO_ROOT,encoding:'utf8'}).trim() !== args['--expected-index-sha']) die('scope expected index SHA mismatch'); if (args['--scope']) { const rec = readJson(args['--scope']); validateNested(loadContracts().schemas.get('scope-receipt'),rec,'scope-receipt'); same(rec.run_id,args['--run-id'],'scope run_id'); for (const field of ['committed','cached','unstaged','untracked','ignored']) if (canonical([...rec[field]].sort()) !== canonical(actual[field])) die(`scope receipt mismatch: ${field}`); if (canonical(rec.allowlist) !== canonical(allowlist) || !rec.preflight_preserved || rec.verdict !== 'pass') die('invalid scope receipt'); } return JSON.stringify({ ...actual, allowlist, unexpected: [] },null,2)+'\n'; }
function authorization(obj) { validateNested(loadContracts().schemas.get('authorization-receipt'),obj,'authorization-receipt'); if (!Number.isInteger(obj.approval_line) || obj.approval_line < 1) die('invalid authorization approval_line'); requireString(obj.nonce,'authorization nonce'); requireString(obj.authorized_action,'authorization action'); }
function operationalClosure(obj, verify = false) { validateClosureManifest(obj); if (obj.entries.length !== 5) die('closure manifest must contain exactly five paths'); sortedUnique(obj.entries.map((entry) => entry.path),'closure paths'); unique(obj.entries.map((entry) => entry.replacement_path),'closure replacement paths'); for (const entry of obj.entries) { if (!isRepoRelativePath(entry.path) || !isRepoRelativePath(entry.replacement_path)) die('invalid closure path'); if (!['add','replace','delete'].includes(entry.allowed_transition)) die(`invalid closure transition: ${entry.allowed_transition}`); requireNonEmptyArray(entry.validation_commands,'closure validation commands'); if (entry.allowed_transition !== 'delete') { const replacement = resolveRepoArtifact(entry.replacement_path), bytes = fileSha(replacement), oid = execFileSync('git',['hash-object',entry.replacement_path],{cwd:REPO_ROOT,encoding:'utf8'}).trim(); if (bytes !== entry.replacement_sha256 || oid !== entry.expected_blob_oid) die(`closure replacement mismatch: ${entry.path}`); } if (verify) { let preimage = ZERO_OID; try { preimage = execFileSync('git',['rev-parse',`${obj.preclosure_head_sha}:${entry.path}`],{cwd:REPO_ROOT,encoding:'utf8',stdio:['ignore','pipe','ignore']}).trim(); } catch {} if (preimage !== entry.preimage_oid) die(`closure preimage mismatch: ${entry.path}`); if (preimage !== ZERO_OID) { const tracked = execFileSync('git',['show',`${obj.preclosure_head_sha}:${entry.path}`],{cwd:REPO_ROOT,encoding:'utf8'}); if (/TODO-DEPTH-001[\s\S]{0,120}\*\*Status:\*\*\s*Completed/i.test(tracked)) die(`completion claim exists in preclosure tree: ${entry.path}`); } } } requireNonEmptyArray(obj.validation_commands,'closure validation commands'); }
function closureMode(args) { operationalClosure(readJson(pick(args,['--closure','--manifest'],null,'closure')), args['--verify-repository'] === true || args['--verify-repository'] === 'true'); return 'pass\n'; }
function applyClosureMode(args) { const closure = readJson(args['--closure']), auth = readJson(args['--authorization']); operationalClosure(closure,true); authorization(auth); sameIdentity([closure,auth],'apply-closure'); same(closure.run_id,args['--run-id'],'apply-closure run_id'); if (!['apply-closure','closure'].includes(auth.authorized_action)) die('authorization receipt does not authorize closure application'); const expected = args['--expected-head'] ?? closure.preclosure_head_sha, head = execFileSync('git',['rev-parse','HEAD'],{cwd:REPO_ROOT,encoding:'utf8'}).trim(); if (head !== expected || closure.preclosure_head_sha !== expected) die('apply-closure expected head mismatch'); if (auth.preclosure_finalize_sha256 === ZERO_SHA) die('apply-closure authorization is not bound to preclosure finalize'); return 'pass\n'; }

function aggregateMode(args) {
  const deepPaths = csv(args['--deep-audits']);
  if (deepPaths.length !== 4) die('aggregate requires exactly four deep-audit fragments');
  const candidates = readJson(pick(args,['--candidates','--candidate-manifest'],'tools/audit/examples/candidate-manifest.valid.json','aggregate'));
  const execution = readJson(pick(args,['--evidence-manifest','--execution'],'tools/audit/examples/evidence-execution-manifest.valid.json','aggregate'));
  const audit = readJson(pick(args,['--audit'],'tools/audit/examples/capability-audit.valid.json','aggregate'));
  const tests = readJson(pick(args,['--test-reconciliation'],'tools/audit/examples/test-reconciliation.valid.json','aggregate'));
  const bundle = readJson(pick(args,['--bundle'],'tools/audit/examples/bundle-manifest.valid.json','aggregate'));
  trustedBootstrap(args, [candidates, execution, audit, tests, bundle]);
  validateCandidateManifestSemantic(candidates);
  validateEvidenceManifestSemantic(execution, { validateReceipts: !(args['--fixture-mode'] === true || args['--fixture-mode'] === 'true') });
  validateCapabilityAuditSemantic(audit);
  validateTestReconciliation(tests);
  bundleMode({ ...args, '--bundle': pick(args,['--bundle'],'tools/audit/examples/bundle-manifest.valid.json','aggregate') });
  sameIdentity([candidates,execution,audit,tests,bundle],'aggregate');
  const candidateIds = new Set(candidates.candidates.map((item) => item.candidate_key));
  const bindings = new Set(candidates.candidates.flatMap((item) => item.bindings.map((binding) => binding.binding_id)));
  const capabilities = new Set(audit.capabilities.map((item) => item.capability_id));
  const evidence = new Set(audit.evidence.map((item) => item.evidence_id));
  for (const claim of execution.claims) {
    if (!capabilities.has(claim.capability_id)) die(`aggregate unknown capability: ${claim.capability_id}`);
    if (!bindings.has(claim.binding_id)) die(`aggregate unknown binding: ${claim.binding_id}`);
  }
  for (const mapping of tests.mappings) {
    if (!candidateIds.has(mapping.candidate_key)) die(`aggregate unknown test candidate: ${mapping.candidate_key}`);
    if (!bindings.has(mapping.binding_id)) die(`aggregate unknown test binding: ${mapping.binding_id}`);
  }
  const requiredFamilies = new Set(['durable-physical-causality','production-causal-bootstrap','analytical-validity','causal-legibility-benchmark-readiness']);
  const observedFamilies = new Set();
  for (const item of deepPaths) {
    const fragment = readArtifact(item).value;
    validateNested(loadContracts().schemas.get('deep-audit-fragment'),fragment,'deep-audit-fragment');
    sameIdentity([audit,fragment],'aggregate deep audit');
    if (!requiredFamilies.has(fragment.family) || observedFamilies.has(fragment.family)) die(`aggregate invalid deep-audit family: ${fragment.family}`);
    observedFamilies.add(fragment.family);
    for (const key of fragment.candidate_keys) if (!candidateIds.has(key)) die(`aggregate missing candidate evidence: ${key}`);
    for (const id of fragment.capability_ids) if (!capabilities.has(id)) die(`aggregate missing capability evidence: ${id}`);
    for (const id of fragment.evidence_ids) if (!evidence.has(id)) die(`aggregate missing evidence: ${id}`);
  }
  if (observedFamilies.size !== requiredFamilies.size) die('aggregate required deep-audit families are incomplete');
  for (const cap of audit.capabilities) for (const id of cap.derived_level_evidence_ids ?? []) if (!evidence.has(id)) die(`aggregate missing evidence: ${id}`);
  return 'pass\n';
}
function finalizeMode(args) { const rec = readJson(pick(args,['--finalize','--receipt'],'tools/audit/examples/finalize-receipt.valid.json','finalize')); validateNested(loadContracts().schemas.get('finalize-receipt'),rec,'finalize-receipt'); if (rec.verdict !== 'pass') die('finalize verdict must pass'); requireNonEmptyArray(rec.commands,'finalize commands'); requireNonEmptyArray(rec.checks,'finalize checks'); requireNonEmptyArray(rec.input_hashes,'finalize input hashes'); for (const hash of rec.input_hashes) validateFormatValue(hash,'sha256','finalize input hash'); const reviews = csv(args['--reviews'] ?? 'tools/audit/examples/review-receipt.valid.json').map((item) => readJson(item)); if (!reviews.length) die('finalize requires review receipts'); for (const review of reviews) { validateReviewReceipt(review); if (review.verdict !== 'pass') die('finalize review receipt failed'); } sameIdentity([rec,...reviews],'finalize'); if (rec.phase !== 'collection') { const closure = readJson(pick(args,['--closure'],null,'finalize')), auth = readJson(pick(args,['--authorization'],null,'finalize')); validateClosureManifest(closure); authorization(auth); sameIdentity([rec,closure,auth],'finalize closure'); } same(rec.head_sha,args['--expected-head'],'finalize head'); same(rec.head_tree_oid,args['--expected-tree'],'finalize tree'); same(rec.index_tree_oid,args['--expected-index'],'finalize index'); return 'pass\n'; }
function attestation(obj) { validateAttestationManifest(obj); requireNonEmptyArray(obj.entries,'attestation entries'); unique(obj.entries.map((entry) => entry.receipt_id),'attestation receipt IDs'); for (const entry of obj.entries) for (const field of ['attempt_id','closure_commit_sha','attestation_parent_sha','closed_finalize_sha256','attempt_chain_head_sha256']) if (entry[field] !== obj[field]) die(`attestation entry mismatch: ${field}`); if (obj.closure_commit_sha === ZERO_OID || obj.attempt_chain_head_sha256 === ZERO_SHA) die('attestation references an empty closure commit or attempt chain'); }
function buildAttestationMode(args) { const obj = readJson(pick(args,['--attestation','--manifest'],'tools/audit/examples/attestation-manifest.valid.json','build-attestation')); attestation(obj); same(obj.closure_commit_sha,args['--closure-commit'],'attestation closure commit'); same(obj.attempt_chain_head_sha256,args['--attempt-chain-head'],'attestation chain head'); return 'pass\n'; }
function attestMode(args) { const obj = readJson(pick(args,['--attestation','--manifest'],null,'attest')); attestation(obj); const phase = args['--phase']; if (!['staged','committed'].includes(phase)) die('attest phase must be staged or committed'); if (phase === 'staged') { if (!args['--expected-tree']) die('staged attestation requires --expected-tree'); const tree = execFileSync('git',['write-tree'],{cwd:REPO_ROOT,encoding:'utf8'}).trim(); if (tree !== args['--expected-tree']) die('staged attestation tree mismatch'); } else { const commit = args['--expected-commit'] ?? obj.closure_commit_sha, parent = args['--expected-parent'] ?? obj.attestation_parent_sha, tree = args['--expected-tree']; if (!tree) die('committed attestation requires --expected-tree'); const actualParent = execFileSync('git',['rev-parse',`${commit}^`],{cwd:REPO_ROOT,encoding:'utf8'}).trim(), actualTree = execFileSync('git',['rev-parse',`${commit}^{tree}`],{cwd:REPO_ROOT,encoding:'utf8'}).trim(); if (commit !== obj.closure_commit_sha || parent !== actualParent || tree !== actualTree) die('committed attestation commit/parent/tree mismatch'); } return 'pass\n'; }
function recoverMode(args) { const obj = readJson(pick(args,['--recovery','--receipt'],'tools/audit/examples/recovery-receipt.valid.json','recover')); validateNested(loadContracts().schemas.get('recovery-receipt'),obj,'recovery-receipt'); if (obj.verdict !== 'pass') die('recovery verdict must pass'); requireNonEmptyArray(obj.checks,'recovery checks'); const checks = obj.checks.join(' ').toLowerCase(), post = ['commit','attest','attestation','postcommit'].some((word) => String(obj.failure_phase).toLowerCase().includes(word)); if (post) { if (!checks.includes('revert')) die('postcommit recovery must verify attestation revert'); if (obj.revert_commit_sha === obj.failed_closure_sha) die('postcommit recovery revert commit is invalid'); } else { if (!checks.includes('restore') || !checks.includes('index') || !checks.includes('worktree')) die('precommit recovery must restore index and worktree from preimages'); if (obj.preclosure_tree_oid !== obj.staged_tree_oid) die('precommit recovery did not restore staged tree'); } return 'pass\n'; }
function ingestMode(args) { const operation = String(args['--operation']), obj = readJson(args['--response']); if (['index_status','graph_schema','query_graph','trace_path'].includes(operation)) { validateGraphReceipt(obj); if (obj.operation !== operation) die('ingest operation/response mismatch'); if (obj.index_sha !== obj.source_baseline_sha) die('ingest graph index SHA does not match baseline'); if (obj.returned > obj.total || obj.returned < 0) die('invalid graph result structure'); } else if (['lsp_symbols','lsp_references'].includes(operation)) { validateLspReceipt(obj); if (obj.complete !== true || !Array.isArray(obj.symbols)) die('invalid LSP result structure'); if ((args['--index-sha'] ?? obj.source_baseline_sha) !== obj.source_baseline_sha) die('ingest LSP index SHA does not match baseline'); } else if (operation === 'schema_absence') { own(obj,['schema_version','run_id','source_baseline_sha','index_sha','operation','absent','result_count'],'schema absence response'); if (obj.operation !== operation || obj.absent !== true || obj.result_count !== 0) die('invalid schema absence result structure'); if (obj.index_sha !== obj.source_baseline_sha) die('ingest schema absence index SHA does not match baseline'); } else die(`unsupported operation: ${operation}`); return 'pass\n'; }

function inputPath(args, mode, flags = ['--input']) {
  return pick(args, flags, null, mode);
}

function validateSourceBlobsSemantic(obj, verifyRepository = true) {
  validateSourceBlobs(obj);
  requireNonEmptyArray(obj.entries, 'source-blobs entries');
  sortedUnique(obj.entries.map((entry) => entry.logical_path), 'source-blobs logical paths');
  unique(obj.entries.map((entry) => entry.tracked_path), 'source-blobs tracked paths');
  for (const entry of obj.entries) {
    if (!isRepoRelativePath(entry.logical_path) || !isRepoRelativePath(entry.tracked_path)) die('invalid source-blobs path');
    requireString(entry.kind, 'source-blobs kind');
    if (!verifyRepository || entry.tracked_blob_oid === ZERO_OID) continue;
    let oid;
    let bytes;
    try {
      oid = execFileSync('git', ['rev-parse', `${obj.source_baseline_sha}:${entry.tracked_path}`], { cwd: REPO_ROOT, encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] }).trim();
      bytes = execFileSync('git', ['show', `${obj.source_baseline_sha}:${entry.tracked_path}`], { cwd: REPO_ROOT });
    } catch {
      die(`source-blobs path missing at baseline: ${entry.tracked_path}`);
    }
    if (oid !== entry.tracked_blob_oid) die(`source-blobs oid mismatch: ${entry.tracked_path}`);
    if (crypto.createHash('sha256').update(bytes).digest('hex') !== entry.byte_sha256) die(`source-blobs byte hash mismatch: ${entry.tracked_path}`);
  }
  if (verifyRepository && obj.tree_oid !== ZERO_OID) {
    let tree;
    try { tree = execFileSync('git', ['rev-parse', `${obj.source_baseline_sha}^{tree}`], { cwd: REPO_ROOT, encoding: 'utf8' }).trim(); }
    catch { die('source-blobs baseline is not a commit'); }
    if (tree !== obj.tree_oid) die('source-blobs tree_oid mismatch');
  }
  return obj;
}

function validateCandidateManifestSemantic(obj) {
  validateCandidateManifest(obj);
  requireNonEmptyArray(obj.candidates, 'candidate-manifest candidates');
  unique(obj.candidates.map((candidate) => candidate.candidate_key), 'candidate-manifest candidate keys');
  unique(obj.graph_queries.map((query) => query.query_id), 'candidate-manifest graph query IDs');
  unique(obj.exclusions.map((exclusion) => exclusion.exclusion_id), 'candidate-manifest exclusion IDs');
  const exclusions = new Set(obj.exclusions.map((exclusion) => exclusion.exclusion_id));
  const bindings = new Map();
  for (const candidate of obj.candidates) {
    if (candidate.line_start < 1 || candidate.line_end < candidate.line_start || candidate.definition_character < 0) die(`invalid candidate location: ${candidate.candidate_key}`);
    if (candidate.exclusion_id !== null && !exclusions.has(candidate.exclusion_id)) die(`candidate references unknown exclusion: ${candidate.candidate_key}`);
    for (const binding of candidate.bindings) {
      assertSameKeys(binding, ['binding_id', 'capability_id', 'role', 'lifecycle_boundary'], `candidate binding ${candidate.candidate_key}`);
      requireString(binding.binding_id, `candidate binding ID ${candidate.candidate_key}`);
      requireString(binding.capability_id, `candidate binding capability ${binding.binding_id}`);
      if (!['primary', 'shared', 'carrier', 'support'].includes(binding.role)) die(`invalid candidate binding role: ${binding.binding_id}`);
      requireString(binding.lifecycle_boundary, `candidate binding lifecycle boundary ${binding.binding_id}`);
      if (bindings.has(binding.binding_id)) die(`duplicate candidate binding: ${binding.binding_id}`);
      bindings.set(binding.binding_id, { binding, candidate });
    }
    unique(candidate.failure_cases, `candidate failure cases ${candidate.candidate_key}`);
  }
  const contexts = obj.extensions?.binding_contexts;
  requireNonEmptyArray(contexts, 'candidate-manifest binding_contexts');
  const contextIds = new Set();
  for (const context of contexts) {
    assertSameKeys(context, ['binding_id', 'candidate_key', 'grouping', 'endpoint_declaration'], 'candidate binding context');
    requireString(context.binding_id, 'candidate binding context ID');
    requireString(context.candidate_key, `candidate binding context key ${context.binding_id}`);
    if (contextIds.has(context.binding_id)) die(`duplicate candidate binding context: ${context.binding_id}`);
    const entry = bindings.get(context.binding_id);
    if (!entry || entry.candidate.candidate_key !== context.candidate_key) die(`candidate binding context mismatch: ${context.binding_id}`);
    assertSameKeys(context.grouping, ['domain', 'capability_class', 'state_path', 'mutation_owner', 'lifecycle_boundary'], `candidate binding grouping ${context.binding_id}`);
    if (context.grouping.lifecycle_boundary !== entry.binding.lifecycle_boundary) die(`candidate binding lifecycle mismatch: ${context.binding_id}`);
    contextIds.add(context.binding_id);
  }
  if (contextIds.size !== bindings.size) die('candidate binding context coverage mismatch');
  const manifests = obj.extensions?.receipt_manifests;
  requireObject(manifests, 'candidate receipt manifests');
  assertSameKeys(manifests, ['graph', 'lsp', 'data_flows_absence'], 'candidate receipt manifests');
  for (const key of ['graph', 'lsp', 'data_flows_absence']) {
    assertSameKeys(manifests[key], ['receipt_id', 'path', 'sha256'], `candidate ${key} receipt manifest`);
    requireString(manifests[key].receipt_id, `candidate ${key} receipt ID`);
    requireString(manifests[key].path, `candidate ${key} receipt path`);
    validateFormatValue(manifests[key].sha256, 'sha256', `candidate ${key} receipt digest`);
  }
  const semanticReview = obj.extensions?.semantic_review;
  requireObject(semanticReview, 'candidate semantic review');
  assertSameKeys(semanticReview, ['receipt_id', 'path', 'sha256'], 'candidate semantic review');
  requireString(semanticReview.receipt_id, 'candidate semantic review ID');
  requireString(semanticReview.path, 'candidate semantic review path');
  validateFormatValue(semanticReview.sha256, 'sha256', 'candidate semantic review digest');
  return obj;
}

function validateEvidenceManifestSemantic(obj, { validateReceipts = true } = {}) {
  validateEvidenceExecutionManifest(obj);
  requireNonEmptyArray(obj.claims, 'evidence-execution-manifest claims');
  const { adapters } = loadContracts();
  unique(obj.claims.map((claim) => claim.receipt_id), 'evidence claim receipt IDs');
  unique(obj.claims.map((claim) => claim.receipt_path), 'evidence claim receipt paths');
  for (const claim of obj.claims) {
    const adapter = adapters.get(claim.adapter);
    if (!adapter) die(`evidence claim has unknown adapter: ${claim.adapter}`);
    requireNonEmptyArray(claim.argv, `evidence claim argv ${claim.binding_id}`);
    if (typeof claim.baseline_target === 'string') {
      if (!isRepoRelativePath(claim.baseline_target)) die(`invalid evidence baseline target: ${claim.baseline_target}`);
    } else {
      requireObject(claim.baseline_target, `evidence baseline target ${claim.binding_id}`);
      if (!isRepoRelativePath(claim.baseline_target.path)) die(`invalid evidence baseline target path: ${claim.binding_id}`);
      if (claim.baseline_target.blob_oid !== undefined) validateFormatValue(claim.baseline_target.blob_oid, 'git-oid', `evidence baseline target blob ${claim.binding_id}`);
      if (claim.baseline_target.line_start !== undefined && (!Number.isInteger(claim.baseline_target.line_start) || claim.baseline_target.line_start < 1)) die(`invalid evidence baseline target line: ${claim.binding_id}`);
      if (claim.baseline_target.line_end !== undefined && claim.baseline_target.line_end < claim.baseline_target.line_start) die(`invalid evidence baseline target range: ${claim.binding_id}`);
    }
    for (const required of adapter.required_facets) if (!claim.facets.some((facet) => facet === required || facet.startsWith(`${required}:`))) die(`evidence claim ${claim.binding_id} missing facet: ${required}`);
    if (validateReceipts) {
      const receipt = readJson(claim.receipt_path, { label: `evidence claim receipt ${claim.receipt_id}` });
      commandReceipt(receipt, claim.adapter);
      if (receipt.receipt_sha256 !== claim.receipt_sha256) die(`evidence claim ${claim.receipt_id} receipt_sha256 mismatch`);
      if (receipt.receipt_id !== claim.receipt_id || receipt.run_id !== obj.run_id || receipt.source_baseline_sha !== obj.source_baseline_sha) die(`evidence claim ${claim.receipt_id} receipt identity mismatch`);
      if (canonical(receipt.argv) !== canonical(claim.argv)) die(`evidence claim ${claim.receipt_id} argv mismatch`);
    }
  }
  return obj;
}

function validateInventoryEvidenceReceipts(inventory, execution) {
  const claimsByReceipt = new Map(execution.claims.map((claim) => [claim.receipt_path, claim]));
  for (const evidence of inventory.evidence) {
    const claim = claimsByReceipt.get(evidence.receipt_path);
    if (!claim) die(`inventory evidence ${evidence.evidence_id} has no execution claim`);
    if (claim.receipt_sha256 !== evidence.receipt_sha256 || claim.adapter !== evidence.adapter) die(`inventory evidence ${evidence.evidence_id} receipt binding mismatch`);
    if (evidence.run_id !== inventory.run_id || evidence.source_baseline_sha !== inventory.source_baseline_sha) die(`inventory evidence ${evidence.evidence_id} identity mismatch`);
    if (evidence.exit_code !== 0 && !['confirmed_violation'].includes(evidence.adapter)) die(`inventory evidence ${evidence.evidence_id} records unsuccessful execution`);
    for (const facet of claim.facets) if (!evidence.facets.some((value) => value === facet || value.startsWith(`${facet}:`) || facet.startsWith(`${value}:`))) die(`inventory evidence ${evidence.evidence_id} facet mismatch: ${facet}`);
  }
}

function validateClaimBindings(candidateManifest, executionManifest, label) {
  const bindings = new Map();
  for (const candidate of candidateManifest.candidates) {
    for (const binding of candidate.bindings) {
      if (bindings.has(binding.binding_id)) die(`${label} duplicate binding: ${binding.binding_id}`);
      bindings.set(binding.binding_id, binding);
    }
  }
  for (const claim of executionManifest.claims) {
    const binding = bindings.get(claim.binding_id);
    if (!binding) die(`${label} unknown claim binding: ${claim.binding_id}`);
    if (binding.capability_id !== claim.capability_id) die(`${label} capability/binding mismatch: ${claim.capability_id}/${claim.binding_id}`);
  }
}

function readLinkedArtifact(baseDir, reference, label) {
  const artifact = resolveRepoArtifact(reference.path, { baseDir, label });
  const value = JSON.parse(fs.readFileSync(artifact, 'utf8'));
  if (sha256Json(value) !== reference.sha256) die(`${label} hash mismatch`);
  return { file: artifact, value };
}

function baselineBlob(sourceBaseline, repoPath, label) {
  try {
    return execFileSync('git', ['rev-parse', `${sourceBaseline}:${repoPath}`], { cwd: REPO_ROOT, encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] }).trim();
  } catch {
    die(`${label} is missing from source baseline: ${repoPath}`);
  }
}

function flattenLspSymbols(symbols, prefix = []) {
  const flattened = [];
  for (const symbol of symbols) {
    const next = [...prefix, {
      name: symbol.name,
      kind: symbol.kind,
      line_start: symbol.line_start,
      line_end: symbol.line_end,
      definition_character: symbol.definition_character,
    }];
    flattened.push(next.at(-1));
    flattened.push(...flattenLspSymbols(symbol.children, next));
  }
  return flattened;
}

function validateTask4ReceiptLinkage(candidateManifest, candidateManifestFile, blobs, execution, inventory) {
  const manifests = candidateManifest.extensions.receipt_manifests;
  const manifestBase = path.dirname(candidateManifestFile);
  const graph = readLinkedArtifact(manifestBase, manifests.graph, 'Task 4 graph receipt manifest');
  assertSameKeys(graph.value, ['schema_version', 'run_id', 'source_baseline_sha', 'graph_schema_sha256', 'slots'], 'Task 4 graph receipt manifest');
  if (graph.value.run_id !== candidateManifest.run_id || graph.value.source_baseline_sha !== candidateManifest.source_baseline_sha) die('Task 4 graph receipt manifest identity mismatch');
  const dataFlows = candidateManifest.graph_queries.filter((query) => query.edge_type === 'DATA_FLOWS');
  const expectedGraphQueries = dataFlows.length === 1 ? 23 : 19;
  if (dataFlows.length > 1 || candidateManifest.graph_queries.length !== expectedGraphQueries || !Array.isArray(graph.value.slots) || graph.value.slots.length !== expectedGraphQueries) {
    die('Task 4 graph query receipt cardinality must be exactly 19, or 23 with DATA_FLOWS');
  }
  const graphSlots = new Map();
  const graphReceipts = new Map();
  for (const slot of graph.value.slots) {
    assertSameKeys(slot, ['query_id', 'edge_type', 'receipt_id', 'receipt_path', 'receipt_sha256'], 'Task 4 graph receipt slot');
    if (graphSlots.has(slot.receipt_id)) die(`duplicate Task 4 graph receipt: ${slot.receipt_id}`);
    const receiptFile = resolveRepoArtifact(slot.receipt_path, { baseDir: manifestBase, label: `Task 4 graph receipt ${slot.receipt_id}` });
    const receipt = JSON.parse(fs.readFileSync(receiptFile, 'utf8'));
    validateGraphReceipt(receipt);
    if (receipt.receipt_sha256 !== slot.receipt_sha256 || receipt.receipt_id !== slot.receipt_id || receipt.run_id !== candidateManifest.run_id || receipt.source_baseline_sha !== candidateManifest.source_baseline_sha || receipt.index_sha !== candidateManifest.source_baseline_sha) die(`Task 4 graph receipt linkage mismatch: ${slot.receipt_id}`);
    if (receipt.parameters.query_id !== slot.query_id || receipt.parameters.edge_type !== slot.edge_type || receipt.query !== candidateManifest.graph_queries.find((query) => query.query_id === slot.query_id)?.query) die(`Task 4 graph query linkage mismatch: ${slot.query_id}`);
    if (receipt.operation !== 'query_graph') die(`Task 4 graph receipt uses an untruthful operation: ${slot.receipt_id}`);
    if (/baseline source review slot/i.test(receipt.query) || !/^MATCH \(n\) WHERE /i.test(receipt.query) || receipt.total < 0 || receipt.returned < 0 || receipt.returned > receipt.total) die(`Task 4 graph receipt has an invalid query identity: ${slot.receipt_id}`);
    const rawFile = resolveRepoArtifact(receipt.raw_path, { baseDir: manifestBase, label: `Task 4 graph raw ${slot.receipt_id}` });
    if (fileSha(rawFile) !== receipt.raw_sha256) die(`Task 4 graph raw hash mismatch: ${slot.receipt_id}`);
    graphSlots.set(slot.receipt_id, slot);
    graphReceipts.set(slot.receipt_id, receipt);
  }
  const queryIds = new Set(candidateManifest.graph_queries.map((query) => query.query_id));
  if (queryIds.size !== expectedGraphQueries || queryIds.size !== graph.value.slots.length || [...queryIds].some((id) => ![...graphSlots.values()].some((slot) => slot.query_id === id))) die('Task 4 graph query receipt coverage mismatch');
  if (dataFlows.length === 0) {
    const absence = readLinkedArtifact(manifestBase, manifests.data_flows_absence, 'Task 4 DATA_FLOWS absence receipt');
    assertSameKeys(absence.value, ['schema_version', 'run_id', 'source_baseline_sha', 'receipt_id', 'operation', 'edge_type', 'graph_schema_sha256', 'graph_schema_path', 'graph_schema_raw_sha256', 'reason'], 'Task 4 DATA_FLOWS absence receipt');
    if (absence.value.run_id !== candidateManifest.run_id || absence.value.source_baseline_sha !== candidateManifest.source_baseline_sha || absence.value.receipt_id !== manifests.data_flows_absence.receipt_id || absence.value.operation !== 'schema_absence' || absence.value.edge_type !== 'DATA_FLOWS' || absence.value.graph_schema_sha256 !== graph.value.graph_schema_sha256) die('Task 4 DATA_FLOWS absence receipt linkage mismatch');
    const schemaFile = resolveRepoArtifact(absence.value.graph_schema_path, { baseDir: manifestBase, label: 'Task 4 captured graph schema' });
    if (fileSha(schemaFile) !== absence.value.graph_schema_raw_sha256) die('Task 4 captured graph schema raw hash mismatch');
    let schema;
    try { schema = JSON.parse(fs.readFileSync(schemaFile, 'utf8')); } catch { die('Task 4 captured graph schema is invalid JSON'); }
    if (!Array.isArray(schema.edge_types) || schema.edge_types.some((edge) => edge?.type === 'DATA_FLOWS')) die('Task 4 captured graph schema does not prove DATA_FLOWS absence');
  }

  const lsp = readLinkedArtifact(manifestBase, manifests.lsp, 'Task 4 LSP receipt manifest');
  assertSameKeys(lsp.value, ['schema_version', 'run_id', 'source_baseline_sha', 'tree_oid', 'entries'], 'Task 4 LSP receipt manifest');
  if (lsp.value.run_id !== candidateManifest.run_id || lsp.value.source_baseline_sha !== candidateManifest.source_baseline_sha) die('Task 4 LSP receipt manifest identity mismatch');
  const candidateFiles = new Map();
  for (const candidate of candidateManifest.candidates) {
    const existing = candidateFiles.get(candidate.path) ?? [];
    existing.push(candidate);
    candidateFiles.set(candidate.path, existing);
  }
  if (candidateFiles.size !== 132 || lsp.value.entries.length !== 132) die('Task 4 LSP manifest must cover exactly 132 candidate files');
  const blobsByPath = new Map(blobs.entries.map((entry) => [entry.tracked_path, entry]));
  const lspEntries = new Map();
  let complete = 0;
  for (const entry of lsp.value.entries) {
    assertSameKeys(entry, ['path', 'baseline_blob_oid', 'worktree_blob_oid', 'complete', 'receipt_id', 'receipt_path', 'receipt_sha256', 'flattened_symbols_sha256'], 'Task 4 LSP receipt entry');
    if (lspEntries.has(entry.path)) die(`duplicate Task 4 LSP entry: ${entry.path}`);
    if (typeof entry.complete !== 'boolean') die(`invalid Task 4 LSP provider state: ${entry.path}`);
    const expectedId = `task-04-file-${sha256Json({ path: entry.path, blob_oid: entry.baseline_blob_oid }).slice(0, 12)}-lsp-symbols`;
    if (entry.receipt_id !== expectedId) die(`non-deterministic Task 4 LSP receipt ID: ${entry.path}`);
    const blob = blobsByPath.get(entry.path);
    if (!blob || blob.tracked_blob_oid !== entry.baseline_blob_oid || baselineBlob(candidateManifest.source_baseline_sha, entry.path, 'Task 4 LSP path') !== entry.baseline_blob_oid || entry.worktree_blob_oid !== entry.baseline_blob_oid) die(`stale Task 4 LSP blob binding: ${entry.path}`);
    const receiptFile = resolveRepoArtifact(entry.receipt_path, { baseDir: manifestBase, label: `Task 4 LSP receipt ${entry.receipt_id}` });
    const receipt = JSON.parse(fs.readFileSync(receiptFile, 'utf8'));
    validateLspReceipt(receipt);
    if (receipt.receipt_sha256 !== entry.receipt_sha256 || receipt.receipt_id !== entry.receipt_id || receipt.run_id !== candidateManifest.run_id || receipt.source_baseline_sha !== candidateManifest.source_baseline_sha || receipt.path !== entry.path || receipt.baseline_blob_oid !== entry.baseline_blob_oid || receipt.worktree_blob_oid !== entry.worktree_blob_oid || receipt.complete !== entry.complete) die(`Task 4 LSP receipt linkage mismatch: ${entry.path}`);
    const flattened = flattenLspSymbols(receipt.symbols);
    if (sha256Json(flattened) !== receipt.flattened_symbols_sha256 || receipt.flattened_symbols_sha256 !== entry.flattened_symbols_sha256) die(`Task 4 LSP flattened symbol mismatch: ${entry.path}`);
    const rawFile = resolveRepoArtifact(receipt.raw_path, { baseDir: manifestBase, label: `Task 4 LSP raw ${entry.receipt_id}` });
    if (fileSha(rawFile) !== receipt.raw_sha256) die(`Task 4 LSP raw hash mismatch: ${entry.path}`);
    lspEntries.set(entry.path, entry);
    if (entry.complete) complete++;
  }
  if (complete !== 105 || lsp.value.entries.length - complete !== 27 || [...candidateFiles.keys()].some((candidatePath) => !lspEntries.has(candidatePath))) die('Task 4 LSP provider aggregate must be 105 complete and 27 incomplete files');
  const bindingsByCandidate = new Map(candidateManifest.candidates.flatMap((candidate) => candidate.bindings.map((binding) => [binding.binding_id, candidate])));
  for (const candidate of candidateManifest.candidates) {
    const entry = lspEntries.get(candidate.path);
    if (candidate.lsp_receipt_ids.length !== 1 || candidate.lsp_receipt_ids[0] !== entry.receipt_id) die(`Task 4 candidate LSP receipt mismatch: ${candidate.candidate_key}`);
    if (entry.complete && !flattenLspSymbols(JSON.parse(fs.readFileSync(resolveRepoArtifact(entry.receipt_path, { baseDir: manifestBase, label: `Task 4 LSP receipt ${entry.receipt_id}` }), 'utf8')).symbols).some((symbol) => symbol.line_start <= candidate.line_start && symbol.line_end >= candidate.line_end)) die(`Task 4 candidate LSP range is not covered: ${candidate.candidate_key}`);
    for (const graphReceiptId of candidate.graph_receipt_ids) if (!graphSlots.has(graphReceiptId)) die(`Task 4 candidate graph receipt mismatch: ${candidate.candidate_key}`);
    if (!entry.complete && candidate.bindings.some((binding) => inventory.capabilities.find((capability) => capability.capability_id === binding.capability_id)?.levels?.M1?.status === 'satisfied')) {
      die(`incomplete Task 4 LSP file cannot support M1 maturity: ${candidate.path}`);
    }
  }
  const semanticRef = candidateManifest.extensions.semantic_review;
  const semantic = readLinkedArtifact(manifestBase, semanticRef, 'Task 4 semantic review');
  assertSameKeys(semantic.value, ['schema_version', 'run_id', 'source_baseline_sha', 'collection_method', 'rows'], 'Task 4 semantic review');
  if (semantic.value.run_id !== candidateManifest.run_id || semantic.value.source_baseline_sha !== candidateManifest.source_baseline_sha || semantic.value.collection_method !== 'graph_first_git_fallback' || !Array.isArray(semantic.value.rows) || semantic.value.rows.length !== 61) die('Task 4 semantic review identity or cardinality mismatch');
  const bindingsById = new Map(candidateManifest.candidates.flatMap((candidate) => candidate.bindings.map((binding) => [binding.binding_id, { binding, candidate }])));
  const reviewedCapabilities = new Set();
  for (const row of semantic.value.rows) {
    assertSameKeys(row, ['capability_id', 'binding_id', 'candidate_key', 'path', 'blob_oid', 'symbol', 'line_start', 'line_end', 'query_id', 'graph_receipt_id', 'resolver_kind', 'fallback_argv', 'fallback_stdout_sha256', 'status'], 'Task 4 semantic review row');
    const mapped = bindingsById.get(row.binding_id), receipt = graphReceipts.get(row.graph_receipt_id);
    if (!mapped || reviewedCapabilities.has(row.capability_id) || mapped.binding.capability_id !== row.capability_id || mapped.candidate.candidate_key !== row.candidate_key || mapped.candidate.path !== row.path || mapped.candidate.blob_oid !== row.blob_oid || mapped.candidate.symbol !== row.symbol || mapped.candidate.line_start !== row.line_start || mapped.candidate.line_end !== row.line_end || !['graph', 'git_fallback'].includes(row.resolver_kind) || row.status !== 'pass') die(`Task 4 semantic review mapping mismatch: ${row.capability_id}`);
    const expectedArgv = ['git', 'show', `${candidateManifest.source_baseline_sha}:${row.path}`];
    if (row.resolver_kind === 'graph') {
      if (!receipt || !mapped.candidate.graph_receipt_ids.includes(row.graph_receipt_id) || receipt.parameters.query_id !== row.query_id || row.fallback_argv !== null || row.fallback_stdout_sha256 !== null) die(`Task 4 semantic review graph linkage mismatch: ${row.capability_id}`);
      const raw = JSON.parse(fs.readFileSync(resolveRepoArtifact(receipt.raw_path, { baseDir: manifestBase, label: `Task 4 graph raw ${receipt.receipt_id}` }), 'utf8')), columns = new Map(raw.columns.map((name, index) => [name, index]));
      if (!raw.rows.some((entry) => entry[columns.get('name')] === row.symbol && entry[columns.get('file_path')] === row.path && Number(entry[columns.get('start_line')]) <= row.line_start && Number(entry[columns.get('end_line')]) >= row.line_end)) die(`Task 4 semantic review graph result mismatch: ${row.capability_id}`);
    } else {
      if (row.graph_receipt_id !== null || mapped.candidate.graph_receipt_ids.length !== 0 || canonical(row.fallback_argv) !== canonical(expectedArgv)) die(`Task 4 semantic review fallback command mismatch: ${row.capability_id}`);
      let source;
      try { source = execFileSync('git', expectedArgv.slice(1), { cwd: REPO_ROOT }); } catch { die(`Task 4 semantic review fallback source is unavailable: ${row.path}`); }
      const lines = source.toString('utf8').split('\n');
      if (crypto.createHash('sha256').update(source).digest('hex') !== row.fallback_stdout_sha256 || !lines[row.line_start - 1]?.includes(row.symbol)) die(`Task 4 semantic review fallback result mismatch: ${row.capability_id}`);
    }
    reviewedCapabilities.add(row.capability_id);
  }
  if (reviewedCapabilities.size !== bindingsById.size || [...bindingsById.values()].some(({ binding }) => !reviewedCapabilities.has(binding.capability_id))) die('Task 4 semantic review coverage mismatch');
  for (const claim of execution.claims) {
    const candidate = bindingsByCandidate.get(claim.binding_id);
    if (candidate && !lspEntries.get(candidate.path).complete && claim.adapter === 'source_definition') die(`incomplete Task 4 LSP file cannot grant source maturity: ${candidate.path}`);
  }
}

function inventoryDiagnostics(inventory, candidateManifest = null) {
  const actualDomains = new Set((inventory.domains ?? []).map((domain) => domain.domain));
  const diagnostics = AUDIT_DOMAINS.filter((domain) => !actualDomains.has(domain)).map((domain) => `missing domain: ${domain}`);
  const hasUnmappedCandidate = candidateManifest?.candidates?.some((candidate) => candidate.exclusion_id === null && candidate.bindings.length === 0) ?? false;
  if (hasUnmappedCandidate || inventory.source_reconciliation?.status !== 'present') diagnostics.push('unmapped source');
  return diagnostics;
}

function validateTestReconciliationSemantic(obj, candidateManifest = null) {
  validateTestReconciliation(obj);
  unique(obj.tests.map((test) => test.test_id), 'test-reconciliation test IDs');
  const tests = new Set(obj.tests.map((test) => test.test_id));
  const candidates = candidateManifest ? new Set(candidateManifest.candidates.map((candidate) => candidate.candidate_key)) : null;
  const mapped = new Set();
  for (const mapping of obj.mappings) {
    if (!tests.has(mapping.test_id)) die(`test reconciliation references unknown test: ${mapping.test_id}`);
    if (candidates && !candidates.has(mapping.candidate_key)) die(`test reconciliation references unknown candidate: ${mapping.candidate_key}`);
    mapped.add(mapping.test_id);
  }
  if (obj.completeness.status === 'complete') for (const test of obj.tests) if (test.eligibility === 'exact_test' && !mapped.has(test.test_id)) die(`incomplete test reconciliation: ${test.test_id}`);
  return obj;
}

function sourceBlobsMode(args) {
  validateSourceBlobsSemantic(readJson(inputPath(args, 'source-blobs', ['--input', '--blobs', '--source-blobs'])));
  return 'pass\n';
}

function evidenceManifestMode(args) {
  validateEvidenceManifestSemantic(readJson(inputPath(args, 'evidence-execution-manifest', ['--input', '--manifest', '--evidence-execution-manifest'])));
  return 'pass\n';
}

function candidateManifestMode(args) {
  validateCandidateManifestSemantic(readJson(inputPath(args, 'candidate-manifest', ['--input', '--candidate-manifest'])));
  return 'pass\n';
}

function testReconciliationMode(args) {
  const candidates = args['--candidate-manifest'] ? validateCandidateManifestSemantic(readJson(args['--candidate-manifest'])) : null;
  validateTestReconciliationSemantic(readJson(inputPath(args, 'test-reconciliation', ['--input', '--test-reconciliation'])), candidates);
  return 'pass\n';
}

function capabilityInventoryMode(args) {
  const artifact = readArtifact(inputPath(args, 'capability-inventory', ['--input', '--inventory']));
  validateCapabilityAuditInputSemantic(artifact.value, artifact.text);
  thirty(artifact.value, 'capability inventory');
  same(artifact.value.run_id, args['--run-id'], 'capability inventory run_id');
  unique(artifact.value.capabilities.map((capability) => capability.capability_id), 'capability inventory IDs');
  if (artifact.value.capabilities.length !== 61) die(`capability inventory must contain exactly 61 concrete capabilities`);
  const placeholder = artifact.value.capabilities.find((capability) => /\.audited_foundation$/i.test(capability.capability_id));
  if (placeholder) die(`placeholder capability cannot satisfy inventory: ${placeholder.capability_id}`);
  return 'pass\n';
}

function inventoryValidationMode(args) {
  const obj = readJson(inputPath(args, 'inventory-validation'));
  own(obj, ['schema_version', 'run_id', 'source_baseline_sha', 'expected', 'actual', 'verdict'], 'inventory validation');
  validateFormatValue(obj.schema_version, 'integer', 'inventory validation schema_version');
  validateFormatValue(obj.run_id, 'run-id', 'inventory validation run_id');
  validateFormatValue(obj.source_baseline_sha, 'git-sha1', 'inventory validation source_baseline_sha');
  const fields = ['domains', 'unmapped_sources', 'invalid_bindings', 'incomplete_test_relations'];
  own(obj.expected, fields, 'inventory validation expected');
  own(obj.actual, fields, 'inventory validation actual');
  for (const field of fields) {
    if (!Number.isInteger(obj.expected[field]) || obj.expected[field] < 0 || !Number.isInteger(obj.actual[field]) || obj.actual[field] < 0) die(`invalid inventory validation count: ${field}`);
    if (obj.expected[field] !== obj.actual[field]) die(`inventory validation mismatch: ${field}`);
  }
  if (obj.verdict !== 'pass') die('inventory validation verdict must pass');
  return 'pass\n';
}

function inventoryNegativeMode(args) {
  const obj = readJson(inputPath(args, 'inventory-negative'));
  own(obj, ['schema_version', 'run_id', 'source_baseline_sha', 'command', 'exit_code', 'stdout', 'stderr', 'verdict'], 'inventory negative');
  validateFormatValue(obj.schema_version, 'integer', 'inventory negative schema_version');
  validateFormatValue(obj.run_id, 'run-id', 'inventory negative run_id');
  validateFormatValue(obj.source_baseline_sha, 'git-sha1', 'inventory negative source_baseline_sha');
  if (!Number.isInteger(obj.exit_code) || obj.exit_code === 0) die('inventory negative must record a non-zero exit code');
  requireString(obj.command, 'inventory negative command');
  requireString(obj.stderr, 'inventory negative stderr');
  if (obj.stdout !== '' || obj.verdict !== 'pass-as-negative') die('invalid inventory negative result');
  return 'pass\n';
}

function rustReceiptMode(args, adapter, listMode) {
  const obj = readJson(inputPath(args, listMode ? 'rust-test-list' : 'rust-test-results', ['--input', '--receipt']));
  commandReceipt(obj, adapter);
  if (obj.exit_code !== 0 || obj.argv[0] !== 'cargo' || obj.argv[1] !== 'test') die(`invalid ${listMode ? 'rust-test-list' : 'rust-test-results'} command`);
  const hasList = obj.argv.includes('--list');
  if (hasList !== listMode) die(`invalid ${listMode ? 'rust-test-list' : 'rust-test-results'} argv`);
  return 'pass\n';
}

function deepAuditArtifactMode(args) {
  const obj = readArtifact(inputPath(args, 'deep-audit', ['--input', '--fragment', '--deep-audit'])).value;
  validateNested(loadContracts().schemas.get('deep-audit-fragment'), obj, 'deep-audit-fragment');
  for (const field of ['qualified_roots', 'capability_ids', 'candidate_keys', 'evidence_ids', 'findings']) requireNonEmptyArray(obj[field], `deep-audit ${field}`);
  requireString(obj.family, 'deep-audit family');
  requireString(obj.stop_condition, 'deep-audit stop condition');
  for (const root of obj.qualified_roots) if (!isRepoRelativePath(root)) die(`invalid deep-audit root: ${root}`);
  return 'pass\n';
}

function closureManifestMode(args) {
  const obj = readJson(inputPath(args, 'closure-manifest', ['--input', '--closure', '--manifest']));
  validateClosureManifest(obj);
  requireNonEmptyArray(obj.entries, 'closure-manifest entries');
  unique(obj.entries.map((entry) => entry.path), 'closure-manifest paths');
  return 'pass\n';
}

function reviewReceiptMode(args) {
  const obj = readJson(inputPath(args, 'review-receipt', ['--input', '--review', '--receipt']));
  validateReviewReceipt(obj);
  if (obj.verdict !== 'pass') die('review receipt verdict must pass');
  return 'pass\n';
}

function validateSchemaDrift(obj) {
  const canonicalContracts = readJson(CANONICAL_SCHEMA);
  if (canonical(obj) !== canonical(canonicalContracts)) die('schema drift');
  return true;
}

function validateInvalidFixture(filePath) {
  const { text, value } = readArtifact(filePath, { label: 'audit fixture' });
  try {
    if (Array.isArray(value.capabilities)) {
      if (value.capabilities.some((capability) => Object.prototype.hasOwnProperty.call(capability, 'derived_maturity'))) {
        validateCapabilityAuditSemantic(value, text);
      } else {
        validateCapabilityAuditInputSemantic(value, text);
        const diagnostics = inventoryDiagnostics(value);
        if (diagnostics.length) die(diagnostics.join('\n'));
      }
      return null;
    }
    if (Array.isArray(value.nodes) && Array.isArray(value.edges)) {
      sequencingMode({ '--input': filePath });
      return null;
    }
    if (Array.isArray(value.reviewed_artifact_oids)) { validateReviewReceipt(value); return null; }
    if (Array.isArray(value.tests) && isPlainObject(value.completeness)) { validateTestReconciliation(value); return null; }
    if (Array.isArray(value.schemas)) { validateSchemaDrift(value); return null; }
    if (Array.isArray(value.entries) && Object.prototype.hasOwnProperty.call(value, 'preclosure_head_sha')) { validateClosureManifest(value); return null; }
    if (Object.prototype.hasOwnProperty.call(value, 'portable_payload_sha256')) { validateBundleWrapper(value); return null; }
    if (Object.prototype.hasOwnProperty.call(value, 'attestation_parent_sha')) { validateAttestationManifest(value); return null; }
    if (typeof value.backlog_item === 'string') { validateGovernance(text); return null; }
    if (Array.isArray(value.affected_paths)) { validateSelectedPlan(text); return null; }
    if (/\[[^\]]+\]\([^)]+\)/.test(text)) { validateLinks(text, resolveRepoArtifact(filePath)); return null; }
    if (/^[-*]\s+.+$/m.test(text)) { validatePlanIndex(text); return null; }
    die('unhandled invalid fixture content');
  } catch (error) {
    return String(error?.message ?? error);
  }
}

export function validateSupportFixture(fileName, obj, schemas, adapters) {
  if (fileName === 'fixture-blobs.json') return validateSourceBlobs(obj);
  if (fileName === 'fixture-candidate-manifest.valid.json') return validateCandidateManifest(obj);
  if (fileName === 'fixture-evidence-execution.valid.json') return validateEvidenceExecutionManifest(obj);
  if (fileName === 'fixture-test-reconciliation.valid.json') return validateTestReconciliation(obj);
  if (fileName === 'fixture-test-list.command-receipt.valid.json') return validateCommandReceiptLike(obj, 'cargo_test_list');
  if (fileName === 'fixture-test-results.command-receipt.valid.json') return validateCommandReceiptLike(obj, 'exact_test');
  if (fileName === 'fixture-graph-receipts.valid.json' || fileName === 'fixture-trace-receipts.valid.json' || fileName === 'fixture-writes-receipts.valid.json') {
    validateRootExact(obj, ['schema_version','run_id','source_baseline_sha','receipts','receipt_sha256'], fileName);
    return true;
  }
  if (fileName === 'fixture-manifest.valid.json') return validateSchemaExample('fixture-manifest', obj, schemas.get('fixture-manifest'));
  if (fileName === 'fixture-manifest.json') return validateSchemaExample('fixture-manifest', obj, schemas.get('fixture-manifest'));
  if (fileName === 'command-intent.valid.json') return validateCommandIntent(obj);
  if (fileName === 'graph-receipt.valid.json') return validateGraphReceipt(obj);
  if (fileName === 'lsp-receipt.valid.json') return validateLspReceipt(obj);
  if (fileName === 'review-receipt.valid.json') return validateReviewReceipt(obj);
  if (fileName === 'reviewer-dispatch.valid.json') {
    validateNested(loadContracts().schemas.get('reviewer-dispatch'), obj, 'reviewer-dispatch');
    return true;
  }
  if (fileName === 'closure-manifest.valid.json') return validateClosureManifest(obj);
  if (fileName === 'attestation-manifest.valid.json') return validateAttestationManifest(obj);
  if (fileName === 'bundle-manifest.valid.json') { validateNested(loadContracts().schemas.get('bundle-manifest'), obj, 'bundle-manifest'); return true; }
  if (fileName === 'bundle-wrapper.valid.json') return validateBundleWrapper(obj);
  if (fileName === 'run-manifest.valid.json') { validateNested(loadContracts().schemas.get('run-manifest'), obj, 'run-manifest'); return true; }
  if (fileName === 'source-blobs.valid.json') return validateSourceBlobs(obj);
  if (fileName === 'test-reconciliation.valid.json') return validateTestReconciliation(obj);
  if (fileName === 'capability-audit.valid.json') return validateCapabilityAuditSemantic(obj);
  if (fileName === 'capability-audit-input.valid.json') return validateCapabilityAuditInputSemantic(obj, readText(`tools/audit/examples/${fileName}`));
  if (fileName === 'deep-audit-fragment.valid.json') { validateNested(loadContracts().schemas.get('deep-audit-fragment'), obj, 'deep-audit-fragment'); return true; }
  if (fileName === 'sequencing.valid.json') { validateNested(loadContracts().schemas.get('sequencing'), obj, 'sequencing'); return true; }
  if (fileName === 'command-receipt.valid.json') return validateCommandReceiptLike(obj, 'source_definition');
  if (fileName.startsWith('command-receipt.') && fileName.endsWith('.valid.json')) return validateCommandReceiptLike(obj, fileName.slice('command-receipt.'.length, -'.valid.json'.length));
  if (fileName.endsWith('.valid.json')) {
    const schemaName = fileName.replace(/\.valid\.json$/, '');
    const spec = schemas.get(schemaName);
    if (!spec) die(`unknown schema example: ${fileName}`);
    return validateSchemaExample(schemaName, obj, spec);
  }
  die(`unhandled support fixture: ${fileName}`);
}

export function validateAllFixtures(schemas, adapters) {
  const fixtureManifest = validateFixtureManifest();
  const expectedList = [...new Set(fixtureManifest.files)].sort();
  const actualList = [...new Set(expectedFixturePaths())].sort();
  if (JSON.stringify(expectedList) !== JSON.stringify(actualList)) die('fixture manifest drift');
  for (const rel of expectedList) {
    const full = resolveRepoArtifact(rel, { label: 'fixture' });
    const base = path.basename(rel);
    const { text, value } = readArtifactResolved(full);
    if (rel.startsWith('tools/audit/fixtures/invalid-')) {
      const err = validateInvalidFixture(rel);
      if (!err) die(`invalid fixture unexpectedly accepted: ${rel}`);
    } else if (rel.startsWith('tools/audit/fixtures/')) {
      validateSupportFixture(base, value, schemas, adapters);
    } else if (rel.startsWith('tools/audit/examples/')) {
      if (base.startsWith('command-receipt.') && base !== 'command-receipt.valid.json') {
        validateCommandReceiptLike(value, base.slice('command-receipt.'.length, -'.valid.json'.length));
      } else if (base === 'command-receipt.valid.json') {
        validateCommandReceiptLike(value, 'source_definition');
      } else {
        const schemaName = base.replace(/\.valid\.json$/, '');
        const spec = schemas.get(schemaName);
        if (!spec) die(`unknown schema example: ${base}`);
        validateSchemaExample(schemaName, value, spec);
      }
    }
  }
  const bindingCandidates = validateCandidateManifestSemantic(readJson('tools/audit/fixtures/fixture-candidate-manifest.valid.json'));
  const bindingExecution = validateEvidenceManifestSemantic(readJson('tools/audit/fixtures/fixture-evidence-execution.valid.json'), { validateReceipts: false });
  validateClaimBindings(bindingCandidates, bindingExecution, 'fixture binding');
  const mismatchedExecution = JSON.parse(JSON.stringify(bindingExecution));
  mismatchedExecution.claims[0].capability_id = 'cap-space-mismatch';
  try {
    validateClaimBindings(bindingCandidates, mismatchedExecution, 'fixture binding');
    die('invalid binding fixture unexpectedly accepted');
  } catch (error) {
    if (!String(error?.message ?? error).includes('capability/binding mismatch')) throw error;
  }
  return expectedList.filter((p) => p.includes('/fixtures/invalid-')).length;
}

function requireFlags(args, flags, mode) {
  for (const flag of flags) {
    if (!(flag in args)) die(`missing ${flag} for ${mode}`);
  }
}

export function validateModeInvocation(mode, args) {
  const flags = MODE_REQUIREMENTS.get(mode);
  if (flags) requireFlags(args, flags, mode);
  if (mode === 'ingest-tool-response' && !INGEST_OPERATIONS.includes(String(args['--operation']))) die(`unsupported operation: ${args['--operation']}`);
}

export function schemaMode(args = {}) {
  const { schemas, adapters } = validateContractFiles(args['--contracts'] ?? CANONICAL_SCHEMA, args['--adapter-contracts'] ?? CANONICAL_ADAPTER);
  const invalidCount = validateAllFixtures(schemas, adapters);
  return `schemas=${schemas.size} adapters=${adapters.size} fixtures=${expectedFixturePaths().length} invalid_cases=${invalidCount} pass\n`;
}

export function inventoryMode(args = {}) {
  const { schemas, adapters } = validateContractFiles(args['--contracts'] ?? CANONICAL_SCHEMA, args['--adapter-contracts'] ?? CANONICAL_ADAPTER);
  const manifest = validateFixtureManifest();
  const inventoryPath = args['--input'] ?? args['--inventory'];
  if (inventoryPath !== undefined) {
    const fixtureMode = args['--fixture-mode'] === true || args['--fixture-mode'] === 'true';
    const required = [...(fixtureMode ? [] : ['--bootstrap', '--preflight-receipt']), '--candidate-manifest', '--blobs', '--test-list-receipt', '--test-results-receipt', '--test-reconciliation', '--evidence-execution-manifest', '--out'];
    requireFlags(args, required, 'inventory');
    if (fixtureMode && [inventoryPath, args['--candidate-manifest'], args['--blobs'], args['--test-list-receipt'], args['--test-results-receipt'], args['--test-reconciliation'], args['--evidence-execution-manifest']].some((value) => typeof value !== 'string' || !value.startsWith('tools/audit/fixtures/'))) die('inventory fixture mode requires canonical fixture inputs');
    const inventory = readArtifact(inventoryPath);
    const candidateManifestFile = resolveRepoArtifact(args['--candidate-manifest'], { label: 'inventory candidate manifest' });
    const candidates = validateCandidateManifestSemantic(JSON.parse(fs.readFileSync(candidateManifestFile, 'utf8')));
    const diagnostics = inventoryDiagnostics(inventory.value, candidates);
    if (diagnostics.length) die(diagnostics.join('\n'));
    const blobs = validateSourceBlobsSemantic(readJson(args['--blobs']));
    const testList = readJson(args['--test-list-receipt']);
    const testResults = readJson(args['--test-results-receipt']);
    const reconciliation = validateTestReconciliationSemantic(readJson(args['--test-reconciliation']), candidates);
    const execution = validateEvidenceManifestSemantic(readJson(args['--evidence-execution-manifest']));
    if (!fixtureMode) {
      const bootstrap = trustedBootstrap(args, [inventory.value, candidates, blobs, testList, testResults, reconciliation, execution], { requireRunnerAttestation: true });
      preflightMode({ '--run-id': args['--run-id'], '--bootstrap': args['--bootstrap'], '--receipt': args['--preflight-receipt'] });
      assertReceiptRunnerAttestation(bootstrap, testList, { required: true });
      assertReceiptRunnerAttestation(bootstrap, testResults, { required: true });
      assertFrozenBaselineReceipt(bootstrap, testList, 'inventory test-list receipt');
      assertFrozenBaselineReceipt(bootstrap, testResults, 'inventory test-results receipt');
    }
    validateClaimBindings(candidates, execution, 'inventory');
    validateTask4ReceiptLinkage(candidates, candidateManifestFile, blobs, execution, inventory.value);
    validateCapabilityAuditInputSemantic(inventory.value, inventory.text);
    if (inventory.value.capabilities.length !== 61) die('capability inventory must contain exactly 61 concrete capabilities');
    const placeholder = inventory.value.capabilities.find((capability) => /\.audited_foundation$/i.test(capability.capability_id));
    if (placeholder) die(`placeholder capability cannot satisfy inventory: ${placeholder.capability_id}`);
    validateInventoryEvidenceReceipts(inventory.value, execution);
    thirty(inventory.value, 'capability inventory');
    commandReceipt(testList, 'cargo_test_list');
    commandReceipt(testResults, 'exact_test');
    if (testList.exit_code !== 0 || !testList.argv.includes('--list')) die('inventory test-list receipt did not complete discovery');
    if (testResults.exit_code !== 0 || testResults.argv.includes('--list')) die('inventory test-results receipt did not complete execution');
    const target = reconciliation.targets.find((entry) => entry.package === LEGACY_BASELINE_TYPES_PACKAGE && entry.target_kind === 'integration-test' && entry.target_name === 'coords');
    if (!target || target.path !== LEGACY_BASELINE_COORDS_PATH || target.blob_oid !== baselineBlob(inventory.value.source_baseline_sha, target.path, 'inventory integration target')) die('inventory target metadata is not bound to the baseline coords integration test');
    if (!testList.argv.includes('-p') || !testList.argv.includes(LEGACY_BASELINE_TYPES_PACKAGE) || !testList.argv.includes('--test') || !testList.argv.includes('coords') || !testResults.argv.includes('--test') || !testResults.argv.includes('coords')) die('inventory cargo receipts are not bound to the coords integration target');
    for (const test of reconciliation.tests) if (test.target_name === 'coords' && (test.path !== target.path || test.blob_oid !== target.blob_oid)) die(`inventory test metadata does not match coords target: ${test.test_id}`);
    sameIdentity([inventory.value, candidates, blobs, testList, testResults, reconciliation, execution], 'inventory');
    same(inventory.value.run_id, args['--run-id'], 'inventory run_id');

    const candidateKeys = new Set(candidates.candidates.map((candidate) => candidate.candidate_key));
    const bindingIds = new Set(candidates.candidates.flatMap((candidate) => candidate.bindings.map((binding) => binding.binding_id)));
    const capabilityIds = new Set(inventory.value.capabilities.map((capability) => capability.capability_id));
    const blobPaths = new Set(blobs.entries.map((entry) => entry.tracked_path));
    const testIds = new Set(reconciliation.tests.map((test) => test.test_id));
    const mappedTestIds = new Set(reconciliation.mappings.map((mapping) => mapping.test_id));
    const unmappedSources = candidates.candidates.filter((candidate) => candidate.exclusion_id === null && candidate.bindings.length === 0).length;
    let invalidBindings = candidates.candidates.filter((candidate) => !blobPaths.has(candidate.path)).length;
    invalidBindings += execution.claims.filter((claim) => !capabilityIds.has(claim.capability_id) || !bindingIds.has(claim.binding_id)).length;
    invalidBindings += reconciliation.mappings.filter((mapping) => !candidateKeys.has(mapping.candidate_key) || !bindingIds.has(mapping.binding_id) || !testIds.has(mapping.test_id)).length;
    const incompleteTestRelations = reconciliation.tests.filter((test) => test.eligibility === 'exact_test' && !mappedTestIds.has(test.test_id)).length;
    const actual = { domains: inventory.value.domains.length, unmapped_sources: unmappedSources, invalid_bindings: invalidBindings, incomplete_test_relations: incompleteTestRelations };
    const expected = { domains: 30, unmapped_sources: 0, invalid_bindings: 0, incomplete_test_relations: 0 };
    for (const field of Object.keys(expected)) if (actual[field] !== expected[field]) die(`inventory validation mismatch: ${field} expected ${expected[field]} actual ${actual[field]}`);
    const report = { schema_version: 1, run_id: inventory.value.run_id, source_baseline_sha: inventory.value.source_baseline_sha, expected, actual, verdict: 'pass' };
    const output = resolveRepoArtifact(args['--out'], { label: 'inventory output', mustExist: false });
    fs.writeFileSync(output, JSON.stringify(report, null, 2) + '\n', { encoding: 'utf8', flag: 'w' });
    return JSON.stringify(report, null, 2) + '\n';
  }
  return JSON.stringify({ schemas: schemas.size, adapters: adapters.size, fixtures: manifest.files.length }, null, 2) + '\n';
}

function auditMode(args) {
  const auditPath = args['--audit'];
  if (!auditPath) die('missing --audit');
  const artifact = readArtifact(auditPath);
  validateCapabilityAuditSemantic(artifact.value, artifact.text);
  return 'pass\n';
}

function dispatchMode(mode, args) {
  const modes = {
    capture: captureMode, 'prepare-intent': prepareIntentMode, 'execute-intent': executeIntentMode,
    preflight: preflightMode, schema: schemaMode, inventory: inventoryMode, 'deep-audit': deepAuditMode,
    bundle: bundleMode, 'materialize-audit': materializeAuditMode, audit: auditMode, execplan: execplanMode,
    sequencing: sequencingMode, 'selected-plan': selectedPlanMode, governance: governanceMode, links: linksMode,
    'plan-index': planIndexMode, scope: scopeMode, 'apply-closure': applyClosureMode, closure: closureMode,
    aggregate: aggregateMode, finalize: finalizeMode, 'build-attestation': buildAttestationMode,
    attest: attestMode, recover: recoverMode, 'ingest-tool-response': ingestMode,
    blobs: sourceBlobsMode, 'source-blobs': sourceBlobsMode,
    'evidence-execution-manifest': evidenceManifestMode,
    'test-reconciliation': testReconciliationMode,
    'candidate-manifest': candidateManifestMode,
    'capability-inventory': capabilityInventoryMode,
    'inventory-validation': inventoryValidationMode,
    'inventory-negative': inventoryNegativeMode,
    'rust-test-list': (options) => rustReceiptMode(options, 'cargo_test_list', true),
    'rust-test-results': (options) => rustReceiptMode(options, 'exact_test', false),
    'deep-audit-physical-causality': deepAuditArtifactMode,
    'deep-audit-production-bootstrap': deepAuditArtifactMode,
    'deep-audit-cognitive-architecture': deepAuditArtifactMode,
    'deep-audit-spatial-architecture': deepAuditArtifactMode,
    'deep-audit-observer-protocol': deepAuditArtifactMode,
    'deep-audit-explanation-engine': deepAuditArtifactMode,
    'deep-audit-integration': deepAuditArtifactMode,
    'deep-audit-performance': deepAuditArtifactMode,
    'deep-audit-security': deepAuditArtifactMode,
    'closure-manifest': closureManifestMode,
    'review-receipt': reviewReceiptMode,
  };
  const handler = modes[mode]; if (!handler) die(`unknown mode: ${mode || '<none>'}`); return handler(args);
}

export function runCli(argv) {
  const mode = argv[0];
  if (mode === '--help' || mode === '-h' || mode === 'help') {
    return `Usage: node tools/audit/validate-capability-audit.mjs <mode> [--flag value]\n\nStable modes:\n  ${[...STABLE_MODES, 'source-blobs', 'candidate-manifest', 'evidence-execution-manifest', 'test-reconciliation', 'capability-inventory'].sort().join('\n  ')}\n`;
  }
  const args = {};
  for (let i = 1; i < argv.length; i++) {
    const token = argv[i];
    if (token.startsWith('--')) {
      const next = argv[i + 1];
      args[token] = next && !next.startsWith('--') ? next : true;
      if (args[token] !== true) i++;
    }
  }
  validateModeInvocation(mode, args);
  return dispatchMode(mode, args);
}
