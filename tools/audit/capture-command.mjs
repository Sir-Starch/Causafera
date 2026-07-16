#!/usr/bin/env node
import crypto from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

import { ROOT, assertSourceDefinitionArgv, canonical, resolveRepoArtifact, runCli, sha256Json } from './lib/validate-capability-audit-core.mjs';
import { assertStableExecutableDigest, containsSensitiveCarrier, executableDigest } from './lib/capture-security.mjs';

function fail(message) {
  process.stderr.write(`${message}\n`);
  process.exit(2);
}

const USAGE = `Usage: node tools/audit/capture-command.mjs \\
  --run-id RUN_ID --source-baseline SHA --receipt-id RECEIPT_ID \\
  --adapter ADAPTER --stdout PATH --stderr PATH --receipt PATH \\
  [--phase PHASE --attempt-id ATTEMPT_ID] [--workload NAME --purpose PURPOSE] \\
  -- COMMAND [ARG ...]\n`;

function parse(argv) {
  const separator = argv.indexOf('--');
  if (separator < 0 || separator === argv.length - 1) fail('capture requires -- followed by inner argv');
  const options = {};
  for (let index = 0; index < separator; index += 2) {
    const flag = argv[index];
    const value = argv[index + 1];
    if (!flag?.startsWith('--') || value === undefined || value.startsWith('--')) fail(`invalid capture option: ${flag ?? '<missing>'}`);
    options[flag] = value;
  }
  for (const flag of ['--run-id', '--source-baseline', '--receipt-id', '--adapter', '--stdout', '--stderr', '--receipt', '--bootstrap']) {
    if (!options[flag]) fail(`missing ${flag}`);
  }
  return { options, innerArgv: argv.slice(separator + 1) };
}

function byteHash(bytes) {
  return crypto.createHash('sha256').update(bytes).digest('hex');
}

function loadAdapter(name) {
  const registry = JSON.parse(fs.readFileSync(path.join(ROOT, 'tools/audit/adapter-contracts.json'), 'utf8'));
  const adapter = registry.adapters?.find((candidate) => candidate.name === name);
  if (!adapter) fail(`capture adapter is not registered: ${name}`);
  return adapter;
}

function loadTrustedBootstrap(relativePath, runId, sourceBaseline) {
  const absolute = resolveRepoArtifact(relativePath, { label: 'capture bootstrap' });
  const bootstrap = JSON.parse(fs.readFileSync(absolute, 'utf8'));
  if (bootstrap.run_id !== runId || bootstrap.source_baseline_sha !== sourceBaseline) {
    fail('capture trusted bootstrap run/baseline mismatch');
  }
  const git = fs.realpathSync('/usr/bin/git');
  const commit = spawnSync(git, ['rev-parse', `${sourceBaseline}^{commit}`], { cwd: ROOT, encoding: 'utf8' });
  const tree = spawnSync(git, ['rev-parse', `${sourceBaseline}^{tree}`], { cwd: ROOT, encoding: 'utf8' });
  if (commit.status !== 0 || tree.status !== 0 || commit.stdout.trim() !== sourceBaseline
      || tree.stdout.trim() !== bootstrap.source_baseline_tree_oid) {
    fail('capture trusted bootstrap baseline/tree mismatch');
  }
  if (fs.realpathSync(bootstrap.audit_worktree_path) !== fs.realpathSync(ROOT)) {
    fail('capture trusted bootstrap worktree mismatch');
  }
  if (bootstrap.original_worktree_path !== undefined) {
    if (!path.isAbsolute(bootstrap.original_worktree_path)) fail('capture trusted frozen baseline worktree path is invalid');
    const sourceHead = spawnSync(git, ['rev-parse', 'HEAD'], { cwd: bootstrap.original_worktree_path, encoding: 'utf8' });
    const sourceTree = spawnSync(git, ['rev-parse', 'HEAD^{tree}'], { cwd: bootstrap.original_worktree_path, encoding: 'utf8' });
    const sourceStatus = spawnSync(git, ['status', '--porcelain=v1'], { cwd: bootstrap.original_worktree_path, encoding: 'utf8' });
    if (sourceHead.status !== 0 || sourceTree.status !== 0 || sourceStatus.status !== 0 || sourceHead.stdout.trim() !== sourceBaseline || sourceTree.stdout.trim() !== bootstrap.source_baseline_tree_oid || sourceStatus.stdout.trim() !== '') fail('capture trusted frozen baseline worktree is not clean at declared revision/tree');
  }
  return bootstrap;
}

