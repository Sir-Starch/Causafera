#!/usr/bin/env node
import crypto from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { execFileSync, spawn, spawnSync } from 'node:child_process';
import { fileURLToPath, pathToFileURL } from 'node:url';

import {
  LEGACY_BASELINE_COORDS_PATH,
  LEGACY_BASELINE_GRAPH_PROJECT,
  LEGACY_BASELINE_TYPES_PACKAGE,
  LEGACY_BASELINE_TYPES_SOURCE_PATH,
} from './lib/validate-capability-audit-core.mjs';

const ROOT = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..', '..');
const CATALOG = path.join(ROOT, 'tools/audit/capability-catalog.json');
const CANDIDATE_PATH = /^(?:crates\/.*\.rs|apps\/observer\/(?:src\/.*\.(?:ts|tsx)|vite\.config\.ts)|packages\/observer-protocol\/src\/index\.ts|proto\/.*\.proto)$/;
const RUST_PATH = /^crates\/.*\.rs$/;
const BASELINE = /^[0-9a-f]{40}$/;
const GRAPH_CLI = '/home/lorfit/.local/bin/codebase-memory-mcp';
const GRAPH_PROJECT = LEGACY_BASELINE_GRAPH_PROJECT;
const OMO_CACHE = '/home/lorfit/.codex/plugins/cache/sisyphuslabs/omo';
const usage = 'usage: node tools/audit/produce-task4-evidence.mjs --run-id <run-id> --source-baseline <40-hex-sha> --out <.omo/evidence/run-id>\n       node tools/audit/produce-task4-evidence.mjs --lsp-lifecycle-smoke\n';

