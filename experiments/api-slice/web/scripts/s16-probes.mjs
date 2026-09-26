// Omitted-edit experiments run in a disposable source copy, never the checkout.
import { execFileSync, spawnSync } from "node:child_process";
import { mkdtemp, readFile, writeFile, mkdir, copyFile, symlink, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve, dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import openapiTS, { astToString } from "openapi-typescript";
const web=resolve(dirname(fileURLToPath(import.meta.url)),"..");
const root=resolve(web,"../../..");
const temp=await mkdtemp(join(tmpdir(),"iris-s16-probes-"));
const domain="experiments/api-slice/server/src/s16/action.rs";
const adapter="experiments/api-slice/server/src/s16/mod.rs";
const tests="experiments/api-slice/server/src/s16/tests.rs";
const generated="experiments/api-slice/web/s16/generated.ts";
const baseline=new Map();
const cargo=["--quiet","--locked","-p","iris-api-spike"];
function run(command,args) {
  return spawnSync(command,args,{cwd:temp,encoding:"utf8",env:{...process.env,CARGO_TARGET_DIR:resolve(root,"target")},maxBuffer:8*1024*1024});
}
function expect(label,command,args,pattern,pass=false) {
  const result=run(command,args);
  const output=result.stdout+result.stderr;
  if ((result.status===0)!==pass || !pattern.test(output)) throw new Error(`${label}: unexpected result ${result.status}\n${output}`);
  const evidence=output.split("\n").find(line=>pattern.test(line)) ?? pattern.source;
  console.log(`${pass?"CONTROL":"CAUGHT"}: ${label}: ${evidence.trim()}`);
}
async function reset() { for(const [path,text] of baseline) await writeFile(join(temp,path),text); }
async function edit(path,from,to) {
  const text=await readFile(join(temp,path),"utf8");
  if(!text.includes(from) || text.indexOf(from)!==text.lastIndexOf(from)) throw new Error(`Probe anchor drift: ${path}: ${from}`);
  await writeFile(join(temp,path),text.replace(from,to));
}
async function added(omit="") {
  if(omit!=="variant") await edit(domain,"    LastOwner,","    LastOwner,\n    ProjectArchived,");
  if(omit!=="descriptor") await edit(domain,"            Self::Forbidden => (","            Self::ProjectArchived => (\"memberships.project_archived\", \"Project archived.\", None, None),\n            Self::Forbidden => (");
  if(omit!=="projection") await edit(adapter,"        Rejection::Forbidden => 403,","        Rejection::ProjectArchived => 409,\n        Rejection::Forbidden => 403,");
  if(omit!=="body") await edit(domain,"        let role: Option<String> =", "        if input.project_id == 41 { return Err(StopReason::Rejected(Rejection::ProjectArchived)); }\n        let role: Option<String> =");
  const source=await readFile(join(temp,tests),"utf8");
  await writeFile(join(temp,tests),source+`
#[tokio::test]
async fn archived_probe() {
    let f=Fixture::new().await;
    let mut conn=connect(&f.state.database).await.unwrap();
    let result=action::change_role(&mut conn,&Actor(11),ChangeRole {project_id:41,user_id:29,role:MemberRole::Viewer}).await;
    assert_eq!(result.unwrap_err(),ActionError::Rejected(Rejection::ProjectArchived));
}
`);
}
try {
  const files=execFileSync("git",["ls-files","--cached","--others","--exclude-standard","-z"],{cwd:root,encoding:"utf8"}).split("\0").filter(Boolean);
  for(const file of files) {
    await mkdir(dirname(join(temp,file)),{recursive:true});
    await copyFile(join(root,file),join(temp,file));
  }
  await symlink(resolve(web,"node_modules"),join(temp,"experiments/api-slice/web/node_modules"),"dir");
  for(const file of [domain,adapter,tests,generated]) baseline.set(file,await readFile(join(temp,file),"utf8"));
  for(const omission of ["variant","descriptor","projection"]) {
    await reset(); await added(omission);
    expect(`new rejection, omitted ${omission}`,"cargo",["check",...cargo],omission==="variant"?/no variant.*named `ProjectArchived`/:/non-exhaustive patterns.*ProjectArchived/);
  }
  await reset(); await added("body");
  expect("new rejection, omitted policy check","cargo",["test",...cargo,"--lib","archived_probe"],/called `Result::unwrap_err\(\)` on an `Ok` value/);
  await reset(); await added();
  expect("complete added rejection reaches behavior without route edit","cargo",["test",...cargo,"--lib","archived_probe"],/1 passed/,true);
  expect("new rejection, omitted compatibility expectation","cargo",["test",...cargo,"--lib","independent_contract"],/assertion `left == right` failed/);
  expect("new variant automatically exported","cargo",["run",...cargo,"--bin","export-s16","--","probe.json"],/^$/,true);
  const addedDoc=await readFile(join(temp,"probe.json"),"utf8");
  if(!addedDoc.includes("memberships.project_archived")) throw new Error("derive enumeration omitted new variant");
  console.log("CONTROL: new rejection enumerated into export without a handwritten list");

  await reset();
  await edit(adapter,"Rejection::LastOwner => 409","Rejection::LastOwner => 422");
  expect("status 422, omitted independent compatibility edit","cargo",["test",...cargo,"--lib","independent_contract"],/assertion `left == right` failed/);
  expect("status 422, omitted regeneration","node",["experiments/api-slice/web/scripts/s16.mjs"],/S16 contract drift/);
  expect("export changed response","cargo",["run",...cargo,"--bin","export-s16","--","probe.json"],/^$/,true);
  await writeFile(join(temp,generated),astToString(await openapiTS(JSON.parse(await readFile(join(temp,"probe.json"),"utf8")))));
  expect("status 422, regenerated types but omitted client handling","node",[resolve(web,"node_modules/typescript/bin/tsc"),"-p","experiments/api-slice/web/s16/tsconfig.json"],/types.*409.*have no overlap/);

  await reset();
  await edit(adapter,"    completion: Completion,","    completion: Completion,\n    role: String,");
  expect("required success field, omitted projector","cargo",["check",...cargo],/missing field `role`/);
  await reset();
  await edit(adapter,"body[\"data\"] = serde_json::to_value(data.expect(\"success projection\")).unwrap();","let _ = data; body[\"data\"] = json!({});");
  expect("raw response bypasses success DTO","cargo",["test",...cargo,"--lib","whole_request_contract_and_fixtures"],/schema rejected/);

  await reset();
  await edit(adapter,"        .routes(utoipa_axum::routes!(endpoint))\n","");
  expect("omitted route collection","cargo",["run",...cargo,"--bin","export-s16","--","probe.json"],/S16 requires exactly one collected path/);
  await reset();
  await edit(adapter,"    bridge(&mut api);","    // deliberately omitted bridge");
  expect("omitted response linkage","cargo",["test",...cargo,"--lib","independent_contract"],/assertion `left == right` failed/);
  await reset();
  await edit(adapter,'path = "/api/memberships/role"','path = "/api/memberships/wrong"');
  expect("wrong collected path passes compiler","cargo",["check",...cargo],/^$/,true);
  expect("wrong collected path caught by independent contract","cargo",["test",...cargo,"--lib","independent_contract"],/assertion `left == right` failed/);
  await reset();
  await edit(adapter,"auth.layer(routes().0)","auth.layer(Router::new())");
  expect("omitted runtime route mounting, exporter unchanged","cargo",["test",...cargo,"--lib","whole_request_contract_and_fixtures"],/Adding a route_layer before any routes is a no-op/);
  await reset();
  await edit(adapter,"Some(ErrorCode::Csrf) => Some(Shared::Csrf)","Some(ErrorCode::Csrf) => None");
  expect("profile declared but CSRF producer linkage omitted","cargo",["test",...cargo,"--lib","whole_request_contract_and_fixtures"],/assertion `left == right` failed/);
  await reset();
  await edit(adapter,"        .chain(Shared::VARIANTS.iter().copied().map(shared));","        .chain(Shared::VARIANTS.iter().copied().filter(|s| !matches!(s, Shared::Csrf)).map(shared));");
  expect("CSRF producer retained but profile branch omitted","cargo",["test",...cargo,"--lib","independent_contract"],/assertion `left == right` failed/);
  console.log("PASS: omitted-edit probes detected; disposable source copy removed on exit");
} finally { await rm(temp,{recursive:true,force:true}); }
