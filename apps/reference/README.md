# Iris reference application

The S17 reference application: membership management over session identity,
built to test whether the S16 contract generalizes across operations before any
of it becomes framework API. It is application code; `crates/iris` holds only
what two operations demonstrably share and is private and provisional. See
[S17](../../docs/design-spec.md#s17--reference-application-and-first-reads) for
the design and its checkpoint A
[server-side](../../docs/design-spec.md#checkpoint-a-server-side-evidence) and
[client](../../docs/design-spec.md#checkpoint-a-client-evidence) evidence.

**Status:** checkpoint A is implemented: both membership operations, their React
client and one browser workflow. Checkpoint B (reads) is outstanding.

## Run it

From the repository root, with Rust 1.98.1 and Node available (the tests start
the local OIDC issuer fixture from `experiments/api-slice/checks`):

```sh
cargo test --locked -p iris -p iris-reference
node apps/reference/scripts/probes.mjs
npm --prefix apps/reference/web ci
npm --prefix apps/reference/web run verify
node apps/reference/scripts/browser.mjs --artifacts /tmp/reference-artifacts
cargo run --locked -p iris-reference --bin export-openapi -- apps/reference/openapi.json
npm --prefix apps/reference/web run generate
```

`openapi.json` is the committed export. A Rust test fails on drift and names the
export command; `verify` fails when `web/src/generated.ts` no longer matches it
and names `generate`. `verify` also captures real responses from the Rust
whole-request tests, type-checks, runs the decoder and presentation cases, and
builds the client. The browser workflow needs `agent-browser` 0.38.1 and its
browser (`agent-browser install`); it builds and starts everything it uses and
stops it on exit.

To use the console by hand, start the local issuer, the development server and
Vite, then open `http://127.0.0.1:5175` and sign in as Alice or Bob:

```sh
node experiments/api-slice/checks/oidc-provider.mjs --port 4001 --issuer http://127.0.0.1:4001 --redirect-uri http://127.0.0.1:5175/api/auth/callback
IRIS_PUBLIC_ORIGIN=http://127.0.0.1:5175 IRIS_OIDC_ISSUER=http://127.0.0.1:4001 cargo run --locked -p iris-reference --bin reference-dev -- --local-oidc-demo
npm --prefix apps/reference/web run dev
```

The development server listens on `127.0.0.1:3003` (`IRIS_LISTEN` overrides it),
uses a disposable SQLite database, and seeds Bob as an editor of Alice's project
41 so a role change or removal can succeed without invitations. Restarting it
resets the data.

## Layout

| Path                                | Owns                                                                                                   |
| ----------------------------------- | ------------------------------------------------------------------------------------------------------ |
| `src/app.rs`                        | State, connections, the migration, seeds, and the one checked assembly behind `app()` and `openapi()`  |
| `src/identity.rs`                   | Session/OIDC identity copied as application code; session endpoints keep their `{code,message}` bodies |
| `src/domains/memberships.rs`        | Commands, the shared rejection type, one private SQLite transaction for both operations                |
| `src/http/`                         | DTOs, declarations, endpoints, mounts, the session-marker classifier and wire-ID parsing               |
| `src/bin/reference-dev.rs`          | The disposable `--local-oidc-demo` development server                                                  |
| `web/src/client.ts`                 | The whole-request boundary: linkage, single bounded reads and validation against the bundled export    |
| `web/src/session.ts`, `main.tsx`    | Session bootstrap over the existing session contracts, and the checkpoint A console                    |
| `web/src/present.ts`                | Outcome wording, exhaustive over both operations' codes                                                |
| `migrations/`, `tests/`, `scripts/` | Schema; contract, session and development-server tests; omission probes and the browser workflow       |
| `../../crates/iris`                 | Envelope rendering, the response bridge, the shared profile, the request-ID boundary, assembly checks  |

## Operations

| Operation                   | Route                          | Success                           | Rejections                                                                                    |
| --------------------------- | ------------------------------ | --------------------------------- | --------------------------------------------------------------------------------------------- |
| `memberships.change_role`   | `POST /api/memberships/role`   | 200 `{completion:"acknowledged"}` | 403 `memberships.forbidden`; 404 `memberships.member_not_found`; 409 `memberships.last_owner` |
| `memberships.remove_member` | `POST /api/memberships/remove` | 200 `{completion:"acknowledged"}` | Same as `change_role`                                                                         |

Both also declare 400 `http.invalid_request`, 401 `http.unauthenticated`, 403
`http.csrf_refused`, 500 `iris.internal` and 503 `iris.unavailable`.

## Verification matrix

Measured September 26–27, 2026, on macOS with Node 24.20.0 (CI pins 26.10.0);
the client checks also passed under Node 26.8.1.

| Check                  | Result                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| ---------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `iris` unit tests      | 9 passed: component and path conflicts, duplicate IDs and names, surviving handler IDs, unbridged operations, and five kinds of shared-code metadata drift                                                                                                                                                                                                                                                                                                                                                       |
| Ported S16 Rust suites | 10 passed. Whole-request fixtures run against the assembled application; focused tests exercise domain code or hand-built routers. Expectations are unchanged except that the marked-response case uses `LoginFailed` (401), because the session enum no longer has `Forbidden`                                                                                                                                                                                                                                  |
| `remove_member`        | Independent contract, whole request (every status, schema-validated), and two concurrent owner self-removals leaving one owner                                                                                                                                                                                                                                                                                                                                                                                   |
| Cross-operation        | Identical declared mappings; full recovery metadata written by hand for both                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| Contract               | Snapshot drift, and a hand-written inventory of six paths, methods and operation IDs                                                                                                                                                                                                                                                                                                                                                                                                                             |
| Session                | 7 passed, including CSRF matrices, a wrong-token case and revocation after user deletion; CSRF and permission checks cover both domain routes                                                                                                                                                                                                                                                                                                                                                                    |
| Development server     | 2 passed: it refuses to start without `--local-oidc-demo`, and serves the session, the domain boundary (an anonymous 401 envelope) and a plain 404                                                                                                                                                                                                                                                                                                                                                               |
| Client decoder         | 122 cases: 19 and 17 real responses, counted per status, kind and code; independent, cross-operation, malformed, oversize and unusable-body cases; one attempt each and no raw diagnostics                                                                                                                                                                                                                                                                                                                       |
| Client types           | Status and code narrowing for both operations, distinct operation literals, and the bundled snapshot accepted as a document without a cast                                                                                                                                                                                                                                                                                                                                                                       |
| Presentation           | 24 cases: every declared outcome of both operations and `client_unknown`; unconfirmed outcomes claim no effect                                                                                                                                                                                                                                                                                                                                                                                                   |
| Browser workflow       | Passed five consecutive runs of the final script: OIDC sign-in and bootstrap recovery, role change, one unconfirmed attempt, non-owner refusal, removal, absence, last-owner protection, HttpOnly session, narrow layout and no browser storage. A taken port, foreign servers, a spawn error, SIGTERM, a child that never reports ready, a child killed mid-run and a build that ignores SIGTERM each stop it without a PASS or leftover processes; a second invocation is refused without disturbing the first |
| Workspace              | 83 passed; the frozen experiments stay green                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |

## Cost of runtime validation

| Measurement                                  | Result                                                                               |
| -------------------------------------------- | ------------------------------------------------------------------------------------ |
| Production bundle (`vite build`)             | 374.32 kB JavaScript, 111.87 kB gzip                                                 |
| Ajv (difference from a stub-validator build) | 130.62 kB, 38.24 kB gzip                                                             |
| Validator compile, production build, browser | 13.3–24.1 ms, median 13.9 ms, across nine runs; headless Chrome 154                  |
| Validator compile, Node                      | 24.20.0: 41.5 ms cold, 10.4 ms warm median; 26.8.1: 30.6 ms cold, 9.0 ms warm median |

The browser figure is the `iris:client-compile` performance measure, taken once
per page load.

## Omission probes

`scripts/probes.mjs` edits a disposable source copy, builds it in its own
temporary target, and removes both on exit. It never edits the checkout. It ran
in 39 s locally with a cold target.

| Temporary change                                           | Executed signal                                                              |
| ---------------------------------------------------------- | ---------------------------------------------------------------------------- |
| Second operation collected without the bridge              | Assembly panics: surviving inferred handler ID `remove_member_endpoint`      |
| Second operation reuses `changeMemberRole`                 | Assembly panics: duplicate OpenAPI operation ID                              |
| Second operation changes `memberships.forbidden`'s message | Assembly panics: the public code differs between the two operations          |
| Wrong route path                                           | Compiler passes; the independent inventory fails                             |
| Same-named success component with a different definition   | Assembly panics: component conflict `schemas/ChangeRoleSuccess`              |
| Same-named success component with an identical definition  | Control: assembles, exported once and referenced by both operations          |
| Second operation left out of collection                    | The independent inventory fails; this does not probe a missing runtime mount |

These are selected seeded mistakes, not exhaustive mutation testing. The client
and browser checks were also mutation-tested while they were built; the
[decision record](../../docs/decisions.md#reference-application-checkpoint-a-client--september-27-2026)
lists those mutations.

## Limits

- A failed connection open is tested to return 500; its busy-to-503
  classification is source-inspected only.
- The client bundles the export it was built with, so client and server must
  ship from the same commit.
- Ajv compiles validators with `new Function`; a strict content security policy
  would need precompiled validators. None is set here.
- The browser workflow checks selected paths in one browser, not every code.
- HEAD and GET operations, reads and pagination are checkpoint B.
- Only macOS was used. No GitHub Actions run has been observed.
