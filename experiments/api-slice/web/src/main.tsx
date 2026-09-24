import { StrictMode, useRef, useState } from "react";
import type { FormEvent } from "react";
import { createRoot } from "react-dom/client";
import { api, errorTitle } from "./api";
import type { AcceptRequest, Acceptance, Problem } from "./api";
import "./style.css";

type Result =
  | { kind: "success"; status: number; body: Acceptance }
  | { kind: "error"; status: number; body: Problem }
  | { kind: "network" };

function App() {
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
      const { data, error, response } = await api.POST("/api/invitations/accept", {
        headers: identity ? { "x-iris-dev-user": identity } : {},
        body: { token } satisfies AcceptRequest,
      });
      if (data) setResult({ kind: "success", status: response.status, body: data });
      else if (error) setResult({ kind: "error", status: response.status, body: error });
      else setResult({ kind: "network" });
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
        <h1>One invitation.<br />An end-to-end contract.</h1>
        <p className="lede">A small experiment in making application behavior explicit.<br className="desktop" /> Send a request and inspect what the API actually returns.</p>
      </section>

      <aside className="notice"><span aria-hidden="true">◈</span><div><strong>Development identity — not real authentication</strong><p>Two synthetic users. Disposable SQLite data. Never use this identity header in a deployed application.</p></div></aside>

      <div className="lab">
        <section className="panel">
          <div className="panel-heading"><span className="step">01</span><h2>Send a request</h2></div>
          <form onSubmit={submit}>
            <fieldset disabled={pending}>
              <label htmlFor="identity">Development identity</label>
              <select id="identity" value={identity} onChange={e => { setIdentity(e.target.value); setResult(null); }}>
                <option value="11">Alice · user 11</option>
                <option value="29">Bob · user 29</option>
                <option value="">Unauthenticated</option>
              </select>
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
              <button className="submit" type="submit">{pending ? "Sending request…" : "Accept invitation"}<span aria-hidden="true">→</span></button>
            </fieldset>
          </form>
          <p className="footnote">Valid invitations can be accepted once. Restarting the demo server resets the fixtures.</p>
        </section>

        <section className="panel response-panel" aria-busy={pending}>
          <div className="panel-heading"><span className="step">02</span><h2>Inspect the response</h2></div>
          <div className="endpoint"><b>POST</b><code>/api/invitations/accept</code></div>
          <div aria-live="polite" aria-atomic="true" className="result">
            {pending ? <div className="empty"><span className="pulse" aria-hidden="true" /><h3>Waiting for the API</h3><p>The server is processing your request.</p></div>
              : !result ? <div className="empty"><span className="empty-icon" aria-hidden="true">↳</span><h3>Ready when you are</h3><p>Submit an invitation to see its status<br />and JSON response here.</p></div>
              : result.kind === "network" ? <div className="status error"><strong>Couldn’t read an API response</strong><p>Check that the demo server is running. No automatic retry was sent; a request may have reached the server.</p></div>
              : <>
                  <div className={`status ${result.kind === "success" ? "success" : "error"}`}>
                    <strong>{result.status} · {result.kind === "success" ? "Invitation accepted" : errorTitle(result.body.code)}</strong>
                    <p>{result.kind === "success" ? `User ${result.body.user_id} is a member of project ${result.body.project_id}.` : result.body.message}</p>
                  </div>
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
