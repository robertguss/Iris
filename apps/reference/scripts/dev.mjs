// The reference console for development in one command: the local issuer on
// 4001, the development server on 3003 with a persistent database, and Vite on
// 5175 proxying /api to that server.
//
//   node apps/reference/scripts/dev.mjs [--database PATH] [--reset]
//
// The database defaults to the gitignored apps/reference/.dev/reference.db; a
// relative PATH is resolved against the current directory, and the directory
// that holds PATH must exist. It is an argument, never an environment
// variable, like the server's own. The command sets every address it owns, overriding inherited
// IRIS_API_TARGET, IRIS_LISTEN, IRIS_PUBLIC_ORIGIN and IRIS_OIDC_ISSUER, so the
// console cannot reach another API or database. With --reset it runs the
// server's own reset on the database and exits, starting nothing else.
//
// It refuses to start if any of its ports is taken, builds the server, starts
// the three in order, each only once the one before reported its address, and
// prints one `dev: ready` line when all three have. A child's exit stops the
// others and exits 1. SIGINT or SIGTERM stops everything: each process group
// gets SIGTERM, then SIGKILL after 5 s. That stop exits 0 once every child has
// exited, whatever their exit codes, which it prints: the server exits 1 when
// its drain deadline expires or its connections' closure is not established,
// and that is reported, not a failure of the command. A child that needed
// SIGKILL makes it exit 1. Later signals do not shorten the stop.
import { existsSync, mkdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import {
  Completed,
  Supervisor,
  describe,
  listening,
  prefixed,
  runSupervised,
} from "./supervise.mjs";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");
const web = join(root, "apps/reference/web");
const vite = join(web, "node_modules/.bin/vite");
const ORIGIN = "http://127.0.0.1:5175";
const ISSUER = "http://127.0.0.1:4001";
const API = "127.0.0.1:3003";
const PORTS = [4001, 3003, 5175];
const USAGE =
  "usage: node apps/reference/scripts/dev.mjs [--database PATH] [--reset]";

/** The arguments, or `null` when they are not understood. */
function parse(args) {
  let database;
  let reset = false;
  for (let i = 0; i < args.length; i++) {
    // An empty path is refused: it must never fall back to the default.
    if (
      args[i] === "--database" &&
      database === undefined &&
      i + 1 < args.length &&
      args[i + 1] !== ""
    )
      database = args[++i];
    else if (args[i] === "--reset" && !reset) reset = true;
    else return null;
  }
  return { database, reset };
}

const options = parse(process.argv.slice(2));
if (!options) {
  console.error(USAGE);
  process.exit(2);
}
const database =
  options.database !== undefined
    ? resolve(process.cwd(), options.database)
    : join(root, "apps/reference/.dev/reference.db");
/** Creates the default database's directory; an explicit path's must exist. */
function prepare() {
  if (options.database === undefined)
    mkdirSync(dirname(database), { recursive: true });
}

const supervisor = new Supervisor({ cwd: root, report: true });
// Before anything is spawned, so a signal at any point reaches cleanup.
supervisor.handleSignals();
supervisor.say(`data at ${database}`);

// Cargo builds from the checkout, so a relative target directory is under it.
const target = resolve(root, process.env.CARGO_TARGET_DIR ?? "target");
const binary = join(target, "debug/reference-dev");

/** Runs one owned command to its exit, with its output shown. */
async function step(name, command, args, options = {}) {
  if (supervisor.stop.signal.aborted) throw supervisor.stop.signal.reason;
  const { child, exited } = supervisor.own(
    command,
    args,
    { stdio: ["ignore", "pipe", "pipe"], ...options },
    name,
  );
  const output = prefixed(name);
  child.stdout?.on("data", (chunk) => output(chunk, "stdout"));
  child.stderr?.on("data", (chunk) => output(chunk, "stderr"));
  child.stdout?.on("end", () => output.end?.("stdout"));
  child.stderr?.on("end", () => output.end?.("stderr"));
  child.once("close", () => {
    output.end?.("stdout");
    output.end?.("stderr");
  });
  const result = await Promise.race([exited, supervisor.stopped]);
  if (result.code !== 0) throw new Error(`${name} ${describe(result)}`);
}

async function build() {
  supervisor.say("building reference-dev");
  await step("build", "cargo", [
    "build",
    "--quiet",
    "--locked",
    "-p",
    "iris-reference",
    "--bin",
    "reference-dev",
  ]);
}

async function reset() {
  prepare();
  await build();
  await step(
    "reset",
    binary,
    ["--local-oidc-demo", "--database", database, "--reset"],
    {
      env: { ...process.env, IRIS_OIDC_ISSUER: ISSUER },
    },
  );
  throw new Completed("reset completed");
}

async function serve() {
  const taken = [];
  for (const port of PORTS) if (await listening(port)) taken.push(port);
  if (taken.length)
    throw new Error(
      `ports already in use: ${taken.join(", ")}; this command must own ${PORTS.join(", ")}`,
    );
  if (!existsSync(vite))
    throw new Error(
      "Vite is not installed; run npm --prefix apps/reference/web ci",
    );
  prepare();
  await build();
  await runSupervised(
    supervisor,
    [
      {
        name: "issuer",
        command: process.execPath,
        args: [
          "experiments/api-slice/checks/oidc-provider.mjs",
          "--port",
          "4001",
          "--issuer",
          ISSUER,
          "--redirect-uri",
          `${ORIGIN}/api/auth/callback`,
        ],
        readiness: /listening on 127\.0\.0\.1:4001\b/,
      },
      {
        name: "api",
        command: binary,
        args: ["--local-oidc-demo", "--database", database],
        readiness: /listening on http:\/\/127\.0\.0\.1:3003\b/,
        options: {
          env: {
            ...process.env,
            IRIS_PUBLIC_ORIGIN: ORIGIN,
            IRIS_OIDC_ISSUER: ISSUER,
            IRIS_LISTEN: API,
          },
        },
      },
      {
        name: "web",
        command: vite,
        args: ["--host", "127.0.0.1", "--port", "5175", "--strictPort"],
        readiness: /Local:\s+http:\/\/127\.0\.0\.1:5175\//,
        options: {
          cwd: web,
          env: { ...process.env, IRIS_API_TARGET: `http://${API}` },
        },
      },
    ],
    {
      ready: `console ${ORIGIN}, API http://${API}, issuer ${ISSUER}, data ${database}`,
    },
  );
}

// A stop is the way to end serving, but a reset it interrupts did not happen.
process.exitCode = options.reset
  ? await supervisor.run(reset, { signalled: false })
  : await supervisor.run(serve);
