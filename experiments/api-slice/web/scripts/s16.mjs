import { execFileSync } from "node:child_process";
import { mkdtemp, readFile, writeFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve, dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import openapiTS, { astToString } from "openapi-typescript";

const web = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const root = resolve(web,"../../..");
const temp = await mkdtemp(join(tmpdir(),"iris-s16-"));
const run = (command,args,options={}) => execFileSync(command,args,{cwd:root,stdio:"inherit",...options});
try {
  const specPath=join(temp,"s16.json");
  const fixturePath=join(temp,"responses.json");
  run("cargo",["run","--quiet","--locked","-p","iris-api-spike","--bin","export-s16","--",specPath]);
  const spec=await readFile(specPath,"utf8");
  const types=astToString(await openapiTS(JSON.parse(spec)));
  for (const [path,text] of [[resolve(web,"../contracts/s16.json"),spec],[resolve(web,"s16/generated.ts"),types]]) {
    if(process.argv.includes("--generate")) await writeFile(path,text);
    else if(await readFile(path,"utf8")!==text) throw new Error(`S16 contract drift: ${path}; run npm run generate:s16`);
  }
  run("cargo",["test","--quiet","--locked","-p","iris-api-spike","--lib","s16"],{env:{...process.env,IRIS_S16_FIXTURES:fixturePath}});
  run(resolve(web,"node_modules/.bin/tsc"),["-p",resolve(web,"s16/tsconfig.json")]);
  run("node",[resolve(web,"s16/client.test.ts"),specPath,fixturePath]);
  console.log("PASS: S16 export, generated types, Rust contract/behavior and runtime validation");
} finally { await rm(temp,{recursive:true,force:true}); }
