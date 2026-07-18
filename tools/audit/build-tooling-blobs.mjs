#!/usr/bin/env node
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { execFileSync, spawnSync } from 'node:child_process';

import { ROOT, canonical, resolveRepoArtifact } from './lib/validate-capability-audit-core.mjs';

const SCOPE = 'tools/audit';
const HELP = `usage:
  node tools/audit/build-tooling-blobs.mjs build --run-id ID --source-baseline SHA --tooling-commit SHA --out PATH
  node tools/audit/build-tooling-blobs.mjs verify --input PATH [--run-id ID] [--source-baseline SHA] [--tooling-commit SHA]
`;

function fail(message) {
  throw new Error(message);
}

function option(name, required = true) {
  const index = process.argv.indexOf(name);
  const value = index >= 0 ? process.argv[index + 1] : undefined;
  if (required && (!value || value.startsWith('--'))) fail(`missing ${name}`);
  return value;
}

function git(args, encoding = 'utf8') {
  return execFileSync('git', args, { cwd: ROOT, encoding, stdio: ['ignore', 'pipe', 'ignore'] });
}

function commitAndTree(revision, label) {
  if (typeof revision !== 'string' || !/^[0-9a-f]{40}$/.test(revision)) fail(`invalid ${label}: expected full commit SHA`);
  let commit;
  let tree;
  try {
    commit = git(['rev-parse', '--verify', `${revision}^{commit}`]).trim();
    tree = git(['rev-parse', '--verify', `${revision}^{tree}`]).trim();
  } catch {
    fail(`invalid ${label}: not a commit`);
  }
  if (commit !== revision) fail(`invalid ${label}: revision did not resolve exactly`);
  return { commit, tree };
}

function assertAncestor(ancestor, descendant) {
  const result = spawnSync('git', ['merge-base', '--is-ancestor', ancestor, descendant], { cwd: ROOT, stdio: 'ignore' });
  if (result.status !== 0) fail('tooling commit does not descend from source baseline');
}

function treeEntries(commit) {
  const raw = git(['ls-tree', '-r', '-z', commit, '--', SCOPE]);
  const entries = raw.split('\0').filter(Boolean).map((record) => {
    const match = /^(\d{6}) (\S+) ([0-9a-f]{40})\t(.+)$/.exec(record);
    if (!match) fail('invalid git ls-tree record');
    const [, mode, type, blobOid, repoPath] = match;
    if (type !== 'blob' || !['100644', '100755'].includes(mode)) fail(`unsupported tooling tree entry: ${repoPath}`);
    const bytes = git(['cat-file', 'blob', blobOid], null);
    return {
      path: repoPath,
      mode,
      blob_oid: blobOid,
      byte_sha256: crypto.createHash('sha256').update(bytes).digest('hex'),
    };
  });
  if (entries.length === 0) fail('tooling tree is empty');
  entries.sort((left, right) => left.path < right.path ? -1 : left.path > right.path ? 1 : 0);
  return entries;
}

function worktreeFiles(directory) {
  return fs.readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const absolute = path.join(directory, entry.name);
    if (entry.isSymbolicLink()) fail(`unsupported tooling symlink: ${path.relative(ROOT, absolute)}`);
    if (entry.isDirectory()) return worktreeFiles(absolute);
    if (!entry.isFile()) fail(`unsupported tooling entry: ${path.relative(ROOT, absolute)}`);
    return [path.relative(ROOT, absolute).split(path.sep).join('/')];
  });
}

function assertCleanScope(entries) {
  const expected = entries.map((entry) => entry.path);
  const actual = worktreeFiles(path.join(ROOT, SCOPE)).sort();
  if (canonical(actual) !== canonical(expected)) fail('tooling worktree scope has missing, extra, or temporary files');
  for (const entry of entries) {
    const absolute = path.join(ROOT, entry.path);
    const worktreeOid = git(['hash-object', '--', entry.path]).trim();
    if (worktreeOid !== entry.blob_oid) fail(`tooling worktree differs from pinned commit: ${entry.path}`);
    const executable = (fs.statSync(absolute).mode & 0o111) !== 0;
    if (executable !== (entry.mode === '100755')) fail(`tooling worktree mode differs from pinned commit: ${entry.path}`);
  }
}

