// What the page says for every declared outcome of both mutations and both
// reads, written by hand. Runs under Node, which also exercises the bundled
// JSON import.
import assert from "node:assert/strict";
import { MUTATIONS, READS } from "./client.ts";
import { present, presentRead } from "./present.ts";

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
for (const op of MUTATIONS) {
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
// Reads. A page is present state when read (S14, S17): its wording is scoped
// to that page, and a listing shows only the declared fields.
const READ_NAMES = {
  listProjectMembers: "memberships.list",
  listMyProjects: "projects.list_mine",
} as const;
const NOUN = { listProjectMembers: "members", listMyProjects: "projects" };
const TITLE = {
  listProjectMembers: "Project members",
  listMyProjects: "Your projects",
};
const ITEMS = {
  listProjectMembers: [
    { user_id: "11", display_name: "Alice Example", role: "owner" },
    { user_id: "29", display_name: "Bob Example", role: "editor" },
  ],
  listMyProjects: [
    { project_id: "41", name: "Launch plan", role: "owner" },
    { project_id: "43", name: "Field notes", role: "editor" },
  ],
};
const ROWS = {
  listProjectMembers: [
    { id: "11", name: "Alice Example", role: "owner" },
    { id: "29", name: "Bob Example", role: "editor" },
  ],
  listMyProjects: [
    { id: "41", name: "Launch plan", role: "owner" },
    { id: "43", name: "Field notes", role: "editor" },
  ],
};
// Claims no read wording may make: a page is neither the whole collection nor
// evidence about an earlier attempt, and a read has no effect to report.
const READ_CLAIMS = [
  /\ball (members|projects)\b/i,
  /complete/i,
  /snapshot/i,
  /no longer/i,
  /removed/i,
  /any project/i,
  /\bno (members|projects)\b(?! on this page)/i,
  /\bnot a member\b/i,
  /applied/i,
  /changed/i,
  /succeeded/i,
  /safe to retry/i,
  /retrying/i,
];
const claimsNothing = (shown: { title: string; detail: string }) => {
  for (const claim of READ_CLAIMS) {
    assert.doesNotMatch(shown.title, claim);
    assert.doesNotMatch(shown.detail, claim);
  }
};

type ReadInput = Parameters<typeof presentRead>[1];
for (const op of READS) {
  const noun = NOUN[op];
  const base = {
    schema_version: 1,
    operation: READ_NAMES[op],
    request_id: "req_0123456789abcdef0123456789abcdef",
  };
  const server = (status: number, body: object) =>
    ({
      kind: "server",
      response: { status, body: { ...base, ...body } },
    }) as ReadInput;
  const listed = (items: object[], next_cursor: string | null) =>
    presentRead(
      op,
      server(200, { kind: "success", data: { items, next_cursor } }),
    );

  // Each combination of rows and cursor, including an empty continuation: a
  // later page can be empty while the caller still has rows on earlier ones.
  for (const [items, next, rows, detail] of [
    [
      ITEMS[op],
      "c1.29",
      ROWS[op],
      `More ${noun} existed when this page was read.`,
    ],
    [
      ITEMS[op],
      null,
      ROWS[op],
      `No further ${noun} existed when this page was read.`,
    ],
    [[], null, [], `No ${noun} on this page when it was read.`],
    [
      [],
      "c1.29",
      [],
      `No ${noun} on this page when it was read. More ${noun} existed then.`,
    ],
  ] as const) {
    const shown = listed([...items], next);
    assert.equal(shown.tone, "listed", `${op} listing`);
    assert.ok(shown.tone === "listed");
    assert.equal(shown.title, TITLE[op]);
    assert.equal(shown.detail, detail);
    assert.deepEqual(shown.rows, rows);
    assert.equal(shown.next, next);
    claimsNothing(shown);
    checks++;
  }

  // Additive fields, including a contact detail a server must never send, are
  // dropped rather than shown.
  const leaky = listed(
    ITEMS[op].map((item) => ({
      ...item,
      email: "secret-canary@example.com",
      extra: "secret-canary",
    })),
    null,
  );
  assert.ok(leaky.tone === "listed");
  assert.deepEqual(leaky.rows, ROWS[op]);
  assert.ok(!JSON.stringify(leaky).includes("secret-canary"));
  checks++;

  const declared = [
    [400, "refused", "http.invalid_request", "refused", "Invalid request"],
    [401, "refused", "http.unauthenticated", "refused", "Sign-in required"],
    [500, "failure", "iris.internal", "unavailable", "Server error"],
    [
      503,
      "failure",
      "iris.unavailable",
      "unavailable",
      "Temporarily unavailable",
    ],
    ...(op === "listProjectMembers"
      ? ([
          [
            403,
            "rejected",
            "memberships.forbidden",
            "rejected",
            "Project not available",
          ],
        ] as const)
      : []),
  ] as const;
  for (const [status, kind, code, tone, title] of declared) {
    const shown = presentRead(
      op,
      server(status, { kind, code, message: "Server message." }),
    );
    assert.equal(shown.tone, tone, `${op} ${code}`);
    assert.equal(shown.title, title, `${op} ${code}`);
    if (tone === "unavailable")
      assert.equal(
        shown.detail,
        `${op === "listProjectMembers" ? "Members" : "Projects"} were not loaded. No automatic retry was sent; loading them again is a new read.`,
      );
    else assert.equal(shown.detail, "Server message.");
    claimsNothing(shown);
    checks++;
  }

  for (const reason of [
    "request_failed",
    "body_oversize",
    "contract_mismatch",
  ] as const) {
    const shown = presentRead(op, {
      kind: "client_unknown",
      local_attempt_id: "attempt",
      reason,
    });
    assert.equal(shown.tone, "unavailable");
    assert.equal(shown.title, "No usable response");
    assert.match(shown.detail, /^(Members|Projects) were not loaded\./);
    claimsNothing(shown);
    checks++;
  }
}

console.log(
  `PASS: ${checks} outcome presentations; unconfirmed outcomes claim no effect; read wording is scoped to the page`,
);
