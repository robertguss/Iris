// How each domain request is built, written by hand: reads are one GET with
// no body and no CSRF header; mutations are one POST carrying the session's
// CSRF token. Runs under Node with `fetch` stubbed.
import assert from "node:assert/strict";
import { read, send } from "./membership.ts";
import { refreshSession } from "./session.ts";

type Call = { url: string; init: RequestInit };
let calls: Call[] = [];
let reply: () => Promise<Response>;
globalThis.fetch = async (input, init = {}) => {
  calls.push({ url: String(input), init });
  return reply();
};
/** Runs `action` once and returns the single request it made. */
async function only<T>(action: () => Promise<T>) {
  calls = [];
  const result = await action();
  assert.equal(calls.length, 1, "exactly one request");
  return { ...calls[0], result };
}
const params = (url: string) => new URL(url, "http://127.0.0.1").searchParams;

const REQUEST_ID = "req_0123456789abcdef0123456789abcdef";
const page = {
  schema_version: 1,
  operation: "memberships.list",
  request_id: REQUEST_ID,
  kind: "success",
  data: {
    items: [{ user_id: "11", display_name: "Alice Example", role: "owner" }],
    next_cursor: "c1.11",
  },
};
reply = async () => new Response(JSON.stringify(page));

let checks = 0;
// A read is a GET without a body or CSRF header, sent with the session cookie.
for (const [op, run, path] of [
  [
    "listProjectMembers",
    () => read("listProjectMembers", { project_id: "41" }),
    "/api/projects/41/members",
  ],
  ["listMyProjects", () => read("listMyProjects", {}), "/api/projects"],
] as const) {
  const { url, init } = await only<unknown>(run);
  assert.equal(url, path, `${op}: no query unless given`);
  assert.equal(init.method, "GET");
  assert.equal(init.credentials, "same-origin");
  assert.equal(init.body, undefined, `${op}: no body`);
  assert.equal(new Headers(init.headers).get("x-iris-csrf"), null);
  checks++;
}

// The project ID fills its path segment encoded; the server validates it.
{
  const { url } = await only(() =>
    read("listProjectMembers", { project_id: "4/1?x" }),
  );
  assert.equal(url, "/api/projects/4%2F1%3Fx/members");
  checks++;
}

// Limit and cursor are sent only when given, and the cursor arrives exactly
// as it was echoed, whatever it contains.
for (const cursor of ["c1.29", "c1.29&limit=100", "c1 +%2F"]) {
  const { url } = await only(() =>
    read("listProjectMembers", { project_id: "41", limit: "1", cursor }),
  );
  assert.ok(url.startsWith("/api/projects/41/members?"));
  const query = params(url);
  assert.deepEqual([...query.keys()], ["limit", "cursor"]);
  assert.equal(query.get("limit"), "1");
  assert.equal(query.get("cursor"), cursor);
  checks++;
}
{
  const { url } = await only(() => read("listMyProjects", { cursor: "c1.43" }));
  assert.deepEqual([...params(url).entries()], [["cursor", "c1.43"]]);
  const limited = await only(() => read("listMyProjects", { limit: "100" }));
  assert.deepEqual([...params(limited.url).entries()], [["limit", "100"]]);
  checks++;
}

// The boundary classifies what comes back: a declared page decodes, a thrown
// request is unknown, and neither is resent.
{
  const { result } = await only(() =>
    read("listProjectMembers", { project_id: "41" }),
  );
  assert.equal(result.kind, "server");
  reply = async () => {
    throw new Error("secret-canary");
  };
  const lost = await only(() => read("listMyProjects", {}));
  assert.equal(lost.result.kind, "client_unknown");
  assert.ok(
    lost.result.kind === "client_unknown" &&
      lost.result.reason === "request_failed",
  );
  checks++;
}

// A mutation is one POST with a JSON body and the session's CSRF token.
{
  reply = async () =>
    new Response(JSON.stringify({ csrf_token: "csrf-token", user_id: "11" }));
  await refreshSession();
  reply = async () => new Response("", { status: 200 });
  const body = { project_id: "41", user_id: "29" };
  const { url, init } = await only(() => send("removeMember", body));
  assert.equal(url, "/api/memberships/remove");
  assert.equal(init.method, "POST");
  assert.equal(init.credentials, "same-origin");
  const headers = new Headers(init.headers);
  assert.equal(headers.get("x-iris-csrf"), "csrf-token");
  assert.equal(headers.get("content-type"), "application/json");
  assert.deepEqual(JSON.parse(String(init.body)), body);
  checks++;
}
console.log(
  `PASS: ${checks} request constructions; reads are bodiless GETs without CSRF, one attempt each`,
);
