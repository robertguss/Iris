// Tests for the development command (`dev.mjs`) and the supervision it shares
// with the browser runner (`supervise.mjs`). The command owns the fixed ports
// 4001, 3003 and 5175, so these tests refuse to start while any is taken, run
// one at a time, and must not run beside the browser workflow:
//
//   node --test apps/reference/scripts/test/dev.test.mjs
//
// Every command runs in its own process group. After each test, every process
// group it reported (and the command's own) is killed, so a failed assertion
// leaks nothing. Each test's data lives in its own temporary directory.
import assert from "node:assert/strict";
import { execFileSync, spawn } from "node:child_process";
import {
  chmodSync,
  copyFileSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { connect, createServer } from "node:net";
import { createServer as createHttpServer } from "node:http";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { afterEach, before, test } from "node:test";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "../../../..");
const DEV = join(root, "apps/reference/scripts/dev.mjs");
const GATED = join(here, "gated-supervisor.mjs");
const ORIGIN = "http://127.0.0.1:5175";
const API = "http://127.0.0.1:3003";
const ISSUER = "http://127.0.0.1:4001";
const PORTS = [4001, 3003, 5175];
// Generous for a start with every build cached; the build itself is done once
// before the tests.
const START = 60_000;

const sleep = (ms) => new Promise((done) => setTimeout(done, ms));
const temporary = () => mkdtempSync(join(tmpdir(), "iris-dev-test-"));

/** Whether something accepts connections on the port. */
function listening(port) {
  return new Promise((done) => {
    const socket = connect({ host: "127.0.0.1", port });
    socket.once("connect", () => {
      socket.destroy();
      done(true);
    });
    socket.once("error", () => done(false));
  });
}
async function portsFree() {
  for (const port of PORTS)
    assert.equal(await listening(port), false, `port ${port} still accepts`);
}
/** Whether no process in the group remains. */
function gone(pid) {
  try {
    process.kill(-pid, 0);
    return false;
  } catch (error) {
    return error.code === "ESRCH";
  }
}

const launched = [];
/** Runs a script as a command in its own process group, capturing output. */
function launch(script, args, { env = process.env, cwd = root } = {}) {
  const child = spawn(process.execPath, [script, ...args], {
    cwd,
    env,
    detached: true,
    stdio: ["ignore", "pipe", "pipe"],
  });
  const command = { child, output: "", started: Date.now() };
  child.stdout.on("data", (chunk) => (command.output += chunk));
  child.stderr.on("data", (chunk) => (command.output += chunk));
  command.exited = new Promise((done) =>
    child.once("exit", (code, signal) => done({ code, signal })),
  );
  command.exited.then(() => (command.done = true));
  launched.push(command);
  return command;
}
const command = (args, options) => launch(DEV, args, options);

afterEach(async () => {
  for (const command of launched.splice(0)) {
    const groups = [
      command.child.pid,
      ...[...command.output.matchAll(/pid (\d+)/g)].map((m) => Number(m[1])),
    ];
    for (const pid of groups)
      for (const signal of ["SIGCONT", "SIGKILL"])
        try {
          process.kill(-pid, signal);
        } catch {}
    await command.exited;
  }
});

/** Waits, bounded, until the output matches; returns the match. */
async function until(command, pattern, ms = START) {
  const deadline = Date.now() + ms;
  for (;;) {
    const match = command.output.match(pattern);
    if (match) return match;
    if (Date.now() > deadline)
      throw new Error(`no ${pattern} within ${ms} ms:\n${command.output}`);
    await sleep(20);
  }
}
/** Waits, bounded, for the command's exit. */
async function exit(command, ms = 20_000) {
  const result = await Promise.race([command.exited, sleep(ms)]);
  if (!result)
    throw new Error(`still running after ${ms} ms:\n${command.output}`);
  return result;
}
/** The process group a command reported for one of its children. */
async function pid(command, name) {
  return Number(
    (
      await until(command, new RegExp(`dev: started ${name} \\(pid (\\d+)\\)`))
    )[1],
  );
}
const ready = (command) => until(command, /^dev: ready: .*$/m);
/** The command's last line: its exit code and the stop's cause. */
function final(command) {
  const lines = command.output.trimEnd().split("\n");
  return lines[lines.length - 1];
}

