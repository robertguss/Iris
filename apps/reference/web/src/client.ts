// The whole-request boundary for the reference application's domain
// operations, generalized from S16. It owns request execution and body
// reading, and validates each response against the exported document. Session
// endpoints keep their own contracts and do not pass through it.
import Ajv2020 from "ajv/dist/2020.js";
import type { ValidateFunction } from "ajv/dist/2020.js";
import type { operations } from "./generated.ts";

/** Every operation the export marks with `x-iris`, and nothing else. */
export const DOMAIN_OPERATIONS = [
  "changeMemberRole",
  "removeMember",
  "listProjectMembers",
  "listMyProjects",
] as const;
export type Operation = (typeof DOMAIN_OPERATIONS)[number];
/** The operations that change state; reads have no request body. */
export const MUTATIONS = ["changeMemberRole", "removeMember"] as const;
export type Mutation = (typeof MUTATIONS)[number] & Operation;
/** The operations that read present state. */
export const READS = ["listProjectMembers", "listMyProjects"] as const;
export type Read = (typeof READS)[number] & Operation;

/** Each operation's method, written by hand and checked against the export. */
const METHODS: Record<Operation, "get" | "post"> = {
  changeMemberRole: "post",
  removeMember: "post",
  listProjectMembers: "get",
  listMyProjects: "get",
};

/**
 * A read a mutation declares for after an unresolved attempt (S15). A page it
 * returns describes present state when read, under the caller's current
 * authorization: it neither confirms nor rules out the attempt, and it
 * authorizes no new submission.
 */
export type CurrentStateRead = {
  operation_id: Read;
  /** Each of the read's path parameters, with the request-body field that supplies it. */
  path_inputs: Record<string, { request_body_field: string }>;
};
/** A mutation's declared recovery. The client supports no inspection or replay. */
export type Recovery = {
  inspect: false;
  read: false | CurrentStateRead;
  replay: false;
  new_submission: string;
};

/** Response bodies are read up to this many bytes; declared bodies are far smaller. */
export const BODY_LIMIT = 64 * 1024;

type Json<R> = R extends { content: { "application/json": infer Body } }
  ? Body
  : never;
type Responses<Op extends Operation> = operations[Op]["responses"];
/** A status and body the exported document declares for the operation. */
export type Known<Op extends Operation> = {
  [S in keyof Responses<Op>]: { status: S; body: Json<Responses<Op>[S]> };
}[keyof Responses<Op>];

export type Reason =
  | "request_failed"
  | "body_unreadable"
  | "body_oversize"
  | "non_json"
  | "unsupported_version"
  | "contract_mismatch";
/** No action-effect conclusion follows from this; it is never retried. */
export type ClientUnknown = {
  kind: "client_unknown";
  local_attempt_id: string;
  reason: Reason;
};
export type Result<Op extends Operation> =
  { kind: "server"; response: Known<Op> } | ClientUnknown;

// Session responses may have no body (a 303 or 204); only domain operations
// must declare a JSON schema for every status, which `client` checks.
type Declared = {
  operationId: string;
  responses: Record<
    string,
    {
      content?: { "application/json"?: { schema?: object } };
      [field: string]: unknown;
    }
  >;
  requestBody?: {
    content?: { "application/json"?: { schema?: { $ref?: string } } };
  };
  "x-iris"?: {
    schema_version: number;
    /** Absent for reads, which declare no recovery capabilities. */
    recovery?: unknown;
    prerequisites?: Record<string, string>;
  };
};
/** The exported document, not an arbitrary remote one. */
export type Document = {
  components: {
    schemas?: Record<string, { required?: string[]; [field: string]: unknown }>;
  };
  paths: Record<string, Record<string, Declared>>;
};

type Entry = {
  path: string;
  version: number;
  /** Absent for reads. */
  recovery: Recovery | undefined;
  prerequisites: Record<string, string>;
  validators: Map<number, ValidateFunction>;
};

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null && !Array.isArray(value);
const hasKeys = (value: Record<string, unknown>, keys: string[]) =>
  Object.keys(value).sort().join() === [...keys].sort().join();

// Assembly checks that a declared read is compatible with its mutation. This
// checks only the shape the client relies on: an exported read, bound path
// parameters, and required request-body fields. Which field feeds which
// parameter is pinned by hand-written tests, not here.
function parseRecovery(
  document: Document,
  paths: Map<string, string>,
  operation: Declared,
): Recovery {
  const mismatch = new Error("Reference recovery mismatch");
  const recovery = operation["x-iris"]!.recovery;
  if (
    !isRecord(recovery) ||
    !hasKeys(recovery, ["inspect", "read", "replay", "new_submission"]) ||
    recovery.inspect !== false ||
    recovery.replay !== false ||
    typeof recovery.new_submission !== "string"
  )
    throw mismatch;
  const { read, new_submission } = recovery;
  if (read === false)
    return { inspect: false, read: false, replay: false, new_submission };
  if (
    !isRecord(read) ||
    !hasKeys(read, ["operation_id", "path_inputs"]) ||
    !READS.some((op) => op === read.operation_id) ||
    !isRecord(read.path_inputs)
  )
    throw mismatch;
  const target = read.operation_id as Read;
  const parameters = [...paths.get(target)!.matchAll(/\{([^}]+)\}/g)].map(
    ([, name]) => name,
  );
  // Only a reference to a named component schema is resolved.
  const [, component] =
    operation.requestBody?.content?.["application/json"]?.schema?.$ref?.match(
      /^#\/components\/schemas\/([^/]+)$/,
    ) ?? [];
  const body =
    component === undefined
      ? undefined
      : document.components.schemas?.[component];
  const path_inputs: CurrentStateRead["path_inputs"] = {};
  for (const [parameter, input] of Object.entries(read.path_inputs)) {
    if (
      !isRecord(input) ||
      !hasKeys(input, ["request_body_field"]) ||
      typeof input.request_body_field !== "string" ||
      !body?.required?.includes(input.request_body_field)
    )
      throw mismatch;
    path_inputs[parameter] = { request_body_field: input.request_body_field };
  }
  if (!hasKeys(path_inputs, parameters)) throw mismatch;
  return {
    inspect: false,
    read: { operation_id: target, path_inputs },
    replay: false,
    new_submission,
  };
}

