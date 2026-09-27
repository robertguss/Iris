// Compile-time cases: generated types narrow each operation's responses by
// status and code. Checked by `tsc`; never executed.
import snapshot from "../../openapi.json";
import { client } from "./client.ts";
import type { Known } from "./client.ts";

// The committed export, imported as the UI bundles it, is a `Document` as is.
export const bundled = () => client(snapshot);

export function changeRole(response: Known<"changeMemberRole">) {
  const operation: "memberships.change_role" = response.body.operation;
  if (response.status === 200) {
    const completion: "acknowledged" = response.body.data.completion;
    // @ts-expect-error Success does not claim a stored role.
    response.body.data.role;
    return [operation, completion];
  }
  if (response.status === 403) {
    const code: "http.csrf_refused" | "memberships.forbidden" =
      response.body.code;
    if (response.body.code === "http.csrf_refused") {
      const kind: "refused" = response.body.kind;
      return kind;
    }
    const kind: "rejected" = response.body.kind;
    return [code, kind];
  }
  if (response.status === 404) {
    const code: "memberships.member_not_found" = response.body.code;
    return code;
  }
  if (response.status === 409) {
    const code: "memberships.last_owner" = response.body.code;
    // @ts-expect-error Unrelated codes cannot occur at 409.
    const forbidden: "memberships.forbidden" = response.body.code;
    return [code, forbidden];
  }
  return response.body.kind;
}

export function removeMember(response: Known<"removeMember">) {
  const operation: "memberships.remove_member" = response.body.operation;
  if (response.status === 200) {
    const completion: "acknowledged" = response.body.data.completion;
    // @ts-expect-error Removal success does not report a role.
    response.body.data.role;
    return [operation, completion];
  }
  if (response.status === 403) {
    const code: "http.csrf_refused" | "memberships.forbidden" =
      response.body.code;
    if (response.body.code === "memberships.forbidden") {
      const kind: "rejected" = response.body.kind;
      return [code, kind];
    }
    const kind: "refused" = response.body.kind;
    return kind;
  }
  if (response.status === 404) {
    const code: "memberships.member_not_found" = response.body.code;
    // @ts-expect-error Unrelated codes cannot occur at 404.
    const lastOwner: "memberships.last_owner" = response.body.code;
    return [code, lastOwner];
  }
  if (response.status === 409) {
    const code: "memberships.last_owner" = response.body.code;
    return code;
  }
  return response.body.kind;
}

export function crossed(
  response: Known<"removeMember">,
): Known<"changeMemberRole"> {
  // @ts-expect-error A removal response is not a role-change response.
  return response;
}
