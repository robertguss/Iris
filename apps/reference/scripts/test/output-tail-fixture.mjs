// Narrow fixture for ROB-1119 output-tail supervision tests.
//
//   node output-tail-fixture.mjs ROLE MODE DIR
//
// Roles are intentionally small: `start` runs a Supervisor around this fixture as
// a child, while `child` provides process behaviours that are awkward to express
// portably from a shell shim.
import { spawn } from "node:child_process";
import { existsSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { Supervisor } from "../supervise.mjs";

const [role, mode, dir] = process.argv.slice(2);
const self = fileURLToPath(import.meta.url);
const sleep = (ms) => new Promise((done) => setTimeout(done, ms));
const waitFor = async (name) => {
  while (!existsSync(join(dir, `${name}.go`))) await sleep(20);
};

if (role === "start") {
  const supervisor = new Supervisor({ report: true });
  if (mode === "escaped-pipe") {
    const cleanup = supervisor.cleanup.bind(supervisor);
    supervisor.cleanup = async () => {
      writeFileSync(join(dir, "cleanup-entry"), "");
      return cleanup();
    };
  }
  supervisor.handleSignals();
  process.exitCode = await supervisor.run(async () => {
    const { prefixed } = await import("../supervise.mjs");
    const formatted = prefixed("tail");
    let prefixAcked = false;
    const output = (chunk, stream) => {
      formatted(chunk, stream);
      if (
        mode.startsWith("leader-") &&
        stream === "stdout" &&
        !prefixAcked &&
        String(chunk).includes("prefix-")
      ) {
        prefixAcked = true;
        writeFileSync(join(dir, "prefix-seen"), "");
      }
    };
    output.end = formatted.end;
    const { ready, server } = supervisor.start(
      "tail",
      process.execPath,
      [self, "child", mode, dir],
      /ready/m,
      { output },
    );
    await ready;
    supervisor.say("ready: tail fixture");
    if (mode === "escaped-pipe") {
      await server.gone;
      writeFileSync(join(dir, "gone-seen"), "");
    }
    await supervisor.stopped;
  });
} else if (role === "child" && mode === "stdout-end-alive") {
  process.stdout.write("tail-without-newline");
  process.stdout.end();
  process.stderr.write("ready\n");
  process.on("SIGTERM", () => process.exit(0));
  setInterval(() => {}, 60_000);
} else if (role === "child" && mode === "plain-events") {
  process.stdout.write("one");
  process.stdout.write("two\nthree");
  process.stderr.write("err");
  process.on("SIGTERM", () => process.exit(0));
  setInterval(() => {}, 60_000);
} else if (role === "child" && mode === "escaped-pipe") {
  const helper = `
    const { existsSync } = await import("node:fs");
    const { join } = await import("node:path");
    const sleep = (ms) => new Promise((done) => setTimeout(done, ms));
    setTimeout(() => process.exit(33), 15000);
    while (!existsSync(join(${JSON.stringify(dir)}, "release.go"))) await sleep(20);
    process.stdout.write("suffix");
    process.exit(0);
  `;
  const escaped = spawn(
    process.execPath,
    ["--input-type=module", "-e", helper],
    {
      detached: true,
      stdio: ["ignore", process.stdout, process.stderr],
    },
  );
  writeFileSync(join(dir, "escaped-pid"), String(escaped.pid));
  escaped.unref();
  process.stdout.write("prefix-");
  process.stderr.write("ready\n");
  process.exit(0);
} else if (role === "child" && mode.startsWith("leader-")) {
  const stubborn = mode.endsWith("stubborn");
  const helper = `
    const { existsSync, writeFileSync } = await import("node:fs");
    const { join } = await import("node:path");
    const sleep = (ms) => new Promise((done) => setTimeout(done, ms));
    writeFileSync(join(${JSON.stringify(dir)}, "helper-started"), "");
    process.on('SIGTERM', () => {});
    while (!existsSync(join(${JSON.stringify(dir)}, "suffix.go"))) await sleep(20);
    process.stdout.write("suffix");
    ${stubborn ? "setInterval(() => {}, 60000);" : "process.exit(0);"}
  `;
  spawn(process.execPath, ["--input-type=module", "-e", helper], {
    stdio: ["ignore", "inherit", "inherit"],
  });
  process.stdout.write("prefix-");
  process.stderr.write("ready\n");
  writeFileSync(join(dir, "leader-ready"), "");
  await waitFor("helper-started");
  setInterval(() => {}, 60_000);
} else {
  console.error(`unknown output-tail fixture role/mode: ${role} ${mode}`);
  process.exit(2);
}
