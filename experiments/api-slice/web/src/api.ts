import createClient from "openapi-fetch";
import type { paths, components } from "./generated/utoipa";

export const api = createClient<paths>();
export type AcceptRequest = components["schemas"]["AcceptRequest"];
export type Acceptance = components["schemas"]["Acceptance"];
export type Problem = components["schemas"]["Problem"];

// Exhaustiveness makes a newly added Rust error code a TypeScript decision.
export function errorTitle(code: Problem["code"]): string {
  switch (code) {
    case "invalid_request": return "Invalid request";
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