function seal(value) {
  return { ...value, receipt_sha256: crypto.createHash('sha256').update(canonical(value)).digest('hex') };
}

function validateManifest(manifest, expected = {}) {
  const keys = [
    'schema_version', 'run_id', 'source_baseline_sha', 'source_baseline_tree_oid',
    'tooling_commit_sha', 'tooling_tree_oid', 'scope', 'entries', 'receipt_sha256',
  ].sort();
  if (!manifest || typeof manifest !== 'object' || Array.isArray(manifest) || canonical(Object.keys(manifest).sort()) !== canonical(keys)) fail('invalid tooling manifest fields');
  if (manifest.schema_version !== 1 || manifest.scope !== SCOPE || typeof manifest.run_id !== 'string' || !manifest.run_id) fail('invalid tooling manifest identity');
  const source = commitAndTree(manifest.source_baseline_sha, 'source baseline');
  const tooling = commitAndTree(manifest.tooling_commit_sha, 'tooling commit');
  if (manifest.source_baseline_tree_oid !== source.tree) fail('source baseline tree mismatch');
  if (manifest.tooling_tree_oid !== tooling.tree) fail('tooling tree mismatch');
  assertAncestor(source.commit, tooling.commit);
  for (const [field, value] of Object.entries(expected)) if (value !== undefined && manifest[field] !== value) fail(`tooling manifest ${field} invocation mismatch`);
  if (!Array.isArray(manifest.entries)) fail('invalid tooling manifest entries');
  const entries = treeEntries(tooling.commit);
  if (canonical(manifest.entries) !== canonical(entries)) fail('tooling manifest entries do not match immutable commit tree');
  const unsigned = Object.fromEntries(Object.entries(manifest).filter(([key]) => key !== 'receipt_sha256'));
  const digest = crypto.createHash('sha256').update(canonical(unsigned)).digest('hex');
  if (manifest.receipt_sha256 !== digest) fail('invalid tooling manifest receipt_sha256');
  assertCleanScope(entries);
  return entries.length;
}

function build() {
  const runId = option('--run-id');
  const sourceBaselineSha = option('--source-baseline');
  const toolingCommitSha = option('--tooling-commit');
  const output = resolveRepoArtifact(option('--out'), { label: 'tooling blobs output', mustExist: false });
  const outputRelative = path.relative(ROOT, output).split(path.sep).join('/');
  if (outputRelative === SCOPE || outputRelative.startsWith(`${SCOPE}/`)) fail('tooling blobs output must be outside tools/audit');
  const source = commitAndTree(sourceBaselineSha, 'source baseline');
  const tooling = commitAndTree(toolingCommitSha, 'tooling commit');
  assertAncestor(source.commit, tooling.commit);
  const entries = treeEntries(tooling.commit);
  assertCleanScope(entries);
  const manifest = seal({
    schema_version: 1,
    run_id: runId,
    source_baseline_sha: source.commit,
    source_baseline_tree_oid: source.tree,
    tooling_commit_sha: tooling.commit,
    tooling_tree_oid: tooling.tree,
    scope: SCOPE,
    entries,
  });
  fs.mkdirSync(path.dirname(output), { recursive: true });
  fs.writeFileSync(output, `${JSON.stringify(manifest, null, 2)}\n`, { encoding: 'utf8', flag: 'wx' });
  process.stdout.write(`tooling_blobs=${entries.length} pinned=${tooling.commit} pass\n`);
}

function verify() {
  const input = resolveRepoArtifact(option('--input'), { label: 'tooling blobs input' });
  const count = validateManifest(JSON.parse(fs.readFileSync(input, 'utf8')), {
    run_id: option('--run-id', false),
    source_baseline_sha: option('--source-baseline', false),
    tooling_commit_sha: option('--tooling-commit', false),
  });
  process.stdout.write(`tooling_blobs=${count} verified pass\n`);
}

try {
  const command = process.argv[2];
  if (command === '--help' || command === '-h') process.stdout.write(HELP);
  else if (command === 'build') build();
  else if (command === 'verify') verify();
  else fail(`unknown command: ${command ?? '<none>'}\n${HELP.trimEnd()}`);
} catch (error) {
  process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
  process.exitCode = 1;
}
