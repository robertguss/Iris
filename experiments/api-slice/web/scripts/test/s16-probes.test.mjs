// Regression for probe:s16 target isolation. The runner must give the nested
// verifier a target inside its disposable copy, and remove that copy when a
// probe throws.
//
// The fixture is an independent initialized Git repository. It copies the
// actual runner, verifier and four baseline files; only node_modules is linked.
// The cargo shim and the runner's own temp live under a fixture-owned parent
// outside that inventory. Real Node runs the copied runner. One fake cargo logs
// cwd and the target it received, writes a marker only after that target is
// inside the fixture, then exits with a distinct diagnostic. It does not
// emulate mutant compiler, test or client output. The copied verifier is the
// real s16.mjs.
//
//   node --test experiments/api-slice/web/scripts/test/s16-probes.test.mjs
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import {
  chmodSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  realpathSync,
  readdirSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";

const here = dirname(fileURLToPath(import.meta.url));
const web = resolve(here, "../..");
const root = resolve(web, "../../..");
const runner = "experiments/api-slice/web/scripts/s16-probes.mjs";
const verifier = "experiments/api-slice/web/scripts/s16.mjs";
const baseline = [
  "experiments/api-slice/server/src/s16/action.rs",
  "experiments/api-slice/server/src/s16/mod.rs",
  "experiments/api-slice/server/src/s16/tests.rs",
  "experiments/api-slice/web/s16/generated.ts",
];

// Cargo only. Node stays the real interpreter, so the nested verifier is the
// copied s16.mjs.
const cargoShim = [
  "#!/bin/sh",
  'printf \'%s\\n\' "$PWD" "$#" "$@" "$CARGO_TARGET_DIR" >> "$IRIS_S16_CARGO_LOG"',
  'case "$CARGO_TARGET_DIR" in',
  '  "$IRIS_S16_FIXTURE"/*) ;;',
  '  *) echo "iris-s16-fake-cargo: target outside fixture: $CARGO_TARGET_DIR" >&2; exit 19 ;;',
  "esac",
  'mkdir -p "$CARGO_TARGET_DIR"',
  "printf 'mutated\\n' > \"$CARGO_TARGET_DIR/iris-s16-marker\"",
  "echo 'iris-s16-fake-cargo: refused' >&2",
  "exit 17",
  "",
].join("\n");

function copy(from, to) {
  mkdirSync(dirname(to), { recursive: true });
  writeFileSync(to, readFileSync(from));
}

function contained(parent, child) {
  const from = resolve(parent) + sep;
  const to = resolve(child);
  return to.startsWith(from);
}

function calls(log) {
  const lines = readFileSync(log, "utf8").split("\n");
  const parsed = [];
  for (let index = 0; index < lines.length;) {
    if (lines[index] === "") break;
    const cwd = lines[index];
    const count = Number(lines[index + 1]);
    const args = lines.slice(index + 2, index + 2 + count);
    const target = lines[index + 2 + count];
    parsed.push({ cwd, args, target });
    index += count + 3;
  }
  return parsed;
}

test("nested verifier inherits the disposable target, which the runner removes when a probe throws", () => {
  const outer = mkdtempSync(join(tmpdir(), "iris-s16-probe-test-"));
  try {
    const inventory = join(outer, "repo");
    const scratch = join(outer, "scratch");
    const bin = join(scratch, "bin");
    const log = join(scratch, "cargo.log");
    const inherited = join(scratch, "inherited-target");
    const checkoutTarget = join(root, "target");
    mkdirSync(inventory, { recursive: true });
    mkdirSync(bin, { recursive: true });
    mkdirSync(inherited, { recursive: true });
    writeFileSync(join(inherited, "sentinel"), "untouched\n");
    writeFileSync(log, "");
    for (const file of [runner, verifier, ...baseline])
      copy(join(root, file), join(inventory, file));
    symlinkSync(
      join(web, "node_modules"),
      join(inventory, "experiments/api-slice/web/node_modules"),
      "dir",
    );
    writeFileSync(join(bin, "cargo"), cargoShim);
    chmodSync(join(bin, "cargo"), 0o755);
    writeFileSync(join(inventory, ".gitignore"), "node_modules\n");
    spawnSync("git", ["init", "-q"], { cwd: inventory });
    spawnSync("git", ["add", "--", runner, verifier, ...baseline], {
      cwd: inventory,
    });

    const script = join(inventory, runner);
    assert.equal(contained(inventory, realpathSync(script)), true, script);
    const checkoutBefore = existsSync(checkoutTarget)
      ? readdirSync(checkoutTarget).sort()
      : null;
    const sources = Object.fromEntries(
      baseline.map((file) => [file, readFileSync(join(root, file))]),
    );

    const result = spawnSync(process.execPath, [script], {
      cwd: inventory,
      encoding: "utf8",
      env: {
        ...process.env,
        PATH: `${bin}:${process.env.PATH}`,
        TMPDIR: scratch,
        CARGO_TARGET_DIR: inherited,
        IRIS_S16_CARGO_LOG: log,
        IRIS_S16_FIXTURE: outer,
      },
    });
    const output = result.stdout + result.stderr;
    assert.equal(result.status, 1, output);
    assert.match(output, /iris-s16-fake-cargo: refused/);
    assert.match(output, /s16\.mjs/);
    assert.equal(
      existsSync(join(inherited, "iris-s16-marker")),
      false,
      "inherited sentinel target was written",
    );
    assert.equal(
      readFileSync(join(inherited, "sentinel"), "utf8"),
      "untouched\n",
    );
    if (checkoutBefore === null)
      assert.equal(existsSync(checkoutTarget), false);
    else assert.deepEqual(readdirSync(checkoutTarget).sort(), checkoutBefore);
    for (const file of baseline)
      assert.ok(readFileSync(join(root, file)).equals(sources[file]), file);

    const seen = calls(log);
    const nested = seen.filter((call) => call.args.includes("export-s16"));
    assert.equal(
      nested.length,
      1,
      `nested verifier did not invoke cargo: ${JSON.stringify(seen)}`,
    );
    const runnerTemp = nested[0].cwd;
    assert.equal(contained(scratch, runnerTemp), true, runnerTemp);
    assert.equal(nested[0].target, join(runnerTemp, "target"));
    assert.equal(
      existsSync(runnerTemp),
      false,
      "runner temp survived the thrown probe",
    );
    assert.equal(
      existsSync(nested[0].target),
      false,
      "runner target survived the thrown probe",
    );
    assert.notEqual(resolve(nested[0].target), resolve(inherited));
    assert.notEqual(resolve(nested[0].target), resolve(checkoutTarget));
    assert.equal(
      existsSync(log),
      true,
      "observation log must outlive the runner temp",
    );
  } finally {
    rmSync(outer, { recursive: true, force: true });
  }
});
