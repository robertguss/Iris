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

/** Each operation's method, written by hand and checked against the export. */
const METHODS: Record<Operation, "get" | "post"> = {
  changeMemberRole: "post",
  removeMember: "post",
  listProjectMembers: "get",
  listMyProjects: "get",
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
  "x-iris"?: {
    schema_version: number;
    /** Absent for reads, which declare no recovery capabilities. */
    recovery?: object;
    prerequisites?: Record<string, string>;
  };
};
/** The exported document, not an arbitrary remote one. */
export type Document = {
  components: object;
  paths: Record<string, Record<string, Declared>>;
};

type Entry = {
  path: string;
  version: number;
  recovery: object | undefined;
  prerequisites: Record<string, string>;
  validators: Map<number, ValidateFunction>;
};

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
  const entries = new Map<string, Entry>(
    declared.map(({ path, operation }) => [
      operation.operationId,
      {
        path,
        version: operation["x-iris"]!.schema_version,
        recovery: operation["x-iris"]!.recovery,
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
    recovery: (op: Operation) => entry(op).recovery,
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
