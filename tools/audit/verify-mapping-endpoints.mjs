#!/usr/bin/env node
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { execFileSync } from 'node:child_process';

import { ROOT, canonical, resolveRepoArtifact, runCli } from './lib/validate-capability-audit-core.mjs';

const HELP = `usage: node tools/audit/verify-mapping-endpoints.mjs \\
  --candidate-manifest PATH --test-reconciliation PATH --graph-manifest PATH \\
  --module-receipt PATH [--lsp-reference-receipts PATH,PATH] [--out PATH]\n`;

function fail(message) { throw new Error(message); }
function option(name, required = true) {
  const index = process.argv.indexOf(name);
  const value = index >= 0 ? process.argv[index + 1] : undefined;
  if (required && (!value || value.startsWith('--'))) fail(`missing ${name}`);
  return value;
}
function repoPath(name, required = true) {
  const value = option(name, required);
  return value === undefined ? undefined : resolveRepoArtifact(value, { label: name });
}
function json(file) { return JSON.parse(fs.readFileSync(file, 'utf8')); }
function sha(bytes) { return crypto.createHash('sha256').update(bytes).digest('hex'); }
function relative(file) { return path.relative(ROOT, file).split(path.sep).join('/'); }
function git(args, encoding = 'utf8') { return execFileSync('git', args, { cwd: ROOT, encoding, stdio: ['ignore', 'pipe', 'ignore'] }); }
function sealed(obj, label) {
  const unsigned = Object.fromEntries(Object.entries(obj).filter(([key]) => key !== 'receipt_sha256'));
  if (obj.receipt_sha256 !== sha(canonical(unsigned))) fail(`invalid ${label} receipt_sha256`);
}
function sameIdentity(objects) {
  if (new Set(objects.map((item) => item.run_id)).size !== 1) fail('endpoint run_id mismatch');
  if (new Set(objects.map((item) => item.source_baseline_sha)).size !== 1) fail('endpoint source_baseline_sha mismatch');
}
function evidenceFile(root, repoRelative, label) {
  const file = resolveRepoArtifact(repoRelative, { baseDir: root, label });
  if (path.relative(root, file).startsWith('..')) fail(`${label} escapes evidence root`);
  return file;
}
function readRaw(receipt, root, label) {
  sealed(receipt, label);
  const rawFile = evidenceFile(root, receipt.raw_path, `${label} raw`);
  const bytes = fs.readFileSync(rawFile);
  if (sha(bytes) !== receipt.raw_sha256) fail(`${label} raw_sha256 mismatch`);
  return JSON.parse(bytes);
}
function loadGraphReceipt(file, evidenceRoot, expectedHash = undefined) {
  const bytes = fs.readFileSync(file);
  const receipt = JSON.parse(bytes);
  if (expectedHash !== undefined && receipt.receipt_sha256 !== expectedHash) fail(`graph receipt digest mismatch: ${relative(file)}`);
  if (receipt.index_sha !== receipt.source_baseline_sha) fail(`graph receipt index mismatch: ${receipt.receipt_id}`);
  const raw = readRaw(receipt, evidenceRoot, `graph receipt ${receipt.receipt_id}`);
  if (!Array.isArray(raw.columns) || !Array.isArray(raw.rows) || raw.total !== raw.rows.length) fail(`invalid graph raw rows: ${receipt.receipt_id}`);
  if (receipt.returned !== raw.rows.length || receipt.total !== raw.total || receipt.ordered_result_sha256 !== sha(canonical(raw.rows))) fail(`graph receipt/raw mismatch: ${receipt.receipt_id}`);
  return { receipt, raw };
}
function declarationRows(graphManifest, graphFile, moduleFile) {
  const evidenceRoot = path.dirname(graphFile);
  const receipts = new Map();
  const declarations = [];
  for (const slot of graphManifest.slots) {
    if (!['node', 'edge'].includes(slot.kind)) continue;
    const loaded = loadGraphReceipt(evidenceFile(evidenceRoot, slot.receipt_path, 'graph receipt'), evidenceRoot, slot.receipt_sha256);
    if (loaded.receipt.receipt_id !== slot.receipt_id) fail(`graph slot receipt mismatch: ${slot.receipt_id}`);
    receipts.set(slot.receipt_id, loaded);
    if (slot.kind === 'node') {
      if (canonical(loaded.raw.columns) !== canonical(['qualified_name', 'file_path', 'start_line', 'end_line'])) fail(`invalid declaration columns: ${slot.receipt_id}`);
      for (const row of loaded.raw.rows) declarations.push({ qn: row[0], path: row[1], start: Number(row[2]), end: Number(row[3]), receipt_id: slot.receipt_id });
    }
  }
  const module = loadGraphReceipt(moduleFile, evidenceRoot);
  if (canonical(module.raw.columns) !== canonical(['qualified_name', 'file_path', 'start_line', 'end_line'])) fail('invalid module declaration columns');
  receipts.set(module.receipt.receipt_id, module);
  for (const row of module.raw.rows) declarations.push({ qn: row[0], path: row[1], start: Number(row[2]), end: Number(row[3]), receipt_id: module.receipt.receipt_id });
  return { receipts, declarations, evidenceRoot };
}
function baselineLines(baseline, repoPath) {
  return git(['show', `${baseline}:${repoPath}`]).split('\n');
}
function baselineBlob(baseline, repoPath) {
  try { return git(['rev-parse', `${baseline}:${repoPath}`]).trim(); }
  catch { fail(`path missing at baseline: ${repoPath}`); }
}
function braceEnd(lines, start) {
  let depth = 0;
  let opened = false;
  for (let index = start - 1; index < lines.length; index++) {
    for (const character of lines[index]) {
      if (character === '{') { depth++; opened = true; }
      else if (character === '}') depth--;
    }
    if (opened && depth === 0) return index + 1;
  }
  return null;
}
function genuineCandidate(candidate, context, declarations, baseline) {
  const endpoint = context?.endpoint_declaration;
  if (!endpoint || context.candidate_key !== candidate.candidate_key || endpoint.qualified_name !== candidate.symbol || endpoint.line_start !== candidate.line_start || endpoint.line_end !== candidate.line_end) return false;
  const graphMatch = declarations.some((item) => item.qn === candidate.symbol && item.path === candidate.path && item.start === candidate.line_start && item.end === candidate.line_end);
  if (graphMatch) return true;
  if (!String(endpoint.kind).startsWith('lsp_') || !candidate.lsp_receipt_ids.length) return false;
  const lines = baselineLines(baseline, candidate.path);
  const declarationLine = lines[candidate.line_start - 1] ?? '';
  const expectedName = candidate.symbol.replace(/^impl\s+/, '');
  return declarationLine.includes(expectedName) && braceEnd(lines, candidate.line_start) === candidate.line_end;
}
function declarationAt(declarations, qn, repoPath) {
  const matches = declarations.filter((item) => item.qn === qn && item.path === repoPath && Number.isInteger(item.start) && Number.isInteger(item.end));
  matches.sort((left, right) => (left.end - left.start) - (right.end - right.start));
  return matches[0];
}
function declarationContaining(declarations, repoPath, line) {
  const matches = declarations.filter((item) => item.path === repoPath && item.start <= line && item.end >= line);
  matches.sort((left, right) => (left.end - left.start) - (right.end - right.start));
  return matches[0];
}
function candidateContains(candidate, target) {
  return candidate.path === target.path && candidate.line_start <= target.start && candidate.line_end >= target.end;
}
function verifyGraphMapping(mapping, test, candidate, loaded, declarations) {
  const { columns, rows } = loaded.raw;
  const call = mapping.relation === 'call_path';
  const expected = call
    ? ['caller_qn', 'callee_qn', 'caller_path', 'callee_path', 'line', 'arg_expression']
    : ['source_qn', 'target_qn', 'source_path', 'target_path', 'source_start', 'source_end', 'target_start', 'target_end'];
  if (canonical(columns) !== canonical(expected)) fail(`wrong raw columns for ${mapping.relation}`);
  for (const row of rows) {
    const sourceQn = row[0], targetQn = row[1], sourcePath = row[2], targetPath = row[3];
    if (sourceQn !== test.test_root_qn || sourcePath !== test.path) continue;
    const sourceDeclaration = declarationAt(declarations, sourceQn, sourcePath);
    if (!sourceDeclaration || sourceDeclaration.start > test.line_end || sourceDeclaration.end < test.line_start) continue;
    if (call && row[4] !== '' && !(Number(row[4]) >= test.line_start && Number(row[4]) <= test.line_end)) continue;
    if (!call && !(Number(row[4]) <= test.line_end && Number(row[5]) >= test.line_start)) continue;
    const target = declarationAt(declarations, targetQn, targetPath);
    if (!call && target && (target.start !== Number(row[6]) || target.end !== Number(row[7]))) continue;
    if (target && candidateContains(candidate, target)) return { target, row };
  }
  return null;
}
function loadLspReferences(files, evidenceRoot) {
  const receipts = new Map();
  for (const file of files) {
    const receipt = json(file);
    const raw = readRaw(receipt, evidenceRoot, `LSP receipt ${receipt.receipt_id}`);
    if (!Array.isArray(raw.references) || receipt.complete !== true) fail(`invalid LSP references: ${receipt.receipt_id}`);
    const blob = baselineBlob(receipt.source_baseline_sha, receipt.path);
    if (receipt.baseline_blob_oid !== blob || receipt.worktree_blob_oid !== blob) fail(`LSP receipt baseline mismatch: ${receipt.receipt_id}`);
    receipts.set(receipt.receipt_id, { receipt, raw });
  }
  return receipts;
}
function verifyLspMapping(mapping, test, candidate, loaded, declarations) {
  const query = /^references:(\d+):(\d+)$/.exec(loaded.receipt.parameters?.query ?? '');
  if (!query) fail(`invalid LSP reference query: ${loaded.receipt.receipt_id}`);
  const target = declarationContaining(declarations, loaded.receipt.path, Number(query[1]));
  if (!target || !candidateContains(candidate, target)) return null;
  const row = loaded.raw.references.find((item) => item.path === test.path && item.line >= test.line_start && item.line <= test.line_end);
  return row ? { target, row } : null;
}
function bindingContext(candidate, bindingId, contexts, testId) {
  const binding = candidate.bindings.find((item) => item?.binding_id === bindingId);
  const context = contexts.get(bindingId);
  if (!binding || !context || context.candidate_key !== candidate.candidate_key) fail(`mapping binding mismatch: ${testId}`);
  if (!['primary', 'shared', 'carrier', 'support'].includes(binding.role) || typeof binding.lifecycle_boundary !== 'string' || binding.lifecycle_boundary.length === 0) {
    fail(`invalid mapping binding object: ${bindingId}`);
  }
  if (!context.grouping || context.grouping.lifecycle_boundary !== binding.lifecycle_boundary) {
    fail(`mapping binding context mismatch: ${bindingId}`);
  }
  return { binding, context };
}
function main() {
  const candidateFile = repoPath('--candidate-manifest');
  const reconciliationFile = repoPath('--test-reconciliation');
  const graphFile = repoPath('--graph-manifest');
  const moduleFile = repoPath('--module-receipt');
  runCli(['candidate-manifest', '--input', relative(candidateFile)]);
  runCli(['test-reconciliation', '--input', relative(reconciliationFile), '--candidate-manifest', relative(candidateFile)]);
  const candidates = json(candidateFile), reconciliation = json(reconciliationFile), graphManifest = json(graphFile), moduleReceipt = json(moduleFile);
  sameIdentity([candidates, reconciliation, graphManifest, moduleReceipt]);
  const baseline = candidates.source_baseline_sha;
  git(['rev-parse', '--verify', `${baseline}^{commit}`]);
  const graph = declarationRows(graphManifest, graphFile, moduleFile);
  const lspFiles = (option('--lsp-reference-receipts', false) ?? '').split(',').filter(Boolean).map((item) => resolveRepoArtifact(item, { label: 'LSP reference receipt' }));
  const lsp = loadLspReferences(lspFiles, graph.evidenceRoot);
  sameIdentity([candidates, reconciliation, graphManifest, moduleReceipt, ...[...graph.receipts.values()].map((item) => item.receipt), ...[...lsp.values()].map((item) => item.receipt)]);
  const candidateByKey = new Map(candidates.candidates.map((item) => [item.candidate_key, item]));
  const contexts = new Map((candidates.extensions?.binding_contexts ?? []).map((item) => [item.binding_id, item]));
  const tests = new Map(reconciliation.tests.map((item) => [item.test_id, item]));
  const proofs = [];
  for (const mapping of reconciliation.mappings) {
    const candidate = candidateByKey.get(mapping.candidate_key), test = tests.get(mapping.test_id);
    if (!candidate || !test) fail(`mapping binding mismatch: ${mapping.test_id}`);
    const { binding, context } = bindingContext(candidate, mapping.binding_id, contexts, mapping.test_id);
    if (baselineBlob(baseline, candidate.path) !== candidate.blob_oid || baselineBlob(baseline, test.path) !== test.blob_oid) fail(`endpoint baseline blob mismatch: ${mapping.test_id}`);
    if (!genuineCandidate(candidate, context, graph.declarations, baseline)) fail(`candidate is not a genuine declaration: ${candidate.symbol}`);
    const relationReceipt = mapping.evidence_receipt_ids.find((id) => id !== test.discovered_receipt_id);
    const proof = mapping.relation === 'lsp_reference'
      ? (lsp.has(relationReceipt) ? verifyLspMapping(mapping, test, candidate, lsp.get(relationReceipt), graph.declarations) : null)
      : (graph.receipts.has(relationReceipt) ? verifyGraphMapping(mapping, test, candidate, graph.receipts.get(relationReceipt), graph.declarations) : null);
    if (!proof) fail(`endpoint containment failed: ${test.test_name} -> ${candidate.symbol}`);
    proofs.push({ test_id: test.test_id, candidate_key: candidate.candidate_key, binding: { binding_id: binding.binding_id, capability_id: binding.capability_id, role: binding.role, lifecycle_boundary: binding.lifecycle_boundary }, binding_context: context.grouping, relation: mapping.relation, receipt_id: relationReceipt, target_qn: proof.target.qn, target_path: proof.target.path, target_start: proof.target.start, target_end: proof.target.end, evidence_row_sha256: sha(canonical(proof.row)) });
  }
  const report = { schema_version: 1, run_id: candidates.run_id, source_baseline_sha: baseline, mapping_count: proofs.length, proofs, verdict: 'pass' };
  const output = option('--out', false);
  if (output) fs.writeFileSync(resolveRepoArtifact(output, { label: 'endpoint output', mustExist: false }), `${JSON.stringify(report, null, 2)}\n`);
  process.stdout.write(`mapping_endpoints=${proofs.length} pass\n`);
}

try {
  if (process.argv[2] === '--help' || process.argv[2] === '-h') process.stdout.write(HELP);
  else main();
} catch (error) {
  process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
  process.exitCode = 1;
}
