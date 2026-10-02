// Omitted-edit probes for checkpoints A and B and the current-state read. Each
// probe edits a disposable source copy, never the checkout, and builds into
// that copy's own target directory so no mutated artifact can outlive the run.
import { execFileSync, spawnSync } from "node:child_process";
import { access, copyFile, cp, mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { homedir, tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { stripVTControlCharacters } from "node:util";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");
const temp = await mkdtemp(join(tmpdir(), "iris-reference-probes-"));
const adapter = "apps/reference/src/http/memberships.rs";
const domain = "apps/reference/src/domains/memberships.rs";
const tests = "apps/reference/src/http/memberships/tests.rs";
const web = "apps/reference/web";
const schema = "apps/reference/openapi.json";
const generated = `${web}/src/generated.ts`;
const narrowing = `${web}/src/narrowing.ts`;
const cargo = ["--quiet", "--locked", "-p", "iris-reference"];
// Each touched file's original text, restored by `reset`.
const baselines = new Map();
const counts = { CAUGHT: 0, CONTROL: 0 };
const env = {
  ...process.env,
  CARGO_TARGET_DIR: join(temp, "target"),
  CARGO_HOME: join(temp, "cargo-home"),
  CARGO_NET_OFFLINE: "true",
  XDG_CACHE_HOME: join(temp, "cache"),
  npm_config_cache: join(temp, "npm-cache"),
  TMPDIR: join(temp, "tmp"),
};
delete env.IRIS_REFERENCE_FIXTURES;

function run(command, args, cwd = temp) {
  return spawnSync(command, args, {
    cwd,
    encoding: "utf8",
    env,
    maxBuffer: 16 * 1024 * 1024,
  });
}

// A spawn error or a signal is a broken probe, never an expected failure.
function expect(label, command, args, pattern, pass = false, cwd = temp) {
  const result = run(command, args, cwd);
  if (result.error || result.signal !== null) {
    throw new Error(`${label}: did not run to completion (${result.error ?? result.signal})`);
  }
  const output = result.stdout + result.stderr;
  const plainOutput = stripVTControlCharacters(output);
  const patterns = Array.isArray(pattern) ? pattern : [pattern];
  if ((result.status === 0) !== pass || !patterns.every((p) => p.test(plainOutput))) {
    throw new Error(`${label}: unexpected result ${result.status}\n${output}`);
  }
  const evidence = patterns.map((p) => p.exec(plainOutput)[0].trim() || "(empty output)");
  const kind = pass ? "CONTROL" : "CAUGHT";
  counts[kind]++;
  console.log(`${kind}: ${label}: ${evidence.join("; ")}`);
}

async function edit(from, to, file = adapter) {
  const path = join(temp, file);
  const text = await readFile(path, "utf8");
  if (!baselines.has(file)) throw new Error(`Unregistered pristine baseline: ${file}`);
  if (!text.includes(from) || text.indexOf(from) !== text.lastIndexOf(from)) {
    throw new Error(`Probe anchor drift: ${file}: ${from}`);
  }
  await writeFile(path, text.replace(from, to));
}

async function reset() {
  for (const [file, text] of baselines) await writeFile(join(temp, file), text);
  for (const [file, text] of baselines) {
    if ((await readFile(join(temp, file), "utf8")) !== text) throw new Error(`Restoration failed: ${file}`);
  }
}

// The named test must be among the failures (quiet libtest prints one
// "---- <path> stdout ----" block per failure); another failure is not this
// probe's signal.
const failed = (name) => new RegExp(`^---- (?:\\S*::)?${name} stdout ----$`, "m");
const lib = (filter) => ["test", ...cargo, "--lib", filter];

const contract = ["test", ...cargo, "--test", "contract"];
const check = ["check", ...cargo];
const exportTo = (file) => ["run", ...cargo, "--bin", "export-openapi", "--", file];
const assertion = /assertion `left == right` failed/;
const behavior = (label, name, diagnostic = assertion) =>
  expect(label, "cargo", lib(name), [failed(name), diagnostic]);
const healthy = (when) => {
  expect(`${when}: reference Rust`, "cargo", ["test", "--quiet", "--locked", "-p", "iris", "-p", "iris-reference"], /111 passed; 0 failed/, true);
  expect(`${when}: reference web`, process.execPath, [`${web}/scripts/verify.mjs`], /PASS: reference client types/, true);
};

async function appendTests(source) {
  // The pristine test file was registered before any subprocess or edit.
  await writeFile(join(temp, tests), baselines.get(tests) + source);
}

async function added(omit) {
  if (omit !== "variant") await edit("    LastOwner,", "    LastOwner,\n    ProjectArchived,", domain);
  if (omit !== "descriptor") await edit(
    "            Self::MemberNotFound => Descriptor {",
    `            Self::ProjectArchived => Descriptor {
                code: "memberships.project_archived",
                summary: "Project archived.",
                rule: None,
                prerequisite: None,
            },
            Self::MemberNotFound => Descriptor {`, domain);
  if (omit !== "status") await edit("        Rejection::LastOwner => 409,", "        Rejection::ProjectArchived => 409,\n        Rejection::LastOwner => 409,");
  if (omit !== "policy") await edit(
    "        if !owner { return Err(StopReason::Rejected(Rejection::Forbidden)); }",
    `        if !owner { return Err(StopReason::Rejected(Rejection::Forbidden)); }
        if project_id == 41 { return Err(StopReason::Rejected(Rejection::ProjectArchived)); }`, domain);
  await appendTests(`
#[tokio::test]
async fn archived_probe() {
    let f = Fixture::new().await;
    let mut conn = connect(&f.state.database).await.unwrap();
    let changed = action::change_role(&mut conn, &Actor(11), ChangeRole {
        project_id: 41, user_id: 29, role: MemberRole::Viewer,
    }).await;
    assert_eq!(changed.unwrap_err(), ActionError::Rejected(Rejection::ProjectArchived));
    let removed = action::remove_member(&mut conn, &Actor(11), action::RemoveMember {
        project_id: 41, user_id: 29,
    }).await;
    assert_eq!(removed.unwrap_err(), ActionError::Rejected(Rejection::ProjectArchived));
}
`);
}

try {
  console.log(`Disposable probe copy: ${temp}`);
  try {
    for (const dependency of ["typescript/bin/tsc", "openapi-typescript/package.json", "vite/bin/vite.js"])
      await access(join(root, web, "node_modules", dependency));
  } catch (cause) {
    throw new Error("Install reference web dependencies before probes: npm --prefix apps/reference/web ci (the runner never installs)", { cause });
  }
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

  for (const file of [adapter, domain, tests, schema, generated, narrowing, "apps/reference/src/app.rs", "apps/reference/src/http/mod.rs", "apps/reference/src/domains/projects.rs", "crates/iris/src/lib.rs"])
    baselines.set(file, await readFile(join(temp, file), "utf8"));
  await mkdir(env.TMPDIR, { recursive: true });
  // Vite writes node_modules/.vite-temp: a symlink would write into the checkout.
  await cp(join(root, web, "node_modules"), join(temp, web, "node_modules"), { recursive: true, verbatimSymlinks: true });
  const cargoHome = process.env.CARGO_HOME ?? join(homedir(), ".cargo");
  await mkdir(env.CARGO_HOME, { recursive: true });
  for (const directory of ["registry", "git"]) {
    try { await access(join(cargoHome, directory)); } catch { continue; }
    await cp(join(cargoHome, directory), join(env.CARGO_HOME, directory), { recursive: true });
  }
  // Offline Cargo uses only the copied cache. Missing crates require setup
  // outside the runner (cargo fetch --locked), not installation by a probe.
  console.log("Prerequisite: cached Rust dependencies (run cargo fetch --locked outside the runner if missing); Cargo is offline here");
  healthy("pristine before first mutation");

  for (const omission of ["variant", "descriptor", "status"]) {
    await reset();
    await added(omission);
    expect(`new shared rejection omits ${omission}`, "cargo", check,
      omission === "variant" ? /error\[E0599\]: no variant, associated function, or constant named `ProjectArchived` found for enum `Rejection`/ : /error\[E0004\]: non-exhaustive patterns: `Rejection::ProjectArchived` not covered/);
  }
  await reset();
  await added("policy");
  behavior("new shared rejection omits policy", "archived_probe", /called `Result::unwrap_err\(\)` on an `Ok` value: Acknowledged/);
  await reset();
  await added();
  expect("complete shared rejection reaches both policies", "cargo", lib("archived_probe"), /1 passed; 0 failed/, true);
  for (const name of ["independent_contract", "remove_member_independent_contract"])
    behavior(`new rejection without compatibility update: ${name}`, name);
  expect("export added rejection", "cargo", exportTo("probe.json"), /^$/, true);
  const addedDoc = JSON.parse(await readFile(join(temp, "probe.json"), "utf8"));
  for (const path of ["/api/memberships/role", "/api/memberships/remove"]) {
    const branches = addedDoc.paths[path].post.responses["409"].content["application/json"].schema.oneOf;
    if (branches.filter((b) => b.properties.code?.enum?.[0] === "memberships.project_archived" && b.properties.kind.enum[0] === "rejected").length !== 1)
      throw new Error(`New rejection missing or duplicated in actual response branches: ${path}`);
  }
  console.log("CONTROL: new rejection in actual 409 response branches of both mutations");
  counts.CONTROL++;

  await reset();
  await edit("Rejection::LastOwner => 409", "Rejection::LastOwner => 422");
  for (const name of ["independent_contract", "remove_member_independent_contract"])
    behavior(`422 without compatibility update: ${name}`, name);
  expect("422 without export regeneration", "cargo", contract, [failed("exported_document_is_current"), /reference contract drift/]);
  expect("export 422 to temporary committed schema", "cargo", exportTo(schema), /^$/, true);
  expect("real verifier rejects stale generated types before Rust", process.execPath, [`${web}/scripts/verify.mjs`], /Reference client drift:/);
  expect("generation-only child", process.execPath, ["--input-type=module", "-e", `
import openapiTS, { astToString } from "openapi-typescript";
import { readFile, writeFile } from "node:fs/promises";
await writeFile("src/generated.ts", astToString(await openapiTS(JSON.parse(await readFile("../openapi.json", "utf8")))));
`], /^$/, true, join(temp, web));
  expect("422 types reject obsolete 409 client branch", process.execPath, [`${web}/node_modules/typescript/bin/tsc`, "-p", web], /narrowing.ts\(\d+,\d+\): error TS2367:.*409.*have no overlap/);
  // Include each function's distinct return to select only mutation branches;
  // the read-only @ts-expect-error 409 cases must stay untouched.
  for (const ending of [
    `    // @ts-expect-error Unrelated codes cannot occur at 409.
    const forbidden: "memberships.forbidden" = response.body.code;
    return [code, forbidden];`,
    `    return code;`,
  ]) {
    const branch = `  if (response.status === 409) {
    const code: "memberships.last_owner" = response.body.code;
${ending}
  }`;
    await edit(branch, branch.replaceAll("409", "422"), narrowing);
  }
  expect("both repaired mutation branches typecheck", process.execPath, [`${web}/node_modules/typescript/bin/tsc`, "-p", web], /^$/, true);

  await reset();
  await edit("struct ChangeRoleSuccess {\n    completion: Completion,\n}", "struct ChangeRoleSuccess {\n    completion: Completion,\n    role: String,\n}");
  expect("required success field without projector", "cargo", check, /missing field `role` in initializer of `ChangeRoleSuccess`/);
  await edit("    ChangeRoleSuccess {\n        completion: Completion::Acknowledged,\n    }", "    ChangeRoleSuccess {\n        completion: Completion::Acknowledged,\n        role: String::from(\"viewer\"),\n    }");
  expect("complete projector compiles", "cargo", check, /^$/, true);

  await reset();
  await edit("Some(serde_json::to_value(project(ack)).unwrap()),", "Some({ let _ = project(ack); serde_json::json!({}) }),");
  expect("raw DTO bypass compiles", "cargo", check, /^$/, true);
  behavior("raw DTO bypass fails whole-request schema", "whole_request_contract_and_fixtures", /schema rejected/);

  await reset();
  await appendTests(`
#[tokio::test]
async fn runtime_mount_probe() {
    let f = Fixture::new().await;
    let alice = f.cookie(Some(11)).await;
    let response = f.app().oneshot(request(&alice, &body(29, "viewer"))).await.unwrap();
    // Unmatched routes have no envelope headers or JSON. Check status first.
    assert_eq!(response.status().as_u16(), 200, "runtime mount must serve valid request");
    let response = collect(response).await;
    expect(&response, 200, "success", None);
}
`);
  expect("known-valid mounted request returns 200", "cargo", lib("runtime_mount_probe"), /1 passed; 0 failed/, true);
  await edit("|router, (operation, mount)| router.merge(mount(operation, auth.clone())),", "|router, (_operation, _mount)| router,", "apps/reference/src/app.rs");
  expect("runtime mount omission compiles", "cargo", check, /^$/, true);
  expect("runtime mount omission keeps both contract integration tests green", "cargo", contract, /2 passed; 0 failed/, true);
  behavior("runtime mount omission is raw 404 versus 200", "runtime_mount_probe", /runtime mount must serve valid request\n\s+left: 404\n\s+right: 200/);

  await reset();
  await edit("Some(ErrorCode::Csrf) => Some(Shared::Csrf)", "Some(ErrorCode::Csrf) => None", "apps/reference/src/http/mod.rs");
  expect("CSRF producer linkage omission compiles", "cargo", check, /^$/, true);
  expect("CSRF producer linkage omission preserves export and inventory", "cargo", contract, /2 passed; 0 failed/, true);
  behavior("CSRF producer linkage omission fails runtime envelope", "whole_request_contract_and_fixtures", /left: Null\n\s+right: "refused"/);
  await reset();
  await edit(".filter(|s| csrf || !matches!(s, Shared::Csrf))", ".filter(|s| (csrf || !matches!(s, Shared::Csrf)) && !matches!(s, Shared::Csrf))", "crates/iris/src/lib.rs");
  for (const name of ["independent_contract", "remove_member_independent_contract"])
    behavior(`CSRF declaration omission: ${name}`, name);

  await reset();

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
  expect("omitted bridge on the second operation", "cargo", contract, [failed("independent_inventory"), /surviving inferred handler ID: remove_member_endpoint/]);

  await reset();
  await edit(`public_id: "removeMember",`, `public_id: "changeMemberRole",`);
  expect("duplicate OpenAPI ID", "cargo", contract, [failed("independent_inventory"), /duplicate OpenAPI operation ID: changeMemberRole/]);

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
    [failed("independent_inventory"), /public code memberships\.forbidden differs between memberships\.change_role and memberships\.remove_member/],
  );

  await reset();
  await edit(`path = "/api/memberships/remove"`, `path = "/api/memberships/wrong"`);
  expect("wrong path passes the compiler", "cargo", ["check", ...cargo], /^$/, true);
  expect("wrong path caught by the independent inventory", "cargo", contract, [failed("independent_inventory"), assertion]);

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
  expect("same-named but different component", "cargo", contract, [failed("independent_inventory"), /component conflict: schemas\/ChangeRoleSuccess/]);

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
  counts.CONTROL++;

  await reset();
  await edit(`        (remove_member(), mount_remove_member),\n`, "");
  expect("omitted operation collection", "cargo", contract, [failed("independent_inventory"), assertion]);

  // Checkpoint B: a GET parameter bound and each read's visibility predicate.
  // The compiler accepts every edit; an independent test catches it.
  await reset();
  await edit("(1..=100).contains(n)", "(1..=101).contains(n)", "apps/reference/src/http/mod.rs");
  expect("runtime GET limit bound widened passes the compiler", "cargo", ["check", ...cargo], /^$/, true);
  for (const name of ["list_members_input_refusals", "list_mine_input_refusals"]) {
    behavior(`runtime GET limit bound widened caught by ${name}`, name, /limit=101:.*\n\s+left: 200\n\s+right: 400/);
  }

  await reset();
  await edit(`#[param(pattern = "^(100|[1-9][0-9]?)$")]`, `#[param(pattern = "^(10[01]|[1-9][0-9]?)$")]`, "apps/reference/src/http/mod.rs");
  expect("exported GET limit bound widened passes the compiler", "cargo", ["check", ...cargo], /^$/, true);
  expect(
    "exported GET limit bound widened caught by the hand-written parameters",
    "cargo",
    lib("list_members_independent_contract"),
    [failed("list_members_independent_contract"), assertion],
  );
  expect("exported GET limit bound widened caught by export drift", "cargo", contract, [failed("exported_document_is_current"), /reference contract drift/]);

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
    [failed("list_members_authorization"), /left: 200\n\s+right: 403/],
  );

  await reset();
  await edit("WHERE m.user_id=? AND m.project_id>?", "WHERE ? IS NOT NULL AND m.project_id>?", "apps/reference/src/domains/projects.rs");
  expect("own-project listing without its actor filter passes the compiler", "cargo", ["check", ...cargo], /^$/, true);
  expect(
    "own-project listing without its actor filter caught by the authorization test",
    "cargo",
    lib("list_mine_authorization"),
    [failed("list_mine_authorization"), assertion],
  );

  // The mutations' current-state read. Assembly resolves the named read; it
  // cannot tell which of two same-schema required fields is the right input.
  await reset();
  await edit(`operation: "listProjectMembers",`, `operation: "listMembers",`);
  expect(
    "current-state read naming no operation caught by the catalog check",
    "cargo",
    contract,
    [failed("independent_inventory"), /memberships\.(change_role|remove_member) recovery read listMembers is not in the document/],
  );

  await reset();
  await edit(`path_inputs: &[("project_id", "project_id")],`, `path_inputs: &[("project_id", "user_id")],`);
  expect("member ID bound to the project path assembles", "cargo", ["run", ...cargo, "--bin", "export-openapi", "--", "probe.json"], /^$/, true);
  expect(
    "member ID bound to the project path caught by the hand-written recovery test",
    "cargo",
    lib("recovery_contract_declares_the_member_list"),
    [failed("recovery_contract_declares_the_member_list"), assertion],
  );

  await reset();
  healthy("pristine after final restoration");
} finally {
  await rm(temp, { recursive: true, force: true });
}
console.log(`PASS: ${counts.CAUGHT} caught, ${counts.CONTROL} controls; disposable source, generated files, target and caches removed`);
