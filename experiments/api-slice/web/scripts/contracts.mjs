import { execFileSync } from "node:child_process";
import { mkdtemp, readFile, writeFile, mkdir, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import openapiTS, { astToString } from "openapi-typescript";

const web = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const root = resolve(web, "../../..");
const check = process.argv.includes("--check");
const temp = await mkdtemp(join(tmpdir(), "iris-contracts-"));
try {
  execFileSync("cargo", ["run", "--quiet", "--locked", "-p", "iris-api-spike", "--bin", "export-contracts", "--", temp], { cwd: root, stdio: "inherit" });
  for (const candidate of ["utoipa", "aide"]) {
    const spec = await readFile(join(temp, `${candidate}.json`), "utf8");
    const types = astToString(await openapiTS(JSON.parse(spec)));
    for (const [path, contents] of [
      [resolve(web, `../contracts/${candidate}.json`), spec],
      [join(web, `src/generated/${candidate}.ts`), types],
    ]) {
      if (check) {
        if (await readFile(path, "utf8") !== contents) throw new Error(`Contract drift: ${path}. Run npm run generate.`);
      } else {
        await mkdir(dirname(path), { recursive: true });
        await writeFile(path, contents);
      }
    }
  }
  console.log(check ? "PASS: both OpenAPI snapshots and generated clients are current" : "Generated both OpenAPI snapshots and TypeScript clients");
} finally {
  await rm(temp, { recursive: true, force: true });
}
