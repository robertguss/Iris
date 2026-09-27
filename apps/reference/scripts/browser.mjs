// The reference console's browser workflows against the local issuer. It builds
// and owns the issuer, the development binary and a preview of the production
// client, then drives checkpoint A's membership operations and checkpoint B's
// reads from the member directory through agent-browser.
//
//   node apps/reference/scripts/browser.mjs [--artifacts DIR]
//
// It refuses to start if any of its ports is taken, treats a server as ready
// only when its own child reports the bound address, and stops at once when a
// server exits unexpectedly or SIGINT/SIGTERM arrives. The one expected exit
// is the API's: between the two workflows it is retired and replaced, so
// checkpoint B starts from the seed data, and nothing is started once a stop
// has begun. Every child it spawns (builds,
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
const NOT_LOADED = (noun) =>
  `${noun} were not loaded. No automatic retry was sent; loading them again is a new read.`;

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
/** Servers whose exit is expected: only the API, once, when it is replaced. */
const retired = new Set();
/**
 * A server: output goes to `<artifacts>/<name>.log` and is scanned for the
 * child's own readiness line; its exit before cleanup stops the workflow,
 * unless it was retired. Returns the owned server and its readiness.
 */
function start(name, command, args, readiness, options = {}) {
  // Checked just before spawning: nothing starts once a stop has begun.
  if (stop.signal.aborted) throw stop.signal.reason;
  const log = createWriteStream(join(artifacts, `${name}.log`));
  logs.push(log);
  const server = own(command, args, {
    stdio: ["ignore", "pipe", "pipe"],
    ...options,
  });
  server.exited.then((result) => {
    if (!cleaning && !retired.has(server))
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
  return { server, ready: Promise.race([ready, stopped]) };
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

const target = process.env.CARGO_TARGET_DIR ?? join(root, "target");
const startApi = (name) =>
  start(
    name,
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
/** One bounded request through the preview proxy to the owned API. */
async function throughPreview() {
  const session = await fetch(`${ORIGIN}/api/auth/session`, {
    signal: AbortSignal.any([stop.signal, AbortSignal.timeout(10_000)]),
  });
  if (!session.ok)
    throw new Error(`Session through the preview: ${session.status}`);
}
/**
 * Retires the API and starts a fresh one on its port, so the next workflow
 * starts from the seed data. The port must be free again first.
 */
async function restartApi(api) {
  retired.add(api);
  await terminate(api);
  if (stop.signal.aborted) throw stop.signal.reason;
  if (await listening(3003))
    throw new Error(
      "Port 3003 was taken during the API restart; this workflow must own it",
    );
  await startApi("api-b").ready;
  await throughPreview();
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
/** The outcome panel's status line and its validated body's operation. */
async function outcome(status, operation) {
  try {
    await wait(status);
  } catch (error) {
    if (stop.signal.aborted) throw error;
    const shown = await browser(
      "eval",
      `document.querySelector("#outcome .status strong")?.textContent ?? "no outcome"`,
    );
    throw new Error(`Expected "${status}"; the page shows ${shown.trim()}`);
  }
  await check(
    `JSON.parse(document.querySelector("#outcome pre").textContent).operation === ${JSON.stringify(operation)}`,
    `${status} is a ${operation} response`,
  );
}
/** Waits until a panel's list has finished its read. */
const settled = (panel) =>
  browser(
    "wait",
    "--fn",
    `document.querySelector("${panel} .listing")?.getAttribute("aria-busy") === "false"`,
  );
async function openProject(name) {
  await settled("#projects");
  await button(`Open ${name}`);
  await settled("#members");
}
const select = (name) => button(`Select ${name}`);
async function remove(name) {
  await browser("select", "#operation", "removeMember");
  await select(name);
  await check(
    `document.querySelector("#change button[type=submit]").disabled`,
    "removal needs confirmation",
  );
  await browser("check", "#change input[type=checkbox]");
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
/** The outcome panel still shows this attempt, whatever the form now says. */
async function retained(path, target, detail) {
  await check(
    `document.querySelector("#outcome .endpoint code").textContent === ${JSON.stringify(path)} && document.querySelector("#outcome .target").textContent.includes(${JSON.stringify(target)}) && document.querySelector("#outcome .status p").textContent === ${JSON.stringify(detail)}`,
    `outcome keeps ${path} for ${target}`,
  );
}

async function checkpointA() {
  // Anonymous, then a failed and recovered session bootstrap.
  opened = true;
  await browser("open", ORIGIN);
  await browser("set", "viewport", "1280", "1000");
  await wait("Not signed in");
  await check(
    `document.querySelector("#change fieldset").disabled`,
    "anonymous form disabled",
  );
  await capture("anonymous");
  await browser("network", "route", "**/api/auth/session", "--abort");
  await browser("reload");
  await wait("Session unavailable");
  await check(
    `document.querySelector("#change fieldset").disabled && document.querySelector(".auth-panel button").disabled`,
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

  // An owner changes a listed member's role; the listing is marked when the
  // attempt is sent, before its response arrives.
  await login("Alice", "11");
  await openProject("Launch plan");
  await select("Bob Example");
  await check(
    `!document.querySelector("#change fieldset").disabled && !document.cookie.includes("iris-session")`,
    "signed in; session cookie is HttpOnly",
  );
  await browser(
    "eval",
    `(() => {
      const original = window.fetch;
      let release;
      window.__irisHeld = new Promise((done) => (release = done));
      window.__irisRelease = () => release();
      window.fetch = async (...args) => {
        const response = await original(...args);
        if (String(args[0]).startsWith("/api/memberships/")) {
          window.fetch = original;
          await window.__irisHeld;
        }
        return response;
      };
      return true;
    })()`,
  );
  await button("Change member role");
  await wait("Waiting for the API");
  await check(
    `document.querySelector("#members .stale") !== null`,
    "listing marked while the response is pending",
  );
  await browser("eval", "(window.__irisRelease(), true)");
  await outcome("200 · Role change acknowledged", "memberships.change_role");
  await capture("role-changed");

  // One lost response: unconfirmed, one attempt. Reading the members again,
  // or pointing the form elsewhere, leaves that outcome as it was.
  await browser("network", "requests", "--clear");
  await browser("network", "route", "**/api/memberships/role", "--abort");
  await button("Change member role");
  await wait("Unconfirmed · No usable response");
  await check(
    `document.querySelector("#outcome .status p").textContent === ${JSON.stringify(UNCONFIRMED)}`,
    "unconfirmed wording",
  );
  const attempts = await requests("/api/memberships/role");
  if (attempts !== 1) throw new Error(`Expected one attempt, saw ${attempts}`);
  await capture("unconfirmed");
  await browser("network", "unroute", "**/api/memberships/role");
  await browser("click", "#members .reload");
  await settled("#members");
  await retained("/api/memberships/role", "Bob Example", UNCONFIRMED);
  await browser("select", "#operation", "removeMember");
  await select("Alice Example");
  await retained("/api/memberships/role", "Bob Example", UNCONFIRMED);
  // Refreshing the same session, or failing to, is no session change.
  await button("Refresh session");
  await browser(
    "wait",
    "--fn",
    `!document.querySelector(".auth-panel button").disabled`,
  );
  await wait("Signed in as user 11");
  await retained("/api/memberships/role", "Bob Example", UNCONFIRMED);
  await browser("network", "route", "**/api/auth/session", "--abort");
  await button("Refresh session");
  await browser(
    "wait",
    "--fn",
    `document.querySelector("[role=alert]") !== null`,
  );
  await browser("network", "unroute", "**/api/auth/session");
  await retained("/api/memberships/role", "Bob Example", UNCONFIRMED);

  // A session change does clear it.
  await logout();
  await check(
    `document.querySelector("#outcome .status") === null && document.querySelector("#outcome .target") === null`,
    "signing out clears the attempt",
  );

  // A non-owner is refused.
  await login("Bob", "29");
  await openProject("Launch plan");
  await select("Alice Example");
  await button("Change member role");
  await outcome("403 · Owner permission required", "memberships.change_role");
  await capture("forbidden");

  // The owner removes a member. Confirmation belongs to one member; the stale
  // row then reaches absence, and the last owner is protected.
  await logout();
  await login("Alice", "11");
  await openProject("Launch plan");
  await browser("select", "#operation", "removeMember");
  await select("Bob Example");
  await browser("check", "#change input[type=checkbox]");
  await select("Alice Example");
  await check(
    `!document.querySelector("#change input[type=checkbox]").checked && document.querySelector("#change button[type=submit]").disabled`,
    "confirmation does not follow another member",
  );
  await remove("Bob Example");
  await outcome("200 · Removal acknowledged", "memberships.remove_member");
  await check(
    `document.querySelector("#members .stale") !== null`,
    "listing may predate the removal",
  );
  await remove("Bob Example");
  await outcome("404 · Member not found", "memberships.remove_member");
  await remove("Alice Example");
  await outcome("409 · Last owner must remain", "memberships.remove_member");
  await wait("Another owner is required first.");
  await capture("last-owner");
}

/** A panel's listed names, in order; none when it lists no rows. */
function rows(panel, names) {
  return check(
    `JSON.stringify([...document.querySelectorAll("${panel} .rows .row-name")].map((e) => e.textContent)) === ${JSON.stringify(JSON.stringify(names))}`,
    `${panel} lists ${names.join(", ") || "nothing"}`,
  );
}
function pageLine(panel, text) {
  return check(
    `document.querySelector("${panel} .read-status")?.textContent === ${JSON.stringify(text)}`,
    `${panel}: ${text}`,
  );
}
/** A panel's read failed with this title and wording; no rows, no next page. */
function refused(panel, title, detail) {
  return check(
    `document.querySelector("${panel} .status strong")?.textContent === ${JSON.stringify(title)} && document.querySelector("${panel} .status p").textContent === ${JSON.stringify(detail)} && !document.querySelector("${panel} .rows") && document.querySelector("${panel} .next").disabled`,
    `${panel}: ${title}`,
  );
}
async function click(selector, panel) {
  await browser("click", selector);
  await settled(panel);
}

async function checkpointB() {
  // The restarted API has a fresh database, so the old session is gone.
  await browser("reload");
  await wait("Not signed in");

  // The projects read that signing in starts is lost: shown as not loaded,
  // and never resent, until the user reloads.
  await browser("network", "requests", "--clear");
  await browser("network", "route", "**/api/projects*", "--abort");
  await login("Alice", "11");
  await settled("#projects");
  await refused("#projects", "No usable response", NOT_LOADED("Projects"));
  await browser("wait", "2000");
  const initial = await requests("/api/projects");
  if (initial !== 1)
    throw new Error(`Expected one projects read, saw ${initial}`);
  await capture("b-projects-not-loaded");
  await browser("network", "unroute", "**/api/projects*");
  await click("#projects .reload", "#projects");
  await rows("#projects", ["Launch plan"]);
  await pageLine(
    "#projects",
    "Page 1 · No further projects existed when this page was read.",
  );

  // One member per page: forward traversal lists each once, in key order.
  await browser("select", "#page-size", "1");
  await settled("#projects");
  await openProject("Launch plan");
  await rows("#members", ["Alice Example"]);
  await pageLine(
    "#members",
    "Page 1 · More members existed when this page was read.",
  );
  await click("#members .next", "#members");
  await rows("#members", ["Bob Example"]);
  await pageLine(
    "#members",
    "Page 2 · No further members existed when this page was read.",
  );
  await check(
    `document.querySelector("#members .next").disabled`,
    "no page after the last",
  );
  await capture("b-traversal");

  // A lost members read: not loaded, one request, no resend.
  await browser("network", "requests", "--clear");
  await browser("network", "route", "**/api/projects/41/members*", "--abort");
  await click("#members .reload", "#members");
  await refused("#members", "No usable response", NOT_LOADED("Members"));
  await browser("wait", "2000");
  const lost = await requests("/api/projects/41/members");
  if (lost !== 1) throw new Error(`Expected one members read, saw ${lost}`);
  await capture("b-members-not-loaded");
  await browser("network", "unroute", "**/api/projects/41/members*");
  await click("#members .reload", "#members");
  await rows("#members", ["Alice Example"]);

  // Alice makes Bob an owner, so that Bob can later leave the project.
  await click("#members .next", "#members");
  await select("Bob Example");
  await browser("select", "#role", "owner");
  await button("Change member role");
  await outcome("200 · Role change acknowledged", "memberships.change_role");

  // Bob's own projects, one per page.
  await logout();
  await login("Bob", "29");
  await settled("#projects");
  await browser("select", "#page-size", "1");
  await settled("#projects");
  await rows("#projects", ["Launch plan"]);
  await click("#projects .next", "#projects");
  await rows("#projects", ["Field notes"]);
  await pageLine(
    "#projects",
    "Page 2 · No further projects existed when this page was read.",
  );
  await capture("b-own-projects");

  // Removed between pages: with page 1 shown, Bob removes their own membership. Reading
  // the next page is a new read, authorized again, and refused; the removal
  // keeps its own outcome.
  await click("#projects .reload", "#projects");
  await openProject("Launch plan");
  await click("#members .next", "#members");
  await select("Bob Example");
  await click("#members .reload", "#members");
  await rows("#members", ["Alice Example"]);
  await browser("select", "#operation", "removeMember");
  await check(
    `document.querySelector("#change .target").textContent.startsWith("Bob Example (user 29)")`,
    "Bob still selected on page 1",
  );
  await browser("check", "#change input[type=checkbox]");
  await button("Remove member");
  await outcome("200 · Removal acknowledged", "memberships.remove_member");
  await check(
    `document.querySelector("#members .stale") !== null`,
    "page 1 may predate the removal",
  );
  await click("#members .next", "#members");
  await refused(
    "#members",
    "Project not available",
    "This operation is not permitted.",
  );
  await check(
    `document.querySelector("#outcome .status strong").textContent === "200 · Removal acknowledged" && document.querySelector("#outcome .target").textContent.includes("Bob Example")`,
    "the removal keeps its own outcome",
  );
  await capture("b-removed-between-pages");
  await click("#projects .reload", "#projects");
  await rows("#projects", ["Field notes"]);
}

/** Checks on the last page load: validation cost, layout and storage. */
async function finish() {
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
    `document.querySelector("#change fieldset").disabled && localStorage.length === 0 && sessionStorage.length === 0`,
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
  ).ready;
  const api = startApi("api");
  await api.ready;
  await start(
    "web",
    vite,
    ["preview", "--host", "127.0.0.1", "--port", "5175", "--strictPort"],
    /Local:\s+http:\/\/127\.0\.0\.1:5175\//,
    { cwd: web },
  ).ready;
  await throughPreview();
  await Promise.race([checkpointA(), stopped]);
  await Promise.race([restartApi(api.server), stopped]);
  await Promise.race([checkpointB(), stopped]);
  const compile = await Promise.race([finish(), stopped]);
  console.log(
    `PASS: OIDC sign-in and bootstrap recovery. Checkpoint A from the member directory: role change with the listing marked at send time, one unconfirmed attempt kept across a reload, a new selection and refreshes, non-owner refusal; removal with per-member confirmation, absence from a stale row and last-owner protection. Checkpoint B on fresh data: a lost initial projects read and a lost members read, each shown once and never resent; member and own-project traversal one row per page to the end; a member who left the project refused their next page. HttpOnly session; narrow layout; nothing stored. Validator compile ${compile.toFixed(1)} ms (production build). Artifacts: ${artifacts}`,
  );
} finally {
  await cleanup();
}