/** A listener that holds a port for the length of a test. */
function hold(port) {
  const server = createServer();
  return new Promise((done, fail) => {
    server.once("error", fail);
    server.listen(port, "127.0.0.1", () => done(server));
  });
}
/** An HTTP server on an ephemeral port that counts every request. */
async function sentinel() {
  const seen = [];
  const server = createHttpServer((request, response) => {
    seen.push(`${request.method} ${request.url}`);
    response.writeHead(500).end();
  });
  await new Promise((done) => server.listen(0, "127.0.0.1", done));
  return { server, seen, port: server.address().port };
}

const cookieOf = (response) => response.headers.getSetCookie()[0].split(";")[0];
/** Signs in as Alice through the issuer, directly against the API. */
async function signIn() {
  const session = await fetch(`${API}/api/auth/session`);
  const cookie = cookieOf(session);
  const info = await session.json();
  const login = await fetch(`${API}/api/auth/login`, {
    method: "POST",
    headers: { origin: ORIGIN, "x-iris-csrf": info.csrf_token, cookie },
  });
  assert.equal(login.status, 200);
  const url = new URL((await login.json()).authorization_url);
  const form = new URLSearchParams(url.searchParams);
  form.append("identity", "alice");
  const chosen = await fetch(`${url.origin}/authorize`, {
    method: "POST",
    body: form,
    redirect: "manual",
  });
  assert.equal(chosen.status, 303);
  const callback = new URL(chosen.headers.get("location"));
  const completed = await fetch(
    `${API}${callback.pathname}${callback.search}`,
    {
      headers: { cookie },
      redirect: "manual",
    },
  );
  assert.equal(completed.status, 303);
  return cookieOf(completed);
}
async function session(cookie) {
  const response = await fetch(`${API}/api/auth/session`, {
    headers: cookie ? { cookie } : {},
  });
  assert.equal(response.status, 200);
  return response.json();
}
/**
 * A role change whose headers the API admitted (its `100 Continue`) and whose
 * body stays partly unsent, so the API's drain cannot finish.
 */
async function admit(cookie) {
  const { csrf_token } = await session(cookie);
  const body = JSON.stringify({
    project_id: "41",
    user_id: "29",
    role: "viewer",
  });
  const socket = connect({ host: "127.0.0.1", port: 3003 });
  await new Promise((done) => socket.once("connect", done));
  let received = "";
  socket.on("data", (chunk) => (received += chunk));
  socket.on("error", () => {});
  socket.write(
    `POST /api/memberships/role HTTP/1.1\r\nhost: 127.0.0.1:3003\r\norigin: ${ORIGIN}\r\n` +
      `x-iris-csrf: ${csrf_token}\r\ncookie: ${cookie}\r\ncontent-type: application/json\r\n` +
      `content-length: ${body.length}\r\nexpect: 100-continue\r\n\r\n`,
  );
  const deadline = Date.now() + 10_000;
  while (!received.includes("\r\n\r\n")) {
    if (Date.now() > deadline) throw new Error("no interim response");
    await sleep(10);
  }
  assert.match(received, /^HTTP\/1\.1 100 Continue\r\n/);
  socket.write(body.slice(0, 10));
  return socket;
}

before(async () => {
  for (const port of PORTS)
    if (await listening(port))
      throw new Error(
        `port ${port} is taken; these tests must own ${PORTS.join(", ")}`,
      );
  // Built once here, so each start below finds it current.
  execFileSync(
    "cargo",
    [
      "build",
      "--quiet",
      "--locked",
      "-p",
      "iris-reference",
      "--bin",
      "reference-dev",
    ],
    { cwd: root, stdio: "inherit" },
  );
});

test("an unknown argument is a usage error that starts nothing", async () => {
  const dir = temporary();
  const run = command(["--database", join(dir, "dev.db"), "--verbose"]);
  const { code } = await exit(run);
  assert.equal(code, 2, run.output);
  assert.match(run.output, /usage: node apps\/reference\/scripts\/dev\.mjs/);
  assert.doesNotMatch(run.output, /building|started/);
  assert.deepEqual(readdirSync(dir), []);
});

