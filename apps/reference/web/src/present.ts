// What the page says about each mutation outcome. The switch is exhaustive
// over both mutations' generated codes, so a new code is a compile-time
// decision here.
import type { Mutation, Result } from "./client.ts";
import { api } from "./membership.ts";

export type Presentation = {
  tone: "success" | "rejected" | "refused" | "unconfirmed";
  title: string;
  detail: string;
};

// A public 500 or 503, like a missing response, is no evidence either way
// (S14): the page claims neither effect nor its absence, and offers no
// readback, which this checkpoint does not have.
const UNCONFIRMED =
  "Outcome unconfirmed: the change may or may not have been applied. No automatic retry was sent; a new submission needs current authority and intent.";

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
      detail: UNCONFIRMED,
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
        detail: UNCONFIRMED,
      };
    case "iris.unavailable":
      return {
        tone: "unconfirmed",
        title: "Temporarily unavailable",
        detail: UNCONFIRMED,
      };
    default: {
      const exhaustive: never = body;
      return exhaustive;
    }
  }
}