function runnerKey(bootstrap) {
  const attestation = bootstrap.runner_attestation;
  if (attestation === undefined) return null;
  const trustedRoot = path.join(ROOT, '.omo', 'audit-trust') + path.sep;
  if (!attestation || attestation.version !== 1 || !path.isAbsolute(attestation.key_path)) fail('capture trusted runner attestation is invalid');
  const keyPath = fs.realpathSync(attestation.key_path);
  const stat = fs.statSync(keyPath);
  if (!keyPath.startsWith(trustedRoot) || !stat.isFile() || (stat.mode & 0o077) !== 0 || byteHash(fs.readFileSync(keyPath)) !== attestation.key_sha256) fail('capture trusted runner key is invalid');
  const unsignedBootstrap = Object.fromEntries(Object.entries(bootstrap).filter(([key]) => key !== 'runner_attestation'));
  const signature = crypto.createHmac('sha256', fs.readFileSync(keyPath)).update(canonical(unsignedBootstrap)).digest('hex');
  if (signature !== attestation.bootstrap_hmac_sha256) fail('capture trusted runner bootstrap signature is invalid');
  return fs.readFileSync(keyPath);
}

function receiptAttestationPayload(receipt) {
  return { run_id: receipt.run_id, source_baseline_sha: receipt.source_baseline_sha, receipt_id: receipt.receipt_id, adapter: receipt.adapter, argv: receipt.argv, cwd: receipt.cwd, exit_code: receipt.exit_code, stdout_sha256: receipt.stdout_sha256, stderr_sha256: receipt.stderr_sha256 };
}

function safePathDirectories(rawPath) {
  const directories = [];
  for (const candidate of String(rawPath ?? '').split(path.delimiter)) {
    if (!path.isAbsolute(candidate)) continue;
    try {
      const resolved = fs.realpathSync(candidate);
      const stat = fs.statSync(resolved);
      const relative = path.relative(ROOT, resolved);
      if (!stat.isDirectory() || (stat.mode & 0o022) !== 0) continue;
      if (relative === '' || (!path.isAbsolute(relative) && !relative.startsWith('..'))) continue;
      if (!directories.includes(resolved)) directories.push(resolved);
    } catch {
      // Ignore missing, unreadable, and dangling PATH entries.
    }
  }
  return directories;
}

function resolveExecutable(command, safeDirectories) {
  if (command === 'node' || command === path.basename(process.execPath)) return fs.realpathSync(process.execPath);
  if (command.includes('/')) return fs.realpathSync(resolveRepoArtifact(command, { label: 'capture executable' }));
  for (const directory of safeDirectories) {
    const candidate = path.join(directory, command);
    try {
      fs.accessSync(candidate, fs.constants.X_OK);
      const resolved = fs.realpathSync(candidate);
      const stat = fs.statSync(resolved);
      if (stat.isFile() && (stat.mode & 0o022) === 0) return resolved;
    } catch {
      // Continue to the next trusted PATH entry.
    }
  }
  fail(`capture executable not found on trusted PATH: ${command}`);
}

function assertTrustedExecutable(executable) {
  let current = executable;
  while (true) {
    const stat = fs.lstatSync(current);
    if ((stat.mode & 0o022) !== 0) fail(`capture executable path is group/world writable: ${current}`);
    const parent = path.dirname(current);
    if (parent === current) break;
    current = parent;
  }
}

function assertSafeArgv(argv) {
  const secretFlag = /^(?:--?)?(?:api[-_]?key|authorization|password|secret|token)(?:=|$)/i;
  const secretAssignment = /^(?:[A-Z0-9_]*(?:API_KEY|AUTHORIZATION|PASSWORD|SECRET|TOKEN))=/i;
  if (argv.some((argument) => secretFlag.test(argument) || secretAssignment.test(argument) || containsSensitiveCarrier(argument))) {
    fail('capture argv contains a secret-prone argument; pass secrets through an approved external mechanism');
  }
}

function assertSafeOutput(stdout, stderr) {
  if (containsSensitiveCarrier(stdout.toString('utf8')) || containsSensitiveCarrier(stderr.toString('utf8'))) {
    fail('capture output contains a secret-prone value; no sidecars were persisted');
  }
}

