// Compile-time cases: generated types narrow each operation's responses by
// status and code, and constrain read parameters. Checked by `tsc`; never
// executed.
import snapshot from "../../openapi.json" with { type: "json" };
import { client } from "./client.ts";
import type { CurrentStateRead, Known, Read, Result } from "./client.ts";
import type { components } from "./generated.ts";
import { read } from "./membership.ts";

type Role = components["schemas"]["Role"];

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

export function listMembers(response: Known<"listProjectMembers">) {
  const operation: "memberships.list" = response.body.operation;
  if (response.status === 200) {
    const next: string | null = response.body.data.next_cursor;
    const item = response.body.data.items[0];
    const fields: [string, string, Role] = [
      item.user_id,
      item.display_name,
      item.role,
    ];
    // @ts-expect-error Member summaries never include contact details.
    item.email;
    // @ts-expect-error A member summary names no project.
    item.project_id;
    return [operation, next, fields];
  }
  if (response.status === 403) {
    const code: "memberships.forbidden" = response.body.code;
    const kind: "rejected" = response.body.kind;
    // @ts-expect-error Reads declare no CSRF refusal.
    const csrf: "http.csrf_refused" = response.body.code;
    return [code, kind, csrf];
  }
  // @ts-expect-error Reads declare no 404.
  if (response.status === 404) return;
  // @ts-expect-error Reads declare no 409.
  if (response.status === 409) return;
  return response.body.kind;
}

export function listMine(response: Known<"listMyProjects">) {
  const operation: "projects.list_mine" = response.body.operation;
  if (response.status === 200) {
    const next: string | null = response.body.data.next_cursor;
    const item = response.body.data.items[0];
    const fields: [string, string, Role] = [
      item.project_id,
      item.name,
      item.role,
    ];
    // @ts-expect-error Project summaries never include contact details.
    item.email;
    // @ts-expect-error A project summary names no member.
    item.user_id;
    return [operation, next, fields];
  }
  // @ts-expect-error Rows are filtered by actor, so no 403 is declared.
  if (response.status === 403) return;
  // @ts-expect-error Reads declare no 404.
  if (response.status === 404) return;
  // @ts-expect-error Reads declare no 409.
  if (response.status === 409) return;
  const code:
    | "http.invalid_request"
    | "http.unauthenticated"
    | "iris.internal"
    | "iris.unavailable" = response.body.code;
  return code;
}

// Presentation takes this explicit union: a mapped `Known` over the union of
// operations keeps only their shared statuses and would lose this 403.
export function eitherRead(
  result: Result<"listProjectMembers"> | Result<"listMyProjects">,
) {
  if (result.kind === "server" && result.response.status === 403) {
    const operation: "memberships.list" = result.response.body.operation;
    const code: "memberships.forbidden" = result.response.body.code;
    return [operation, code];
  }
}

export function crossedReads(
  response: Known<"listMyProjects">,
): Known<"listProjectMembers"> {
  // @ts-expect-error An own-project page is not a member page.
  return response;
}

export function readAsMutation(
  response: Known<"listProjectMembers">,
): Known<"removeMember"> {
  // @ts-expect-error A read response is not a mutation response.
  return response;
}

// Read parameters: the path's project ID only where the path has one, and no
// undeclared query parameters. Never called.
export function readParams() {
  // @ts-expect-error A project's members need its ID.
  read("listProjectMembers", {});
  // @ts-expect-error The caller's own projects take no project ID.
  read("listMyProjects", { project_id: "41" });
  // @ts-expect-error Undeclared query parameters are not sent.
  read("listMyProjects", { page: "2" });
  return read("listProjectMembers", {
    project_id: "41",
    limit: "1",
    cursor: "c1.11",
  });
}

// A caller holding either read must narrow it before reading.
export function eitherReadParams(op: Read) {
  // @ts-expect-error An unnarrowed read matches neither signature,
  read(op, {});
  // @ts-expect-error with or without a project ID.
  read(op, { project_id: "41" });
  if (op === "listProjectMembers") {
    const members: Promise<Result<"listProjectMembers">> = read(op, {
      project_id: "41",
    });
    return members;
  }
  const mine: Promise<Result<"listMyProjects">> = read(op, {});
  return mine;
}

// Only mutations declare recovery; reads are new observations. Never called.
export function mutationRecovery() {
  const api = bundled();
  // @ts-expect-error A read declares no recovery.
  api.recovery("listProjectMembers");
  const read: false | CurrentStateRead = api.recovery("removeMember").read;
  return read;
}
