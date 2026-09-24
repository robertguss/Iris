import { spawnSync } from "node:child_process";
import { cp, mkdtemp, readFile, writeFile, rm } from "node:fs/promises";
import { join, resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import openapiTS, { astToString } from "openapi-typescript";

const web = resolve(dirname(fileURLToPath(import.meta.url)), "..");
// Stay under web so the copied source resolves this installation's node_modules.
const probe = await mkdtemp(join(web, ".contract-probe-"));
try {
  await cp(join(web, "src"), join(probe, "src"), { recursive: true });
  await cp(join(web, "tsconfig.json"), join(probe, "tsconfig.json"));
  const compile = () => spawnSync(join(web, "node_modules/.bin/tsc"), ["--noEmit", "--project", join(probe, "tsconfig.json")], { cwd: web, encoding: "utf8" });
  const baseline = compile();
  if (baseline.status !== 0) throw new Error(`Probe baseline failed: ${baseline.stdout}${baseline.stderr}`);
  const spec = JSON.parse(await readFile(resolve(web, "../contracts/utoipa.json"), "utf8"));
  const success = spec.components.schemas.Acceptance;
  success.properties.member_id = success.properties.user_id;
  delete success.properties.user_id;
  success.required = success.required.map(name => name === "user_id" ? "member_id" : name);
  await writeFile(join(probe, "src/generated/utoipa.ts"), astToString(await openapiTS(spec)));
  const changed = compile();
  if (changed.status === 0 || !changed.stdout.includes("user_id")) throw new Error(`Expected a user_id diagnostic, got: ${changed.stdout}${changed.stderr}`);
  console.log("PASS: renaming a required contract field breaks the actual React consumer");
  console.log(changed.stdout.trim());
} finally {
  await rm(probe, { recursive: true, force: true });
}
