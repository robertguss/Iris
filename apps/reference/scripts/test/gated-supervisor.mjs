// The development command's supervision over three gated stand-in servers, so
// the tests can hold each readiness pending. Everything but the servers and the
// readiness timeout is the command's own (`supervise.mjs`).
//
//   node gated-supervisor.mjs DIR READY_TIMEOUT_MS [STUBBORN,...]
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { Supervisor, runSupervised } from "../supervise.mjs";

const [dir, timeout, stubborn = ""] = process.argv.slice(2);
const server = join(
  dirname(fileURLToPath(import.meta.url)),
  "gated-server.mjs",
);
const supervisor = new Supervisor({ report: true });
supervisor.handleSignals();
process.exitCode = await supervisor.run(() =>
  runSupervised(
    supervisor,
    ["a", "b", "c"].map((name) => ({
      name,
      command: process.execPath,
      args: [
        server,
        name,
        dir,
        stubborn.split(",").includes(name) ? "stubborn" : "",
      ],
      readiness: new RegExp(`^${name} ready$`, "m"),
    })),
    { readyTimeout: Number(timeout), ready: "gated servers" },
  ),
);