function assertAdapterArgv(adapter, argv, sourceBaseline) {
  const command = path.basename(argv[0]);
  const isNode = argv[0] === 'node' || command.startsWith('node');
  const isChecker = isNode && argv[1]?.endsWith('tools/audit/validate-capability-audit.mjs');
  let accepted = false;
  if (adapter === 'source_definition') {
    assertSourceDefinitionArgv(argv, sourceBaseline);
    accepted = true;
  }
  else if (adapter === 'git_baseline') accepted = command === 'git' && argv[1] === 'rev-parse' && argv.slice(2).includes(`${sourceBaseline}^{commit}`) && argv.slice(2).includes(`${sourceBaseline}^{tree}`);
  else if (adapter === 'exact_test') accepted = command === 'cargo' && argv[1] === 'test' && argv.includes('--exact');
  else if (adapter === 'cargo_test_list') accepted = command === 'cargo' && argv[1] === 'test' && argv.includes('--list');
  else if (adapter === 'audit_checker') accepted = isChecker;
  else if (adapter === 'git_preflight') accepted = isChecker && argv[2] === 'preflight';
  else if (adapter === 'git_diff_check') accepted = command === 'git' && argv[1] === 'diff' && argv.includes('--check');
  else if (adapter === 'git_scope') accepted = isChecker && argv[2] === 'scope';
  else if (adapter === 'rust_ci') accepted = command === 'cargo' && argv[1] === 'run' && argv.includes('xtask') && argv.includes('ci');
  else if (adapter === 'pnpm_install') accepted = command === 'pnpm' && argv[1] === 'install';
  else if (adapter === 'pnpm_lint') accepted = command === 'pnpm' && argv[1] === 'lint';
  else if (adapter === 'pnpm_typecheck') accepted = command === 'pnpm' && argv[1] === 'typecheck';
  else if (adapter === 'pnpm_build') accepted = command === 'pnpm' && argv[1] === 'build';
  else if (adapter === 'cargo_metadata') accepted = command === 'cargo' && argv[1] === 'metadata';
  else if (adapter.endsWith('_test') || ['benchmark_diagnostic','representative_benchmark'].includes(adapter)) {
    accepted = command === 'cargo' && argv[1] === 'test';
  } else if (['documentation_contract','confirmed_violation','explanation_metric','observer_projection','production_composition'].includes(adapter)) {
    accepted = isChecker || (command === 'cargo' && ['test','run'].includes(argv[1]));
  }
  if (!accepted) fail(`capture executable/adapter argv policy mismatch for ${adapter}`);
}

function outputPath(relativePath, runId, label) {
  const allowedPrefixes = [`.omo/evidence/${runId}/`, `tools/audit/fixtures/tmp/${runId}/`];
  if (!allowedPrefixes.some((prefix) => relativePath.startsWith(prefix))) {
    fail(`${label} must be under the run evidence root or run fixture temp root`);
  }
  const absolute = resolveRepoArtifact(relativePath, { label, mustExist: false });
  if (fs.existsSync(absolute)) fail(`${label} already exists; capture outputs are immutable`);
  fs.mkdirSync(path.dirname(absolute), { recursive: true, mode: 0o700 });
  resolveRepoArtifact(path.relative(ROOT, path.dirname(absolute)), { label: `${label} directory`, regularFile: false });
  return absolute;
}

function writeExclusive(filePath, bytes) {
  const flags = fs.constants.O_CREAT | fs.constants.O_EXCL | fs.constants.O_WRONLY
    | (fs.constants.O_NOFOLLOW ?? 0);
  const descriptor = fs.openSync(filePath, flags, 0o600);
  try {
    fs.writeFileSync(descriptor, bytes);
    fs.fsyncSync(descriptor);
  } finally {
    fs.closeSync(descriptor);
  }
}

function signalExitCode(signal) {
  const signalNumber = signal === null ? undefined : os.constants.signals[signal];
  return Number.isInteger(signalNumber) ? 128 + signalNumber : 255;
}

function projection(adapter, adapterContract, sourceBaseline, innerArgv, executable, executableHash, exitCode, signal) {
  const operation = innerArgv[1]?.endsWith('validate-capability-audit.mjs') ? (innerArgv[2] ?? 'unknown') : path.basename(innerArgv[0]);
  const mode = [
    `operation=${operation}`,
    `outcome=${exitCode === 0 && signal === null ? 'pass' : 'fail'}`,
    `executable=${path.basename(executable)}`,
    `executable_sha256=${executableHash}`,
    `evidence_class=${adapterContract.evidence_class}`,
    `state_anchor=${adapterContract.state_anchor_policy}`,
  ].join(';');
  const summary = `exit=${exitCode};signal=${signal ?? 'none'}`;
  const result = { adapter, mode, summary, commit_sha: null, tree_oid: null };
  if (adapter === 'git_baseline') {
    const git = resolveExecutable('git', safePathDirectories(process.env.PATH));
    result.commit_sha = spawnSync(git, ['rev-parse', `${sourceBaseline}^{commit}`], { cwd: ROOT, encoding: 'utf8' }).stdout.trim();
    result.tree_oid = spawnSync(git, ['rev-parse', `${sourceBaseline}^{tree}`], { cwd: ROOT, encoding: 'utf8' }).stdout.trim();
  }
  return result;
}

if (process.argv.length === 3 && ['--help', '-h'].includes(process.argv[2])) {
  process.stdout.write(USAGE);
  process.exit(0);
}

