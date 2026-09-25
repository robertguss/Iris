import test from 'node:test';
import assert from 'node:assert/strict';
import { randomUUID } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { mkdir, writeFile, readFile, rm } from 'node:fs/promises';
import { join } from 'node:path';
import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { StdioClientTransport } from '@modelcontextprotocol/sdk/client/stdio.js';
import { classify, execute, fingerprint, inspect, root, run } from './runner.mjs';

const auth = 'provider_waiting\t1\nlogout_status\t204\ncallback_status\t401\nremaining_sessions\t0\n';
const ok = { code: 0, reason: null, duration_ms: 1 };
const canary = 'IRIS_SECRET_CANARY_not_a_real_credential';

test('reports distinguish observed violations, missing evidence, and runner failures', () => {
  assert.equal(classify('auth', ok, auth).status, 'passed');
  const wrong = classify('auth', ok, auth.replace('callback_status\t401', 'callback_status\t200'));
  assert.equal(wrong.status, 'failed');
  assert.deepEqual(wrong.events[2], { sequence: 3, event: 'callback_status', expected: 401, observed: 200, matched: false });
  for (const raw of ['', auth.split('\n').slice(0, 2).join('\n'), auth + auth, auth.replace('204', canary), auth.split('\n').reverse().join('\n')]) {
    const result = classify('auth', ok, raw);
    assert.equal(result.status, 'blocked');
    assert.equal(result.evidence_complete, false);
    assert.ok(!JSON.stringify(result).includes(canary));
  }
  assert.equal(classify('auth', { ...ok, code: 101 }, auth).status, 'failed');
  assert.equal(classify('auth', { ...ok, reason: 'timeout' }, auth).status, 'blocked');
});

test('compiler evidence exports codes, not messages, source excerpts, or process output', () => {
  const stdout = JSON.stringify({ reason: 'compiler-message', message: { level: 'error', code: { code: 'E0308' }, message: canary, rendered: canary } });
  const report = classify('compile', { ...ok, code: 101, stdout }, '');
  assert.deepEqual(report.diagnostics, [{ code: 'E0308' }]);
  assert.ok(!JSON.stringify(report).includes(canary));
  assert.deepEqual(classify('auth', { ...ok, stdout }, auth).diagnostics, []);
});

test('process failures are bounded and missing executables do not look like test failures', async () => {
  assert.equal((await execute(['/iris-no-such-executable'], {})).reason, 'prerequisite_unavailable');
  assert.equal((await execute([process.execPath, '-e', 'setInterval(() => {}, 1000)'], {}, 100)).reason, 'timeout');
  assert.equal((await execute([process.execPath, '-e', 'process.stdout.write("x".repeat(2100000))'], {})).reason, 'output_limit');
  const result = await execute([process.execPath, '-e', `console.error('${canary}'); process.exit(1)`], {});
  assert.equal(result.code, 1);
  assert.ok(!JSON.stringify(classify('auth', result, '')).includes(canary));
});

