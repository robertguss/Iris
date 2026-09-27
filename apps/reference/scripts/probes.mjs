// Omitted-edit probes for checkpoints A and B and the current-state read. Each
// probe edits a disposable source copy, never the checkout, and builds into
// that copy's own target directory so no mutated artifact can outlive the run.
import { execFileSync, spawnSync } from "node:child_process";
import { copyFile, mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");
const temp = await mkdtemp(join(tmpdir(), "iris-reference-probes-"));
const adapter = "apps/reference/src/http/memberships.rs";
const cargo = ["--quiet", "--locked", "-p", "iris-reference"];
// Each touched file's original text, restored by `reset`.
const baselines = new Map();

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

async function edit(from, to, file = adapter) {
  const path = join(temp, file);
  const text = await readFile(path, "utf8");
  if (!baselines.has(file)) baselines.set(file, text);
  if (!text.includes(from) || text.indexOf(from) !== text.lastIndexOf(from)) {
    throw new Error(`Probe anchor drift: ${file}: ${from}`);
  }
  await writeFile(path, text.replace(from, to));
}

async function reset() {
  for (const [file, text] of baselines) await writeFile(join(temp, file), text);
  baselines.clear();
}

// The named test must be among the failures (quiet libtest prints one
// "---- <path> stdout ----" block per failure); another failure is not this
// probe's signal.
const failed = (name) => new RegExp(`^---- \\S*::${name} stdout ----$`, "m");
const lib = (filter) => ["test", ...cargo, "--lib", filter];

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

  // Checkpoint B: a GET parameter bound and each read's visibility predicate.
  // The compiler accepts every edit; an independent test catches it.
  await reset();
  await edit("(1..=100).contains(n)", "(1..=101).contains(n)", "apps/reference/src/http/mod.rs");
  expect("runtime GET limit bound widened passes the compiler", "cargo", ["check", ...cargo], /^$/, true);
  for (const name of ["list_members_input_refusals", "list_mine_input_refusals"]) {
    expect(`runtime GET limit bound widened caught by ${name}`, "cargo", lib(name), failed(name));
  }

  await reset();
  await edit(`#[param(pattern = "^(100|[1-9][0-9]?)$")]`, `#[param(pattern = "^(10[01]|[1-9][0-9]?)$")]`, "apps/reference/src/http/mod.rs");
  expect("exported GET limit bound widened passes the compiler", "cargo", ["check", ...cargo], /^$/, true);
  expect(
    "exported GET limit bound widened caught by the hand-written parameters",
    "cargo",
    lib("list_members_independent_contract"),
    failed("list_members_independent_contract"),
  );
  expect("exported GET limit bound widened caught by export drift", "cargo", contract, /reference contract drift/);

  await reset();
  await edit(
    `"SELECT EXISTS(SELECT 1 FROM memberships WHERE project_id=? AND user_id=?)",`,
    `"SELECT ? > 0 OR ? > 0",`,
    "apps/reference/src/domains/memberships.rs",
  );
  expect("member listing without its visibility predicate passes the compiler", "cargo", ["check", ...cargo], /^$/, true);
  expect(
    "member listing without its visibility predicate caught by the authorization test",
    "cargo",
    lib("list_members_authorization"),
    failed("list_members_authorization"),
  );

  await reset();
  await edit("WHERE m.user_id=? AND m.project_id>?", "WHERE ? IS NOT NULL AND m.project_id>?", "apps/reference/src/domains/projects.rs");
  expect("own-project listing without its actor filter passes the compiler", "cargo", ["check", ...cargo], /^$/, true);
  expect(
    "own-project listing without its actor filter caught by the authorization test",
    "cargo",
    lib("list_mine_authorization"),
    failed("list_mine_authorization"),
  );

  // The mutations' current-state read. Assembly resolves the named read; it
  // cannot tell which of two same-schema required fields is the right input.
  await reset();
  await edit(`operation: "listProjectMembers",`, `operation: "listMembers",`);
  expect(
    "current-state read naming no operation caught by the catalog check",
    "cargo",
    contract,
    /memberships\.(change_role|remove_member) recovery read listMembers is not in the document/,
  );

  await reset();
  await edit(`path_inputs: &[("project_id", "project_id")],`, `path_inputs: &[("project_id", "user_id")],`);
  expect("member ID bound to the project path assembles", "cargo", ["run", ...cargo, "--bin", "export-openapi", "--", "probe.json"], /^$/, true);
  expect(
    "member ID bound to the project path caught by the hand-written recovery test",
    "cargo",
    lib("recovery_contract_declares_the_member_list"),
    failed("recovery_contract_declares_the_member_list"),
  );

  console.log("PASS: checkpoint A and B probes and current-state read probes detected; disposable copy and its target removed on exit");
} finally {
  await rm(temp, { recursive: true, force: true });
}
