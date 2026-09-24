import createClient from "openapi-fetch";
import type { paths as UtoipaPaths, components as UtoipaComponents } from "../src/generated/utoipa";
import type { paths as AidePaths, components as AideComponents } from "../src/generated/aide";

// Never executed. Typecheck both generated clients, not only the UI's selection.
async function contracts() {
  const clients = [createClient<UtoipaPaths>(), createClient<AidePaths>()];
  for (const client of clients) {
    const { data, error } = await client.POST("/api/invitations/accept", { body: { token: "example" } });
    if (data) {
      const project: string = data.project_id;
      const user: string = data.user_id;
      void [project, user];
    }
    if (error) {
      const code: UtoipaComponents["schemas"]["ErrorCode"] = error.code;
      const other: AideComponents["schemas"]["ErrorCode"] = code;
      void other;
    }
    // Known limitation: generic fetch accepts extra fields. Runtime rejects this.
    await client.POST("/api/invitations/accept", { body: { token: "example", user_id: "11" } });
    // Explicit DTO annotations check fresh object literals for both generators.
    // @ts-expect-error Identity must not be a body field.
    const utoipaBody: UtoipaComponents["schemas"]["AcceptRequest"] = { token: "example", user_id: "11" };
    // @ts-expect-error Identity must not be a body field.
    const aideBody: AideComponents["schemas"]["AcceptRequest"] = { token: "example", user_id: "11" };
    void [utoipaBody, aideBody];
    // @ts-expect-error This route is POST-only.
    await client.GET("/api/invitations/accept");
    // @ts-expect-error Unknown endpoint.
    await client.POST("/api/invitations/delete", { body: { token: "example" } });
    // @ts-expect-error Required token missing.
    await client.POST("/api/invitations/accept", { body: {} });
  }
}
void contracts;