test("a taken port refuses startup before anything starts", async () => {
  for (const port of PORTS) {
    const dir = temporary();
    const database = join(dir, "dev.db");
    const holder = await hold(port);
    try {
      const run = command(["--database", database]);
      const { code } = await exit(run);
      assert.equal(code, 1, run.output);
      assert.match(run.output, new RegExp(`already in use: ${port}\\b`));
      assert.doesNotMatch(run.output, /building reference-dev|dev: started/);
      assert.equal(existsSync(database), false);
      assert.equal(existsSync(`${database}.iris-lock`), false);
      for (const other of PORTS.filter((p) => p !== port))
        assert.equal(await listening(other), false, `port ${other}`);
    } finally {
      await new Promise((done) => holder.close(done));
    }
  }
});

test("a signal during the build stops it and starts no server", async () => {
  // A stop is how serving ends (exit 0); a reset it interrupts did not happen.
  for (const [signal, reset, expected] of [
    ["SIGINT", false, 0],
    ["SIGTERM", false, 0],
    ["SIGINT", true, 1],
  ]) {
    const dir = temporary();
    const bin = join(dir, "bin");
    execFileSync("mkdir", [bin]);
    writeFileSync(
      join(bin, "cargo"),
      '#!/bin/sh\necho "shim cargo pid $$"\ntrap "exit 0" TERM\nwhile :; do sleep 0.05; done\n',
    );
    chmodSync(join(bin, "cargo"), 0o755);
    const database = join(dir, "dev.db");
    const run = command(
      ["--database", database, ...(reset ? ["--reset"] : [])],
      { env: { ...process.env, PATH: `${bin}:${process.env.PATH}` } },
    );
    const shim = Number((await until(run, /shim cargo pid (\d+)/))[1]);
    await pid(run, "build");
    const signalled = Date.now();
    process.kill(run.child.pid, signal);
    const { code } = await exit(run);
    assert.ok(Date.now() - signalled < 5_000, run.output);
    assert.equal(code, expected, run.output);
    assert.match(
      final(run),
      new RegExp(`^dev: exit ${expected}: received ${signal}$`),
    );
    assert.doesNotMatch(run.output, /SIGKILL/);
    assert.doesNotMatch(
      run.output,
      /dev: started (issuer|api|web|reset)|dev: ready/,
    );
    assert.ok(gone(shim), "the build still runs");
    assert.equal(existsSync(database), false);
    await portsFree();
  }
});

test("inherited addresses reach neither the proxy nor the API", async () => {
  const target = await sentinel();
  const other = await sentinel();
  try {
    const dir = temporary();
    const database = join(dir, "dev.db");
    const run = command(["--database", database], {
      env: {
        ...process.env,
        IRIS_API_TARGET: `http://127.0.0.1:${target.port}`,
        IRIS_OIDC_ISSUER: `http://127.0.0.1:${target.port}`,
        IRIS_LISTEN: `127.0.0.1:${other.port}`,
        IRIS_PUBLIC_ORIGIN: `http://127.0.0.1:${other.port}`,
      },
    });
    await ready(run);
    const through = await fetch(`${ORIGIN}/api/auth/session`);
    assert.equal(through.status, 200);
    assert.equal(typeof (await through.json()).csrf_token, "string");
    const discovery = await fetch(`${ISSUER}/.well-known/openid-configuration`);
    assert.equal((await discovery.json()).issuer, ISSUER);
    // The API's origin is the console's: a login from it is accepted.
    const cookie = await signIn();
    assert.equal((await session(cookie)).user_id, "11");
    assert.match(
      run.output,
      new RegExp(`\\[api\\] .*persistent data at .*dev\\.db`),
    );
    assert.deepEqual(target.seen, []);
    assert.deepEqual(other.seen, []);
  } finally {
    target.server.close();
    other.server.close();
  }
});

