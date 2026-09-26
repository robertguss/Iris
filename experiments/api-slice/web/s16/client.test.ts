import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { client } from "./client.ts";

const api = client(JSON.parse(readFileSync(process.argv[2], "utf8")));
const fixtures = JSON.parse(readFileSync(process.argv[3], "utf8")) as {status:number;body:Record<string,unknown>}[];
assert.deepEqual(api.recovery, { inspect:false, read:false, replay:false, new_submission:"current authority and intent required" });
let checks = 0;
async function check(request: () => Promise<Response>, expected: "server" | "client_unknown") {
  let calls = 0;
  const result = await api.execute(() => { calls++; return request(); });
  assert.equal(calls,1,"never resend an uncertain attempt");
  assert.equal(result.kind,expected,JSON.stringify(result));
  assert.ok(!JSON.stringify(result).includes("secret-canary"));
  checks++;
  return result;
}
const response = (body: unknown,status=200) => async () => new Response(JSON.stringify(body),{status});
for (const fixture of fixtures) await check(response(fixture.body,fixture.status),"server");
const success = {schema_version:1,operation:"memberships.change_role",request_id:"req_0123456789abcdef0123456789abcdef",kind:"success",data:{completion:"acknowledged"}};
await check(response(success),"server");
await check(response({...success,extra:true,data:{completion:"acknowledged",extra:true}}),"server");
for (const bad of [
  {...success,schema_version:2}, {...success,operation:"other"}, {...success,data:{}}, {...success,data:{completion:"pending"}},
  {...success,request_id:"caller"}, {...success,kind:"failure"}, null, [], {},
]) await check(response(bad),"client_unknown");
const last = {schema_version:1,operation:"memberships.change_role",request_id:success.request_id,kind:"rejected",code:"memberships.last_owner",message:"safe"};
await check(response(last,409),"server");
for (const [value,status] of [[{...last,code:"memberships.forbidden"},409],[{...last,code:"future"},409],[{...last,kind:"refused"},409],[last,403],[success,201]] as const) await check(response(value,status),"client_unknown");
for (const text of ["<html>secret-canary</html>","", "{secret-canary"]) await check(async () => new Response(text,{status:200}),"client_unknown");
await check(async () => { throw new Error("secret-canary"); },"client_unknown");
await check(async () => new Response(new ReadableStream({start(controller){controller.error(new Error("secret-canary"));}})),"client_unknown");
// A validated request failure can follow commit. Neither it nor response loss
// supplies an action-effect conclusion or an automatic retry capability.
const failure=await check(response({...last,kind:"failure",code:"iris.internal"},500),"server");
assert.equal(failure.kind,"server");
const lost=await check(async()=>{throw new Error("response lost after commit");},"client_unknown");
const later=await check(response({...last,code:"memberships.forbidden"},403),"server");
assert.equal(lost.kind,"client_unknown"); assert.equal(later.kind,"server");
console.log(`PASS: ${checks} whole-request runtime cases; one attempt each; no raw diagnostics`);
