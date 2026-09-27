// Domain operations: one attempt each through the whole-request boundary,
// validated against the export bundled at build time.
import snapshot from "../../openapi.json" with { type: "json" };
import { client } from "./client.ts";
import type { Operation, Result } from "./client.ts";
import type { operations } from "./generated.ts";
import { csrfToken } from "./session.ts";

// Validator compilation is the boundary's startup cost; the browser workflow
// reads this measure.
performance.mark("iris:client-compile-start");
export const api = client(snapshot);
performance.measure("iris:client-compile", "iris:client-compile-start");

export type RequestBody<Op extends Operation> =
  operations[Op]["requestBody"]["content"]["application/json"];

export function send<Op extends Operation>(
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
