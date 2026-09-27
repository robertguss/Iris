// What the page says about each outcome. The switches are exhaustive over the
// generated codes of both mutations and both reads, so a new code is a
// compile-time decision here.
import type { Mutation, Read, Recovery, Result } from "./client.ts";
import type { components } from "./generated.ts";
import { api } from "./membership.ts";

export type Presentation = {
  tone: "success" | "rejected" | "refused" | "unconfirmed";
  title: string;
  detail: string;
};

// A public 500 or 503, like a missing response, is no evidence either way
// (S14): the page claims neither effect nor its absence.
const UNCONFIRMED =
  "Outcome unconfirmed: the change may or may not have been applied. No automatic retry was sent; a new submission needs current authority and intent.";

// A declared current-state read the page can offer, scoped to a page as read:
// reading again resolves nothing about the attempt (S14).
const READ_AGAIN: Partial<Record<Read, string>> = {
  listProjectMembers:
    "Reading the project’s members again is a new read. A returned page describes members when it was read and neither confirms nor rules out this attempt.",
};

/** An unconfirmed outcome, pointing to the declared read if the page knows it. */
export function unconfirmed(recovery: Recovery): string {
  const again = recovery.read && READ_AGAIN[recovery.read.operation_id];
  return again ? `${UNCONFIRMED} ${again}` : UNCONFIRMED;
}

const PREREQUISITES: Record<string, string> = {
  "memberships.another_owner_required": "Another owner is required first.",
};

export function present(
  op: Mutation,
  result: Result<"changeMemberRole"> | Result<"removeMember">,
): Presentation {
  if (result.kind === "client_unknown")
    return {
      tone: "unconfirmed",
      title: "No usable response",
      detail: unconfirmed(api.recovery(op)),
    };
  const body = result.response.body;
  if (body.kind === "success")
    return {
      tone: "success",
      title:
        op === "changeMemberRole"
          ? "Role change acknowledged"
          : "Removal acknowledged",
      detail: "The server acknowledged the request.",
    };
  switch (body.code) {
    case "memberships.forbidden":
      return {
        tone: "rejected",
        title: "Owner permission required",
        detail: body.message,
      };
    case "memberships.member_not_found":
      return {
        tone: "rejected",
        title: "Member not found",
        detail: body.message,
      };
    case "memberships.last_owner": {
      const prerequisite = api.prerequisites(op)[body.code];
      const next = prerequisite ? PREREQUISITES[prerequisite] : undefined;
      return {
        tone: "rejected",
        title: "Last owner must remain",
        detail: next ? `${body.message} ${next}` : body.message,
      };
    }
    case "http.invalid_request":
      return {
        tone: "refused",
        title: "Invalid request",
        detail: body.message,
      };
    case "http.unauthenticated":
      return {
        tone: "refused",
        title: "Sign-in required",
        detail: body.message,
      };
    case "http.csrf_refused":
      return {
        tone: "refused",
        title: "Session refresh required",
        detail: body.message,
      };
    case "iris.internal":
      return {
        tone: "unconfirmed",
        title: "Server error",
        detail: unconfirmed(api.recovery(op)),
      };
    case "iris.unavailable":
      return {
        tone: "unconfirmed",
        title: "Temporarily unavailable",
        detail: unconfirmed(api.recovery(op)),
      };
    default: {
      const exhaustive: never = body;
      return exhaustive;
    }
  }
}

/** A listed row: the declared ID, name and role, and nothing else. */
export type Row = {
  id: string;
  name: string;
  role: components["schemas"]["Role"];
};
export type ReadPresentation =
  | {
      tone: "listed";
      title: string;
      detail: string;
      rows: Row[];
      next: string | null;
    }
  | {
      tone: "rejected" | "refused" | "unavailable";
      title: string;
      detail: string;
    };

const LISTED = {
  listProjectMembers: { title: "Project members", noun: "members" },
  listMyProjects: { title: "Your projects", noun: "projects" },
};

// A page is present state when read, not the whole collection (S17): every
// sentence is scoped to this page, which may follow earlier ones. Absence from
// it proves nothing about an earlier attempt (S14).
function listing(op: Read, rows: Row[], next: string | null) {
  const { title, noun } = LISTED[op];
  const detail =
    rows.length === 0
      ? `No ${noun} on this page when it was read.` +
        (next === null ? "" : ` More ${noun} existed then.`)
      : next === null
        ? `No further ${noun} existed when this page was read.`
        : `More ${noun} existed when this page was read.`;
  return { tone: "listed" as const, title, detail, rows, next };
}

// A read has no effect to report; repeating it is a new observation.
const notLoaded = (op: Read) =>
  `${op === "listProjectMembers" ? "Members" : "Projects"} were not loaded. No automatic retry was sent; loading them again is a new read.`;

export function presentRead(
  op: Read,
  result: Result<"listProjectMembers"> | Result<"listMyProjects">,
): ReadPresentation {
  if (result.kind === "client_unknown")
    return {
      tone: "unavailable",
      title: "No usable response",
      detail: notLoaded(op),
    };
  const body = result.response.body;
  if (body.kind === "success") {
    // Named fields only: additive fields never reach the page.
    const rows =
      body.operation === "memberships.list"
        ? body.data.items.map(({ user_id, display_name, role }) => ({
            id: user_id,
            name: display_name,
            role,
          }))
        : body.data.items.map(({ project_id, name, role }) => ({
            id: project_id,
            name,
            role,
          }));
    return listing(op, rows, body.data.next_cursor);
  }
  switch (body.code) {
    // One response for an unknown project and a non-member (S17).
    case "memberships.forbidden":
      return {
        tone: "rejected",
        title: "Project not available",
        detail: body.message,
      };
    case "http.invalid_request":
      return {
        tone: "refused",
        title: "Invalid request",
        detail: body.message,
      };
    case "http.unauthenticated":
      return {
        tone: "refused",
        title: "Sign-in required",
        detail: body.message,
      };
    case "iris.internal":
      return {
        tone: "unavailable",
        title: "Server error",
        detail: notLoaded(op),
      };
    case "iris.unavailable":
      return {
        tone: "unavailable",
        title: "Temporarily unavailable",
        detail: notLoaded(op),
      };
    default: {
      const exhaustive: never = body;
      return exhaustive;
    }
  }
}
