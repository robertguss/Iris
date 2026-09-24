import { spawn, execFileSync } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { readFile, writeFile, mkdir, rm, lstat, readlink } from 'node:fs/promises';
import { resolve, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { checks, conventions, profiles } from './catalog.mjs';
import { reportSchema, scope } from './schema.mjs';

export const root = resolve(fileURLToPath(new URL('../..', import.meta.url)));
const directory = join(root, 'target/iris-verification');
const ids = Object.keys(checks);
const digest = data => createHash('sha256').update(data).digest('hex');

async function interfaceFingerprint() {
  const hash = createHash('sha256');
  for (const name of ['catalog.mjs', 'runner.mjs', 'schema.mjs', 'server.mjs', 'cli.mjs', 'package.json', 'package-lock.json']) {
    hash.update(await readFile(join(root, 'experiments/agent-interface', name)));
  }
  return hash.digest('hex');
}
const loadedInterface = await interfaceFingerprint();
export async function ensureCurrentInterface() {
  if (loadedInterface !== await interfaceFingerprint()) throw new Error('interface_changed_restart_required');
}

export function failure(error) {
  const reason = error?.message === 'interface_changed_restart_required' ? 'interface_changed_restart_required'
    : error?.code === 'EEXIST' ? 'active_run_lock' : 'operation_unavailable';
  return { status: 'blocked', reason, guidance: reason === 'interface_changed_restart_required' ? 'Restart the MCP server to load changed interface code.'
    : reason === 'active_run_lock' ? 'Wait for the active run. Remove target/iris-verification/active only after confirming no check is running.'
    : 'Check arguments, run ID and prerequisites. Use describe to discover supported checks. Raw errors are not exported.' };
}

export async function fingerprint() {
  const paths = execFileSync('git', ['ls-files', '-z', '--cached', '--others', '--exclude-standard'], { cwd: root, stdio: ['ignore', 'pipe', 'pipe'], timeout: 10_000 }).toString().split('\0')
    .filter(p => p && !p.startsWith('.amp/in/'));
  const hash = createHash('sha256');
  for (const path of [...new Set(paths)].sort()) {
    hash.update(path + '\0');
    try {
      const stat = await lstat(join(root, path));
      hash.update(stat.isSymbolicLink() ? await readlink(join(root, path)) : await readFile(join(root, path)));
    } catch (error) { if (error.code === 'ENOENT') hash.update('missing'); else throw error; }
    hash.update('\0');
  }
  return hash.digest('hex');
}

export function execute(command, env, timeoutMs = 180_000) {
  return new Promise(resolveResult => {
    const start = Date.now();
    const child = spawn(command[0], command.slice(1), { cwd: root, env, shell: false, detached: true, stdio: ['ignore', 'pipe', 'pipe'] });
    let stdout = '', bytes = 0, reason = null;
    const stop = why => {
      reason ??= why;
      if (child.pid) { try { process.kill(-child.pid, 'SIGKILL'); } catch {} }
    };
    const timer = setTimeout(() => stop('timeout'), timeoutMs);
    child.stdout.on('data', data => { bytes += data.length; if (bytes > 2_000_000) stop('output_limit'); else stdout += data; });
    child.stderr.on('data', data => { bytes += data.length; if (bytes > 2_000_000) stop('output_limit'); });
    child.on('error', () => { reason = 'prerequisite_unavailable'; });
    child.on('close', code => { clearTimeout(timer); resolveResult({ code, reason, stdout, duration_ms: Date.now() - start }); });
  });
}

export function observations(id, raw) {
  const expected = checks[id].events;
  const rows = raw.trim() ? raw.trim().split('\n') : [];
  let invalid = rows.length > expected.length;
  const events = [];
  rows.forEach((row, index) => {
    const [name, value, extra] = row.split('\t');
    if (extra !== undefined || name !== expected[index]?.[0] || !/^-?\d{1,10}$/.test(value ?? '')) { invalid = true; return; }
    events.push({ sequence: index + 1, event: name, expected: expected[index][1], observed: Number(value), matched: Number(value) === expected[index][1] });
  });
  return { events, complete: !invalid && events.length === expected.length, invalid };
}

export function compilerDiagnostics(stdout) {
  const diagnostics = [];
  for (const line of stdout.split('\n')) {
    let record;
    try { record = JSON.parse(line); } catch { continue; }
    if (record?.reason !== 'compiler-message' || record.message?.level !== 'error') continue;
    const code = record.message.code?.code;
    if (/^E\d{4}$/.test(code ?? '')) diagnostics.push({ code });
  }
  return diagnostics.slice(0, 100);
}

export function classify(id, result, raw) {
  const evidence = observations(id, raw);
  const mismatch = evidence.events.some(e => !e.matched);
  const status = result.reason ? 'blocked' : result.code !== 0 || mismatch ? 'failed' : !evidence.complete ? 'blocked' : 'passed';
  const reason = result.reason ?? (mismatch ? 'invariant_mismatch' : result.code !== 0 ? 'check_failed' : !evidence.complete ? 'incomplete_evidence' : 'completed');
  return { id, status, reason, exit_code: result.code, duration_ms: result.duration_ms, command: checks[id].command,
    sources: checks[id].sources, events: evidence.events, evidence_complete: evidence.complete,
    diagnostics: id === 'compile' ? compilerDiagnostics(result.stdout ?? '') : [],
    reproduce: ['node', 'experiments/agent-interface/cli.mjs', 'reproduce', id] };
}

function environment(evidence) {
  const env = { IRIS_EVIDENCE_FILE: evidence, CARGO_TERM_COLOR: 'never' };
  for (const key of ['PATH', 'HOME', 'CARGO_HOME', 'RUSTUP_HOME', 'TMPDIR']) if (process.env[key]) env[key] = process.env[key];
  return env;
}

async function version(binary) {
  const result = await execute([binary, '--version'], environment(''), 10_000);
  return result.code === 0 ? result.stdout.match(new RegExp(`^${binary} \\d+\\.\\d+\\.\\d+`))?.[0] ?? null : null;
}

export async function describe() {
  await ensureCurrentInterface();
  return { name: 'Iris', interface_version: 1, source_digest: await fingerprint(), profiles, checks, conventions,
    scope: 'Trusted local checkout only. Allowlisted commands still execute repository code. Not a security sandbox, production inspector, or protected evaluator.' };
}

export async function run(selected) {
  await ensureCurrentInterface();
  if (!Array.isArray(selected) || !selected.length || selected.some(id => !ids.includes(id))) throw new Error('Unknown check');
  await mkdir(directory, { recursive: true, mode: 0o700 });
  // Serializes CLI and MCP processes sharing this checkout. A stale lock fails closed.
  const lock = join(directory, 'active');
  await mkdir(lock);
  const id = randomUUID();
  const scratch = join(directory, `${id}.events`);
  try {
    const before = await fingerprint();
    const rust_version = await version('rustc');
    const cargo_version = await version('cargo');
    const results = [];
    for (const check of selected) {
      await writeFile(scratch, '', { mode: 0o600 });
      const result = await execute(checks[check].command, environment(scratch));
      const stat = await lstat(scratch);
      const raw = stat.size <= 16_384 ? await readFile(scratch, 'utf8') : 'invalid';
      results.push(classify(check, result, raw));
    }
    const after = await fingerprint();
    const report = reportSchema.parse({ schema_version: 1, id, created_at: new Date().toISOString(), source_digest: before,
      source_changed_during_run: before !== after,
      lock_digest: digest(await readFile(join(root, 'Cargo.lock'))), node_version: process.version, rust_version, cargo_version,
      status: before !== after ? 'blocked' : results.some(r => r.status === 'failed') ? 'failed' : results.some(r => r.status === 'blocked') ? 'blocked' : 'passed',
      plan: selected, results, scope });
    await writeFile(join(directory, `${id}.json`), JSON.stringify(report, null, 2) + '\n', { mode: 0o600, flag: 'wx' });
    return report;
  } finally { await rm(scratch, { force: true }); await rm(lock, { recursive: true, force: true }); }
}

export async function inspect(id) {
  if (!/^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/.test(id)) throw new Error('Invalid run ID');
  const path = join(directory, `${id}.json`);
  const stat = await lstat(path);
  if (!stat.isFile() || stat.size > 128_000) throw new Error('Invalid report');
  const report = reportSchema.parse(JSON.parse(await readFile(path, 'utf8')));
  if (report.id !== id) throw new Error('Invalid report');
  return { report, stale: report.source_changed_during_run || report.source_digest !== await fingerprint() };
}
