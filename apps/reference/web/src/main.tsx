// The reference console for checkpoint A: session bootstrap, then a role
// change or removal by ID (reads arrive in checkpoint B).
import { StrictMode, useEffect, useRef, useState } from "react";
import type { FormEvent } from "react";
import { createRoot } from "react-dom/client";
import type { Operation } from "./client.ts";
import type { components } from "./generated.ts";
import { api, send } from "./membership.ts";
import { present } from "./present.ts";
import type { Presentation } from "./present.ts";
import { logout, refreshSession, startLogin } from "./session.ts";
import type { SessionInfo } from "./session.ts";
import "./style.css";

type Role = components["schemas"]["Role"];
type Outcome = Presentation & {
  label: string;
  attempt?: string;
  body?: unknown;
};

const TONE = {
  success: "success",
  rejected: "error",
  refused: "error",
  unconfirmed: "unconfirmed",
};

function App() {
  const [session, setSession] = useState<SessionInfo | null>(null);
  const [authBusy, setAuthBusy] = useState(true);
  const [authError, setAuthError] = useState("");
  useEffect(() => {
    let active = true;
    refreshSession()
      .then((s) => active && setSession(s))
      .catch(
        () =>
          active && setAuthError("Session unavailable. Refresh and try again."),
      )
      .finally(() => active && setAuthBusy(false));
    return () => {
      active = false;
    };
  }, []);
  const [operation, setOperation] = useState<Operation>("changeMemberRole");
  const [project, setProject] = useState("41");
  const [member, setMember] = useState("29");
  const [role, setRole] = useState<Role>("viewer");
  const [confirmed, setConfirmed] = useState(false);
  const [pending, setPending] = useState(false);
  const inFlight = useRef(false);
  const [outcome, setOutcome] = useState<Outcome | null>(null);
  const reset = () => {
    setOutcome(null);
    setConfirmed(false);
  };

  async function authenticate(action: "login" | "logout" | "refresh") {
    // No session change while a domain request is in flight.
    if (authBusy || inFlight.current) return;
    setAuthBusy(true);
    setAuthError("");
    reset();
    try {
      if (action === "login") {
        setSession(await refreshSession());
        window.location.assign(await startLogin());
        return;
      }
      if (action === "logout") await logout();
      setSession(await refreshSession());
    } catch (error) {
      setAuthError(
        error instanceof Error
          ? error.message
          : "Authentication request failed.",
      );
    } finally {
      setAuthBusy(false);
    }
  }

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (
      inFlight.current ||
      authBusy ||
      (operation === "removeMember" && !confirmed)
    )
      return;
    inFlight.current = true;
    setPending(true);
    setOutcome(null);
    try {
      const result =
        operation === "changeMemberRole"
          ? await send(operation, {
              project_id: project,
              user_id: member,
              role,
            })
          : await send(operation, { project_id: project, user_id: member });
      const presentation = present(operation, result);
      setOutcome(
        result.kind === "server"
          ? {
              ...presentation,
              label: String(result.response.status),
              body: result.response.body,
            }
          : {
              ...presentation,
              label: "Unconfirmed",
              attempt: `${result.reason} · local attempt ${result.local_attempt_id}`,
            },
      );
      setConfirmed(false);
    } finally {
      inFlight.current = false;
      setPending(false);
    }
  }

  const signedIn = Boolean(session?.user_id);
  return (
    <main>
      <header>
        <div className="brand">
          <span className="mark" aria-hidden="true">
            i
          </span>
          <strong>Iris</strong>
          <span className="tag">REFERENCE / A</span>
        </div>
      </header>

      <section className="intro">
        <p className="eyebrow">RUST → OPENAPI → RUNTIME VALIDATION → REACT</p>
        <h1>Manage project members.</h1>
        <p className="lede">
          Only owners can change roles or remove members. Every project keeps
          its last owner.
        </p>
      </section>

      <aside className="notice">
        <span aria-hidden="true">◈</span>
        <div>
          <strong>Local OIDC experiment — test identities only</strong>
          <p>
            Real OIDC token validation and browser sessions; the local issuer
            does not verify human identity. Disposable data. Not a production
            login service.
          </p>
        </div>
      </aside>

      <section className="panel auth-panel" aria-busy={authBusy}>
        <div aria-live="polite">
          <h2>
            {authBusy
              ? "Checking session…"
              : signedIn
                ? `Signed in as user ${session!.user_id}`
                : "Not signed in"}
          </h2>
          <p className="help">Signing out affects this session only.</p>
          {authError && <p role="alert">{authError}</p>}
        </div>
        <div className="examples">
          <button
            type="button"
            disabled={authBusy || pending || !session}
            onClick={() => authenticate(signedIn ? "logout" : "login")}
          >
            {signedIn ? "Sign out" : "Sign in with test provider"}
          </button>
          <button
            type="button"
            disabled={authBusy || pending}
            onClick={() => authenticate("refresh")}
          >
            Refresh session
          </button>
        </div>
      </section>

      <div className="lab">
        <section className="panel">
          <div className="panel-heading">
            <span className="step">01</span>
            <h2>Send a request</h2>
          </div>
          <form onSubmit={submit}>
            <fieldset disabled={pending || authBusy || !signedIn}>
              <label htmlFor="operation">Action</label>
              <select
                id="operation"
                value={operation}
                onChange={(e) => {
                  setOperation(e.target.value as Operation);
                  reset();
                }}
              >
                <option value="changeMemberRole">Change a member’s role</option>
                <option value="removeMember">Remove a member</option>
              </select>
              <label htmlFor="project">Project ID</label>
              <input
                id="project"
                value={project}
                onChange={(e) => {
                  setProject(e.target.value);
                  reset();
                }}
              />
              <label htmlFor="member">Member user ID</label>
              <input
                id="member"
                value={member}
                onChange={(e) => {
                  setMember(e.target.value);
                  reset();
                }}
              />
              <p className="help">
                Initially, Alice (11) owns project 41, where Bob (29) is an
                editor, and Bob owns project 43.
              </p>
              {operation === "changeMemberRole" ? (
                <>
                  <label htmlFor="role">New role</label>
                  <select
                    id="role"
                    value={role}
                    onChange={(e) => {
                      setRole(e.target.value as Role);
                      setOutcome(null);
                    }}
                  >
                    <option value="viewer">Viewer</option>
                    <option value="editor">Editor</option>
                    <option value="owner">Owner</option>
                  </select>
                </>
              ) : (
                <>
                  <p className="help">
                    Removes this project membership, not the account. The last
                    owner cannot leave.
                  </p>
                  <label>
                    <input
                      type="checkbox"
                      checked={confirmed}
                      onChange={(e) => setConfirmed(e.target.checked)}
                    />{" "}
                    Confirm removal of user {member} from project {project}
                  </label>
                </>
              )}
              <button
                className="submit"
                type="submit"
                disabled={operation === "removeMember" && !confirmed}
              >
                {pending
                  ? "Sending request…"
                  : operation === "changeMemberRole"
                    ? "Change member role"
                    : "Remove member"}
                <span aria-hidden="true">→</span>
              </button>
            </fieldset>
          </form>
          <p className="footnote">
            Each submission is one attempt; nothing is retried automatically.
            Restarting the server resets the data.
          </p>
        </section>

        <section className="panel response-panel" aria-busy={pending}>
          <div className="panel-heading">
            <span className="step">02</span>
            <h2>Inspect the response</h2>
          </div>
          <div className="endpoint">
            <b>POST</b>
            <code>{api.path(operation)}</code>
          </div>
          <div aria-live="polite" aria-atomic="true" className="result">
            {pending ? (
              <div className="empty">
                <span className="pulse" aria-hidden="true" />
                <h3>Waiting for the API</h3>
              </div>
            ) : !outcome ? (
              <div className="empty">
                <span className="empty-icon" aria-hidden="true">
                  ↳
                </span>
                <h3>Ready when you are</h3>
                <p>Submit a request to see its validated response.</p>
              </div>
            ) : (
              <>
                <div className={`status ${TONE[outcome.tone]}`}>
                  <strong>
                    {outcome.label} · {outcome.title}
                  </strong>
                  <p>{outcome.detail}</p>
                  {outcome.attempt && (
                    <p className="attempt">{outcome.attempt}</p>
                  )}
                </div>
                {outcome.body !== undefined && (
                  <>
                    <p className="json-label">
                      RESPONSE BODY <span>validated against the export</span>
                    </p>
                    <pre>{JSON.stringify(outcome.body, null, 2)}</pre>
                  </>
                )}
              </>
            )}
          </div>
          <div className="contract-note">
            <span className="dot" /> Generated types · runtime validation · one
            attempt
          </div>
        </section>
      </div>
      <footer>
        <span>Iris / Reference application</span>
        <span>SQLite + Axum + utoipa</span>
      </footer>
    </main>
  );
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
