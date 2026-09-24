# API slice: issue and accept through React

This experiment connects SQLite invitation actions to Axum, exports OpenAPI with
two libraries, and consumes generated TypeScript types in React. It is not a
framework release or a production authentication implementation.

## Try the complete workflow

Select **Issue an invitation**, Alice, project `41`, and Bob as recipient.
Submit, then use **Switch to recipient and load token** and accept as Bob. The
HTTP tests check the database before and after: issuing creates no membership;
acceptance creates Bob's editor membership. Bob owns a separate project, `43`,
for the reverse example. The original project `7`/`19` acceptance fixtures still
work.

Creation semantics for this experiment:

- `POST /api/invitations` accepts `{project_id,recipient_id}` only. IDs are
  canonical positive signed-64-bit decimal strings. Schema patterns describe
  decimal syntax/length; the handler additionally checks integer range.
- Only an owner of the requested project can issue invitations. Unknown project,
  unrelated project owner, editor, viewer, and nonmember all get 403
  `forbidden`. Authorization precedes recipient lookup, membership, and
  pending-invite checks.
- The recipient must already exist (404 `recipient_not_found`) and must not
  already belong to the project (409 `already_member`, including self-invites).
- An unaccepted invitation with `expires_at > now` causes 409
  `invitation_pending`. Expiration equality permits replacement. Old expired
  tokens remain expired; new tokens do not reactivate them.
- A write transaction protects the owner check, duplicate check, and insert.
  Concurrent issuance yields one creation and one pending conflict when the lock
  is acquired within the timeout; sustained contention returns 503.
- The server generates 32 random bytes via `getrandom`, encodes them as 64 hex
  characters, and stores only SHA-256. Token generation, role (`editor`), and
  lifetime (3,600 seconds from the captured action timestamp) are server-owned.
- Success is 201 with `{project_id,recipient_id,token,expires_at}` and
  `Cache-Control: no-store`. Expiry is Unix seconds represented as a string.
  Direct token return is **demo delivery**, not a production email design. A
  lost response cannot recover the token by repeating the request. No automatic
  retry, idempotency key, resend, cancellation, or token recovery is
  implemented.
- Revoking an owner's role prevents future issuance, but does not revoke already
  issued invitations. Acceptance uses the existing recipient/token policy.

Migration `0002_owners.sql` adds the owner membership role while preserving
existing memberships; invitation roles remain viewer/editor. SQLite and Turso
acceptance tests use the updated schema. Creation is implemented only for SQLx /
SQLite; this does not claim creation support on Turso. Migration tests use
disposable databases, not a shared or production database.

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

This consumes the original valid invitations and exercises creation, owner
rejection, recipient handoff, acceptance, and duplicate conflicts. It uses real
HTTP, holds requests to test loading and duplicate submission, injects transport
failures, and checks the narrow layout. Screenshots still require inspection.
Restart `iris-api` afterward to reset the demo for manual exploration.

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
framework has been introduced. The second action reuses identity extraction,
error responses, database-error mapping, generated clients, and the
contract-test request helper. Its policy and transaction remain in a plain Rust
function.

The two actions have different input/output types, permission checks, and
success statuses. That is evidence for a shared **convention**, not yet for a
generic Action trait. Keep the explicit action → handler → DTO/OpenAPI → typed
client boundary. Response declarations are still duplicated for the library
comparison; do not turn that experimental duplication into a framework
abstraction.

## utoipa versus aide

Both integrations use the **same handlers**, DTOs, fixtures, status
expectations, and JSON Schema validation. Exact versions are pinned in
Cargo.lock.

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
- Acceptance responses are 200, 400, 401, 404, 409, 500, or 503. Creation adds
  201 and 403 (with no 200 success). Errors have `{code,message}`; this envelope
  is **not RFC 9457 Problem Details**. Unknown token and wrong recipient
  deliberately share 404. Database errors do not expose SQL details.
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

## Measured feedback loops

Run after installing dependencies, from the repository root:

```sh
python3 experiments/api-slice/measure.py --samples 3 --output target/api-loop-results.json
```

The script copies source into a temporary directory, reuses installed npm
dependencies, uses a fresh Cargo target, and deletes its modified source
afterward. It records exact commands, versions, source hashes, and wall times.
It never clears the checkout's target or mutates its source/contracts. The
recorded [results](results.json) use Rust 1.98.1, four build jobs, incremental
debug compilation, the default linker, no compiler wrapper, and warm downloaded
crates/OS caches in this orb.

| Loop                                                                     | Median of three warm samples | Range         |
| ------------------------------------------------------------------------ | ---------------------------- | ------------- |
| No-change targeted HTTP workflow test                                    | 0.267 s                      | 0.267–0.318 s |
| Error-message body edit → same test passes                               | 1.494 s                      | 1.431–1.644 s |
| Optional response-field edit → both OpenAPI/TS exports → React typecheck | 3.523 s                      | 3.519–3.770 s |

One fresh-target build-and-test took 37.965 s; the initial contract export and
typecheck took another 10.565 s. These are single priming observations, not
clean-build medians. The warm test exercises issuance and acceptance through
both routers; it is not the whole suite. The shape probe adds a temporary
optional field and initializes it in the response, rather than measuring a
comment-only change. The TypeScript loop does not include Vite rendering,
browser reload, or human editing time. Samples reuse one target and run
sequentially, so they are not independent cold-build trials or a comparison
against the old database spike.

This establishes a usable baseline, not a compile-time optimization claim. The
experimental package still includes both OpenAPI libraries. Removing the unused
comparison integration, feature unification, and linker experiments are future
measurements; no hot-patching system or new Cargo abstraction was added.

Executed verification: 26 default workspace tests, six feature-enabled API
tests, Clippy with warnings denied, contract drift/type/build/breaking-change
checks, and the browser smoke suite passed. The browser suite verifies identity
handoff, project/user IDs after acceptance, disabled controls, duplicate-submit
guards, and narrow-width overflow; loading/success/error screenshots were also
inspected.

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
