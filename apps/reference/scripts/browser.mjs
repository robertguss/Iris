// Checkpoint A's browser workflow against the local issuer. It builds and owns
// the issuer, the development binary and a preview of the production client,
// then drives both membership operations through agent-browser.
//
//   node apps/reference/scripts/browser.mjs [--artifacts DIR]
//
// It refuses to start if any of its ports is taken, treats a server as ready
// only when its own child reports the bound address, and stops at once when a
// server exits or SIGINT/SIGTERM arrives. Every child it spawns (builds,
// servers, browser commands) runs in its own process group; cleanup sends
// SIGTERM, escalates to SIGKILL after 5 s and awaits each exit. The browser
// session is unique to this invocation and closed only if this run opened it.
import { spawn } from "node:child_process";
import { randomUUID } from "node:crypto";
import { createWriteStream, mkdirSync, mkdtempSync } from "node:fs";
import { connect } from "node:net";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");
const web = join(root, "apps/reference/web");
const vite = join(web, "node_modules/.bin/vite");
const ORIGIN = "http://127.0.0.1:5175";
const ISSUER = "http://127.0.0.1:4001";
const API = "127.0.0.1:3003";
const PORTS = [4001, 3003, 5175];
const SESSION = `iris-reference-${randomUUID().slice(0, 8)}`;
// Written independently of present.ts: the page must say exactly this.
const UNCONFIRMED =
  "Outcome unconfirmed: the change may or may not have been applied. No automatic retry was sent; a new submission needs current authority and intent.";

const flag = process.argv.indexOf("--artifacts");
const artifacts =
  flag > 0
    ? resolve(process.argv[flag + 1])
    : mkdtempSync(join(tmpdir(), "iris-reference-browser-"));
mkdirSync(artifacts, { recursive: true });

// One stop signal for everything: a signal, or a server exiting early.
const stop = new AbortController();
for (const signal of ["SIGINT", "SIGTERM"])
  process.once(signal, () => stop.abort(new Error(`Interrupted by ${signal}`)));
const stopped = new Promise((_, reject) =>
  stop.signal.addEventListener("abort", () => reject(stop.signal.reason), {
    once: true,
  }),
);
stopped.catch(() => {});

const owned = new Set();
/** Spawns a child in its own process group and owns it until it exits. */
function own(command, args, options = {}) {
  const child = spawn(command, args, { cwd: root, detached: true, ...options });
  let running = true;
  const exited = new Promise((done) => {
    child.once("error", (error) => {
      running = false;
      done({ error });
    });
    child.once("exit", (code, signal) => {
      running = false;
      done({ code, signal });
    });
  });
  const entry = { child, exited, running: () => running };
  owned.add(entry);
  exited.then(() => owned.delete(entry));
  return entry;
}
function signalGroup({ child, running }, signal) {
  if (!running()) return;
  try {
    process.kill(-child.pid, signal);
  } catch {
    child.kill(signal);
  }
}
/** SIGTERM, then SIGKILL after 5 s; resolves once the child has exited. */
async function terminate(entry) {
  signalGroup(entry, "SIGTERM");
  const force = setTimeout(() => signalGroup(entry, "SIGKILL"), 5_000);
  await entry.exited;
  clearTimeout(force);
}
const describe = ({ error, code, signal }) =>
  error
    ? `failed to start (${error.code ?? error.message})`
    : `exited (${signal ?? code})`;

/** One command with captured output; a stop or timeout abandons it to cleanup. */
async function run(command, args, { timeout = 40_000 } = {}) {
  if (stop.signal.aborted) throw stop.signal.reason;
  const child = own(command, args, { stdio: ["ignore", "pipe", "pipe"] });
  let output = "";
  child.child.stdout?.on("data", (chunk) => (output += chunk));
  child.child.stderr?.on("data", (chunk) => (output += chunk));
  let late = false;
  const timer = setTimeout(() => {
    late = true;
    void terminate(child);
  }, timeout);
  try {
    const result = await Promise.race([child.exited, stopped]);
    if (late) throw new Error(`${command} ${args.join(" ")} timed out`);
    if (result.code !== 0)
      throw new Error(
        `${command} ${args.join(" ")} ${describe(result)}:\n${output}`,
      );
    return output;
  } finally {
    clearTimeout(timer);
  }
}
/** A build step with its output shown. */
async function build(command, args, options = {}) {
  if (stop.signal.aborted) throw stop.signal.reason;
  const { exited } = own(command, args, { stdio: "inherit", ...options });
  const result = await Promise.race([exited, stopped]);
  if (result.code !== 0)
    throw new Error(`${command} ${args.join(" ")} ${describe(result)}`);
}

