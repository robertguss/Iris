// Session bootstrap over the session endpoints' existing contracts. They keep
// their `{code,message}` bodies and do not pass through the domain boundary.
import type { components } from "./generated.ts";

export type SessionInfo = components["schemas"]["SessionInfo"];
type LoginInfo = components["schemas"]["LoginInfo"];

let csrf = "";
let bootstrap: Promise<SessionInfo> | undefined;

/** The latest CSRF token; every unsafe request sends it. */
export const csrfToken = () => csrf;

export function refreshSession(): Promise<SessionInfo> {
  // StrictMode and simultaneous consumers share the initial cookie bootstrap.
  return (bootstrap ??= fetch("/api/auth/session", {
    credentials: "same-origin",
  })
    .then(async (response) => {
      if (!response.ok) throw new Error("Could not read the session.");
      const session = (await response.json()) as SessionInfo;
      csrf = session.csrf_token;
      return session;
    })
    .finally(() => {
      bootstrap = undefined;
    }));
}

const post = (path: string) =>
  fetch(path, {
    method: "POST",
    credentials: "same-origin",
    headers: { "x-iris-csrf": csrf },
  });

/** Returns the issuer URL to navigate to; refresh the session first. */
export async function startLogin(): Promise<string> {
  const response = await post("/api/auth/login");
  if (!response.ok)
    throw new Error(
      "Could not start login. Refresh the session and try again.",
    );
  return ((await response.json()) as LoginInfo).authorization_url;
}

/** Logout answers 204 with no body. */
export async function logout(): Promise<void> {
  const response = await post("/api/auth/logout");
  if (!response.ok)
    throw new Error(
      "Logout was not confirmed. Refresh the session before trying again.",
    );
}
