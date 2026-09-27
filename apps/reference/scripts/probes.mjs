// Checkpoint A omitted-edit probes. Each probe edits a disposable source copy,
// never the checkout, and builds into that copy's own target directory so no
// mutated artifact can outlive the run.
import { execFileSync, spawnSync } from "node:child_process";
import { copyFile, mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");
const temp = await mkdtemp(join(tmpdir(), "iris-reference-probes-"));
const adapter = "apps/reference/src/http/memberships.rs";
const cargo = ["--quiet", "--locked", "-p", "iris-reference"];
let baseline;

function run(command, args) {
  return spawnSync(command, args, {
    cwd: temp,
    encoding: "utf8",
    env: { ...process.env, CARGO_TARGET_DIR: join(temp, "target") },
    maxBuffer: 16 * 1024 * 1024,
  });
}

// A spawn error or a signal is a broken probe, never an expected failure.
function expect(label, command, args, pattern, pass = false) {
  const result = run(command, args);
  if (result.error || result.signal !== null) {
    throw new Error(`${label}: did not run to completion (${result.error ?? result.signal})`);
  }
  const output = result.stdout + result.stderr;
  if ((result.status === 0) !== pass || !pattern.test(output)) {
    throw new Error(`${label}: unexpected result ${result.status}\n${output}`);
  }
  const evidence = output.split("\n").find((line) => pattern.test(line)) ?? pattern.source;
  console.log(`${pass ? "CONTROL" : "CAUGHT"}: ${label}: ${evidence.trim()}`);
}

async function edit(from, to) {
  const path = join(temp, adapter);
  const text = await readFile(path, "utf8");
  if (!text.includes(from) || text.indexOf(from) !== text.lastIndexOf(from)) {
    throw new Error(`Probe anchor drift: ${adapter}: ${from}`);
  }
  await writeFile(path, text.replace(from, to));
}

async function reset() {
  await writeFile(join(temp, adapter), baseline);
}

const contract = ["test", ...cargo, "--test", "contract"];

try {
  const files = execFileSync("git", ["ls-files", "--cached", "--others", "--exclude-standard", "-z"], {
    cwd: root,
    encoding: "utf8",
  })
    .split("\0")
    .filter(Boolean);
  for (const file of files) {
    await mkdir(dirname(join(temp, file)), { recursive: true });
    await copyFile(join(root, file), join(temp, file));
  }
  baseline = await readFile(join(temp, adapter), "utf8");

  await edit(
    `    iris::collect(
        &REMOVE_MEMBER,
        OpenApiRouter::new().routes(utoipa_axum::routes!(remove_member_endpoint)),
        success_schemas::<RemoveMemberSuccess>(),
    )`,
    `    let (router, api) = OpenApiRouter::new()
        .routes(utoipa_axum::routes!(remove_member_endpoint))
        .split_for_parts();
    let _ = success_schemas::<RemoveMemberSuccess>();
    Collected {
        router,
        api,
        entry: iris::CatalogEntry {
            name: REMOVE_MEMBER.name,
            public_id: REMOVE_MEMBER.public_id,
            handler: REMOVE_MEMBER.handler,
            mappings: REMOVE_MEMBER.mappings(),
        },
    }`,
  );
  expect("omitted bridge on the second operation", "cargo", contract, /surviving inferred handler ID: remove_member_endpoint/);

  await reset();
  await edit(`public_id: "removeMember",`, `public_id: "changeMemberRole",`);
  expect("duplicate OpenAPI ID", "cargo", contract, /duplicate OpenAPI operation ID: changeMemberRole/);

  await reset();
  await edit(
    `    success_schema: "RemoveMemberSuccess",
    rejections: Rejection::VARIANTS,
    rejection: membership_rejection,`,
    `    success_schema: "RemoveMemberSuccess",
    rejections: Rejection::VARIANTS,
    rejection: |r| Mapping {
        message: if matches!(r, Rejection::Forbidden) {
            "Changed."
        } else {
            membership_rejection(r).message
        },
        ..membership_rejection(r)
    },`,
  );
  expect(
    "conflicting metadata for a shared code",
    "cargo",
    contract,
    /public code memberships\.forbidden differs between memberships\.change_role and memberships\.remove_member/,
  );

  await reset();
  await edit(`path = "/api/memberships/remove"`, `path = "/api/memberships/wrong"`);
  expect("wrong path passes the compiler", "cargo", ["check", ...cargo], /^$/, true);
  expect("wrong path caught by the independent inventory", "cargo", contract, /assertion `left == right` failed/);

  const sharedName = async (extraField) => {
    await edit(
      `#[derive(Serialize, utoipa::ToSchema)]
struct RemoveMemberSuccess {
    completion: Completion,
}`,
      `#[derive(Serialize, utoipa::ToSchema)]
#[schema(as = ChangeRoleSuccess)]
struct RemoveMemberSuccess {
    completion: Completion,${extraField ? "\n    removed: bool," : ""}
}`,
    );
    if (extraField) {
      await edit(
        `    RemoveMemberSuccess {
        completion: Completion::Acknowledged,
    }`,
        `    RemoveMemberSuccess {
        completion: Completion::Acknowledged,
        removed: true,
    }`,
      );
    }
    await edit(`success_schema: "RemoveMemberSuccess",`, `success_schema: "ChangeRoleSuccess",`);
  };

  await reset();
  await sharedName(true);
  expect("same-named but different component", "cargo", contract, /component conflict: schemas\/ChangeRoleSuccess/);

  await reset();
  await sharedName(false);
  expect("identical shared component assembles", "cargo", ["run", ...cargo, "--bin", "export-openapi", "--", "probe.json"], /^$/, true);
  const shared = JSON.parse(await readFile(join(temp, "probe.json"), "utf8"));
  const data =
    shared.paths["/api/memberships/remove"].post.responses["200"].content["application/json"].schema.oneOf[0].properties.data;
  if ("RemoveMemberSuccess" in shared.components.schemas || data.$ref !== "#/components/schemas/ChangeRoleSuccess") {
    throw new Error("identical shared component was not the one exported");
  }
  console.log("CONTROL: identical shared component exported once and referenced by both operations");

  await reset();
  await edit(`        (remove_member(), mount_remove_member),\n`, "");
  expect("omitted operation collection", "cargo", contract, /assertion `left == right` failed/);

  console.log("PASS: checkpoint A probes detected; disposable copy and its target removed on exit");
} finally {
  await rm(temp, { recursive: true, force: true });
}