const logs = [];
let cleaning = false;
/**
 * A server: output goes to `<artifacts>/<name>.log` and is scanned for the
 * child's own readiness line; its exit before cleanup stops the workflow.
 */
function start(name, command, args, readiness, options = {}) {
  const log = createWriteStream(join(artifacts, `${name}.log`));
  logs.push(log);
  const server = own(command, args, {
    stdio: ["ignore", "pipe", "pipe"],
    ...options,
  });
  server.exited.then((result) => {
    if (!cleaning)
      stop.abort(new Error(`${name} ${describe(result)}; see ${name}.log`));
  });
  let seen = "";
  const ready = new Promise((done, fail) => {
    const timer = setTimeout(
      () => fail(new Error(`${name} not ready within 60 s; see ${name}.log`)),
      60_000,
    );
    const scan = (chunk) => {
      log.write(chunk);
      if (seen === null) return;
      // Colour codes, if any, never split the readiness line.
      seen += String(chunk).replace(/\x1b\[[0-9;]*m/g, "");
      if (readiness.test(seen)) {
        seen = null;
        clearTimeout(timer);
        done();
      }
    };
    server.child.stdout?.on("data", scan);
    server.child.stderr?.on("data", scan);
    server.exited.then(() => clearTimeout(timer));
  });
  return Promise.race([ready, stopped]);
}
let opened = false;
async function cleanup() {
  cleaning = true;
  await Promise.all([...owned].map(terminate));
  if (opened) {
    const close = own("agent-browser", ["--session", SESSION, "close"], {
      stdio: "ignore",
    });
    const late = setTimeout(() => void terminate(close), 20_000);
    await close.exited;
    clearTimeout(late);
  }
  await Promise.all(logs.map((log) => new Promise((done) => log.end(done))));
}
function listening(port) {
  return new Promise((done) => {
    const socket = connect({ host: "127.0.0.1", port });
    socket.once("connect", () => {
      socket.destroy();
      done(true);
    });
    socket.once("error", () => done(false));
    socket.setTimeout(2_000, () => {
      socket.destroy();
      done(true);
    });
  });
}

const browser = (...args) =>
  run("agent-browser", ["--session", SESSION, ...args]);
const wait = (text) => browser("wait", "--text", text);
const button = (name) =>
  browser("find", "role", "button", "click", "--name", name, "--exact");
function check(expression, label) {
  return browser(
    "eval",
    `(() => { if (!(${expression})) throw new Error(${JSON.stringify(`Browser assertion failed: ${label}`)}); return true; })()`,
  );
}
async function capture(name) {
  await browser(
    "eval",
    "new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r)))",
  );
  await browser(
    "screenshot",
    join(artifacts, `reference-${name}.png`),
    "--full",
  );
}
async function login(identity, user) {
  await button("Sign in with test provider");
  await wait("Choose a fixed test identity");
  await button(`Continue as ${identity}`);
  await wait(`Signed in as user ${user}`);
}
async function logout() {
  await button("Sign out");
  await wait("Not signed in");
}
/** The status line and the validated body's operation. */
async function outcome(status, operation) {
  try {
    await wait(status);
  } catch (error) {
    if (stop.signal.aborted) throw error;
    const shown = await browser(
      "eval",
      `document.querySelector(".status strong")?.textContent ?? "no outcome"`,
    );
    throw new Error(`Expected "${status}"; the page shows ${shown.trim()}`);
  }
  await check(
    `JSON.parse(document.querySelector("pre").textContent).operation === ${JSON.stringify(operation)}`,
    `${status} is a ${operation} response`,
  );
}
async function remove(user) {
  await browser("select", "#operation", "removeMember");
  await browser("fill", "#member", user);
  await check(
    `document.querySelector("button[type=submit]").disabled`,
    "removal needs confirmation",
  );
  await browser("check", "input[type=checkbox]");
  await button("Remove member");
}
async function requests(path) {
  const log = JSON.parse(
    await browser("network", "requests", "--filter", path, "--json"),
  );
  if (!Array.isArray(log.data?.requests))
    throw new Error("Unexpected request log shape");
  return log.data.requests.length;
}

async function workflow() {
  // Anonymous, then a failed and recovered session bootstrap.
  opened = true;
  await browser("open", ORIGIN);
  await browser("set", "viewport", "1280", "1000");
  await wait("Not signed in");
  await check(
    `document.querySelector("fieldset").disabled`,
    "anonymous form disabled",
  );
  await capture("anonymous");
  await browser("network", "route", "**/api/auth/session", "--abort");
  await browser("reload");
  await wait("Session unavailable");
  await check(
    `document.querySelector("fieldset").disabled && document.querySelector(".auth-panel button").disabled`,
    "controls disabled without a session",
  );
  await capture("session-error");
  await browser("network", "unroute", "**/api/auth/session");
  await button("Refresh session");
  await browser(
    "wait",
    "--fn",
    `!document.querySelector(".auth-panel button").disabled`,
  );

  // An owner changes a member's role, then loses one response.
  await login("Alice", "11");
  await check(
    `!document.querySelector("fieldset").disabled && !document.cookie.includes("iris-session")`,
    "signed in; session cookie is HttpOnly",
  );
  await button("Change member role");
  await outcome("200 · Role change acknowledged", "memberships.change_role");
  await capture("role-changed");
  await browser("network", "requests", "--clear");
  await browser("network", "route", "**/api/memberships/role", "--abort");
  await button("Change member role");
  await wait("Unconfirmed · No usable response");
  await check(
    `document.querySelector(".status p").textContent === ${JSON.stringify(UNCONFIRMED)}`,
    "unconfirmed wording",
  );
  const attempts = await requests("/api/memberships/role");
  if (attempts !== 1) throw new Error(`Expected one attempt, saw ${attempts}`);
  await capture("unconfirmed");
  await browser("network", "unroute", "**/api/memberships/role");

  // A non-owner is refused; the owner removes, then meets absence and the
  // last-owner rule.
  await logout();
  await login("Bob", "29");
  await button("Change member role");
  await outcome("403 · Owner permission required", "memberships.change_role");
  await capture("forbidden");
  await logout();
  await login("Alice", "11");
  await remove("29");
  await outcome("200 · Removal acknowledged", "memberships.remove_member");
  await remove("29");
  await outcome("404 · Member not found", "memberships.remove_member");
  await remove("11");
  await outcome("409 · Last owner must remain", "memberships.remove_member");
  await wait("Another owner is required first.");
  await capture("last-owner");

  // eval prints its result as JSON.
  const compile = JSON.parse(
    await browser(
      "eval",
      `performance.getEntriesByName("iris:client-compile").map(e => e.duration)`,
    ),
  );
  if (compile.length !== 1)
    throw new Error(`Expected one compile measure, saw ${compile.length}`);

  await browser("set", "viewport", "390", "844");
  await check(
    `document.documentElement.scrollWidth <= innerWidth`,
    "no horizontal scroll at 390 px",
  );
  await capture("narrow");
  await logout();
  await browser("reload");
  await wait("Not signed in");
  await check(
    `document.querySelector("fieldset").disabled && localStorage.length === 0 && sessionStorage.length === 0`,
    "signed out; nothing stored",
  );
  return compile[0];
}

try {
  const taken = [];
  for (const port of PORTS) if (await listening(port)) taken.push(port);
  if (taken.length)
    throw new Error(
      `Ports already in use: ${taken.join(", ")}; this workflow must own them`,
    );
  await build("cargo", [
    "build",
    "--quiet",
    "--locked",
    "-p",
    "iris-reference",
    "--bin",
    "reference-dev",
  ]);
  await build(vite, ["build", "--logLevel", "warn"], { cwd: web });
  await start(
    "issuer",
    "node",
    [
      "experiments/api-slice/checks/oidc-provider.mjs",
      "--port",
      "4001",
      "--issuer",
      ISSUER,
      "--redirect-uri",
      `${ORIGIN}/api/auth/callback`,
    ],
    /listening on 127\.0\.0\.1:4001\b/,
  );
  const target = process.env.CARGO_TARGET_DIR ?? join(root, "target");
  await start(
    "api",
    join(target, "debug/reference-dev"),
    ["--local-oidc-demo"],
    /listening on http:\/\/127\.0\.0\.1:3003\b/,
    {
      env: {
        ...process.env,
        IRIS_PUBLIC_ORIGIN: ORIGIN,
        IRIS_OIDC_ISSUER: ISSUER,
        IRIS_LISTEN: API,
      },
    },
  );
  await start(
    "web",
    vite,
    ["preview", "--host", "127.0.0.1", "--port", "5175", "--strictPort"],
    /Local:\s+http:\/\/127\.0\.0\.1:5175\//,
    { cwd: web },
  );
  // One bounded request through the preview proxy to the owned API.
  const session = await fetch(`${ORIGIN}/api/auth/session`, {
    signal: AbortSignal.any([stop.signal, AbortSignal.timeout(10_000)]),
  });
  if (!session.ok)
    throw new Error(`Session through the preview: ${session.status}`);
  const compile = await Promise.race([workflow(), stopped]);
  console.log(
    `PASS: OIDC sign-in and bootstrap recovery; role change, one unconfirmed attempt, non-owner refusal; removal, absence and last-owner protection; HttpOnly session; narrow layout; nothing stored. Validator compile ${compile.toFixed(1)} ms (production build). Artifacts: ${artifacts}`,
  );
} finally {
  await cleanup();
}
