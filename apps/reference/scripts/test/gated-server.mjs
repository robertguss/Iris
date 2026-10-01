// A stand-in server for the supervision tests: it reports readiness only once
// the test creates its gate file, then idles until signalled. It also starts a
// helper in its process group that outlives it unless the whole group is
// signalled, so a stop that signals only this process leaves its group behind.
// With `stubborn`, that helper also ignores SIGTERM.
//
//   node gated-server.mjs NAME DIR [stubborn]
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { join } from "node:path";

const [name, dir, stubborn] = process.argv.slice(2);
const helper =
  stubborn === "stubborn"
    ? 'process.on("SIGTERM", () => {}); setInterval(() => {}, 60_000)'
    : "setInterval(() => {}, 60_000)";
spawn(process.execPath, ["-e", helper], { stdio: "ignore" });
console.log(`${name} spawned`);
const gate = setInterval(() => {
  if (!existsSync(join(dir, `${name}.go`))) return;
  clearInterval(gate);
  console.log(`${name} ready`);
  setInterval(() => {}, 60_000);
}, 20);
