// Domain operations: one attempt each through the whole-request boundary,
// validated against the export bundled at build time.
import snapshot from "../../openapi.json" with { type: "json" };
import { client } from "./client.ts";
import type { Mutation, Read, Recovery, Result } from "./client.ts";
import type { operations } from "./generated.ts";
import { csrfToken } from "./session.ts";

// Validator compilation is the boundary's startup cost; the browser workflow
// reads this measure.
performance.mark("iris:client-compile-start");
export const api = client(snapshot);
performance.measure("iris:client-compile", "iris:client-compile-start");

export type RequestBody<Op extends Mutation> =
  operations[Op]["requestBody"]["content"]["application/json"];

export function send<Op extends Mutation>(
  op: Op,
  body: RequestBody<Op>,
): Promise<Result<Op>> {
  return api.execute(op, () =>
    fetch(api.path(op), {
      method: "POST",
      credentials: "same-origin",
      headers: {
        "content-type": "application/json",
        "x-iris-csrf": csrfToken(),
      },
      body: JSON.stringify(body),
    }),
  );
}

type Parameters<Op extends Read> = operations[Op]["parameters"];
/** Query parameters, plus the path's project ID for a project's members. */
export type ReadParams<Op extends Read> = NonNullable<Parameters<Op>["query"]> &
  (Parameters<Op> extends { path: infer Path } ? Path : unknown);

/**
 * One GET without a body or CSRF header. The server validates the values;
 * `cursor` is an earlier page's `next_cursor`, sent exactly as received. One
 * signature per read, so a caller holding either read must narrow it first:
 * a union would drop the members' path parameter and their 403.
 */
export function read(
  op: "listProjectMembers",
  params: ReadParams<"listProjectMembers">,
): Promise<Result<"listProjectMembers">>;
export function read(
  op: "listMyProjects",
  params: ReadParams<"listMyProjects">,
): Promise<Result<"listMyProjects">>;
export function read(
  op: Read,
  { project_id, limit, cursor }: { project_id?: string } & ReadParams<Read>,
): Promise<Result<"listProjectMembers"> | Result<"listMyProjects">> {
  const path =
    project_id === undefined
      ? api.path(op)
      : api.path(op).replace("{project_id}", encodeURIComponent(project_id));
  const query = new URLSearchParams();
  if (limit !== undefined) query.set("limit", limit);
  if (cursor !== undefined) query.set("cursor", cursor);
  const search = query.toString();
  const request = () =>
    fetch(search ? `${path}?${search}` : path, {
      method: "GET",
      credentials: "same-origin",
    });
  // Each branch keeps its own result type; `execute` over the union would not.
  return op === "listProjectMembers"
    ? api.execute(op, request)
    : api.execute(op, request);
}

/**
 * The declared current-state read for an attempt, with each path input taken
 * from the attempt's own request body through its binding; null when no read
 * is declared. Sends nothing, and says nothing about the attempt.
 */
export function readback(
  recovery: Recovery,
  body: Record<string, string>,
): { op: Read; path: Record<string, string> } | null {
  if (!recovery.read) return null;
  const path: Record<string, string> = {};
  for (const [parameter, { request_body_field }] of Object.entries(
    recovery.read.path_inputs,
  ))
    path[parameter] = body[request_body_field];
  return { op: recovery.read.operation_id, path };
}
