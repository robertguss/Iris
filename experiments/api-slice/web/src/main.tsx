import { StrictMode, useRef, useState } from "react";
import type { FormEvent } from "react";
import { createRoot } from "react-dom/client";
import { api, errorTitle } from "./api";
import type { AcceptRequest, Acceptance, IssueRequest, IssuedInvitation, Problem } from "./api";
import "./style.css";

type Result =
  | { kind: "success"; status: number; body: Acceptance }
  | { kind: "issued"; status: number; body: IssuedInvitation }
  | { kind: "error"; status: number; body: Problem }
  | { kind: "network" };

function App() {
  const [operation, setOperation] = useState("accept");
  const [project, setProject] = useState("41");
  const [recipient, setRecipient] = useState("29");
  const [identity, setIdentity] = useState("11");
  const [token, setToken] = useState("iris-valid");
  const [pending, setPending] = useState(false);
  const inFlight = useRef(false);
  const [result, setResult] = useState<Result | null>(null);

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (inFlight.current) return;
    inFlight.current = true;
    setPending(true);
    setResult(null);
    try {
      if (operation === "issue") {
        const { data, error, response } = await api.POST("/api/invitations", {
          headers: identity ? { "x-iris-dev-user": identity } : {},
          body: { project_id: project, recipient_id: recipient } satisfies IssueRequest,
        });
        if (data) setResult({ kind: "issued", status: response.status, body: data });
        else if (error) setResult({ kind: "error", status: response.status, body: error });
        else setResult({ kind: "network" });
      } else {
      const { data, error, response } = await api.POST("/api/invitations/accept", {
        headers: identity ? { "x-iris-dev-user": identity } : {},
        body: { token } satisfies AcceptRequest,
      });
      if (data) setResult({ kind: "success", status: response.status, body: data });
      else if (error) setResult({ kind: "error", status: response.status, body: error });
      else setResult({ kind: "network" });
      }
    } catch {
      setResult({ kind: "network" });
    } finally {
      inFlight.current = false;
      setPending(false);
    }
  }

  function example(value: string, user: string) {
    setToken(value);
    setIdentity(user);
    setResult(null);
  }

  return (
    <main>
      <header>
        <div className="brand"><span className="mark" aria-hidden="true">i</span><strong>Iris</strong><span className="tag">LAB / 02</span></div>
        <a href="/api/openapi.json" target="_blank" rel="noreferrer">OpenAPI contract ↗</a>
      </header>

      <section className="intro">
        <p className="eyebrow">RUST → OPENAPI → TYPESCRIPT → REACT</p>
        <h1>Issue. Switch identity. Accept.</h1>
        <p className="lede">Two actions, one explicit permission boundary.<br className="desktop" /> Only an owner can invite. Only the recipient can accept.</p>
      </section>

      <aside className="notice"><span aria-hidden="true">◈</span><div><strong>Development identity — not real authentication</strong><p>Two synthetic users. Disposable SQLite data. Never use this identity header in a deployed application.</p></div></aside>

      <div className="lab">
        <section className="panel">
          <div className="panel-heading"><span className="step">01</span><h2>Send a request</h2></div>
          <form onSubmit={submit}>
            <fieldset disabled={pending}>
              <label htmlFor="operation">Action</label>
              <select id="operation" value={operation} onChange={e => { setOperation(e.target.value); setResult(null); }}>
                <option value="issue">Issue an invitation</option>
                <option value="accept">Accept an invitation</option>
              </select>
              <label htmlFor="identity">Development identity</label>
              <select id="identity" value={identity} onChange={e => { setIdentity(e.target.value); setResult(null); }}>
                <option value="11">Alice · user 11</option>
                <option value="29">Bob · user 29</option>
                <option value="">Unauthenticated</option>
              </select>
              {operation === "issue" ? <>
                <label htmlFor="project">Project ID</label>
                <input id="project" value={project} onChange={e => { setProject(e.target.value); setResult(null); }} />
                <p className="help">Alice owns project 41. Bob owns project 43.</p>
                <label htmlFor="recipient">Recipient</label>
                <select id="recipient" value={recipient} onChange={e => { setRecipient(e.target.value); setResult(null); }}>
                  <option value="29">Bob · user 29</option><option value="11">Alice · user 11</option>
                </select>
                <p className="help">Creates a one-hour invitation for an editor role. No membership is granted yet.</p>
              </> : <>
              <label htmlFor="token">Invitation token</label>
              <input id="token" value={token} onChange={e => { setToken(e.target.value); setResult(null); }} autoComplete="off" spellCheck={false} aria-describedby="token-help" />
              <p className="help" id="token-help">Identity travels in a header, never in the request body.</p>
              <p className="example-label">LOAD AN EXAMPLE</p>
              <div className="examples">
                <button type="button" onClick={() => example("iris-valid", "11")}>Valid</button>
                <button type="button" onClick={() => example("iris-expired", "11")}>Expired</button>
                <button type="button" onClick={() => example("unknown", "11")}>Unknown</button>
                <button type="button" onClick={() => example("iris-bob", "29")}>Bob’s invitation</button>
              </div>
              </>}
              <button className="submit" type="submit">{pending ? "Sending request…" : operation === "issue" ? "Issue invitation" : "Accept invitation"}<span aria-hidden="true">→</span></button>
            </fieldset>
          </form>
          <p className="footnote">Valid invitations can be accepted once. Restarting the demo server resets the fixtures.</p>
        </section>

        <section className="panel response-panel" aria-busy={pending}>
          <div className="panel-heading"><span className="step">02</span><h2>Inspect the response</h2></div>
          <div className="endpoint"><b>POST</b><code>{operation === "issue" ? "/api/invitations" : "/api/invitations/accept"}</code></div>
          <div aria-live="polite" aria-atomic="true" className="result">
            {pending ? <div className="empty"><span className="pulse" aria-hidden="true" /><h3>Waiting for the API</h3><p>The server is processing your request.</p></div>
              : !result ? <div className="empty"><span className="empty-icon" aria-hidden="true">↳</span><h3>Ready when you are</h3><p>Submit an invitation to see its status<br />and JSON response here.</p></div>
              : result.kind === "network" ? <div className="status error"><strong>Couldn’t read an API response</strong><p>Check that the demo server is running. No automatic retry was sent; a request may have reached the server.</p></div>
              : <>
                  <div className={`status ${result.kind === "error" ? "error" : "success"}`}>
                    <strong>{result.status} · {result.kind === "success" ? "Invitation accepted" : result.kind === "issued" ? "Invitation issued" : errorTitle(result.body.code)}</strong>
                    <p>{result.kind === "success" ? `User ${result.body.user_id} is a member of project ${result.body.project_id}.` : result.kind === "issued" ? "Invitation created. Membership is unchanged until acceptance." : result.body.message}</p>
                  </div>
                  {result.kind === "issued" && <>
                    <p className="help">Demo-only token delivery, not email. This credential is shown once; a duplicate request will not recover it.</p>
                    <button className="submit" type="button" onClick={() => {
                      setToken(result.body.token); setIdentity(result.body.recipient_id); setOperation("accept"); setResult(null);
                    }}>Switch to recipient and load token →</button>
                  </>}
                  <p className="json-label">RESPONSE BODY <span>application/json</span></p>
                  <pre>{JSON.stringify(result.body, null, 2)}</pre>
                </>}
          </div>
          <div className="contract-note"><span className="dot" /> Generated types · explicit errors · real database</div>
        </section>
      </div>
      <footer><span>Iris / Application framework research</span><span>SQLite + Axum + utoipa</span></footer>
    </main>
  );
}

createRoot(document.getElementById("root")!).render(<StrictMode><App /></StrictMode>);
