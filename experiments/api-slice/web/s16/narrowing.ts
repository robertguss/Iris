import type { Known } from "./client.ts";

export function handle(response: Known) {
  if (response.status === 200) {
    const completion: "acknowledged" = response.body.data.completion;
    // @ts-expect-error Success does not claim a stored role.
    response.body.data.role;
    return completion;
  }
  if (response.status === 403) {
    const code: "http.csrf_refused" | "memberships.forbidden" = response.body.code;
    if (response.body.code === "http.csrf_refused") {
      const kind: "refused" = response.body.kind;
      return kind;
    }
    const kind: "rejected" = response.body.kind;
    return [code, kind];
  }
  if (response.status === 409) {
    const code: "memberships.last_owner" = response.body.code;
    // @ts-expect-error Unrelated codes cannot occur at 409.
    const forbidden: "memberships.forbidden" = response.body.code;
    return [code, forbidden];
  }
  return response.body.kind;
}
