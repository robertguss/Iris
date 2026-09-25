import createClient from "openapi-fetch";
import type { paths, components } from "./generated/utoipa";

export const api = createClient<paths>();
export const legacyDemo = import.meta.env.VITE_IRIS_LEGACY_DEMO === "true";
export type SessionInfo = components["schemas"]["SessionInfo"];
let csrf = "";
api.use({ onRequest({ request }) {
  if (!legacyDemo && !["GET", "HEAD", "OPTIONS"].includes(request.method)) request.headers.set("x-iris-csrf", csrf);
  return request;
} });
let bootstrap: Promise<SessionInfo> | undefined;
export function refreshSession(): Promise<SessionInfo> {
  // StrictMode and simultaneous consumers must share the initial cookie bootstrap.
  return bootstrap ??= api.GET("/api/auth/session").then(({ data }) => {
    if (!data) throw new Error("Could not read the session.");
    csrf = data.csrf_token;
    return data;
  }).finally(() => { bootstrap = undefined; });
}
export type AcceptRequest = components["schemas"]["AcceptRequest"];
export type Acceptance = components["schemas"]["Acceptance"];
export type IssueRequest = components["schemas"]["IssueRequest"];
export type IssuedInvitation = components["schemas"]["IssuedInvitation"];
export type ChangeRoleRequest = components["schemas"]["ChangeRoleRequest"];
export type RemoveMemberRequest = components["schemas"]["RemoveMemberRequest"];
export type MemberChange = components["schemas"]["MemberChange"];
export type Role = components["schemas"]["Role"];
export type Problem = components["schemas"]["Problem"];

// Exhaustiveness makes a newly added Rust error code a TypeScript decision.
export function errorTitle(code: Problem["code"]): string {
  switch (code) {
    case "invalid_request": return "Invalid request";
    case "csrf": return "Session refresh required";
    case "login_failed": return "Login failed";
    case "forbidden": return "Owner permission required";
    case "recipient_not_found": return "Recipient not found";
    case "member_not_found": return "Member not found";
    case "last_owner": return "Last owner must remain";
    case "already_member": return "Already a member";
    case "invitation_pending": return "Invitation pending";
    case "unauthorized": return "Identity required";
    case "not_found": return "Invitation not found";
    case "expired": return "Invitation expired";
    case "already_accepted": return "Already accepted";
    case "internal": return "Server error";
    case "unavailable": return "Database busy";
    default: {
      const exhaustive: never = code;
      return exhaustive;
    }
  }
}
