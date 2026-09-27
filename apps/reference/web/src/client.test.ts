// Runtime cases for the whole-request boundary: real responses captured by the
// Rust whole-request tests, then independent hand-written cases. Expectations
// here are written by hand, not derived from the export.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { BODY_LIMIT, DOMAIN_OPERATIONS, client } from "./client.ts";
import type { Document, Operation, Reason, Result } from "./client.ts";

const [documentPath, fixtureDir] = process.argv.slice(2);
const document = JSON.parse(readFileSync(documentPath, "utf8")) as Document;
const api = client(document);

const NAMES = {
  changeMemberRole: "memberships.change_role",
  removeMember: "memberships.remove_member",
} as const;
const PATHS = {
  changeMemberRole: "/api/memberships/role",
  removeMember: "/api/memberships/remove",
} as const;
// How many responses of each status, kind and code each Rust test captures.
const SHARED = {
  "401 refused http.unauthenticated": 1,
  "403 refused http.csrf_refused": 2,
  "403 rejected memberships.forbidden": 1,
  "404 rejected memberships.member_not_found": 1,
  "409 rejected memberships.last_owner": 1,
  "500 failure iris.internal": 1,
  "503 failure iris.unavailable": 1,
};
const CAPTURED: Record<Operation, Record<string, number>> = {
  changeMemberRole: {
    ...SHARED,
    "200 success": 2,
    "400 refused http.invalid_request": 9,
  },
  removeMember: {
    ...SHARED,
    "200 success": 1,
    "400 refused http.invalid_request": 8,
  },
};

// Linkage: the boundary accepts exactly the two POST domain operations.
const copy = () => structuredClone(document);
const role = (doc: Document) => doc.paths[PATHS.changeMemberRole];
const extra = copy();
extra.paths["/api/extra"] = {
  post: { ...role(extra).post, operationId: "extraOperation" },
};
const unmarked = copy();
delete unmarked.paths[PATHS.removeMember].post["x-iris"];
const method = copy();
role(method).put = role(method).post;
delete role(method).post;
for (const doc of [extra, unmarked, method])
  assert.throws(() => client(doc), /Reference operation linkage mismatch/);
// Session responses may omit a body; a domain response may not.
const bodiless = copy();
delete bodiless.paths[PATHS.removeMember].post.responses["409"].content;
assert.throws(() => client(bodiless), /lacks a JSON schema/);

for (const op of DOMAIN_OPERATIONS) {
  assert.equal(api.path(op), PATHS[op]);
  assert.deepEqual(api.recovery(op), {
    inspect: false,
    read: false,
    replay: false,
    new_submission: "current authority and intent required",
  });
  assert.deepEqual(api.prerequisites(op), {
    "memberships.last_owner": "memberships.another_owner_required",
  });
}

let checks = 0;
const attempts = new Set<string>();
async function check<Op extends Operation>(
  op: Op,
  request: () => Promise<Response>,
  expected: "server" | Reason,
): Promise<Result<Op>> {
  let calls = 0;
  const result = await api.execute(op, () => {
    calls++;
    return request();
  });
  assert.equal(calls, 1, "never resend an uncertain attempt");
  const outcome = result.kind === "server" ? "server" : result.reason;
  assert.equal(outcome, expected, `${op}: ${JSON.stringify(result)}`);
  assert.ok(!JSON.stringify(result).includes("secret-canary"));
  if (result.kind === "client_unknown") {
    assert.ok(!attempts.has(result.local_attempt_id), "fresh local attempt ID");
    attempts.add(result.local_attempt_id);
  }
  checks++;
  return result;
}
const respond =
  (body: unknown, status = 200) =>
  async () =>
    new Response(JSON.stringify(body), { status });

const encoder = new TextEncoder();
/** `envelope` as JSON, padded with whitespace to exactly `size` bytes. */
function padded(envelope: object, size: number) {
  const text = JSON.stringify(envelope);
  return encoder.encode(text + " ".repeat(size - text.length));
}
/** A lazy stream of `bytes` that records pulls and cancellation. */
function chunked(bytes: Uint8Array, size: number, failIfPulled = false) {
  const state = { pulls: 0, cancelled: false };
  let offset = 0;
  const stream = new ReadableStream<Uint8Array>(
    {
      pull(controller) {
        state.pulls++;
        if (failIfPulled) throw new Error("secret-canary: pulled");
        if (offset >= bytes.length) return controller.close();
        controller.enqueue(bytes.slice(offset, (offset += size)));
      },
      cancel() {
        state.cancelled = true;
      },
    },
    { highWaterMark: 0 },
  );
  return { stream, state };
}

