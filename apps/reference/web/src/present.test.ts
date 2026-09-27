// What the page says for every declared outcome of both operations, written
// by hand. Runs under Node, which also exercises the bundled JSON import.
import assert from "node:assert/strict";
import { DOMAIN_OPERATIONS } from "./client.ts";
import { present } from "./present.ts";

const NAMES = {
  changeMemberRole: "memberships.change_role",
  removeMember: "memberships.remove_member",
} as const;
// Status, kind and code, with the tone and title the page must show.
const CASES = [
  [
    403,
    "rejected",
    "memberships.forbidden",
    "rejected",
    "Owner permission required",
  ],
  [
    404,
    "rejected",
    "memberships.member_not_found",
    "rejected",
    "Member not found",
  ],
  [
    409,
    "rejected",
    "memberships.last_owner",
    "rejected",
    "Last owner must remain",
  ],
  [400, "refused", "http.invalid_request", "refused", "Invalid request"],
  [401, "refused", "http.unauthenticated", "refused", "Sign-in required"],
  [403, "refused", "http.csrf_refused", "refused", "Session refresh required"],
  [500, "failure", "iris.internal", "unconfirmed", "Server error"],
  [
    503,
    "failure",
    "iris.unavailable",
    "unconfirmed",
    "Temporarily unavailable",
  ],
] as const;
// Claims the page must never make about an unconfirmed outcome.
const OVERCLAIMS = [
  /was applied/i,
  /not applied/i,
  /nothing (was )?changed/i,
  /no change/i,
  /succeeded/i,
  /failed to/i,
  /safe to retry/i,
  /retrying/i,
  /check the current/i,
  /read ?back/i,
  /inspect/i,
];

type Input = Parameters<typeof present>[1];
let checks = 0;
for (const op of DOMAIN_OPERATIONS) {
  const base = {
    schema_version: 1,
    operation: NAMES[op],
    request_id: "req_0123456789abcdef0123456789abcdef",
  };
  const server = (status: number, body: object) =>
    ({
      kind: "server",
      response: { status, body: { ...base, ...body } },
    }) as Input;

  const success = present(
    op,
    server(200, { kind: "success", data: { completion: "acknowledged" } }),
  );
  assert.equal(success.tone, "success");
  assert.equal(
    success.title,
    op === "changeMemberRole"
      ? "Role change acknowledged"
      : "Removal acknowledged",
  );
  assert.doesNotMatch(
    success.detail,
    /role|owner|editor|viewer/i,
    "no stored-state claim",
  );
  checks++;

  for (const [status, kind, code, tone, title] of CASES) {
    const shown = present(
      op,
      server(status, { kind, code, message: "Server message." }),
    );
    assert.equal(shown.tone, tone, `${op} ${code}`);
    assert.equal(shown.title, title, `${op} ${code}`);
    if (tone === "unconfirmed") {
      assert.match(shown.detail, /^Outcome unconfirmed/);
      for (const claim of OVERCLAIMS) assert.doesNotMatch(shown.detail, claim);
    } else assert.ok(shown.detail.startsWith("Server message."));
    if (code === "memberships.last_owner")
      assert.match(shown.detail, /Another owner is required first\./);
    checks++;
  }

  for (const reason of [
    "request_failed",
    "body_oversize",
    "contract_mismatch",
  ] as const) {
    const shown = present(op, {
      kind: "client_unknown",
      local_attempt_id: "attempt",
      reason,
    });
    assert.equal(shown.tone, "unconfirmed");
    assert.equal(shown.title, "No usable response");
    assert.match(shown.detail, /^Outcome unconfirmed/);
    for (const claim of OVERCLAIMS) assert.doesNotMatch(shown.detail, claim);
    checks++;
  }
}
console.log(
  `PASS: ${checks} outcome presentations; unconfirmed outcomes claim no effect`,
);