// No schema fetches, payload coercion, default insertion, property removal,
// retries or raw diagnostics.
export function client(document: Document) {
  const declared = Object.entries(document.paths).flatMap(([path, methods]) =>
    Object.entries(methods).flatMap(([method, operation]) =>
      operation["x-iris"] ? [{ path, method, operation }] : [],
    ),
  );
  const ids = declared.map(({ operation }) => operation.operationId).sort();
  if (
    ids.join() !== [...DOMAIN_OPERATIONS].sort().join() ||
    declared.some(
      ({ method, operation }) =>
        METHODS[operation.operationId as Operation] !== method,
    )
  )
    throw new Error("Reference operation linkage mismatch");
  const schema = (response: Declared["responses"][string]) => {
    const declared = response.content?.["application/json"]?.schema;
    if (!declared) throw new Error("Reference operation lacks a JSON schema");
    return declared;
  };
  const ajv = new Ajv2020({
    strict: true,
    allErrors: true,
    validateFormats: false,
  });
  // A JSON Schema resource with the same local pointer layout as OpenAPI.
  // Register only schemas; OpenAPI itself is not a JSON Schema document.
  ajv.addKeyword({ keyword: "components", valid: true });
  const paths = new Map(
    declared.map(({ path, operation }) => [operation.operationId, path]),
  );
  const recovery = (operation: Declared) => {
    if (MUTATIONS.some((op) => op === operation.operationId))
      return parseRecovery(document, paths, operation);
    // Reads are new observations: they declare no recovery.
    if (operation["x-iris"]!.recovery !== undefined)
      throw new Error("Reference recovery mismatch");
    return undefined;
  };
  const entries = new Map<string, Entry>(
    declared.map(({ path, operation }) => [
      operation.operationId,
      {
        path,
        version: operation["x-iris"]!.schema_version,
        recovery: recovery(operation),
        prerequisites: operation["x-iris"]!.prerequisites ?? {},
        validators: new Map(
          Object.entries(operation.responses).map(([status, response]) => [
            Number(status),
            ajv.compile({
              ...schema(response),
              components: document.components,
            }),
          ]),
        ),
      },
    ]),
  );
  const entry = (op: Operation) => entries.get(op)!;
  return {
    path: (op: Operation) => entry(op).path,
    recovery: (op: Mutation) => entry(op).recovery!,
    prerequisites: (op: Operation) => entry(op).prerequisites,
    /** Calls `request` exactly once and classifies whatever comes back. */
    async execute<Op extends Operation>(
      op: Op,
      request: () => Promise<Response>,
    ): Promise<Result<Op>> {
      const { version, validators } = entry(op);
      const local_attempt_id = crypto.randomUUID();
      const unknown = (reason: Reason): ClientUnknown => ({
        kind: "client_unknown",
        local_attempt_id,
        reason,
      });
      let response: Response;
      try {
        response = await request();
      } catch {
        return unknown("request_failed");
      }
      const text = await readBounded(response);
      if (typeof text !== "string") return unknown(text.reason);
      let value: unknown;
      try {
        value = JSON.parse(text);
      } catch {
        return unknown("non_json");
      }
      if (
        typeof value === "object" &&
        value !== null &&
        "schema_version" in value &&
        value.schema_version !== version
      )
        return unknown("unsupported_version");
      const validate = validators.get(response.status);
      if (!validate || !validate(value)) return unknown("contract_mismatch");
      // The status-indexed schema validated the complete tuple. TS cannot
      // express this runtime Map's dependent status/body relationship.
      return {
        kind: "server",
        response: { status: response.status, body: value } as Known<Op>,
      };
    },
  };
}

/** Reads at most `BODY_LIMIT` bytes as strict UTF-8, cancelling the rest. */
async function readBounded(
  response: Response,
): Promise<string | { reason: Reason }> {
  const body = response.body;
  // A consumed, partly read or locked body is no longer the response as sent.
  if (response.bodyUsed || body?.locked) return { reason: "body_unreadable" };
  if (body === null) return "";
  // Cancellation is not awaited: an oversize body is abandoned, not drained.
  if (Number(response.headers.get("content-length")) > BODY_LIMIT) {
    void body.cancel().catch(() => {});
    return { reason: "body_oversize" };
  }
  const chunks: Uint8Array[] = [];
  let size = 0;
  try {
    const reader = body.getReader();
    for (;;) {
      const { done, value } = await reader.read();
      if (done) break;
      if (!(value instanceof Uint8Array)) return { reason: "body_unreadable" };
      size += value.byteLength;
      if (size > BODY_LIMIT) {
        void reader.cancel().catch(() => {});
        return { reason: "body_oversize" };
      }
      chunks.push(value);
    }
  } catch {
    return { reason: "body_unreadable" };
  }
  const bytes = new Uint8Array(size);
  let offset = 0;
  for (const chunk of chunks) {
    bytes.set(chunk, offset);
    offset += chunk.byteLength;
  }
  try {
    return new TextDecoder("utf-8", { fatal: true }).decode(bytes);
  } catch {
    return { reason: "non_json" };
  }
}
