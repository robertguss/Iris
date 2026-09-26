import Ajv2020 from "ajv/dist/2020.js";
import type { operations } from "./generated.ts";

type Responses = operations["changeMemberRole"]["responses"];
export type Known = {
  [S in keyof Responses]: { status: S; body: Responses[S]["content"]["application/json"] }
}[keyof Responses];
export type Result = { kind: "server"; response: Known } | {
  kind: "client_unknown";
  local_attempt_id: string;
  reason: "request_failed" | "body_unreadable" | "non_json" | "unsupported_version" | "contract_mismatch";
};

// This accepts the exported document, not arbitrary remote schemas. No schema
// fetches, payload coercion, default insertion, removal, retries or raw diagnostics.
export function client(document: {
  components: object;
  paths: Record<string, { post?: { operationId: string; responses: Record<string, { content: { "application/json": { schema: object } } }>; "x-iris": { operation: string; schema_version: number; recovery: object } } }>;
}) {
  const operations = Object.values(document.paths).flatMap(p => p.post ? [p.post] : []);
  if (operations.length !== 1 || operations[0].operationId !== "changeMemberRole") throw new Error("S16 operation linkage mismatch");
  const operation = operations[0];
  const ajv = new Ajv2020({ strict: true, allErrors: true, validateFormats: false });
  // A JSON Schema resource with the same local pointer layout as OpenAPI.
  // Register only schemas; OpenAPI itself is not a JSON Schema document.
  ajv.addKeyword({ keyword: "components", valid: true });
  const validators = new Map(Object.entries(operation.responses).map(([status, response]) => [Number(status), ajv.compile<Known["body"]>({ ...response.content["application/json"].schema, components: document.components })]));
  return {
    recovery: operation["x-iris"].recovery,
    async execute(request: () => Promise<Response>): Promise<Result> {
      const local_attempt_id = crypto.randomUUID();
      const unknown = (reason: Extract<Result, { kind: "client_unknown" }>["reason"]): Result => ({ kind: "client_unknown", local_attempt_id, reason });
      let response: Response;
      try { response = await request(); } catch { return unknown("request_failed"); }
      let text: string;
      try { text = await response.text(); } catch { return unknown("body_unreadable"); }
      let value: unknown;
      try { value = JSON.parse(text); } catch { return unknown("non_json"); }
      if (typeof value === "object" && value !== null && "schema_version" in value && value.schema_version !== operation["x-iris"].schema_version) return unknown("unsupported_version");
      const validate = validators.get(response.status);
      if (!validate || !validate(value)) return unknown("contract_mismatch");
      // The status-indexed schema validated the complete tuple. TS cannot
      // express this runtime Map's dependent status/body relationship.
      return { kind: "server", response: { status: response.status, body: value } as Known };
    },
  };
}
