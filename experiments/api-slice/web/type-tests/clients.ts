import createClient from "openapi-fetch";
import type { paths as UtoipaPaths, components as UtoipaComponents } from "../src/generated/utoipa";
import type { paths as AidePaths, components as AideComponents } from "../src/generated/aide";

// Never executed. Typecheck both generated clients, not only the UI's selection.
async function contracts() {
  const clients = [createClient<UtoipaPaths>(), createClient<AidePaths>()];
  for (const client of clients) {
    const changed = await client.POST("/api/memberships/role", { body: { project_id: "41", user_id: "29", role: "owner" } });
    await client.POST("/api/memberships/remove", { body: { project_id: "41", user_id: "29" } });
    if (changed.data) {
      const user: string = changed.data.user_id;
      const role: "owner" | "editor" | "viewer" | null | undefined = changed.data.role;
      void [user, role];
    }
    // @ts-expect-error Role must be one of the supported roles.
    await client.POST("/api/memberships/role", { body: { project_id: "41", user_id: "29", role: "admin" } });
    // @ts-expect-error A role is required, not an implicit removal.
    await client.POST("/api/memberships/role", { body: { project_id: "41", user_id: "29" } });
    // @ts-expect-error Member IDs stay strings.
    await client.POST("/api/memberships/remove", { body: { project_id: "41", user_id: 29 } });
    // @ts-expect-error Removal cannot carry a role.
    const removal: AideComponents["schemas"]["RemoveMemberRequest"] = { project_id: "41", user_id: "29", role: "owner" };
    void removal;
    const issued = await client.POST("/api/invitations", { body: { project_id: "41", recipient_id: "29" } });
    if (issued.data) {
      const token: string = issued.data.token;
      const recipient: string = issued.data.recipient_id;
      void [token, recipient];
    }
    // @ts-expect-error Recipient is required.
    await client.POST("/api/invitations", { body: { project_id: "41" } });
    // @ts-expect-error IDs are strings, not JavaScript numbers.
    await client.POST("/api/invitations", { body: { project_id: 41, recipient_id: "29" } });
    // @ts-expect-error Tokens are server-generated, never input.
    const issueBody: UtoipaComponents["schemas"]["IssueRequest"] = { project_id: "41", recipient_id: "29", token: "chosen" };
    void issueBody;
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
