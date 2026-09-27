// Checks the reference client against the committed export: generated types,
// real responses captured from the Rust whole-request tests, and runtime cases.
import { execFileSync } from "node:child_process";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import openapiTS, { astToString } from "openapi-typescript";

const web = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const root = resolve(web, "../../..");
const documentPath = resolve(web, "../openapi.json");
const generatedPath = resolve(web, "src/generated.ts");
const run = (command, args, options = {}) =>
  execFileSync(command, args, { cwd: root, stdio: "inherit", ...options });

// The Rust drift test guards openapi.json against the server; this guards the
// generated types against openapi.json.
const types = astToString(
  await openapiTS(JSON.parse(await readFile(documentPath, "utf8"))),
);
if (process.argv.includes("--generate")) await writeFile(generatedPath, types);
else if ((await readFile(generatedPath, "utf8")) !== types)
  throw new Error(
    `Reference client drift: ${generatedPath}; run npm --prefix apps/reference/web run generate`,
  );

const fixtures = await mkdtemp(join(tmpdir(), "iris-reference-fixtures-"));
try {
  run(
    "cargo",
    [
      "test",
      "--quiet",
      "--locked",
      "-p",
      "iris-reference",
      "--lib",
      "whole_request",
    ],
    { env: { ...process.env, IRIS_REFERENCE_FIXTURES: fixtures } },
  );
  run(resolve(web, "node_modules/.bin/tsc"), ["-p", web]);
  run(process.execPath, [
    resolve(web, "src/client.test.ts"),
    documentPath,
    fixtures,
  ]);
  run(process.execPath, [resolve(web, "src/present.test.ts")]);
  run(process.execPath, [resolve(web, "src/membership.test.ts")]);
  run(process.execPath, [resolve(web, "src/directory.test.ts")]);
  run(resolve(web, "node_modules/.bin/vite"), ["build"], { cwd: web });
  console.log(
    "PASS: reference client types, captured responses, runtime validation, presentation, request construction, directory transitions and build",
  );
} finally {
  await rm(fixtures, { recursive: true, force: true });
}