test("readiness is announced after all three, and a request goes through at once", async () => {
  const run = command(["--database", join(temporary(), "dev.db")]);
  await ready(run);
  const through = await fetch(`${ORIGIN}/api/auth/session`);
  assert.equal(through.status, 200);
  const at = (pattern) => run.output.search(pattern);
  const order = [
    at(/dev: started issuer/),
    at(/\[issuer\] .*listening on 127\.0\.0\.1:4001/),
    at(/dev: started api/),
    at(/\[api\] listening on http:\/\/127\.0\.0\.1:3003/),
    at(/dev: started web/),
    at(/\[web\] .*Local:\s+http:\/\/127\.0\.0\.1:5175\//),
    at(/dev: ready: /),
  ];
  assert.ok(
    order.every((index) => index >= 0),
    run.output,
  );
  assert.deepEqual(
    [...order].sort((a, b) => a - b),
    order,
    run.output,
  );
  assert.match(
    run.output,
    /dev: ready: console http:\/\/127\.0\.0\.1:5175, API http:\/\/127\.0\.0\.1:3003, issuer http:\/\/127\.0\.0\.1:4001, data \S*dev\.db/,
  );
});

test("a server that fails before its readiness stops the others", async () => {
  const run = command(["--database", join(temporary(), "missing", "dev.db")]);
  const issuer = await pid(run, "issuer");
  const { code } = await exit(run);
  assert.equal(code, 1, run.output);
  assert.match(final(run), /^dev: exit 1: api exited \(1\)/);
  assert.doesNotMatch(run.output, /dev: started web|dev: ready/);
  assert.ok(gone(issuer));
  await portsFree();
});

/** The gated supervision, with its gate directory. */
function gated(timeout = 60_000, stubborn = "") {
  const dir = temporary();
  return {
    dir,
    run: launch(GATED, [dir, String(timeout), stubborn]),
    open: (name) => writeFileSync(join(dir, `${name}.go`), ""),
  };
}

test("gated servers start one at a time and readiness waits for the last", async () => {
  const { run, open } = gated();
  await until(run, /^\[a\] a spawned$/m);
  await sleep(500);
  assert.doesNotMatch(run.output, /dev: started b|dev: ready/);
  open("a");
  await until(run, /^\[b\] b spawned$/m);
  await sleep(300);
  assert.doesNotMatch(run.output, /dev: started c|dev: ready/);
  open("b");
  await until(run, /^\[c\] c spawned$/m);
  await sleep(300);
  assert.doesNotMatch(run.output, /dev: ready/);
  open("c");
  await until(run, /^dev: ready: gated servers$/m);
});

test("a readiness timeout stops the started servers and starts no more", async () => {
  const { run, open } = gated(1_000);
  const a = await pid(run, "a");
  open("a");
  const b = await pid(run, "b");
  const { code } = await exit(run, 8_000);
  assert.equal(code, 1, run.output);
  assert.match(final(run), /^dev: exit 1: b not ready within 1 s$/);
  assert.doesNotMatch(run.output, /dev: started c|c spawned|dev: ready/);
  assert.ok(gone(a) && gone(b));
});

test("a signal while readiness is pending stops without starting the rest", async () => {
  const { run, open } = gated();
  const a = await pid(run, "a");
  open("a");
  const b = await pid(run, "b");
  const signalled = Date.now();
  process.kill(run.child.pid, "SIGINT");
  const { code } = await exit(run);
  assert.ok(Date.now() - signalled < 5_000, run.output);
  assert.equal(code, 0, run.output);
  assert.match(final(run), /^dev: exit 0: received SIGINT$/);
  open("b");
  open("c");
  await sleep(1_000);
  assert.doesNotMatch(run.output, /dev: started c|c spawned/);
  assert.ok(gone(a) && gone(b));
});

test("an unexpected exit while readiness is pending fails the command", async () => {
  const { run, open } = gated();
  const a = await pid(run, "a");
  open("a");
  const b = await pid(run, "b");
  process.kill(-a, "SIGKILL");
  const { code } = await exit(run);
  assert.equal(code, 1, run.output);
  assert.match(final(run), /^dev: exit 1: a exited \(SIGKILL\)$/);
  open("b");
  await sleep(500);
  assert.doesNotMatch(run.output, /dev: started c|c spawned/);
  assert.ok(gone(b));
});

test("any child's unexpected exit stops the others and exits 1", async () => {
  for (const name of ["issuer", "api", "web"]) {
    const run = command(["--database", join(temporary(), "dev.db")]);
    await ready(run);
    const groups = {
      issuer: await pid(run, "issuer"),
      api: await pid(run, "api"),
      web: await pid(run, "web"),
    };
    process.kill(-groups[name], "SIGKILL");
    const { code } = await exit(run);
    assert.equal(code, 1, run.output);
    assert.match(
      final(run),
      new RegExp(`^dev: exit 1: ${name} exited \\(SIGKILL\\)$`),
    );
    for (const group of Object.values(groups))
      assert.ok(gone(group), run.output);
    await portsFree();
  }
});

test("a signal after an unexpected exit keeps the first failure", async () => {
  const run = command(["--database", join(temporary(), "dev.db")]);
  await ready(run);
  const issuer = await pid(run, "issuer");
  const api = await pid(run, "api");
  // The stopped issuer keeps the stop pending for 5 s.
  process.kill(-issuer, "SIGSTOP");
  process.kill(-api, "SIGKILL");
  await until(run, /^dev: stopping: api exited \(SIGKILL\)$/m, 10_000);
  process.kill(run.child.pid, "SIGINT");
  const { code } = await exit(run);
  assert.equal(code, 1, run.output);
  assert.match(run.output, /received SIGINT; already stopping/);
  assert.match(
    final(run),
    /^dev: exit 1: api exited \(SIGKILL\); issuer needed SIGKILL$/,
  );
  assert.ok(gone(issuer));
});

test("SIGINT and SIGTERM stop all three process groups and exit 0", async () => {
  for (const signal of ["SIGINT", "SIGTERM"]) {
    const database = join(temporary(), "dev.db");
    const run = command(["--database", database]);
    await ready(run);
    const groups = [
      await pid(run, "issuer"),
      await pid(run, "api"),
      await pid(run, "web"),
    ];
    process.kill(run.child.pid, signal);
    const { code } = await exit(run);
    assert.equal(code, 0, run.output);
    assert.match(final(run), new RegExp(`^dev: exit 0: received ${signal}$`));
    assert.match(run.output, /\[api\] .*stopped after draining/);
    assert.match(run.output, /dev: api exited \(0\) during the stop/);
    for (const group of groups) assert.ok(gone(group), run.output);
    await portsFree();
    // The database is free at once.
    const reset = command(["--database", database, "--reset"]);
    assert.equal((await exit(reset, START)).code, 0, reset.output);
    assert.match(reset.output, /reset database at/);
  }
});

test("the API's exit 1 during a deliberate stop is reported, not a failure", async () => {
  const run = command(["--database", join(temporary(), "dev.db")]);
  await ready(run);
  const socket = await admit(await signIn());
  const signalled = Date.now();
  process.kill(run.child.pid, "SIGINT");
  const { code } = await exit(run);
  socket.destroy();
  assert.ok(Date.now() - signalled < 5_500, run.output);
  assert.equal(code, 0, run.output);
  assert.match(run.output, /\[api\] .*drain deadline expired/);
  assert.match(run.output, /dev: api exited \(1\) during the stop/);
  assert.match(final(run), /^dev: exit 0: received SIGINT$/);
});

test("a child that ignores SIGTERM is killed after 5 s and the stop exits 1", async () => {
  const run = command(["--database", join(temporary(), "dev.db")]);
  await ready(run);
  const issuer = await pid(run, "issuer");
  process.kill(-issuer, "SIGSTOP");
  const signalled = Date.now();
  process.kill(run.child.pid, "SIGINT");
  const { code } = await exit(run);
  const elapsed = Date.now() - signalled;
  assert.ok(elapsed >= 4_900 && elapsed < 8_000, `${elapsed} ms`);
  assert.equal(code, 1, run.output);
  assert.match(
    run.output,
    /dev: issuer did not exit within 5 s of SIGTERM; sent SIGKILL/,
  );
  assert.match(run.output, /dev: api exited \(0\) during the stop/);
  assert.match(
    final(run),
    /^dev: exit 1: received SIGINT; issuer needed SIGKILL$/,
  );
  assert.ok(gone(issuer));
  await portsFree();
});

test("repeated signals neither end nor shorten the stop", async () => {
  for (const [first, second, third] of [
    ["SIGINT", "SIGINT", "SIGTERM"],
    ["SIGTERM", "SIGTERM", "SIGINT"],
  ]) {
    const run = command(["--database", join(temporary(), "dev.db")]);
    await ready(run);
    const issuer = await pid(run, "issuer");
    process.kill(-issuer, "SIGSTOP");
    const signalled = Date.now();
    process.kill(run.child.pid, first);
    await sleep(1_000);
    process.kill(run.child.pid, second);
    await sleep(1_000);
    process.kill(run.child.pid, third);
    await sleep(2_000);
    assert.equal(run.done, undefined, `ended early:\n${run.output}`);
    const { code } = await exit(run);
    assert.ok(Date.now() - signalled >= 4_900, run.output);
    assert.equal(code, 1, run.output);
    assert.match(
      run.output,
      new RegExp(`received ${second}; already stopping`),
    );
    assert.match(run.output, new RegExp(`received ${third}; already stopping`));
    assert.match(
      final(run),
      new RegExp(`^dev: exit 1: received ${first}; issuer needed SIGKILL$`),
    );
    assert.ok(gone(issuer));
  }
});

test("a relative database persists, and a reset is refused while it is served", async () => {
  const dir = temporary();
  async function bobRole(cookie, expected, phase) {
    const response = await fetch(`${API}/api/projects/41/members`, {
      headers: { cookie },
    });
    assert.equal(response.status, 200, phase);
    const { data } = await response.json();
    assert.equal(
      data.items.find((member) => member.user_id === "29")?.role,
      expected,
      phase,
    );
  }
  const first = command(["--database", "dev.db"], { cwd: dir });
  await ready(first);
  assert.ok(existsSync(join(dir, "dev.db")));
  const firstCookie = await signIn();
  await bobRole(firstCookie, "editor", "initial seeded role");
  const { csrf_token } = await session(firstCookie);
  const changed = await fetch(`${API}/api/memberships/role`, {
    method: "POST",
    headers: {
      origin: ORIGIN,
      cookie: firstCookie,
      "x-iris-csrf": csrf_token,
      "content-type": "application/json",
    },
    body: JSON.stringify({ project_id: "41", user_id: "29", role: "viewer" }),
  });
  assert.equal(changed.status, 200, "sentinel role change");
  assert.equal((await changed.json()).data.completion, "acknowledged");
  await bobRole(firstCookie, "viewer", "sentinel setup");
  process.kill(first.child.pid, "SIGINT");
  assert.equal((await exit(first)).code, 0, first.output);

  const second = command(["--database", "dev.db"], { cwd: dir });
  await ready(second);
  const secondCookie = await signIn();
  await bobRole(secondCookie, "viewer", "after restart");
  const refused = command(["--database", "dev.db", "--reset"], { cwd: dir });
  const { code } = await exit(refused, START);
  assert.equal(code, 1, refused.output);
  assert.match(refused.output, /\[reset\] .*in use/);
  assert.match(final(refused), /^dev: exit 1: reset exited \(1\)$/);
  assert.equal((await fetch(`${ORIGIN}/api/auth/session`)).status, 200);
  await bobRole(secondCookie, "viewer", "after refused reset");
  process.kill(second.child.pid, "SIGINT");
  assert.equal((await exit(second)).code, 0, second.output);

  const reset = command(["--database", "dev.db", "--reset"], { cwd: dir });
  assert.equal((await exit(reset, START)).code, 0, reset.output);
  assert.match(reset.output, /\[reset\] .*reset database at/);
  assert.match(final(reset), /^dev: exit 0: reset completed$/);
  const third = command(["--database", "dev.db"], { cwd: dir });
  await ready(third);
  await bobRole(await signIn(), "editor", "after stopped reset");
  process.kill(third.child.pid, "SIGINT");
  assert.equal((await exit(third)).code, 0, third.output);
  await portsFree();
});

test("a reset binds the seeded identities to the command's issuer", async () => {
  const database = join(temporary(), "dev.db");
  const reset = command(["--database", database, "--reset"], {
    env: { ...process.env, IRIS_OIDC_ISSUER: "http://127.0.0.1:9" },
  });
  assert.equal((await exit(reset, START)).code, 0, reset.output);
  const run = command(["--database", database]);
  await ready(run);
  const cookie = await signIn();
  assert.equal((await session(cookie)).user_id, "11");
});

test("a leader's exit leaves no member of its group behind", async () => {
  const { run, open } = gated();
  const a = await pid(run, "a");
  open("a");
  const b = await pid(run, "b");
  // Only the leader: its helper stays in the group.
  process.kill(a, "SIGKILL");
  const { code } = await exit(run);
  assert.equal(code, 1, run.output);
  assert.match(final(run), /^dev: exit 1: a exited \(SIGKILL\)$/);
  assert.ok(gone(a), "a's group outlived the stop");
  assert.ok(gone(b), "b's group outlived the stop");
});

test("a member that ignores SIGTERM after its leader exits is killed", async () => {
  const { run, open } = gated(60_000, "a");
  const a = await pid(run, "a");
  open("a");
  open("b");
  open("c");
  await until(run, /^dev: ready: gated servers$/m);
  const signalled = Date.now();
  process.kill(run.child.pid, "SIGINT");
  const { code } = await exit(run);
  assert.ok(Date.now() - signalled >= 4_900, run.output);
  assert.equal(code, 1, run.output);
  assert.match(
    run.output,
    /dev: a did not exit within 5 s of SIGTERM; sent SIGKILL/,
  );
  assert.match(final(run), /^dev: exit 1: received SIGINT; a needed SIGKILL$/);
  assert.ok(gone(a));
});

/**
 * A throwaway checkout holding the command's two scripts and the issuer
 * fixture, with the web directory linked in and a `cargo` that builds
 * nothing, so the default database lands in it rather than in this checkout.
 * The server comes from the target directory built before the tests.
 */
function checkout() {
  const dir = temporary();
  const scripts = join(dir, "apps/reference/scripts");
  mkdirSync(scripts, { recursive: true });
  for (const file of ["dev.mjs", "supervise.mjs"])
    copyFileSync(
      join(root, "apps/reference/scripts", file),
      join(scripts, file),
    );
  symlinkSync(
    join(root, "apps/reference/web"),
    join(dir, "apps/reference/web"),
  );
  // Copied, not linked: the fixture serves only when run as the main module,
  // and Node resolves a link before comparing.
  const fixture = "experiments/api-slice/checks/oidc-provider.mjs";
  mkdirSync(dirname(join(dir, fixture)), { recursive: true });
  copyFileSync(join(root, fixture), join(dir, fixture));
  const bin = join(dir, "bin");
  mkdirSync(bin);
  writeFileSync(join(bin, "cargo"), "#!/bin/sh\nexit 0\n");
  chmodSync(join(bin, "cargo"), 0o755);
  return {
    dev: join(scripts, "dev.mjs"),
    data: join(dir, "apps/reference/.dev"),
    env: {
      ...process.env,
      PATH: `${bin}:${process.env.PATH}`,
      // Absolute: a relative target directory names one in this checkout.
      CARGO_TARGET_DIR: resolve(root, process.env.CARGO_TARGET_DIR ?? "target"),
    },
  };
}

test("an empty database path is a usage error, never the default", async () => {
  const { dev, data, env } = checkout();
  for (const args of [
    ["--database", "", "--reset"],
    ["--database", ""],
  ]) {
    const run = launch(dev, args, { env });
    const { code } = await exit(run);
    assert.equal(code, 2, run.output);
    assert.match(run.output, /usage: /);
    assert.doesNotMatch(run.output, /data at|building|started/);
    assert.equal(existsSync(data), false);
  }
});

test("the default database is created by a first reset and by a first start", async () => {
  for (const first of ["reset", "start"]) {
    const { dev, data, env } = checkout();
    const database = join(data, "reference.db");
    if (first === "reset") {
      const reset = launch(dev, ["--reset"], { env });
      assert.equal((await exit(reset, START)).code, 0, reset.output);
      assert.ok(existsSync(database), reset.output);
    }
    const run = launch(dev, [], { env });
    await ready(run);
    assert.match(
      run.output,
      new RegExp(
        `\\[api\\] .*persistent data at \\S*apps/reference/\\.dev/reference\\.db`,
      ),
    );
    assert.ok(existsSync(database));
    assert.equal((await fetch(`${ORIGIN}/api/auth/session`)).status, 200);
    process.kill(run.child.pid, "SIGINT");
    assert.equal((await exit(run)).code, 0, run.output);
  }
});
