// The reference console for checkpoint B: session bootstrap, the caller's
// projects and a project's members read page by page, and a role change or
// removal started from a listed member.
import { StrictMode, useEffect, useRef, useState } from "react";
import type { FormEvent, ReactNode } from "react";
import { createRoot } from "react-dom/client";
import type { Mutation } from "./client.ts";
import {
  PAGE_SIZES,
  attempted,
  initial,
  next,
  openProject,
  pageSize,
  received,
  reload,
  selectMember,
  signedIn,
  signedOut,
} from "./directory.ts";
import type {
  Directory,
  List,
  ListName,
  PageSize,
  Request,
  Step,
} from "./directory.ts";
import type { components } from "./generated.ts";
import { api, read, readback, send } from "./membership.ts";
import type { RequestBody } from "./membership.ts";
import { present, presentRead } from "./present.ts";
import type { Presentation, Row } from "./present.ts";
import { logout, refreshSession, startLogin } from "./session.ts";
import type { SessionInfo } from "./session.ts";
import "./style.css";

type Role = components["schemas"]["Role"];
/**
 * One attempt, recorded when sent. Its operation, request body and target
 * never follow the form afterwards; only the next attempt or a session change
 * replaces it. A read, including its declared readback, never changes it.
 */
type Attempt = (
  | { op: "changeMemberRole"; body: RequestBody<"changeMemberRole"> }
  | { op: "removeMember"; body: RequestBody<"removeMember"> }
) & {
  project: { id: string; name: string };
  member: { id: string; name: string };
  outcome?: Presentation & { label: string; attempt?: string; body?: unknown };
};

const TONE = {
  success: "success",
  rejected: "error",
  refused: "error",
  unconfirmed: "unconfirmed",
  unavailable: "unconfirmed",
};