test('MCP discovery, real focused checks, report inspection, stale detection and serialization', { timeout: 420_000 }, async t => {
  const transport = new StdioClientTransport({ command: process.execPath, args: [join(root, 'experiments/agent-interface/server.mjs')], stderr: 'pipe' });
  const client = new Client({ name: 'iris-protocol-test', version: '1.0.0' });
  t.after(() => client.close());
  await client.connect(transport);
  const tools = await client.listTools();
  assert.equal(tools.tools.length, 6);
  assert.ok(tools.tools.every(tool => tool.outputSchema));
  const call = (name, args = {}) => client.callTool({ name, arguments: args }, undefined, { timeout: 400_000 });
  const description = await call('describe_application');
  assert.equal(description.structuredContent.data.name, 'Iris');
  const resource = await client.readResource({ uri: 'iris://conventions' });
  assert.match(resource.contents[0].text, /not a protected evaluator/);
  const convention = await call('get_convention', { topic: 'delivery' });
  assert.match(convention.structuredContent.data.rule, /stale workers/);
  const memberConvention = await call('get_convention', { topic: 'membership' });
  assert.match(memberConvention.structuredContent.data.rule, /last owner/);
  assert.equal((await call('run_checks', { profile: 'arbitrary-command' })).isError, true);
  assert.equal((await call('run_checks', { profile: 'focused', command: 'anything' })).isError, true);
  assert.equal((await call('inspect_run', { id: '../../etc/passwd' })).isError, true);
  await assert.rejects(run(['not-a-check']));

  const report = (await call('run_checks', { profile: 'focused' })).structuredContent.data;
  assert.equal(report.status, 'passed');
  assert.deepEqual(report.results.map(r => r.events.map(e => e.observed)), [[1, 204, 401, 0], [1, 2, 0, 3, 1], [3, 3, 3, 1]]);
  assert.match(report.rust_version, /^rustc /);
  assert.equal((await call('inspect_run', { id: report.id })).structuredContent.data.stale, false);
  const operation = (await call('inspect_operation', { id: report.id, check: 'outbox' })).structuredContent.data;
  assert.equal(operation.result.events[2].observed, 0);
  const membership = (await call('inspect_operation', { id: report.id, check: 'members' })).structuredContent.data;
  assert.equal(membership.result.events[3].observed, 1);
  assert.equal((await call('reproduce_scenario', { scenario: 'outbox' })).structuredContent.data.status, 'passed');
  assert.equal((await call('reproduce_scenario', { scenario: 'members' })).structuredContent.data.status, 'passed');

  const probe = join(root, `.iris-fingerprint-${randomUUID()}`);
  const before = await fingerprint();
  try {
    await writeFile(probe, 'uncommitted input');
    assert.notEqual(await fingerprint(), before);
    assert.equal((await inspect(report.id)).stale, true);
  } finally { await rm(probe, { force: true }); }
  assert.equal(await fingerprint(), before);

  const lock = join(root, 'target/iris-verification/active');
  await mkdir(lock);
  try { await assert.rejects(run(['outbox']), { code: 'EEXIST' }); }
  finally { await rm(lock, { recursive: true }); }

  const corruptId = randomUUID();
  const corrupt = join(root, `target/iris-verification/${corruptId}.json`);
  try {
    await writeFile(corrupt, JSON.stringify({ ...report, id: corruptId, raw_output: canary }));
    const response = await call('inspect_run', { id: corruptId });
    assert.equal(response.isError, true);
    assert.ok(!JSON.stringify(response).includes(canary));
    await writeFile(corrupt, JSON.stringify({ ...report, id: corruptId, status: 'blocked', source_changed_during_run: true }));
    assert.equal((await call('inspect_run', { id: corruptId })).structuredContent.data.stale, true);
    assert.equal((await call('inspect_operation', { id: corruptId, check: 'auth' })).structuredContent.data.stale, true);
  } finally { await rm(corrupt, { force: true }); }

  const catalog = join(root, 'experiments/agent-interface/catalog.mjs');
  const original = await readFile(catalog, 'utf8');
  try {
    const changed = original.replace("['callback_status', 401]", "['callback_status', 403]");
    assert.notEqual(changed, original);
    await writeFile(catalog, changed);
    for (const name of ['describe_application', 'run_checks']) {
      const response = await call(name, name === 'run_checks' ? { profile: 'focused' } : {});
      assert.equal(response.isError, true);
      assert.match(response.content[0].text, /interface_changed_restart_required/);
    }
  } finally { await writeFile(catalog, original); }
});

test('Git stderr cannot bypass CLI or MCP error filtering', async t => {
  const bin = join(root, `target/iris-git-canary-${randomUUID()}`);
  await mkdir(bin, { recursive: true });
  t.after(() => rm(bin, { recursive: true, force: true }));
  await writeFile(join(bin, 'git'), `#!/bin/sh\nprintf '${canary}' >&2\nexit 1\n`, { mode: 0o700 });
  const env = { PATH: bin, HOME: process.env.HOME };
  try {
    execFileSync(process.execPath, [join(root, 'experiments/agent-interface/cli.mjs'), 'describe'], { env, stdio: ['ignore', 'pipe', 'pipe'] });
    assert.fail('Git must fail');
  } catch (error) {
    assert.equal(error.status, 2);
    assert.equal(error.stderr.toString(), '');
    assert.ok(!error.stdout.toString().includes(canary));
  }
  const transport = new StdioClientTransport({ command: process.execPath, args: [join(root, 'experiments/agent-interface/server.mjs')], env, stderr: 'pipe' });
  let stderr = '';
  transport.stderr.on('data', chunk => { stderr += chunk; });
  const client = new Client({ name: 'iris-canary-test', version: '1.0.0' });
  t.after(() => client.close());
  await client.connect(transport);
  const response = await client.callTool({ name: 'describe_application', arguments: {} });
  assert.equal(response.isError, true);
  assert.ok(!JSON.stringify(response).includes(canary));
  assert.equal(stderr, '');
});