const REQUEST_ID = "req_0123456789abcdef0123456789abcdef";
for (const op of DOMAIN_OPERATIONS) {
  const other = op === "changeMemberRole" ? "removeMember" : "changeMemberRole";

  // Real responses from the assembled application.
  const fixtures = JSON.parse(
    readFileSync(join(fixtureDir, `${op}.json`), "utf8"),
  ) as {
    status: number;
    body: Record<string, unknown>;
  }[];
  const tally: Record<string, number> = {};
  for (const { status, body } of fixtures) {
    assert.equal(body.operation, NAMES[op]);
    const key = [status, body.kind, body.code]
      .filter((part) => part !== undefined)
      .join(" ");
    tally[key] = (tally[key] ?? 0) + 1;
    await check(op, respond(body, status), "server");
  }
  assert.deepEqual(tally, CAPTURED[op], `${op}: captured responses`);

  // Independent envelopes, including additive fields.
  const base = {
    schema_version: 1,
    operation: NAMES[op],
    request_id: REQUEST_ID,
  };
  const success = {
    ...base,
    kind: "success",
    data: { completion: "acknowledged" },
  };
  const envelope = (kind: string, code: string) => ({
    ...base,
    kind,
    code,
    message: "safe",
  });
  const last = envelope("rejected", "memberships.last_owner");
  for (const [body, status] of [
    [success, 200],
    [
      {
        ...success,
        extra: true,
        data: { completion: "acknowledged", extra: true },
      },
      200,
    ],
    [envelope("rejected", "memberships.forbidden"), 403],
    [envelope("rejected", "memberships.member_not_found"), 404],
    [last, 409],
    [envelope("refused", "http.invalid_request"), 400],
    [envelope("refused", "http.unauthenticated"), 401],
    [envelope("refused", "http.csrf_refused"), 403],
    [envelope("failure", "iris.internal"), 500],
    [envelope("failure", "iris.unavailable"), 503],
  ] as const)
    await check(op, respond(body, status), "server");

  // Unsupported, malformed or undeclared responses stay unknown.
  await check(
    op,
    respond({ ...success, schema_version: 2 }),
    "unsupported_version",
  );
  for (const body of [
    { ...success, operation: "other" },
    { ...success, operation: NAMES[other] },
    { ...success, data: {} },
    { ...success, data: { completion: "pending" } },
    { ...success, request_id: "caller" },
    { ...success, kind: "failure" },
    null,
    [],
    {},
  ])
    await check(op, respond(body), "contract_mismatch");
  for (const [body, status] of [
    [envelope("rejected", "memberships.forbidden"), 409],
    [envelope("rejected", "future"), 409],
    [{ ...last, kind: "refused" }, 409],
    [last, 403],
    [envelope("rejected", "memberships.member_not_found"), 403],
    [{ ...last, operation: NAMES[other] }, 409],
    [success, 201],
  ] as const)
    await check(op, respond(body, status), "contract_mismatch");
  for (const text of ["<html>secret-canary</html>", "", "{secret-canary"])
    await check(
      op,
      async () => new Response(text, { status: 200 }),
      "non_json",
    );
  // Strict UTF-8: a lossy decode would turn this byte into a valid message.
  const text = JSON.stringify({ ...last, message: "X" });
  const invalid = encoder.encode(text);
  invalid[text.indexOf('"X"') + 1] = 0xff;
  await check(
    op,
    async () => new Response(invalid, { status: 409 }),
    "non_json",
  );
  await check(
    op,
    async () => {
      throw new Error("secret-canary");
    },
    "request_failed",
  );
  await check(
    op,
    async () =>
      new Response(
        new ReadableStream({
          start(controller) {
            controller.error(new Error("secret-canary"));
          },
        }),
      ),
    "body_unreadable",
  );
  // The boundary reads the body as sent, or not at all: a locked, consumed or
  // partly read body is unreadable, even when the remainder would validate.
  await check(
    op,
    async () => {
      const response = new Response(JSON.stringify(success));
      response.body!.getReader();
      return response;
    },
    "body_unreadable",
  );
  await check(
    op,
    async () => {
      const response = new Response(JSON.stringify(success));
      await response.text();
      return response;
    },
    "body_unreadable",
  );
  await check(
    op,
    async () => {
      const response = new Response(
        new ReadableStream({
          start(controller) {
            controller.enqueue(encoder.encode("<html>secret-canary</html>"));
            controller.enqueue(encoder.encode(JSON.stringify(success)));
            controller.close();
          },
        }),
      );
      const reader = response.body!.getReader();
      await reader.read();
      reader.releaseLock();
      return response;
    },
    "body_unreadable",
  );

  // Bounded reads. A streamed body past the limit is cancelled early.
  const CHUNK = 16 * 1024;
  const over = padded(success, BODY_LIMIT + 4 * CHUNK);
  const streamed = chunked(over, CHUNK);
  await check(op, async () => new Response(streamed.stream), "body_oversize");
  assert.ok(streamed.state.cancelled, "oversize stream cancelled");
  assert.ok(
    streamed.state.pulls < over.length / CHUNK,
    "cancelled before the end",
  );
  const atLimit = chunked(padded(success, BODY_LIMIT), CHUNK);
  await check(op, async () => new Response(atLimit.stream), "server");
  assert.ok(!atLimit.state.cancelled);
  // A declared length past the limit is refused without reading the body.
  const declared = chunked(over, CHUNK, true);
  await check(
    op,
    async () =>
      new Response(declared.stream, {
        headers: { "content-length": String(BODY_LIMIT + 1) },
      }),
    "body_oversize",
  );
  assert.equal(declared.state.pulls, 0, "declared oversize body never pulled");
  assert.ok(declared.state.cancelled, "declared oversize body cancelled");
  const declaredAtLimit = chunked(padded(success, BODY_LIMIT), CHUNK);
  await check(
    op,
    async () =>
      new Response(declaredAtLimit.stream, {
        headers: { "content-length": String(BODY_LIMIT) },
      }),
    "server",
  );

  // A validated request failure can follow commit. Neither it nor response
  // loss supplies an action-effect conclusion or an automatic retry.
  await check(op, respond(envelope("failure", "iris.internal"), 500), "server");
  await check(
    op,
    async () => {
      throw new Error("response lost after commit");
    },
    "request_failed",
  );
  await check(
    op,
    respond(envelope("rejected", "memberships.forbidden"), 403),
    "server",
  );
}
console.log(
  `PASS: ${checks} whole-request runtime cases across ${DOMAIN_OPERATIONS.length} operations; one attempt each; no raw diagnostics`,
);