/** One list's page: its read status, rows, and forward paging controls. */
function Page(props: {
  list: List;
  noun: string;
  disabled: boolean;
  onNext: () => void;
  onReload: () => void;
  row: (row: Row) => ReactNode;
}) {
  const { list, noun, disabled } = props;
  const shown = list.shown;
  const loading = list.pending !== null;
  return (
    <div className="listing" aria-busy={loading}>
      <div aria-live="polite">
        {loading ? (
          <p className="read-status">Loading {noun}…</p>
        ) : shown?.tone === "listed" ? (
          <p className="read-status">
            Page {list.page} · {shown.detail}
          </p>
        ) : shown ? (
          <div className={`status ${TONE[shown.tone]}`}>
            <strong>{shown.title}</strong>
            <p>{shown.detail}</p>
          </div>
        ) : null}
      </div>
      {!loading && shown?.tone === "listed" && shown.rows.length > 0 && (
        <ul className="rows">
          {shown.rows.map((row) => (
            <li key={row.id}>{props.row(row)}</li>
          ))}
        </ul>
      )}
      {!loading && list.stale && shown?.tone === "listed" && (
        <p className="stale">
          This listing may predate your last attempt. Reload starts a new read.
        </p>
      )}
      <div className="page-controls">
        <button
          type="button"
          className="next"
          disabled={
            disabled ||
            loading ||
            shown?.tone !== "listed" ||
            shown.next === null
          }
          onClick={props.onNext}
        >
          Next page
        </button>
        <button
          type="button"
          className="reload"
          disabled={disabled || loading}
          onClick={props.onReload}
        >
          Reload
        </button>
      </div>
    </div>
  );
}

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

  // Every directory change goes through `apply`, which also sends the reads
  // it requests; each result returns with the token it was requested under.
  const [directory, setDirectory] = useState<Directory>(initial);
  const current = useRef(initial);
  function apply(step: Step) {
    current.current = step.state;
    setDirectory(step.state);
    for (const request of step.requests) void load(request);
  }
  async function load(request: Request) {
    const shown =
      request.list === "projects"
        ? presentRead(request.op, await read(request.op, request.params))
        : presentRead(request.op, await read(request.op, request.params));
    apply(received(current.current, request.list, request.token, shown));
  }

  const [operation, setOperation] = useState<Mutation>("changeMemberRole");
  const [role, setRole] = useState<Role>("viewer");
  // Confirmation belongs to one target and action; the outcome does not.
  const [confirmed, setConfirmed] = useState(false);
  const [pending, setPending] = useState(false);
  const inFlight = useRef(false);
  const [attempt, setAttempt] = useState<Attempt | null>(null);

  // Only a session transition reads the projects (a failed read stays shown)
  // and clears the attempt; a same-user or failed refresh keeps both.
  const user = session?.user_id ?? null;
  useEffect(() => {
    setAttempt(null);
    setConfirmed(false);
    apply(user ? signedIn(current.current) : signedOut(current.current));
  }, [user]);

  async function authenticate(action: "login" | "logout" | "refresh") {
    // No session change while a domain request is in flight.
    if (authBusy || inFlight.current) return;
    setAuthBusy(true);
    setAuthError("");
    setConfirmed(false);
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
    const { project, member } = current.current;
    if (
      !project ||
      !member ||
      inFlight.current ||
      authBusy ||
      (operation === "removeMember" && !confirmed)
    )
      return;
    inFlight.current = true;
    setPending(true);
    const target = { project_id: project.id, user_id: member.id };
    const sent: Attempt = {
      ...(operation === "changeMemberRole"
        ? { op: operation, body: { ...target, role } }
        : { op: operation, body: target }),
      project,
      member: { id: member.id, name: member.name },
    };
    setAttempt(sent);
    setConfirmed(false);
    // At send time, whatever the outcome: a shown listing may predate it.
    apply(attempted(current.current, project.id));
    try {
      const result =
        sent.op === "changeMemberRole"
          ? await send(sent.op, sent.body)
          : await send(sent.op, sent.body);
      const presentation = present(sent.op, result);
      setAttempt({
        ...sent,
        outcome:
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
      });
    } finally {
      inFlight.current = false;
      setPending(false);
    }
  }

  const signed = Boolean(session?.user_id);
  // No reads while an attempt is in flight, and none across a session change.
  const busy = pending || authBusy;
  const { project, member } = directory;
  const page = (
    name: ListName,
    list: List,
    noun: string,
    row: (row: Row) => ReactNode,
  ) => (
    <Page
      list={list}
      noun={noun}
      disabled={busy}
      onNext={() => apply(next(current.current, name))}
      onReload={() => apply(reload(current.current, name))}
      row={row}
    />
  );
  const endpoint = attempt?.op ?? operation;
  // The attempt's declared current-state read, offered only while its outcome
  // is unconfirmed: the first page of the attempt's own project, whatever the
  // directory now shows.
  const again =
    attempt?.outcome?.tone === "unconfirmed"
      ? readback(api.recovery(attempt.op), attempt.body)
      : null;
  return (
    <main>
      <header>
        <div className="brand">
          <span className="mark" aria-hidden="true">
            i
          </span>
          <strong>Iris</strong>
          <span className="tag">REFERENCE / B</span>
        </div>
      </header>

      <section className="intro">
        <p className="eyebrow">RUST → OPENAPI → RUNTIME VALIDATION → REACT</p>
        <h1>Manage project members.</h1>
        <p className="lede">
          Any member can list a project’s members. Only owners can change roles
          or remove members. Every project keeps its last owner.
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
              : signed
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
            onClick={() => authenticate(signed ? "logout" : "login")}
          >
            {signed ? "Sign out" : "Sign in with test provider"}
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
        <div className="column">
          <section className="panel" id="projects">
            <div className="panel-heading">
              <span className="step">01</span>
              <h2>Your projects</h2>
              <span className="page-size">
                <label htmlFor="page-size">Rows per page</label>
                <select
                  id="page-size"
                  value={directory.limit}
                  disabled={busy}
                  onChange={(e) =>
                    apply(pageSize(current.current, e.target.value as PageSize))
                  }
                >
                  {PAGE_SIZES.map((size) => (
                    <option key={size} value={size}>
                      {size}
                    </option>
                  ))}
                </select>
              </span>
            </div>
            {directory.projects ? (
              page("projects", directory.projects, "projects", (row) => (
                <>
                  <span>
                    <span className="row-name">{row.name}</span>
                    <span className="row-meta">
                      Project {row.id} · your role: {row.role}
                    </span>
                  </span>
                  <button
                    type="button"
                    aria-label={`Open ${row.name}`}
                    aria-pressed={project?.id === row.id}
                    disabled={busy}
                    onClick={() => {
                      apply(openProject(current.current, row.id, row.name));
                      setConfirmed(false);
                    }}
                  >
                    Open
                  </button>
                </>
              ))
            ) : (
              <p className="help">Sign in to list your projects.</p>
            )}
          </section>

          <section className="panel" id="members">
            <div className="panel-heading">
              <span className="step">02</span>
              <h2>{project ? `Members of ${project.name}` : "Members"}</h2>
            </div>
            {directory.members ? (
              page("members", directory.members, "members", (row) => (
                <>
                  <span>
                    <span className="row-name">{row.name}</span>
                    <span className="row-meta">
                      User {row.id} · {row.role}
                    </span>
                  </span>
                  <button
                    type="button"
                    aria-label={`Select ${row.name}`}
                    aria-pressed={member?.id === row.id}
                    disabled={busy}
                    onClick={() => {
                      apply(selectMember(current.current, row));
                      setConfirmed(false);
                    }}
                  >
                    Select
                  </button>
                </>
              ))
            ) : (
              <p className="help">Open a project to list its members.</p>
            )}
          </section>
        </div>

        <div className="column">
          <section className="panel" id="change">
            <div className="panel-heading">
              <span className="step">03</span>
              <h2>Change membership</h2>
            </div>
            <form onSubmit={submit}>
              <fieldset disabled={busy || !signed || !member}>
                <p className="target">
                  {member && project
                    ? `${member.name} (user ${member.id}) in ${project.name} (project ${project.id}), listed as ${member.role}.`
                    : "Select a listed member to change their role or remove them."}
                </p>
                <label htmlFor="operation">Action</label>
                <select
                  id="operation"
                  value={operation}
                  onChange={(e) => {
                    setOperation(e.target.value as Mutation);
                    setConfirmed(false);
                  }}
                >
                  <option value="changeMemberRole">
                    Change a member’s role
                  </option>
                  <option value="removeMember">Remove a member</option>
                </select>
                {operation === "changeMemberRole" ? (
                  <>
                    <label htmlFor="role">New role</label>
                    <select
                      id="role"
                      value={role}
                      onChange={(e) => setRole(e.target.value as Role)}
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
                      {member && project
                        ? `Confirm removal of ${member.name} from ${project.name}`
                        : "Confirm removal"}
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

          <section
            className="panel response-panel"
            id="outcome"
            aria-busy={pending}
          >
            <div className="panel-heading">
              <span className="step">04</span>
              <h2>Inspect the response</h2>
            </div>
            <div className="endpoint">
              <b>POST</b>
              <code>{api.path(endpoint)}</code>
            </div>
            {attempt && (
              <p className="target">
                {attempt.member.name} (user {attempt.member.id}) ·{" "}
                {attempt.project.name} (project {attempt.project.id})
              </p>
            )}
            <div aria-live="polite" aria-atomic="true" className="result">
              {pending ? (
                <div className="empty">
                  <span className="pulse" aria-hidden="true" />
                  <h3>Waiting for the API</h3>
                </div>
              ) : !attempt?.outcome ? (
                <div className="empty">
                  <span className="empty-icon" aria-hidden="true">
                    ↳
                  </span>
                  <h3>Ready when you are</h3>
                  <p>Submit a request to see its validated response.</p>
                </div>
              ) : (
                <>
                  <div className={`status ${TONE[attempt.outcome.tone]}`}>
                    <strong>
                      {attempt.outcome.label} · {attempt.outcome.title}
                    </strong>
                    <p>{attempt.outcome.detail}</p>
                    {attempt.outcome.attempt && (
                      <p className="attempt">{attempt.outcome.attempt}</p>
                    )}
                    {again?.op === "listProjectMembers" && (
                      <button
                        type="button"
                        className="readback"
                        disabled={busy || !signed}
                        onClick={() => {
                          apply(
                            openProject(
                              current.current,
                              again.path.project_id,
                              attempt.project.name,
                            ),
                          );
                          setConfirmed(false);
                        }}
                      >
                        Read members of {attempt.project.name}
                      </button>
                    )}
                  </div>
                  {attempt.outcome.body !== undefined && (
                    <>
                      <p className="json-label">
                        RESPONSE BODY <span>validated against the export</span>
                      </p>
                      <pre>{JSON.stringify(attempt.outcome.body, null, 2)}</pre>
                    </>
                  )}
                </>
              )}
            </div>
            <div className="contract-note">
              <span className="dot" /> Generated types · runtime validation ·
              one attempt
            </div>
          </section>
        </div>
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
