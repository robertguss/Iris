# Iris reference application

The S17 reference application: membership management over session identity,
built to test whether the S16 contract generalizes across operations before any
of it becomes framework API. It is application code; `crates/iris` holds only
what two operations demonstrably share and is private and provisional. See
[S17](../../docs/design-spec.md#s17--reference-application-and-first-reads) for
the design and its
[checkpoint A evidence](../../docs/design-spec.md#checkpoint-a-server-side-evidence).

**Status:** checkpoint A's server side is implemented. Its client and browser
acceptance, and all of checkpoint B, are outstanding.

## Run it

From the repository root, with Rust 1.98.1 and Node available (the tests start
the local OIDC issuer fixture from `experiments/api-slice/checks`):

```sh
cargo test --locked -p iris -p iris-reference
node apps/reference/scripts/probes.mjs
cargo run --locked -p iris-reference --bin export-openapi -- apps/reference/openapi.json
```

`openapi.json` is the committed export. A test fails on drift and names the
export command. Tests and fixtures use disposable SQLite databases; there is no
development server yet.

## Layout

| Path                                | Owns                                                                                                   |
| ----------------------------------- | ------------------------------------------------------------------------------------------------------ |
| `src/app.rs`                        | State, connections, the migration, seeds, and the one checked assembly behind `app()` and `openapi()`  |
| `src/identity.rs`                   | Session/OIDC identity copied as application code; session endpoints keep their `{code,message}` bodies |
| `src/domains/memberships.rs`        | Commands, the shared rejection type, one private SQLite transaction for both operations                |
| `src/http/`                         | DTOs, declarations, endpoints, mounts, the session-marker classifier and wire-ID parsing               |
| `migrations/`, `tests/`, `scripts/` | Schema, contract and session tests, and the omission probes                                            |
| `../../crates/iris`                 | Envelope rendering, the response bridge, the shared profile, the request-ID boundary, assembly checks  |

## Operations

| Operation                   | Route                          | Success                           | Rejections                                                                                    |
| --------------------------- | ------------------------------ | --------------------------------- | --------------------------------------------------------------------------------------------- |
| `memberships.change_role`   | `POST /api/memberships/role`   | 200 `{completion:"acknowledged"}` | 403 `memberships.forbidden`; 404 `memberships.member_not_found`; 409 `memberships.last_owner` |
| `memberships.remove_member` | `POST /api/memberships/remove` | 200 `{completion:"acknowledged"}` | Same as `change_role`                                                                         |

Both also declare 400 `http.invalid_request`, 401 `http.unauthenticated`, 403
`http.csrf_refused`, 500 `iris.internal` and 503 `iris.unavailable`.

## Verification matrix

Measured September 26, 2026, on macOS with Node 24.20.0 (CI pins 26.10.0).

| Check                  | Result                                                                                                                                                                                                                                                                          |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `iris` unit tests      | 9 passed: component and path conflicts, duplicate IDs and names, surviving handler IDs, unbridged operations, and five kinds of shared-code metadata drift                                                                                                                      |
| Ported S16 Rust suites | 10 passed. Whole-request fixtures run against the assembled application; focused tests exercise domain code or hand-built routers. Expectations are unchanged except that the marked-response case uses `LoginFailed` (401), because the session enum no longer has `Forbidden` |
| `remove_member`        | Independent contract, whole request (every status, schema-validated), and two concurrent owner self-removals leaving one owner                                                                                                                                                  |
| Cross-operation        | Identical declared mappings; full recovery metadata written by hand for both                                                                                                                                                                                                    |
| Contract               | Snapshot drift, and a hand-written inventory of six paths, methods and operation IDs                                                                                                                                                                                            |
| Session                | 7 passed, including CSRF matrices, a wrong-token case and revocation after user deletion; CSRF and permission checks cover both domain routes                                                                                                                                   |
| Workspace              | 81 passed; the frozen experiments stay green                                                                                                                                                                                                                                    |

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

These are selected seeded mistakes, not exhaustive mutation testing.

## Limits

- A failed connection open is tested to return 500; its busy-to-503
  classification is source-inspected only.
- The S16 client cases, the React client and the browser workflow are not yet
  ported; checkpoint A is partial until they are.
- HEAD and GET operations, reads and pagination are checkpoint B.
- Only macOS was used. No GitHub Actions run has been observed.
