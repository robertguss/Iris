# API slice: one action through React

This experiment connects the existing SQLite invitation action to Axum, exports
OpenAPI with two libraries, and consumes generated TypeScript types in React. It
is not a framework release or a production authentication implementation.

## Run and verify

Use the pinned Rust toolchain, a C compiler, and Node with npm (tested with Node
26.10.0 and npm 10.9.9). From the repository root:

```sh
npm --prefix experiments/api-slice/web ci
cargo test --workspace --locked
cargo test --locked -p iris-api-spike --features dev-identity
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
npm --prefix experiments/api-slice/web run verify
```

Run the default and feature-enabled tests separately: all-features alone skips
the check that ordinary builds reject the development identity header.

In an Amp orb, `amp orb services ensure` starts the API and Vite and prints the
browser-accessible portal. Elsewhere, run these in two terminals:

```sh
cargo run --locked -p iris-api-spike --features dev-identity --bin iris-api-demo -- --dev-demo
npm --prefix experiments/api-slice/web run dev
```

The API binds loopback port 3001; Vite binds loopback port 5173 and proxies
`/api`. The demo requires both a Cargo feature and a command-line opt-in. Each
process creates a disposable database; restarting it resets the single-use
invitations. Only synthetic users 11 and 29 are accepted. **The header is
caller-controlled, not authentication. Never deploy this identity scheme with
application data.**

| Example                | Intended result              |
| ---------------------- | ---------------------------- |
| Alice + `iris-valid`   | 200; project `7`, user `11`  |
| Repeat that request    | 409 `already_accepted`       |
| Bob + Alice's token    | 404 `not_found`              |
| Alice + `iris-expired` | 409 `expired`                |
| Bob + `iris-bob`       | 200; project `19`, user `29` |
| No identity            | 401 `unauthorized`           |
| Empty token            | 400 `invalid_request`        |

With `agent-browser` installed and a fresh demo process, run the browser suite:

```sh
python3 experiments/api-slice/checks/browser-smoke.py http://127.0.0.1:5173 \
  --artifacts .amp/in/artifacts
```

This consumes both valid invitations. It exercises real HTTP, holds one request
to test loading and duplicate submission, injects a transport failure, and
checks the narrow layout. Screenshots still require visual inspection. Restart
`iris-api` afterward to reset the demo for manual exploration.

## The application-authoring sketch

The shape is ordinary Rust, rather than a new resource DSL:

1. An application action takes explicit input, identity, time, and a connection.
   It owns the transaction and returns a domain outcome, without HTTP types.
2. An Axum handler extracts identity separately from the request DTO, validates
   input, calls the action, and maps outcomes to documented responses.
3. DTOs and route metadata produce OpenAPI; the route helper wires the same
   handler into the router and schema generation.
4. `openapi-typescript` generates types; `openapi-fetch` provides typed
   requests. React imports those types rather than restating the contract.
5. Tests compare runtime responses to schemas and independent business
   expectations. Generated artifacts are checked for drift.

This is the candidate convention, not a promised public API. No generic Action
trait, universal database layer, CLI generator, policy DSL, or procedural-macro
framework has been introduced. The next useful abstraction must earn its place
across another real action.

## utoipa versus aide

Both integrations use the **same handler**, DTOs, fixtures, status expectations,
and JSON Schema validation. Exact versions are pinned in Cargo.lock.

| Observation                                      | utoipa / utoipa-axum                  | aide / schemars                                               |
| ------------------------------------------------ | ------------------------------------- | ------------------------------------------------------------- |
| Contract declaration                             | Handler attribute plus schema derives | Router builder plus schema derives                            |
| Route and operation wiring                       | `routes!` registration                | `api_route` / `post_with`                                     |
| Custom extractor/error integration in this slice | No extra operation traits             | `OperationInput` for Actor and `OperationOutput` for ApiError |
| Response status mapping                          | Explicit                              | Explicit; response inference disabled for comparison          |
| Generated client and runtime schema tests        | Pass                                  | Pass                                                          |

**Provisional selection: utoipa for the running demo.** It needs less
integration code here; aide's builder style remains a viable alternative. This
does not establish ecosystem-wide superiority or a compile-time advantage. Both
libraries remain in this experimental package, so its build cost is not a
single-library production baseline. Previous database measurements remain
historical results.

## Contract choices and evidence

- `POST /api/invitations/accept` accepts only `token`, 1–256 Unicode characters.
  Rust rejects unknown fields, including caller-supplied identity in the body.
- Wire IDs are strings to avoid JavaScript integer precision loss.
- Responses are 200, 400, 401, 404, 409, 500, or 503. Errors have
  `{code,message}`; this envelope is **not RFC 9457 Problem Details**. Unknown
  token and wrong recipient deliberately share 404. Database errors do not
  expose SQL details.
- Integration tests check actual bodies against each exported schema, exercise
  real lock contention and rollback after a database write failure, and compare
  independent expected statuses, codes, IDs, and resulting membership counts.
- `npm run verify` checks both OpenAPI snapshots and generated TS for drift,
  typechecks/builds React, then intentionally renames `user_id` to `member_id`
  in a temporary contract. The real React consumer must fail compilation. The
  probe removes its temporary files and never changes the committed contract.
- Type tests check methods, paths, required fields, and error-code handling.
  `openapi-fetch`'s generic request typing does not reliably reject extra body
  properties; an explicit generated DTO annotation or `satisfies` on a fresh
  object literal does. Server-side rejection is still required.

After intentional Rust contract changes, regenerate and review the artifacts:

```sh
npm --prefix experiments/api-slice/web run generate
npm --prefix experiments/api-slice/web run verify
```

## What remains unverified or deliberately absent

Generated TypeScript is compile-time assistance, not runtime response
validation. The tests validate response schemas; the browser client does not.
The breaking change probe proves one important incompatibility is detected, not
all API compatibility. Error schemas list all codes rather than narrowing codes
by status. Axum's unmatched-route/method responses are outside this endpoint
contract.

Real authentication, session/CSRF policy, production connection management, rate
limiting, operational logging, deployment, durable email delivery, and
cross-version client compatibility are not implemented. No PostgreSQL or remote
Turso behavior is inferred from SQLite tests. Verification covers named
scenarios; it cannot establish that every business requirement was correctly
understood.