const { options, innerArgv } = parse(process.argv.slice(2));
const adapterContract = loadAdapter(options['--adapter']);
const bootstrap = loadTrustedBootstrap(options['--bootstrap'], options['--run-id'], options['--source-baseline']);
const runnerSigningKey = runnerKey(bootstrap);
assertSafeArgv(innerArgv);
assertAdapterArgv(options['--adapter'], innerArgv, options['--source-baseline']);
const stdoutPath = outputPath(options['--stdout'], options['--run-id'], 'capture stdout');
const stderrPath = outputPath(options['--stderr'], options['--run-id'], 'capture stderr');
const receiptPath = outputPath(options['--receipt'], options['--run-id'], 'capture receipt');
if (new Set([stdoutPath, stderrPath, receiptPath]).size !== 3) fail('capture stdout, stderr, and receipt paths must be distinct');

const persistedEnvironment = { CARGO_TERM_COLOR: 'never', HOME: 'logical', LC_ALL: 'C', NO_COLOR: '1', TZ: 'UTC' };
const safeDirectories = safePathDirectories(process.env.PATH);
const executable = resolveExecutable(innerArgv[0], safeDirectories);
assertTrustedExecutable(executable);
const executableHashBefore = executableDigest(executable);
const temporaryHome = fs.mkdtempSync(path.join(os.tmpdir(), 'ontopolis-audit-home-'));
fs.chmodSync(temporaryHome, 0o700);
const childEnvironment = { PATH: safeDirectories.join(path.delimiter), ...persistedEnvironment, HOME: temporaryHome };
const frozenBaselineExecution = ['cargo_test_list', 'exact_test', 'source_definition', 'git_baseline'].includes(options['--adapter']) && typeof bootstrap.original_worktree_path === 'string';
const executionCwd = frozenBaselineExecution ? bootstrap.original_worktree_path : ROOT;
const invocation = innerArgv[0] === 'cargo' ? 'cargo' : executable;
let result;
try {
  result = spawnSync(invocation, innerArgv.slice(1), { cwd: executionCwd, env: childEnvironment, encoding: null, maxBuffer: 64 * 1024 * 1024 });
} finally {
  fs.rmSync(temporaryHome, { recursive: true, force: true });
}
if (result.error) fail(`capture spawn failed: ${result.error.message}`);
const stdout = result.stdout ?? Buffer.alloc(0);
const stderr = result.stderr ?? Buffer.alloc(0);
assertSafeOutput(stdout, stderr);
try { assertStableExecutableDigest(executable, executableHashBefore); }
catch (error) { fail(error.message); }
if (invocation === 'cargo' && resolveExecutable('cargo', safeDirectories) !== executable) {
  fail('capture cargo executable resolution changed during execution');
}
const signal = result.signal ?? null;
const exitCode = Number.isInteger(result.status) ? result.status : signalExitCode(signal);
const stdoutHash = byteHash(stdout);
const stderrHash = byteHash(stderr);
writeExclusive(stdoutPath, stdout);
writeExclusive(stderrPath, stderr);

const deterministicProjection = projection(options['--adapter'], adapterContract, options['--source-baseline'], innerArgv, executable, executableHashBefore, exitCode, signal);
const receipt = {
  schema_version: 1,
  run_id: options['--run-id'],
  source_baseline_sha: options['--source-baseline'],
  attempt_id: options['--attempt-id'] ?? null,
  receipt_id: options['--receipt-id'],
  phase: options['--phase'] ?? 'collection',
  adapter: options['--adapter'],
  argv: innerArgv,
  cwd: frozenBaselineExecution ? '$FROZEN_BASELINE' : '.',
  environment: persistedEnvironment,
  tool_versions: { node: process.version },
  exit_code: exitCode,
  stdout_path: options['--stdout'],
  stdout_sha256: stdoutHash,
  stderr_path: options['--stderr'],
  stderr_sha256: stderrHash,
  deterministic_projection: deterministicProjection,
  projection_sha256: sha256Json(deterministicProjection),
  workload: { name: options['--workload'] ?? deterministicProjection.mode, purpose: options['--purpose'] ?? 'audit' },
};
if (runnerSigningKey !== null) {
  const unsigned = receiptAttestationPayload(receipt);
  receipt.deterministic_projection.mode += `;runner_hmac_sha256=${crypto.createHmac('sha256', runnerSigningKey).update(canonical(unsigned)).digest('hex')}`;
  receipt.projection_sha256 = sha256Json(receipt.deterministic_projection);
}
receipt.receipt_sha256 = sha256Json(receipt);
writeExclusive(receiptPath, `${JSON.stringify(receipt, null, 2)}\n`);
const validationArgs = ['capture', '--adapter', receipt.adapter, '--run-id', receipt.run_id, '--receipt-id', receipt.receipt_id, '--receipt', options['--receipt'], '--bootstrap', options['--bootstrap']];
if (runnerSigningKey === null) validationArgs.push('--fixture-mode', 'true');
runCli(validationArgs);
process.exit(exitCode);