function fail(message) { throw new Error(message); }
function compareVersions(left, right) {
  const leftParts = left.split('.').map(Number), rightParts = right.split('.').map(Number);
  for (let index = 0; index < 3; index++) if (leftParts[index] !== rightParts[index]) return rightParts[index] - leftParts[index];
  return 0;
}
function discoverLspProvider() {
  let versions;
  try { versions = fs.readdirSync(OMO_CACHE, { withFileTypes: true }).filter((entry) => entry.isDirectory() && /^\d+\.\d+\.\d+$/.test(entry.name)).map((entry) => entry.name); }
  catch { fail('no OMO cache is available for LSP collection'); }
  const candidates = versions.map((version) => ({ version, path: path.join(OMO_CACHE, version, 'components/lsp-daemon/dist/cli.js') })).filter((candidate) => fs.existsSync(candidate.path) && fs.statSync(candidate.path).isFile()).sort((left, right) => compareVersions(left.version, right.version));
  if (!candidates.length) fail('no validated OMO LSP CLI is installed');
  const provider = candidates[0];
  if (candidates.filter((candidate) => candidate.version === provider.version).length !== 1) fail(`ambiguous newest OMO LSP CLI version: ${provider.version}`);
  return { ...provider, sha256: crypto.createHash('sha256').update(fs.readFileSync(provider.path)).digest('hex') };
}
const LSP_PROVIDER = discoverLspProvider();
const LSP_MCP = LSP_PROVIDER.path;
function childEnvironment(extra = {}) {
  const environment = {};
  for (const name of ['PATH', 'HOME', 'RUSTUP_HOME', 'CARGO_HOME', 'CARGO_TERM_COLOR', 'LC_ALL', 'LANG', 'NO_COLOR', 'TZ']) if (process.env[name] !== undefined) environment[name] = process.env[name];
  return { ...environment, ...extra };
}
function verifiedProviderPath(provider) {
  const executable = fs.realpathSync(provider.path);
  if (executable !== provider.path || bytesDigest(fs.readFileSync(executable)) !== provider.sha256) fail(`LSP provider changed before spawn: ${provider.name ?? provider.version}`);
  return executable;
}
function discoverRustAnalyzer() {
  const executable = execFileSync('rustup', ['which', 'rust-analyzer'], { cwd: ROOT, encoding: 'utf8' }).trim();
  const probe = spawnSync(executable, ['--version'], { cwd: ROOT, encoding: 'utf8' });
  if (probe.status !== 0) fail(`rust-analyzer is unavailable: ${probe.stderr || probe.stdout}`);
  return { name: 'rust-analyzer', version: probe.stdout.trim(), path: executable, sha256: crypto.createHash('sha256').update(fs.readFileSync(executable)).digest('hex'), transport: 'stdio' };
}
const RUST_LSP_PROVIDER = discoverRustAnalyzer();
function canonical(value) {
  if (Array.isArray(value)) return `[${value.map(canonical).join(',')}]`;
  if (value && typeof value === 'object') return `{${Object.keys(value).sort().map((key) => `${JSON.stringify(key)}:${canonical(value[key])}`).join(',')}}`;
  return JSON.stringify(value);
}
function digest(value) { return crypto.createHash('sha256').update(canonical(value)).digest('hex'); }
function bytesDigest(bytes) { return crypto.createHash('sha256').update(bytes).digest('hex'); }
function seal(value) { return { ...value, receipt_sha256: digest(value) }; }
function relative(file) { return path.relative(ROOT, file).split(path.sep).join('/'); }
function git(args, encoding = 'utf8') { return execFileSync('git', args, { cwd: ROOT, encoding, stdio: ['ignore', 'pipe', 'pipe'] }); }
function mkdir(directory) { fs.mkdirSync(directory, { recursive: true }); }
function write(file, value) { mkdir(path.dirname(file)); fs.writeFileSync(file, `${JSON.stringify(value, null, 2)}\n`, { encoding: 'utf8' }); }
function createRunnerTrust(runId) {
  const directory = path.join(ROOT, '.omo', 'audit-trust'); mkdir(directory);
  const keyPath = path.resolve(directory, `${runId}.key`);
  if (!keyPath.startsWith(`${directory}${path.sep}`)) fail('runner trust anchor path escapes audit-trust');
  if (fs.existsSync(keyPath)) {
    const stat = fs.lstatSync(keyPath);
    if (!stat.isFile() || stat.isSymbolicLink() || (stat.mode & 0o777) !== 0o600) fail('existing runner trust anchor is not a regular 0600 file');
    const key = fs.readFileSync(keyPath);
    if (key.length !== 32) fail('existing runner trust anchor has an invalid length');
    return { key, key_path: keyPath, key_sha256: bytesDigest(key) };
  }
  const key = crypto.randomBytes(32);
  fs.writeFileSync(keyPath, key, { encoding: 'binary', mode: 0o600, flag: 'wx' });
  fs.chmodSync(keyPath, 0o600);
  return { key, key_path: keyPath, key_sha256: bytesDigest(key) };
}
function attestBootstrap(bootstrap, trust) {
  const bootstrap_hmac_sha256 = crypto.createHmac('sha256', trust.key).update(canonical(bootstrap)).digest('hex');
  return { ...bootstrap, runner_attestation: { version: 1, key_path: trust.key_path, key_sha256: trust.key_sha256, bootstrap_hmac_sha256 } };
}
function reproducibilityEntries(output) {
  const entries = [];
  const visit = (directory) => {
    for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
      const absolute = path.join(directory, entry.name);
      if (entry.isDirectory()) visit(absolute);
      else if (entry.isFile()) {
        const entryPath = path.relative(output, absolute).split(path.sep).join('/');
        if (entryPath !== 'task-4-reproducibility-manifest.json') entries.push({ path: entryPath, sha256: bytesDigest(fs.readFileSync(absolute)) });
      }
    }
  };
  visit(output);
  return entries.sort((left, right) => left.path.localeCompare(right.path));
}
function option(name) { const index = process.argv.indexOf(name); const value = index < 0 ? undefined : process.argv[index + 1]; if (!value || value.startsWith('--')) fail(`missing ${name}`); return value; }
function sourceFiles(baseline) {
  const raw = git(['ls-tree', '-r', '-z', baseline], null);
  const files = raw.toString('utf8').split('\0').filter(Boolean).map((record) => {
    const match = /^\d+ blob ([0-9a-f]{40})\t(.+)$/.exec(record);
    if (!match) fail(`invalid ls-tree record: ${record}`);
    return { path: match[2], blob_oid: match[1] };
  }).filter((entry) => CANDIDATE_PATH.test(entry.path)).sort((left, right) => left.path.localeCompare(right.path));
  const counts = { rs: files.filter((file) => RUST_PATH.test(file.path)).length, ts: files.filter((file) => file.path.endsWith('.ts')).length, tsx: files.filter((file) => file.path.endsWith('.tsx')).length, proto: files.filter((file) => file.path.endsWith('.proto')).length };
  if (files.length !== 132 || canonical(counts) !== canonical({ rs: 105, ts: 5, tsx: 12, proto: 10 })) fail(`baseline candidate split drift: ${JSON.stringify(counts)}`);
  return files;
}
function sourceText(baseline, repoPath) { return git(['show', `${baseline}:${repoPath}`]); }
function graphCli(command, args = []) {
  const stdout = execFileSync(GRAPH_CLI, ['cli', command, '--project', GRAPH_PROJECT, ...args], { cwd: ROOT, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] });
  return JSON.parse(stdout);
}
let lspRuntimeHome = null;
let lspSession = null;
let rustLspSession = null;
function startLspRuntime() {
  if (lspRuntimeHome !== null) fail('LSP runtime home is already active');
  lspRuntimeHome = fs.mkdtempSync(path.join(os.tmpdir(), 'causafera-audit-lsp-'));
  fs.chmodSync(lspRuntimeHome, 0o700);
}
function lspDaemonDir() { if (lspRuntimeHome === null) fail('LSP runtime home is not active'); return path.join(lspRuntimeHome, '.omo', 'lsp-daemon'); }
function lspEnvironment() { return childEnvironment({ OMO_LSP_DAEMON_DIR: lspDaemonDir() }); }
function scopedDaemonPids() { const pids = []; const visit = (directory) => { if (!fs.existsSync(directory)) return; for (const entry of fs.readdirSync(directory, { withFileTypes: true })) { const absolute = path.join(directory, entry.name); if (entry.isDirectory()) visit(absolute); else if (entry.isFile() && entry.name === 'daemon.pid') { const pid = Number(fs.readFileSync(absolute, 'utf8').trim()); if (Number.isSafeInteger(pid) && pid > 1) pids.push(pid); } } }; visit(lspDaemonDir()); return pids; }
function scopedRustAnalyzerPids() { return scopedDaemonPids().flatMap((pid) => [pid, ...scopedProcessTree(pid)]).filter((pid) => { try { return /(?:^|\0)(?:\/[^\0]+\/)?rust-analyzer(?:\0|$)/.test(fs.readFileSync(`/proc/${pid}/cmdline`, 'utf8')); } catch { return false; } }); }
function assertScopedRustAnalyzerBound() { const analyzers = scopedRustAnalyzerPids(); if (analyzers.length > 1) fail(`scoped LSP runtime exceeded one rust-analyzer: ${analyzers.join(',')}`); }
async function stopScopedRustAnalyzers() { const pids = scopedRustAnalyzerPids(); for (const pid of pids) try { process.kill(pid, 'SIGTERM'); } catch { /* Process already exited. */ } for (let attempt = 0; attempt < 10 && scopedRustAnalyzerPids().length; attempt++) await pause(50); for (const pid of scopedRustAnalyzerPids()) try { process.kill(pid, 'SIGKILL'); } catch { /* Process already exited. */ } for (let attempt = 0; attempt < 10 && scopedRustAnalyzerPids().length; attempt++) await pause(50); if (scopedRustAnalyzerPids().length) fail('scoped rust-analyzer did not terminate between workspace groups'); }
function scopedProcessTree(rootPid) {
  const parents = new Map();
  for (const entry of fs.readdirSync('/proc')) {
    if (!/^\d+$/.test(entry)) continue;
    try {
      const stat = fs.readFileSync(`/proc/${entry}/stat`, 'utf8');
      const parent = Number(stat.slice(stat.lastIndexOf(')') + 2).split(' ')[1]);
      const children = parents.get(parent) ?? []; children.push(Number(entry)); parents.set(parent, children);
    } catch { /* Process exited while enumerating. */ }
  }
  const result = [], visit = (pid) => { for (const child of parents.get(pid) ?? []) { visit(child); result.push(child); } };
  visit(rootPid); return result;
}
function processAlive(pid) { try { process.kill(pid, 0); return true; } catch (error) { if (error?.code === 'ESRCH') return false; throw error; } }
function pause(milliseconds) { return new Promise((resolve) => setTimeout(resolve, milliseconds)); }
function startRustLspSession(root) {
  if (rustLspSession !== null) return rustLspSession;
  const child = spawn(verifiedProviderPath(RUST_LSP_PROVIDER), [], { cwd: root, env: childEnvironment(), stdio: ['pipe', 'pipe', 'pipe'] });
  let buffer = Buffer.alloc(0), nextId = 1, closed = false, initialized = false, shutdown = 'pending';
  const pending = new Map(), opened = new Set(), stderr = [];
  const rejectPending = (error) => { for (const { reject, timer } of pending.values()) { clearTimeout(timer); reject(error); } pending.clear(); };
  const send = (message) => { if (closed || !child.stdin.writable) fail('direct rust-analyzer session is closed'); const body = Buffer.from(JSON.stringify(message)); child.stdin.write(Buffer.concat([Buffer.from(`Content-Length: ${body.length}\r\n\r\n`), body])); };
  const parse = () => { for (;;) { const split = buffer.indexOf('\r\n\r\n'); if (split < 0) return; const header = buffer.subarray(0, split).toString('ascii'), match = /(?:^|\r\n)Content-Length: (\d+)(?:\r\n|$)/i.exec(header); if (!match) { rejectPending(new Error('rust-analyzer response lacks Content-Length')); return; } const length = Number(match[1]), bodyStart = split + 4; if (buffer.length < bodyStart + length) return; const body = buffer.subarray(bodyStart, bodyStart + length); buffer = buffer.subarray(bodyStart + length); let message; try { message = JSON.parse(body.toString('utf8')); } catch { rejectPending(new Error('rust-analyzer response is not JSON')); continue; } if (Object.hasOwn(message, 'id') && pending.has(message.id)) { const { resolve, reject, timer } = pending.get(message.id); pending.delete(message.id); clearTimeout(timer); if (message.error) reject(new Error(`rust-analyzer ${message.error.message ?? 'request failed'}`)); else resolve(message.result); } } };
  child.stdout.on('data', (chunk) => { buffer = Buffer.concat([buffer, chunk]); parse(); });
  child.stderr.on('data', (chunk) => stderr.push(chunk.toString('utf8')));
  child.on('error', rejectPending);
  child.on('close', (code, signal) => rejectPending(new Error(`rust-analyzer session closed: exit=${code};signal=${signal ?? 'none'};stderr=${stderr.join('')}`)));
  const request = (method, params, timeoutMs = 120000) => new Promise((resolve, reject) => { const id = nextId++; const timer = setTimeout(() => { pending.delete(id); reject(new Error(`rust-analyzer ${method} timed out after ${timeoutMs}ms`)); }, timeoutMs); pending.set(id, { resolve, reject, timer }); try { send({ jsonrpc: '2.0', id, method, params }); } catch (error) { clearTimeout(timer); pending.delete(id); reject(error); } });
  const notify = (method, params) => send({ jsonrpc: '2.0', method, params });
  const session = {
    async initialize() { if (initialized) return; await request('initialize', { processId: null, rootUri: pathToFileURL(root).href, workspaceFolders: [{ uri: pathToFileURL(root).href, name: path.basename(root) }], capabilities: {} }); notify('initialized', {}); initialized = true; },
    async open(file) { if (opened.has(file)) return; notify('textDocument/didOpen', { textDocument: { uri: pathToFileURL(file).href, languageId: 'rust', version: 1, text: fs.readFileSync(file, 'utf8') } }); opened.add(file); },
    async symbols(file) { await this.open(file); return request('textDocument/documentSymbol', { textDocument: { uri: pathToFileURL(file).href } }); },
    async references(file, line, character) { await this.open(file); return request('textDocument/references', { textDocument: { uri: pathToFileURL(file).href }, position: { line: line - 1, character }, context: { includeDeclaration: false } }); },
    async close() { if (closed) return { sessions: 1, shutdown, retained_processes: 0 }; try { await request('shutdown', null, 30000); shutdown = 'confirmed'; notify('exit'); } finally { closed = true; child.stdin.end(); for (let attempt = 0; attempt < 20 && processAlive(child.pid); attempt++) await pause(50); if (processAlive(child.pid)) { child.kill('SIGTERM'); for (let attempt = 0; attempt < 20 && processAlive(child.pid); attempt++) await pause(50); } if (processAlive(child.pid)) child.kill('SIGKILL'); if (processAlive(child.pid)) fail(`direct rust-analyzer retained PID: ${child.pid}`); } return { sessions: 1, shutdown, retained_processes: 0 }; },
  };
  rustLspSession = session;
  return session;
}
async function directRustLsp(root) { const session = startRustLspSession(root); await session.initialize(); return session; }
async function stopRustLsp() { if (rustLspSession === null) return { sessions: 0, shutdown: 'not_started', retained_processes: 0 }; const session = rustLspSession; rustLspSession = null; return session.close(); }
async function directRustDocumentSymbols(filePath, cwd) { try { return { isError: false, details: { symbols: await (await directRustLsp(cwd)).symbols(filePath), truncated: false } }; } catch (error) { return { isError: true, details: { error: error.message, symbols: [], truncated: false } }; } }
async function directRustFindReferences(filePath, cwd, line, character) { try { const result = await (await directRustLsp(cwd)).references(filePath, line, character); if (result !== null && !Array.isArray(result)) fail('invalid direct rust-analyzer references result'); const references = result ?? []; return { isError: false, details: { references, totalReferences: references.length, truncated: false } }; } catch (error) { return { isError: true, details: { error: error.message, references: [], truncated: false } }; } }
async function stopLspRuntime() {
  if (lspRuntimeHome === null) return;
  await stopLspSession();
  const daemonDirectory = lspDaemonDir();
  const pidFiles = [];
  const visit = (directory) => { if (!fs.existsSync(directory)) return; for (const entry of fs.readdirSync(directory, { withFileTypes: true })) { const absolute = path.join(directory, entry.name); if (entry.isDirectory()) visit(absolute); else if (entry.isFile() && entry.name === 'daemon.pid') pidFiles.push(absolute); } };
  visit(daemonDirectory);
  const daemonPids = new Set(), scopedPids = new Set();
  for (const pidFile of pidFiles) {
    const pid = Number(fs.readFileSync(pidFile, 'utf8').trim());
    if (!Number.isSafeInteger(pid) || pid <= 1) continue;
    daemonPids.add(pid); scopedPids.add(pid);
  }
  const signal = (name, pids) => { for (const pid of pids) if (processAlive(pid)) { try { process.kill(pid, name); } catch { /* Process already exited. */ } } };
  try {
    for (const pid of daemonPids) for (const child of scopedProcessTree(pid)) scopedPids.add(child);
    signal('SIGTERM', [...scopedPids].reverse());
    for (let attempt = 0; attempt < 10 && [...scopedPids].some(processAlive); attempt++) {
      await pause(50);
      for (const pid of daemonPids) if (processAlive(pid)) for (const child of scopedProcessTree(pid)) scopedPids.add(child);
      signal('SIGTERM', [...scopedPids].reverse());
    }
    signal('SIGKILL', [...scopedPids].reverse());
    for (let attempt = 0; attempt < 10 && [...scopedPids].some(processAlive); attempt++) await pause(50);
    const survivors = [...scopedPids].filter(processAlive);
    if (survivors.length) fail(`scoped LSP runtime left live PIDs: ${survivors.join(',')}`);
  } finally {
    fs.rmSync(lspRuntimeHome, { recursive: true, force: true });
    lspRuntimeHome = null;
  }
}
function startLspSession(cwd) {
  if (lspSession !== null) return lspSession;
  const child = spawn(process.execPath, [verifiedProviderPath(LSP_PROVIDER), 'mcp'], { cwd, env: lspEnvironment(), stdio: ['pipe', 'pipe', 'pipe'] });
  let buffer = '', pending = null, nextId = 1, closed = false;
  child.stdout.on('data', (chunk) => { buffer += chunk.toString('utf8'); let newline; while ((newline = buffer.indexOf('\n')) >= 0) { const line = buffer.slice(0, newline); buffer = buffer.slice(newline + 1); if (!line.trim()) continue; try { const message = JSON.parse(line); if (pending && message.id === pending.id) { const resolve = pending.resolve; pending = null; resolve(message.result ?? { isError: true, details: { error: 'lsp response has no result', symbols: [], truncated: false } }); } } catch { if (pending) { const reject = pending.reject; pending = null; reject(new Error('lsp response is not JSON')); } } } });
  child.on('error', (error) => { if (pending) { const reject = pending.reject; pending = null; reject(error); } });
  child.on('close', () => { if (pending) { const reject = pending.reject; pending = null; reject(new Error('LSP MCP session closed before responding')); } });
  const call = (name, arguments_, timeoutMs) => new Promise((resolve, reject) => { if (closed || pending) return reject(new Error('LSP session is unavailable or concurrent')); const id = nextId++; const timer = setTimeout(() => { if (pending?.id === id) { pending = null; reject(new Error(`LSP ${name} request timed out after ${timeoutMs}ms`)); if (lspSession?.close === close) lspSession = null; void close(); } }, timeoutMs); pending = { id, resolve: (value) => { clearTimeout(timer); resolve(value); }, reject: (error) => { clearTimeout(timer); reject(error); } }; const request = { jsonrpc: '2.0', id, method: 'tools/call', params: { name, arguments: { ...arguments_, _context: { cwd, env: {} } } } }; child.stdin.write(`${JSON.stringify(request)}\n`); });
  const close = async () => { if (closed) return; closed = true; if (pending) { pending.reject(new Error('LSP session closed')); pending = null; } child.stdin.end(); child.kill('SIGTERM'); for (let attempt = 0; attempt < 10 && processAlive(child.pid); attempt++) await pause(50); if (processAlive(child.pid)) child.kill('SIGKILL'); };
  lspSession = { call, close }; return lspSession;
}
async function stopLspSession() { if (lspSession === null) return; const session = lspSession; lspSession = null; await session.close(); }
function lspCall(name, arguments_, cwd, { timeoutMs = 15000 } = {}) {
  return startLspSession(cwd).call(name, arguments_, timeoutMs).then((response) => { assertScopedRustAnalyzerBound(); return response; }).catch((error) => ({ isError: true, details: { error: error.message, symbols: [], truncated: false } }));
}
function isRustFile(filePath) { return filePath.endsWith('.rs') && filePath.split(path.sep).includes('crates'); }
function lspDocumentSymbols(filePath, cwd, options) { return isRustFile(filePath) ? directRustDocumentSymbols(filePath, cwd) : lspCall('symbols', { filePath, scope: 'document', limit: 200 }, cwd, options); }
function lspFindReferences(filePath, cwd, line, character) { return isRustFile(filePath) ? directRustFindReferences(filePath, cwd, line, character) : lspCall('find_references', { filePath, line, character, includeDeclaration: false }, cwd); }
async function collectRustFiles(files, worktree, collect) { return mapPool(files, 1, collect); }
async function collectNonRustFiles(files, worktree, collect) { startLspRuntime(); try { return await mapPool(files, 1, collect); } finally { await stopLspRuntime(); } }
async function warmLspRuntime(worktree) {
  let response = null, attempts = 0;
  for (; attempts < 2; attempts++) {
    response = await lspDocumentSymbols(path.join(worktree, LEGACY_BASELINE_TYPES_SOURCE_PATH), worktree, { timeoutMs: 60000 });
    if (lspSymbolsComplete(response)) return attempts + 1;
    if (attempts === 0) await pause(250);
  }
  fail(`LSP runtime warmup failed after ${attempts} attempts: ${lspErrorText(response)}`);
}
function lspErrorText(response) {
  const detailError = response?.details?.error;
  if (typeof detailError === 'string' && detailError) return detailError;
  const contentError = Array.isArray(response?.content) ? response.content.filter((entry) => entry?.type === 'text' && typeof entry.text === 'string').map((entry) => entry.text).join('\n') : '';
  return contentError || (response?.isError === true ? 'LSP response marked error without diagnostic text' : '');
}
function lspSymbolsComplete(response) {
  const details = response?.details ?? {};
  return response?.isError !== true && !details.error && details.truncated !== true && Array.isArray(details.symbols);
}
function transientLspFailure(response) {
  if (lspSymbolsComplete(response)) return false;
  return /timed out|connection (?:closed|reset)|transport|socket|EPIPE|ECONNRESET|response was empty|response has no result|response is not JSON|Cannot read properties of null \(reading 'length'\)/i.test(lspErrorText(response));
}
function lspAttempt(response, ordinal) {
  return { ordinal, complete: lspSymbolsComplete(response), error: lspErrorText(response) || null, response };
}
async function mapPool(items, limit, task) {
  const results = new Array(items.length); let next = 0;
  await Promise.all(Array.from({ length: Math.min(limit, items.length) }, async () => { while (next < items.length) { const index = next++; results[index] = await task(items[index]); } }));
  return results;
}
function normalizeLspSymbols(symbols, depth = 0) {
  return (symbols ?? []).map((symbol) => {
    const selectionRange = symbol.selectionRange ?? symbol.location?.range;
    const range = symbol.range ?? symbol.location?.range;
    if (!selectionRange?.start || !range?.end || typeof symbol.name !== 'string' || !symbol.name) fail('invalid LSP symbol range');
    return { name: symbol.name, kind: String(symbol.kind), line_start: selectionRange.start.line + 1, line_end: range.end.line + 1, definition_character: selectionRange.start.character, children: depth === 0 ? normalizeLspSymbols(symbol.children, depth + 1) : [] };
  });
}
function preserveCarrierLocation(sourceSymbol, lspSymbol) {
  if (sourceSymbol.name !== lspSymbol.name || sourceSymbol.line_start < lspSymbol.line_start || sourceSymbol.line_start > lspSymbol.line_end) fail('LSP carrier range does not contain the baseline declaration');
  return { ...lspSymbol, line_start: sourceSymbol.line_start, definition_character: sourceSymbol.definition_character };
}
function braceEnd(lines, start) {
  let depth = 0, opened = false;
  for (let index = start - 1; index < lines.length; index++) for (const character of lines[index]) {
    if (character === '{') { depth++; opened = true; }
    else if (character === '}') depth--;
    if (opened && depth === 0) return index + 1;
  }
  return lines.length;
}
function rustSymbols(text) {
  const lines = text.split('\n');
  const symbols = [];
  for (let index = 0; index < lines.length; index++) {
    const match = /^\s*(?:pub(?:\([^)]*\))?\s+)?(?:struct|enum|trait|mod|fn|const|type)\s+([A-Za-z_][A-Za-z0-9_]*)/.exec(lines[index])
      ?? /^\s*impl(?:<[^>]+>)?\s+([A-Za-z_][A-Za-z0-9_:]*)/.exec(lines[index]);
    if (match) { const definitionCharacter = lines[index].indexOf(match[1]); if (definitionCharacter < 0) fail(`Rust declaration name is not present: ${match[1]}`); symbols.push({ name: match[1], kind: 'rust_declaration', line_start: index + 1, line_end: braceEnd(lines, index + 1), definition_character: definitionCharacter, children: [] }); }
  }
  return symbols.length ? symbols : [{ name: path.basename('module.rs', '.rs'), kind: 'rust_module', line_start: 1, line_end: lines.length, definition_character: 0, children: [] }];
}
function flatten(symbols) { return symbols.flatMap((symbol) => [{ name: symbol.name, kind: symbol.kind, line_start: symbol.line_start, line_end: symbol.line_end, definition_character: symbol.definition_character }, ...flatten(symbol.children)]); }
function normalizeLspReferences(response, worktree) {
  const details = response?.details ?? {};
  const references = details.references ?? [];
  if (response?.isError === true || details.truncated === true || !Array.isArray(references)) fail(`LSP references failed: ${details.error ?? 'missing references'}`);
  if (details.totalReferences !== undefined && details.totalReferences !== references.length) fail('LSP reference total drift');
  return references.map((reference) => {
    if (typeof reference?.uri !== 'string' || !reference.range?.start || !Number.isInteger(reference.range.start.line) || !Number.isInteger(reference.range.start.character)) fail('invalid LSP reference row');
    const absolute = fileURLToPath(reference.uri), repoPath = path.relative(worktree, absolute).split(path.sep).join('/');
    if (!repoPath || repoPath.startsWith('../') || path.isAbsolute(repoPath)) fail(`LSP reference escapes baseline worktree: ${reference.uri}`);
    return { path: repoPath, line: reference.range.start.line + 1, character: reference.range.start.character };
  });
}
function baselineRustTests(text) {
  const lines = text.split('\n'), tests = [];
  for (let index = 0; index < lines.length; index++) {
    if (lines[index].trim() !== '#[test]') continue;
    const declarationIndex = index + 1, match = /^\s*fn\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(/.exec(lines[declarationIndex] ?? '');
    if (!match) fail(`baseline test declaration missing after line ${index + 1}`);
    const lineStart = declarationIndex + 1, lineEnd = braceEnd(lines, lineStart), definitionCharacter = lines[declarationIndex].indexOf('fn');
    if (definitionCharacter < 0 || lineEnd < lineStart) fail(`invalid baseline test declaration: ${match[1]}`);
    tests.push({ test_name: match[1], line_start: lineStart, line_end: lineEnd, definition_character: definitionCharacter });
  }
  if (!tests.length || new Set(tests.map((test) => test.test_name)).size !== tests.length) fail('invalid baseline test source inventory');
  return tests;
}
function listedRustTests(receipt) {
  const stdout = path.join(ROOT, receipt.stdout_path);
  const bytes = fs.readFileSync(stdout);
  if (bytesDigest(bytes) !== receipt.stdout_sha256) fail('captured cargo --list stdout digest mismatch');
  const names = bytes.toString('utf8').split('\n').flatMap((line) => {
    const match = /^([A-Za-z_][A-Za-z0-9_:]*): test$/.exec(line.trim());
    return match ? [match[1]] : [];
  });
  if (!names.length || new Set(names).size !== names.length) fail('captured cargo --list test inventory is invalid');
  return names.sort((left, right) => left.localeCompare(right));
}
function selectedSymbols(files, baseline) { return new Map(files.filter((file) => RUST_PATH.test(file.path)).map((file) => [file.path, rustSymbols(sourceText(baseline, file.path))])); }
function chooseSymbol(symbols, preferred) { return symbols.find((symbol) => symbol.name === preferred) ?? symbols[0]; }
const CARRIERS = [
  ['space.world_coordinate', 'crates/ontopolis-types/src/coords.rs', 'WorldCoord'],
  ['space.chunk_local_coordinate', 'crates/ontopolis-types/src/coords.rs', 'ChunkCoord'],
  ['space.spatial_hierarchy', 'crates/ontopolis-world/src/hierarchy.rs'],
  ['time.simulation_clock', 'crates/ontopolis-types/src/time.rs'], ['time.phase_schedule', 'crates/ontopolis-core/src/phases.rs'],
  ['matter.material_properties', 'crates/ontopolis-types/src/physics.rs'], ['matter.material_activity', 'crates/ontopolis-domains/src/economy.rs'],
  ['energy.thermal_state', 'crates/ontopolis-types/src/physics.rs'], ['energy.kinetic_motion', 'crates/ontopolis-accelerate/src/accelerator.rs'],
  ['pattern_feature.feature_definition', 'crates/ontopolis-types/src/features.rs'], ['pattern_feature.physical_pattern_history', 'crates/ontopolis-runtime/src/pattern_history.rs'],
  ['spatial_geometry.world_geometry_contract', 'crates/ontopolis-types/src/geometry.rs'], ['spatial_geometry.local_metric_frame', 'crates/ontopolis-types/src/coords.rs'],
  ['geography.terrain_generation', 'crates/ontopolis-geography/src/terrain.rs'], ['geography.terrain_runtime_carrier', 'crates/ontopolis-runtime/src/carrier.rs'],
  ['geology.layer_state', 'crates/ontopolis-geography/src/geology.rs'], ['geology.formation_process', 'crates/ontopolis-geography/src/geology.rs'],
  ['hydrology.cell_state', 'crates/ontopolis-geography/src/hydrology.rs'], ['hydrology.water_flow', 'crates/ontopolis-geography/src/hydrology.rs'],
  ['climate.cell_state', 'crates/ontopolis-geography/src/climate.rs'], ['climate.transport_process', 'crates/ontopolis-geography/src/climate.rs'],
  ['ecology.population_resource_state', 'crates/ontopolis-biology/src/populations.rs'], ['ecology.trophic_material_interaction', 'crates/ontopolis-biology/src/physiology.rs'],
  ['biology.body_structure', 'crates/ontopolis-biology/src/morphology.rs'], ['biology.pathogen_lineage', 'crates/ontopolis-biology/src/pathogens.rs'],
  ['physical_access_perception.signal_acquisition', 'crates/ontopolis-perception/src/access.rs'], ['physical_access_perception.feature_extraction', 'crates/ontopolis-perception/src/extraction.rs'],
  ['cognition.attention_state', 'crates/ontopolis-cognition/src/attention.rs'], ['cognition.subjective_scene', 'crates/ontopolis-cognition/src/scene.rs'],
  ['language.lineage_bootstrap', 'crates/ontopolis-language/src/bootstrap.rs'], ['language.utterance_interpretation', 'crates/ontopolis-language/src/communication.rs'],
  ['mana.field_evolution', 'crates/ontopolis-domains/src/mana.rs'], ['mana.physical_effect_commit', 'crates/ontopolis-domains/src/mana.rs'],
  ['causal_resolution.relevance_field', 'crates/ontopolis-resolution/src/resolution.rs'], ['causal_resolution.actor_population_promotion', 'crates/ontopolis-runtime/src/actors/state.rs'],
  ['society.social_state', 'crates/ontopolis-domains/src/social.rs'], ['society.institutional_rules', 'crates/ontopolis-domains/src/social.rs'],
  ['economy.material_ledger', 'crates/ontopolis-domains/src/economy.rs'], ['economy.production_transformation', 'crates/ontopolis-domains/src/economy.rs'],
  ['city_infrastructure.built_environment', 'crates/ontopolis-domains/src/city.rs'], ['city_infrastructure.network', 'crates/ontopolis-domains/src/city.rs'],
  ['historical_bootstrap.stage_dag', 'crates/ontopolis-world/src/historical.rs'], ['historical_bootstrap.production_bootstrap', 'crates/ontopolis-world/src/historical.rs'],
  ['epistemics.calibrated_measurement', 'crates/ontopolis-epistemics/src/measurement.rs'], ['epistemics.physical_document', 'crates/ontopolis-epistemics/src/documents.rs'],
  ['practice.program', 'crates/ontopolis-domains/src/practices.rs'], ['practice.execution', 'crates/ontopolis-domains/src/practices.rs'],
  ['isekai.cross_world_transfer', 'crates/ontopolis-isekai/src/transfer.rs'], ['isekai.imported_priors', 'crates/ontopolis-isekai/src/priors.rs'],
  ['metaphysics.identity_continuity', 'crates/ontopolis-metaphysics/src/identity.rs'], ['metaphysics.attractor_probe', 'crates/ontopolis-metaphysics/src/attractors.rs'],
  ['simulation_runtime.deterministic_tick', 'crates/ontopolis-runtime/src/runtime.rs'], ['simulation_runtime.snapshot_roundtrip', 'crates/ontopolis-runtime/src/snapshot_sections.rs'],
  ['explanation_analytics.typed_claim_ir', 'crates/ontopolis-explanation/src/ir.rs'], ['explanation_analytics.checkpoint_analysis', 'crates/ontopolis-analytics/src/metrics.rs'],
  ['observer.bounded_query', 'crates/ontopolis-observer-api/src/query.rs'], ['observer.bounded_stream', 'crates/ontopolis-observer-api/src/stream.rs'],
  ['ui.session_control', 'crates/ontopolis-observer-wire/src/protocol.rs'], ['ui.inspection_views', 'crates/ontopolis-observer-wire/src/protocol.rs'],
  ['optional_llm_surface.policy_gate', 'crates/ontopolis-cli/src/main.rs'], ['optional_llm_surface.scheduling_deferral', 'crates/ontopolis-core/src/scheduler.rs'],
];
function runCapture({ runId, baseline, bootstrap, output, receiptId, adapter, argv }) {
  const captures = path.join(output, 'captures'); mkdir(captures);
  const stdout = path.join(captures, `${receiptId}.stdout`), stderr = path.join(captures, `${receiptId}.stderr`), receipt = path.join(captures, `${receiptId}.command-receipt.json`);
  const invocation = [path.join(ROOT, 'tools/audit/capture-command.mjs'), '--run-id', runId, '--source-baseline', baseline, '--bootstrap', relative(bootstrap), '--receipt-id', receiptId, '--adapter', adapter, '--stdout', relative(stdout), '--stderr', relative(stderr), '--receipt', relative(receipt), '--', ...argv];
  const result = spawnSync(process.execPath, invocation, { cwd: ROOT, encoding: 'utf8' });
  if (result.status !== 0) fail(`capture ${receiptId} failed: ${result.stderr || result.stdout}`);
  return { receipt: JSON.parse(fs.readFileSync(receipt, 'utf8')), path: relative(receipt) };
}
function dimensions() { return Object.fromEntries(['resolution', 'persistence', 'provenance', 'observer', 'explanation', 'determinism', 'performance', 'negative_control'].map((name) => [name, { status: 'missing', evidence_ids: [], rationale: 'not inferred from collection' }])); }
async function main() {
  if (process.argv.includes('--help') || process.argv.includes('-h')) {
    process.stdout.write(usage);
    return;
  }
  if (process.argv.includes('--direct-rust-symbol-normalization-smoke')) {
    const symbols = normalizeLspSymbols([{ name: 'CHUNK_SIZE', kind: 14, location: { uri: 'file:///frozen/crates/ontopolis-types/src/coords.rs', range: { start: { line: 2, character: 0 }, end: { line: 3, character: 30 } } } }]);
    if (canonical(symbols) !== canonical([{ name: 'CHUNK_SIZE', kind: '14', line_start: 3, line_end: 4, definition_character: 0, children: [] }])) fail('direct rust-analyzer SymbolInformation normalization drift');
    process.stdout.write('direct_rust_symbol_normalization=pass\n');
    return;
  }
  if (process.argv.includes('--direct-rust-carrier-location-smoke')) {
    const capture = normalizeLspSymbols([{ name: 'WorldCoord', kind: 23, location: { uri: 'file:///frozen/crates/ontopolis-types/src/coords.rs', range: { start: { line: 5, character: 0 }, end: { line: 15, character: 1 } } } }])[0];
    const carrier = preserveCarrierLocation({ name: 'WorldCoord', line_start: 12, definition_character: 0 }, capture);
    if (carrier.line_start !== 12) fail(`direct rust carrier anchor drift: ${carrier.line_start}`);
    process.stdout.write('direct_rust_carrier_location=pass\n');
    return;
  }
  if (process.argv.includes('--direct-rust-null-references-smoke')) {
    const references = normalizeLspReferences({ isError: false, details: { references: null, totalReferences: 0, truncated: false } }, '/frozen');
    if (references.length !== 0) fail('direct rust null references normalization drift');
    process.stdout.write('direct_rust_null_references=pass\n');
    return;
  }
  if (process.argv.includes('--direct-rust-reference-anchor-smoke')) {
    const symbol = rustSymbols('pub struct WorldCoord {\n    pub x: i64,\n}\n').find((item) => item.name === 'WorldCoord');
    if (symbol?.definition_character !== 11) fail(`direct rust reference anchor drift: ${symbol?.definition_character}`);
    process.stdout.write('direct_rust_reference_anchor=pass\n');
    return;
  }
  if (process.argv.includes('--lsp-child-environment-smoke')) {
    const result = spawnSync(process.execPath, ['-e', 'process.stdout.write(process.env.TASK4_SENTINEL_SECRET ?? "absent")'], { env: childEnvironment(), encoding: 'utf8' });
    if (result.status !== 0 || result.stdout !== 'absent') fail('LSP child environment leaked a parent secret');
    process.stdout.write('lsp_child_environment=pass\n');
    return;
  }
  if (process.argv.includes('--lsp-lifecycle-smoke')) {
    const worktrees = git(['worktree', 'list', '--porcelain']).split('\n\n').filter(Boolean).map((block) => Object.fromEntries(block.split('\n').map((line) => { const [key, ...rest] = line.split(' '); return [key, rest.join(' ')]; })));
    const original = worktrees.find((entry) => entry.branch === 'refs/heads/main');
    if (!original?.worktree) fail('main worktree is required for LSP lifecycle smoke');
    let lifecycle;
    try {
      const response = await directRustDocumentSymbols(path.join(original.worktree, LEGACY_BASELINE_TYPES_SOURCE_PATH), original.worktree);
      if (!lspSymbolsComplete(response)) fail(`direct rust-analyzer lifecycle smoke failed: ${lspErrorText(response)}`);
    } finally { lifecycle = await stopRustLsp(); }
    if (lifecycle.sessions !== 1 || lifecycle.shutdown !== 'confirmed' || lifecycle.retained_processes !== 0) fail('direct rust-analyzer lifecycle smoke cleanup failed');
    process.stdout.write('lsp_lifecycle_smoke=pass provider=rust-analyzer transport=stdio sessions=1 shutdown=confirmed retained_processes=0\n');
    return;
  }
  const runId = option('--run-id'), baseline = option('--source-baseline'), outArg = option('--out');
  if (!BASELINE.test(baseline)) fail('invalid --source-baseline');
  const output = path.resolve(ROOT, outArg); const outputRelative = relative(output); if (outputRelative !== `.omo/evidence/${runId}` && !outputRelative.startsWith(`.omo/evidence/${runId}/`)) fail('--out must be under the declared ignored evidence root');
  if (fs.existsSync(output)) fail(`evidence root already exists: ${relative(output)}`);
  const evidenceRelative = (file) => path.relative(output, file).split(path.sep).join('/');
  const baselineTree = git(['rev-parse', `${baseline}^{tree}`]).trim();
  const files = sourceFiles(baseline), symbolsByPath = selectedSymbols(files, baseline), catalog = JSON.parse(fs.readFileSync(CATALOG, 'utf8'));
  if (catalog.capabilities.length !== 61 || CARRIERS.length !== 61 || new Set(CARRIERS.map(([id]) => id)).size !== 61) fail('concrete carrier catalog drift');
  const catalogById = new Map(catalog.capabilities.map((entry) => [entry.capability_id, entry]));
  const carrierByCapability = new Map(CARRIERS.map(([id, repoPath, preferred]) => {
    const file = files.find((entry) => entry.path === repoPath); const symbol = chooseSymbol(symbolsByPath.get(repoPath) ?? [], preferred);
    if (!catalogById.has(id) || !file || !symbol) fail(`invalid concrete carrier mapping: ${id}`);
    return [id, { file, symbol }];
  }));
  mkdir(output);
  const worktrees = git(['worktree', 'list', '--porcelain']).split('\n\n').filter(Boolean).map((block) => Object.fromEntries(block.split('\n').map((line) => { const [key, ...rest] = line.split(' '); return [key, rest.join(' ')]; }))); const original = worktrees.find((entry) => entry.branch === 'refs/heads/main'); if (!original?.worktree) fail('main worktree is required for preflight');
  const targetPreimage = git(['rev-parse', `${baseline}:PLANS.md`]).trim();
  const version = (command) => execFileSync(command, ['--version'], { cwd: ROOT, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }).trim();
  const graphIndex = graphCli('index_status');
  const statusLines = (directory) => execFileSync('git', ['status', '--porcelain=v1'], { cwd: directory, encoding: 'utf8' }).split('\n').filter(Boolean);
  const stagedPaths = (directory) => execFileSync('git', ['diff', '--cached', '--name-only'], { cwd: directory, encoding: 'utf8' }).split('\n').filter(Boolean);
  const isAncestor = (ancestor, descendant, directory) => spawnSync('git', ['merge-base', '--is-ancestor', ancestor, descendant], { cwd: directory }).status === 0;
  const originalHead = execFileSync('git', ['rev-parse', 'HEAD'], { cwd: original.worktree, encoding: 'utf8' }).trim();
  const auditHead = git(['rev-parse', 'HEAD']).trim(), auditTree = git(['rev-parse', 'HEAD^{tree}']).trim();
  const bootstrap = path.join(output, 'task-1-bootstrap.json'); const runnerTrust = createRunnerTrust(runId); write(bootstrap, attestBootstrap({ run_id: runId, source_baseline_sha: baseline, source_baseline_tree_oid: baselineTree, original_worktree_path: original.worktree, audit_worktree_path: ROOT, audit_head_sha: auditHead, audit_tree_oid: auditTree, lsp_provider: LSP_PROVIDER, rust_lsp_provider: { ...RUST_LSP_PROVIDER, root_path: original.worktree }, rust_lsp_lifecycle: { sessions: 1, shutdown: 'pending', retained_processes: null }, tool_versions: { git: git(['--version']).trim(), node: process.version, cargo: version('cargo'), pnpm: version('pnpm') }, graph_status: { status: graphIndex.status, index_sha: graphIndex.git?.head_sha ?? null }, cleanliness: { original_status_porcelain: statusLines(original.worktree), original_staged_paths: stagedPaths(original.worktree), relevant_staged_paths: stagedPaths(ROOT), audit_status_porcelain_before_evidence: statusLines(ROOT), audit_staged_paths: stagedPaths(ROOT) }, inventories: { worktrees: worktrees.map((entry) => entry.worktree) }, ancestry: { source_baseline_is_original_head: originalHead === baseline, audit_worktree_descends_from_baseline: isAncestor(baseline, 'HEAD', ROOT) }, target_preimages: [{ path: 'PLANS.md', blob_oid: targetPreimage }] }, runnerTrust));
  const preflight = runCapture({ runId, baseline, bootstrap, output, receiptId: 'task-01-preflight', adapter: 'git_preflight', argv: ['node', 'tools/audit/validate-capability-audit.mjs', 'preflight', '--run-id', runId, '--bootstrap', relative(bootstrap)] });
  const sourceDefinition = runCapture({ runId, baseline, bootstrap, output, receiptId: 'task-04-source-definition', adapter: 'source_definition', argv: ['git', 'show', `${baseline}:${LEGACY_BASELINE_TYPES_SOURCE_PATH}`] });
  const testList = runCapture({ runId, baseline, bootstrap, output, receiptId: 'task-04-rust-test-list', adapter: 'cargo_test_list', argv: ['cargo', 'test', '-p', LEGACY_BASELINE_TYPES_PACKAGE, '--test', 'coords', '--', '--list'] });
  const testResults = runCapture({ runId, baseline, bootstrap, output, receiptId: 'task-04-rust-test-results', adapter: 'exact_test', argv: ['cargo', 'test', '-p', LEGACY_BASELINE_TYPES_PACKAGE, '--test', 'coords', 'world_to_chunk_local_roundtrip', '--', '--exact'] });
  const graphRoot = path.join(output, 'task-4-graph');
  if (graphIndex.status !== 'ready' || graphIndex.root_path !== original.worktree || graphIndex.git?.head_sha !== baseline || graphIndex.git?.base_sha !== baseline) fail('baseline graph index is not bound to the frozen source baseline');
  const graphSchemaRaw = graphCli('get_graph_schema');
  const graphSchema = digest(graphSchemaRaw);
  write(path.join(graphRoot, 'graph-index-status.json'), graphIndex); write(path.join(graphRoot, 'graph-schema.json'), graphSchemaRaw);
  const fallbackGroups = Array.from({ length: 19 }, () => []);
  CARRIERS.forEach(([capabilityId], index) => fallbackGroups[index % fallbackGroups.length].push(capabilityId));
  const graphQueries = fallbackGroups.map((capabilityIds, index) => {
    const terms = capabilityIds.map((capabilityId) => { const carrier = carrierByCapability.get(capabilityId); return `(n.file_path = '${carrier.file.path}' AND n.name = '${carrier.symbol.name}')`; });
    return { query_id: `task-04-graph-${String(index + 1).padStart(2, '0')}`, edge_type: 'DECLARATION_LOOKUP', query: `MATCH (n) WHERE ${terms.join(' OR ')} RETURN n.name AS name, n.file_path AS file_path, n.start_line AS start_line, n.end_line AS end_line, labels(n) AS labels ORDER BY file_path, start_line`, result_count: 0 };
  });
  const graphQueryByCapability = new Map(fallbackGroups.flatMap((capabilityIds, index) => capabilityIds.map((capabilityId) => [capabilityId, graphQueries[index]])));
  const graphResultsByQuery = new Map();
  const graphSlots = graphQueries.map((query) => {
    const receiptId = `${query.query_id}-receipt`, rawPath = path.join(graphRoot, 'raw', `${receiptId}.json`), receiptPath = path.join(graphRoot, 'receipts', `${receiptId}.json`);
    const raw = graphCli('query_graph', ['--query', query.query, '--max-rows', '100']);
    if (!Array.isArray(raw.columns) || !Array.isArray(raw.rows) || !Number.isInteger(raw.total) || raw.total < raw.rows.length) fail(`invalid graph result: ${query.query_id}`);
    query.result_count = raw.total; graphResultsByQuery.set(query.query_id, raw);
    write(rawPath, raw); const receipt = seal({ schema_version: 1, run_id: runId, source_baseline_sha: baseline, attempt_id: null, receipt_id: receiptId, phase: 'collection', index_sha: baseline, graph_schema_sha256: graphSchema, operation: 'query_graph', parameters: { query_id: query.query_id, edge_type: query.edge_type, limit: 100 }, query: query.query, total: raw.total, returned: raw.rows.length, ordered_result_sha256: digest(raw.rows), raw_path: evidenceRelative(rawPath), raw_sha256: bytesDigest(fs.readFileSync(rawPath)) }); write(receiptPath, receipt);
    return { query_id: query.query_id, edge_type: query.edge_type, receipt_id: receiptId, receipt_path: evidenceRelative(receiptPath), receipt_sha256: receipt.receipt_sha256 };
  });
  const graphManifest = { schema_version: 1, run_id: runId, source_baseline_sha: baseline, graph_schema_sha256: graphSchema, slots: graphSlots };
  const graphManifestPath = path.join(output, 'task-4-graph-receipts-manifest.json'); write(graphManifestPath, graphManifest);
  if (!Array.isArray(graphSchemaRaw.edge_types) || graphSchemaRaw.edge_types.some((edge) => edge?.type === 'DATA_FLOWS')) fail('baseline graph schema contains DATA_FLOWS; absence evidence is invalid');
  const graphSchemaPath = path.join(graphRoot, 'graph-schema.json');
  const absence = { schema_version: 1, run_id: runId, source_baseline_sha: baseline, receipt_id: 'task-04-data-flows-absent', operation: 'schema_absence', edge_type: 'DATA_FLOWS', graph_schema_sha256: graphSchema, graph_schema_path: evidenceRelative(graphSchemaPath), graph_schema_raw_sha256: bytesDigest(fs.readFileSync(graphSchemaPath)), reason: 'baseline graph schema has no DATA_FLOWS edge' };
  const absencePath = path.join(output, 'task-4-data-flows-absence.json'); write(absencePath, absence);
  const collectLsp = async (file) => {
    const response = await lspDocumentSymbols(path.join(original.worktree, file.path), original.worktree);
    const attempts = [lspAttempt(response, 1)];
    while (RUST_PATH.test(file.path) && transientLspFailure(attempts.at(-1).response) && attempts.length < 3) {
      const response = await lspDocumentSymbols(path.join(original.worktree, file.path), original.worktree);
      attempts.push(lspAttempt(response, attempts.length + 1));
    }
    return { file, attempts };
  };
  const rustCaptures = await collectRustFiles(files.filter((file) => RUST_PATH.test(file.path)), original.worktree, collectLsp);
  const nonRustCaptures = await collectNonRustFiles(files.filter((file) => !RUST_PATH.test(file.path)), original.worktree, collectLsp);
  const lspCaptures = [...rustCaptures, ...nonRustCaptures].sort((left, right) => left.file.path.localeCompare(right.file.path));
  const lspEntries = lspCaptures.map(({ file, attempts }) => {
    const response = attempts.at(-1).response, details = response.details ?? {}, complete = lspSymbolsComplete(response), symbols = complete ? normalizeLspSymbols(details.symbols) : [];
    const receiptId = `task-04-file-${digest({ path: file.path, blob_oid: file.blob_oid }).slice(0, 12)}-lsp-symbols`, rawPath = path.join(output, 'task-4-lsp', 'raw', `${receiptId}.json`), receiptPath = path.join(output, 'task-4-lsp', 'receipts', `${receiptId}.json`), raw = { provider: RUST_PATH.test(file.path) ? { ...RUST_LSP_PROVIDER, root_path: original.worktree } : LSP_PROVIDER, attempts, final_response: response };
    write(rawPath, raw); const receipt = seal({ schema_version: 1, run_id: runId, source_baseline_sha: baseline, attempt_id: null, receipt_id: receiptId, phase: 'collection', path: file.path, baseline_blob_oid: file.blob_oid, worktree_blob_oid: file.blob_oid, parameters: { query: 'textDocument/documentSymbol', include_declarations: true, limit: 200 }, complete, symbols, raw_path: evidenceRelative(rawPath), raw_sha256: bytesDigest(fs.readFileSync(rawPath)), ordered_result_sha256: digest(symbols), flattened_symbols_sha256: digest(flatten(symbols)) }); write(receiptPath, receipt);
    return { path: file.path, baseline_blob_oid: file.blob_oid, worktree_blob_oid: file.blob_oid, complete, receipt_id: receiptId, receipt_path: evidenceRelative(receiptPath), receipt_sha256: receipt.receipt_sha256, flattened_symbols_sha256: receipt.flattened_symbols_sha256 };
  });
  const lspManifest = { schema_version: 1, run_id: runId, source_baseline_sha: baseline, tree_oid: baselineTree, entries: lspEntries };
  const lspManifestPath = path.join(output, 'task-4-lsp-receipts-manifest.json'); write(lspManifestPath, lspManifest);
  for (const carrier of carrierByCapability.values()) {
    const lsp = lspEntries.find((entry) => entry.path === carrier.file.path);
    const receipt = JSON.parse(fs.readFileSync(path.join(output, lsp.receipt_path), 'utf8'));
    const symbol = flatten(receipt.symbols).find((item) => item.name === carrier.symbol.name);
    if (!lsp.complete || !symbol) fail(`LSP cannot resolve concrete carrier: ${carrier.file.path}::${carrier.symbol.name}`);
    carrier.symbol = preserveCarrierLocation(carrier.symbol, symbol);
  }
  const candidateByPath = new Map();
  for (const file of files) { const entry = lspEntries.find((item) => item.path === file.path), receipt = JSON.parse(fs.readFileSync(path.join(output, entry.receipt_path), 'utf8')), symbols = entry.complete ? flatten(receipt.symbols) : [], fallback = symbols[0] ?? { name: path.basename(file.path), line_start: 1, line_end: sourceText(baseline, file.path).split('\n').length, definition_character: 0 }; candidateByPath.set(file.path, { candidate_key: digest({ path: file.path, blob_oid: file.blob_oid, symbol: fallback.name }), kind: 'source', path: file.path, blob_oid: file.blob_oid, line_start: fallback.line_start, line_end: fallback.line_end, definition_character: fallback.definition_character, symbol: fallback.name, graph_receipt_ids: [], lsp_receipt_ids: [entry.receipt_id], is_test: false, is_exported: true, bindings: [], exclusion_id: null, failure_cases: [] }); }
  const candidates = [...candidateByPath.values()], carrierCandidates = new Map(), contexts = [];
  for (const [capabilityId, carrier] of carrierByCapability) {
    let candidate = candidateByPath.get(carrier.file.path);
    if (candidate.bindings.length) { candidate = { ...candidate, candidate_key: digest({ path: carrier.file.path, blob_oid: carrier.file.blob_oid, symbol: carrier.symbol.name, capability_id: capabilityId }), bindings: [], exclusion_id: null }; candidates.push(candidate); }
    const entry = catalogById.get(capabilityId), bindingId = `task-04-binding-${capabilityId.replace(/[^a-z0-9]+/g, '-')}`;
    const graphQuery = graphQueryByCapability.get(capabilityId), graphSlot = graphSlots.find((slot) => slot.query_id === graphQuery.query_id), graphRaw = graphResultsByQuery.get(graphQuery.query_id), columns = new Map(graphRaw.columns.map((name, index) => [name, index]));
    const graphMatch = graphRaw.rows.find((row) => row[columns.get('name')] === carrier.symbol.name && row[columns.get('file_path')] === carrier.file.path && Number(row[columns.get('start_line')]) <= carrier.symbol.line_start && Number(row[columns.get('end_line')]) >= carrier.symbol.line_end);
    candidate.symbol = carrier.symbol.name; candidate.line_start = carrier.symbol.line_start; candidate.line_end = carrier.symbol.line_end; candidate.definition_character = carrier.symbol.definition_character; candidate.graph_receipt_ids = graphMatch ? [graphSlot.receipt_id] : []; candidate.bindings.push({ binding_id: bindingId, capability_id: capabilityId, role: 'carrier', lifecycle_boundary: 'source-definition' }); carrierCandidates.set(capabilityId, candidate);
    contexts.push({ binding_id: bindingId, candidate_key: candidate.candidate_key, grouping: { domain: entry.domain, capability_class: 'policy', state_path: null, mutation_owner: null, lifecycle_boundary: 'source-definition' }, endpoint_declaration: { kind: capabilityId === 'space.world_coordinate' ? 'lsp_declaration' : 'baseline_source', qualified_name: candidate.symbol, line_start: candidate.line_start, line_end: candidate.line_end } });
  }
  const exclusions = []; for (const candidate of candidates) if (!candidate.bindings.length) { const exclusionId = `task-04-exclusion-${String(exclusions.length + 1).padStart(3, '0')}`; candidate.exclusion_id = exclusionId; exclusions.push({ exclusion_id: exclusionId, candidate_keys: [candidate.candidate_key], reason: 'no concrete capability mapping in bounded inventory', evidence_ids: ['e-source-definition'], rationale: 'explicit source-only exclusion; no maturity is inferred' }); }
  const semanticReview = { schema_version: 1, run_id: runId, source_baseline_sha: baseline, collection_method: 'graph_first_git_fallback', rows: CARRIERS.map(([capabilityId]) => {
    const candidate = carrierCandidates.get(capabilityId), binding = candidate.bindings.find((item) => item.capability_id === capabilityId), graphQuery = graphQueryByCapability.get(capabilityId);
    const fallbackArgv = ['git', 'show', `${baseline}:${candidate.path}`], source = sourceText(baseline, candidate.path), line = source.split('\n')[candidate.line_start - 1] ?? '';
    if (!line.includes(candidate.symbol)) fail(`baseline symbol fallback does not resolve ${capabilityId}: ${candidate.path}:${candidate.line_start}`);
    const graphReceiptId = candidate.graph_receipt_ids[0] ?? null, resolverKind = graphReceiptId ? 'graph' : 'git_fallback';
    return { capability_id: capabilityId, binding_id: binding.binding_id, candidate_key: candidate.candidate_key, path: candidate.path, blob_oid: candidate.blob_oid, symbol: candidate.symbol, line_start: candidate.line_start, line_end: candidate.line_end, query_id: graphQuery.query_id, graph_receipt_id: graphReceiptId, resolver_kind: resolverKind, fallback_argv: resolverKind === 'git_fallback' ? fallbackArgv : null, fallback_stdout_sha256: resolverKind === 'git_fallback' ? bytesDigest(Buffer.from(source)) : null, status: 'pass' };
  }) };
  const semanticReviewPath = path.join(output, 'task-4-semantic-review.json'); write(semanticReviewPath, semanticReview);
  const candidateManifest = { schema_version: 1, run_id: runId, source_baseline_sha: baseline, tree_files: files.map((file) => file.path), graph_queries: graphQueries, candidates, exclusions, extensions: { receipt_manifests: { graph: { receipt_id: 'task-04-graph-manifest', path: evidenceRelative(graphManifestPath), sha256: digest(graphManifest) }, lsp: { receipt_id: 'task-04-lsp-manifest', path: evidenceRelative(lspManifestPath), sha256: digest(lspManifest) }, data_flows_absence: { receipt_id: absence.receipt_id, path: evidenceRelative(absencePath), sha256: digest(absence) } }, binding_contexts: contexts, semantic_review: { receipt_id: 'task-04-semantic-review', path: evidenceRelative(semanticReviewPath), sha256: digest(semanticReview) } } };
  const candidatePath = path.join(output, 'task-4-candidate-manifest.json'); write(candidatePath, candidateManifest);
  const world = carrierByCapability.get('space.world_coordinate'), worldCandidate = carrierCandidates.get('space.world_coordinate'), worldBinding = worldCandidate.bindings.find((binding) => binding.capability_id === 'space.world_coordinate');
  const testPath = LEGACY_BASELINE_COORDS_PATH, testBlob = files.find((file) => file.path === testPath)?.blob_oid; if (!testBlob) fail('baseline coordinate test not selected');
  const referenceResponse = await lspFindReferences(path.join(original.worktree, world.file.path), original.worktree, world.symbol.line_start, world.symbol.definition_character);
  const referenceRaw = { response: referenceResponse, references: normalizeLspReferences(referenceResponse, original.worktree) };
  if (!referenceRaw.references.length) fail('LSP reference capture returned no references');
  const referenceRawPath = path.join(output, 'task-4-endpoint', 'raw', 'world-coordinate.references.json'); write(referenceRawPath, referenceRaw);
  const referenceReceipt = seal({ schema_version: 1, run_id: runId, source_baseline_sha: baseline, attempt_id: null, receipt_id: 'task-04-world-coordinate-references', phase: 'collection', path: world.file.path, baseline_blob_oid: world.file.blob_oid, worktree_blob_oid: world.file.blob_oid, parameters: { query: `references:${world.symbol.line_start}:${world.symbol.definition_character}`, include_declarations: false, limit: 100 }, complete: true, symbols: symbolsByPath.get(world.file.path), raw_path: evidenceRelative(referenceRawPath), raw_sha256: bytesDigest(fs.readFileSync(referenceRawPath)), ordered_result_sha256: digest(referenceRaw.references), flattened_symbols_sha256: digest(flatten(symbolsByPath.get(world.file.path))) });
  const referenceReceiptPath = path.join(output, 'task-4-endpoint', 'world-coordinate.references.receipt.json'); write(referenceReceiptPath, referenceReceipt);
  const moduleQuery = `MATCH (n:Struct) WHERE n.name = '${world.symbol.name}' AND n.file_path = '${world.file.path}' RETURN n.qualified_name AS qualified_name, n.file_path AS file_path, n.start_line AS start_line, n.end_line AS end_line UNION MATCH (n:Function) WHERE n.file_path = '${testPath}' RETURN n.qualified_name AS qualified_name, n.file_path AS file_path, n.start_line AS start_line, n.end_line AS end_line`;
  const moduleRaw = graphCli('query_graph', ['--query', moduleQuery, '--max-rows', '100']);
  if (canonical(moduleRaw.columns) !== canonical(['qualified_name', 'file_path', 'start_line', 'end_line']) || !Array.isArray(moduleRaw.rows) || moduleRaw.total !== moduleRaw.rows.length) fail('invalid endpoint declaration graph result');
  const moduleRawPath = path.join(output, 'task-4-endpoint', 'raw', 'module-declarations.json'); write(moduleRawPath, moduleRaw);
  const moduleReceipt = seal({ schema_version: 1, run_id: runId, source_baseline_sha: baseline, attempt_id: null, receipt_id: 'task-04-module-declarations', phase: 'collection', index_sha: baseline, graph_schema_sha256: graphSchema, operation: 'query_graph', parameters: { query_id: 'task-04-module-declarations', edge_type: 'DECLARATION_LOOKUP', limit: 100 }, query: moduleQuery, total: moduleRaw.total, returned: moduleRaw.rows.length, ordered_result_sha256: digest(moduleRaw.rows), raw_path: evidenceRelative(moduleRawPath), raw_sha256: bytesDigest(fs.readFileSync(moduleRawPath)) });
  const moduleReceiptPath = path.join(output, 'task-4-endpoint', 'module-declarations.receipt.json'); write(moduleReceiptPath, moduleReceipt);
  const listedTests = listedRustTests(testList.receipt), sourceTests = baselineRustTests(sourceText(baseline, testPath));
  if (canonical(listedTests) !== canonical(sourceTests.map((test) => test.test_name).sort((left, right) => left.localeCompare(right)))) fail('captured cargo --list and baseline test source disagree');
  const referencesInTestSource = referenceRaw.references.filter((reference) => reference.path === testPath);
  const testRows = sourceTests.map((test) => ({ ...test, eligibility: referencesInTestSource.some((reference) => reference.line >= test.line_start && reference.line <= test.line_end) ? 'exact_test' : 'non_testable_target' }));
  for (const reference of referencesInTestSource) if (reference.line !== 1 && !testRows.some((test) => reference.line >= test.line_start && reference.line <= test.line_end)) fail(`LSP reference is outside a baseline test definition: ${reference.path}:${reference.line}`);
  const exactTests = testRows.filter((test) => test.eligibility === 'exact_test');
  if (!exactTests.length || exactTests.some((test) => !listedTests.includes(test.test_name))) fail('missing baseline-discovered exact test relation');
  const moduleColumns = new Map(moduleRaw.columns.map((column, index) => [column, index]));
  const testRootQn = (test) => {
    const row = moduleRaw.rows.find((item) => item[moduleColumns.get('file_path')] === testPath && Number(item[moduleColumns.get('start_line')]) === test.line_start && Number(item[moduleColumns.get('end_line')]) === test.line_end);
    if (!row || typeof row[moduleColumns.get('qualified_name')] !== 'string') fail(`baseline graph declaration missing for cargo-discovered test: ${test.test_name}`);
    return row[moduleColumns.get('qualified_name')];
  };
  const tests = testRows.map((test) => ({ test_id: `task-04-${test.test_name.replace(/[^a-z0-9]+/gi, '-').replace(/^-|-$/g, '').toLowerCase()}`, package: LEGACY_BASELINE_TYPES_PACKAGE, target_kind: 'integration-test', target_name: 'coords', test_name: test.test_name, path: testPath, blob_oid: testBlob, line_start: test.line_start, line_end: test.line_end, definition_character: test.definition_character, test_root_qn: testRootQn(test), eligibility: test.eligibility, discovered_receipt_id: testList.receipt.receipt_id }));
  const mappings = tests.filter((test) => test.eligibility === 'exact_test').map((test) => ({ test_id: test.test_id, candidate_key: worldCandidate.candidate_key, binding_id: worldBinding.binding_id, relation: 'lsp_reference', evidence_receipt_ids: [testList.receipt.receipt_id, referenceReceipt.receipt_id] }));
  if (mappings.length !== exactTests.length || new Set(tests.map((test) => test.test_name)).size !== listedTests.length) fail('incomplete deterministic test reconciliation');
  const reconciliation = seal({ schema_version: 1, run_id: runId, source_baseline_sha: baseline, targets: [{ package: LEGACY_BASELINE_TYPES_PACKAGE, target_kind: 'integration-test', target_name: 'coords', path: testPath, blob_oid: testBlob, eligibility: 'exact_test', rationale: 'the declared Cargo integration target is the immutable baseline coords test source' }], tests, mappings, completeness: { status: 'complete', evidence_ids: ['e-source-definition'], rationale: 'all cargo-discovered tests are recorded; every LSP-linked baseline test is mapped and remaining tests are explicitly non_testable_target' } });
  const reconciliationPath = path.join(output, 'test-reconciliation.json'); write(reconciliationPath, reconciliation);
  const claim = { capability_id: 'space.world_coordinate', binding_id: worldBinding.binding_id, adapter: 'source_definition', argv: sourceDefinition.receipt.argv, baseline_target: { path: world.file.path, blob_oid: world.file.blob_oid, line_start: world.symbol.line_start, line_end: world.symbol.line_end }, facets: ['primitive_boundary', 'carrier:WorldCoord'], receipt_id: sourceDefinition.receipt.receipt_id, receipt_path: sourceDefinition.path, receipt_sha256: sourceDefinition.receipt.receipt_sha256 };
  const execution = seal({ schema_version: 1, run_id: runId, source_baseline_sha: baseline, claims: [claim] }); const executionPath = path.join(output, 'evidence-execution-manifest.json'); write(executionPath, execution);
  const evidence = [{ adapter: claim.adapter, blob_oid: world.file.blob_oid, command: claim.argv.join(' '), evidence_id: 'e-source-definition', exit_code: 0, extensions: {}, facets: ['intent', 'primitive_boundary', 'carriers', 'carrier:WorldCoord', 'risks'], line_end: world.symbol.line_end, line_start: world.symbol.line_start, path: world.file.path, receipt_path: claim.receipt_path, receipt_sha256: claim.receipt_sha256, run_id: runId, source_baseline_sha: baseline, symbol: world.symbol.name, workload: { name: 'baseline-definition', bounded_inputs: ['source-file'] } }];
  const capabilities = catalog.capabilities.map((entry) => { const carrier = carrierByCapability.get(entry.capability_id); const profile = { production_reachability: 'missing', ...dimensions(), target: 'M0', gaps: ['collection does not infer maturity'], dependencies: [] }; return { capability_id: entry.capability_id, domain: entry.domain, capability_class: 'policy', authoritative_state: { path: null, symbol: null, rationale: 'no authoritative-state claim' }, mutation_owner: { path: null, symbol: null, rationale: 'no mutation-owner claim' }, carriers: [`${carrier.file.path}::${carrier.symbol.name}`], semantic_profile: profile, levels: Object.fromEntries(['M0', 'M1', 'M2', 'M3', 'M4', 'M5'].map((level) => [level, { status: 'missing', evidence_ids: [], rationale: 'not evidenced by collection' }])), target_maturity: 'M0', representative_workload: { status: 'missing', name: 'not-applicable', bounded_inputs: [], warmup: null, sample_count: 0, duration: 0, metrics: [], validation_envelope: [] } }; });
  const inventory = { schema_version: 1, run_id: runId, source_baseline_sha: baseline, domains: catalog.domains.map((domain) => ({ domain: domain.domain, capability_ids: domain.capability_ids })), capabilities, evidence, extensions: {}, todo_bindings: [{ todo_id: 'TODO-DEPTH-001', heading: 'Detailed Development maturity audit', goal: 'bounded 30-domain source reconciliation', acceptance: 'deterministic evidence validation', dependency_range: '1-4', clause_hash: digest('TODO-DEPTH-001') }], source_reconciliation: { status: 'present', evidence_ids: ['e-source-definition'], rationale: 'all 132 selected baseline files have an explicit binding or exclusion' } };
  const inventoryPath = path.join(output, 'task-4-capability-inventory.json'); write(inventoryPath, inventory);
  const blobs = seal({ schema_version: 1, run_id: runId, source_baseline_sha: baseline, tree_oid: baselineTree, entries: files.map((file) => ({ logical_path: file.path, tracked_path: file.path, tracked_blob_oid: file.blob_oid, byte_sha256: bytesDigest(git(['cat-file', 'blob', file.blob_oid], null)), kind: 'source' })) }); const blobsPath = path.join(output, 'task-4-source-blobs.json'); write(blobsPath, blobs);
  const inventoryOut = path.join(output, 'task-4-inventory-validation.json');
  const audit = path.join(ROOT, 'tools/audit/validate-capability-audit.mjs');
  const verify = spawnSync(process.execPath, [audit, 'inventory', '--run-id', runId, '--bootstrap', relative(bootstrap), '--preflight-receipt', preflight.path, '--input', relative(inventoryPath), '--candidate-manifest', relative(candidatePath), '--blobs', relative(blobsPath), '--test-list-receipt', testList.path, '--test-results-receipt', testResults.path, '--test-reconciliation', relative(reconciliationPath), '--evidence-execution-manifest', relative(executionPath), '--out', relative(inventoryOut)], { cwd: ROOT, encoding: 'utf8' });
  if (verify.status !== 0) fail(`inventory validation failed: ${verify.stderr || verify.stdout}`);
  const negative = spawnSync(process.execPath, [audit, 'inventory', '--input', relative(inventoryPath)], { cwd: ROOT, encoding: 'utf8' });
  if (negative.status === 0) fail('inventory negative control unexpectedly passed');
  write(path.join(output, 'task-4-inventory-negative.json'), { schema_version: 1, run_id: runId, source_baseline_sha: baseline, command: `${path.basename(process.execPath)} tools/audit/validate-capability-audit.mjs inventory --input ${relative(inventoryPath)}`, exit_code: negative.status, stdout: negative.stdout, stderr: negative.stderr, verdict: 'pass-as-negative' });
  const markdown = path.join(output, 'task-4-capability-inventory.md');
  for (const target of [markdown, `${markdown}.second`]) { const result = spawnSync(process.execPath, [path.join(ROOT, 'tools/audit/build-task4-inventory.mjs'), 'build', '--catalog', relative(CATALOG), '--inventory', relative(inventoryPath), '--out', relative(target)], { cwd: ROOT, encoding: 'utf8' }); if (result.status !== 0) fail(`inventory build failed: ${result.stderr || result.stdout}`); }
  if (bytesDigest(fs.readFileSync(markdown)) !== bytesDigest(fs.readFileSync(`${markdown}.second`))) fail('Task 4 inventory build is not reproducible');
  const lspProjection = (captures) => captures.map(({ file, attempts }) => { const response = attempts.at(-1).response, complete = lspSymbolsComplete(response), symbols = complete ? normalizeLspSymbols(response.details?.symbols) : []; return { path: file.path, blob_oid: file.blob_oid, complete, symbols_sha256: digest(symbols) }; }).sort((left, right) => left.path.localeCompare(right.path));
  const primaryProjection = { baseline_tree_oid: baselineTree, source_listing_sha256: bytesDigest(git(['ls-tree', '-r', '-z', baseline], null)), graph_schema_sha256: digest(graphSchemaRaw), graph_queries: graphQueries.map((query) => ({ query_id: query.query_id, rows_sha256: digest(graphResultsByQuery.get(query.query_id).rows) })), cargo_test_list_stdout_sha256: testList.receipt.stdout_sha256, cargo_test_results_stdout_sha256: testResults.receipt.stdout_sha256, lsp: lspProjection(lspCaptures) };
  const replaySchema = graphCli('get_graph_schema');
  const replayQueries = graphQueries.map((query) => ({ query_id: query.query_id, rows_sha256: digest(graphCli('query_graph', ['--query', query.query, '--max-rows', '100']).rows) }));
  const replayCargo = (argv) => { const result = spawnSync('cargo', argv, { cwd: original.worktree, env: { PATH: process.env.PATH, HOME: process.env.HOME, CARGO_TERM_COLOR: 'never', LC_ALL: 'C', NO_COLOR: '1', TZ: 'UTC' }, encoding: null, maxBuffer: 64 * 1024 * 1024 }); if (result.status !== 0) fail(`independent cargo replay failed: ${Buffer.from(result.stderr ?? []).toString('utf8')}`); return bytesDigest(result.stdout ?? Buffer.alloc(0)); };
  const replayRust = await collectRustFiles(files.filter((file) => RUST_PATH.test(file.path)), original.worktree, collectLsp);
  const replayNonRust = await collectNonRustFiles(files.filter((file) => !RUST_PATH.test(file.path)), original.worktree, collectLsp);
  const independentReplayProjection = { baseline_tree_oid: git(['rev-parse', `${baseline}^{tree}`]).trim(), source_listing_sha256: bytesDigest(git(['ls-tree', '-r', '-z', baseline], null)), graph_schema_sha256: digest(replaySchema), graph_queries: replayQueries, cargo_test_list_stdout_sha256: replayCargo(['test', '-p', LEGACY_BASELINE_TYPES_PACKAGE, '--test', 'coords', '--', '--list']), cargo_test_results_stdout_sha256: replayCargo(['test', '-p', LEGACY_BASELINE_TYPES_PACKAGE, '--test', 'coords', 'world_to_chunk_local_roundtrip', '--', '--exact']), lsp: lspProjection([...replayRust, ...replayNonRust]) };
  if (canonical(primaryProjection) !== canonical(independentReplayProjection)) fail('Task 4 independent collection replay differs from the primary deterministic projection');
  const rustLifecycle = await stopRustLsp();
  if (rustLifecycle.sessions !== 1 || rustLifecycle.shutdown !== 'confirmed' || rustLifecycle.retained_processes !== 0) fail('direct rust-analyzer lifecycle cleanup failed');
  write(path.join(output, 'task-4-rust-lsp-lifecycle.json'), { schema_version: 1, run_id: runId, source_baseline_sha: baseline, provider: { ...RUST_LSP_PROVIDER, root_path: original.worktree }, lifecycle: rustLifecycle });
  const entries = reproducibilityEntries(output);
  write(path.join(output, 'task-4-reproducibility-manifest.json'), { schema_version: 2, run_id: runId, source_baseline_sha: baseline, verdict: 'pass', collection_protocol: 'two independent frozen-baseline git/cargo/graph/LSP collections compared by deterministic projection', primary_projection: primaryProjection, independent_replay_projection: independentReplayProjection, projection_sha256: digest(primaryProjection), entries, entries_sha256: digest(entries) });
  process.stdout.write(`task4_evidence=132 sources 105_lsp_complete 27_lsp_incomplete 61_capabilities pass\n${relative(output)}\n`);
}

main().catch((error) => { process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`); process.exitCode = 1; }).finally(async () => { await stopRustLsp(); await stopLspRuntime(); });
