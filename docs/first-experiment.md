# Verification and first experiment

Status: the database acceptance slice is implemented and measured; see
[findings](embedded-db-findings.md) and the
[runnable experiment](../experiments/embedded-db/README.md). The broader plan
below is retained: invitation creation/authorization, HTTP, React, and email are
not implemented. Engine support and a general framework API remain undecided.

## Purpose

Learn which embedded database and Rust integration best support a productive,
verifiable application framework. Use one realistic feature rather than a large
framework scaffold: project invitations and acceptance.

The first comparison is database-focused. The following slice connects
application actions to HTTP, OpenAPI, a generated TypeScript client, and React.
Do not build both a complete framework and a database abstraction before running
this experiment.

## Stage 1 — Write down behavior independently

Proposed expectations, to finalize before implementation:

- Only actors with the relevant project permission can create invitations.
- An invitation grants access only to its intended recipient under a documented
  identity policy. Token possession alone is not sufficient for this experiment.
- An invitation is valid before expiration and invalid at or after expiration.
- Successful acceptance consumes the invitation and creates the intended
  membership.
- A project/user pair has at most one membership.
- Concurrent requests produce one effective acceptance, not duplicate
  memberships.
- Failure before commit leaves neither a consumed invitation nor a partial
  membership.
- Unauthorized actors cannot discover sensitive project/invitation information.

Settle repeated-acceptance behavior (idempotent success or conflict), role
rules, existing-member behavior, identity normalization, and error disclosure
explicitly. Do not let convenient SQL silently choose these product semantics.

## Stage 2 — Two concrete persistence implementations

Build a SQLite baseline and native local Turso implementation. Use ordinary
functions and concrete driver types; share behavior expectations rather than
prematurely introducing generic persistence/transaction traits.

For each:

1. Pin released dependencies and record engine/driver versions and enabled
   features.
2. Create a fresh temporary directory and database, then apply migrations.
3. Configure and verify foreign-key enforcement on every connection.
4. Store a token hash, not the raw invitation credential.
5. Accept in one explicit transaction using a conditional state transition and a
   unique membership constraint. Include recipient and expiration authorization;
   no illustrative SQL may omit those checks in the actual implementation.
6. Evaluate `BEGIN IMMEDIATE` and conditional `UPDATE … RETURNING` support on
   the selected releases. Fully consume/drop statements before subsequent
   writes.
7. Classify contention separately from domain failures. If retries are used,
   bound them and retry only known-safe transaction failures, not uncertain
   commits.

Avoid depending on PostgreSQL's `SELECT FOR UPDATE`. Do not interpret every
unique constraint violation as success or allow acceptance to silently change an
existing member's role. Explicitly test rollback if membership creation fails.

### Database checks

| Check                         | Evidence required                                                                |
| ----------------------------- | -------------------------------------------------------------------------------- |
| Initialization and migrations | Fresh file reaches expected schema; migration rerun is well-defined              |
| Foreign keys                  | Invalid reference fails on independently opened connections                      |
| Recipient authorization       | Wrong account cannot consume token or create membership                          |
| Expiration boundary           | Just before succeeds; equality and after fail using controlled time              |
| Repeated acceptance           | Chosen semantics hold without extra membership/state changes                     |
| Concurrent acceptance         | Independent connections start together; final state has one effective acceptance |
| Mid-transaction failure       | Both invitation transition and membership write roll back                        |
| Lock contention               | Waiting/timeout/retry behavior is bounded and classified correctly               |
| Existing member               | No accidental privilege change or duplicate membership                           |
| Restart                       | Close/reopen database and verify committed state                                 |

Use temporary files for primary integration and concurrency tests. In-memory
smoke tests are supplemental: sharing and lifetime differ by driver/connection
setup. Do not wrap concurrency tests in one outer transaction that suppresses
the race. Keep fixture data synthetic and secrets out of retained artifacts.

### What to measure

- Setup commands, required services/toolchains, and migration ergonomics.
- Clean build, warm no-change build, handler/function-body edit, shared-type
  edit, and one targeted test.
- Default versus deliberately reduced dependency features, where supported.
- Test isolation overhead, contention outcomes, and binary size if useful.

Record exact commands, toolchain, OS/architecture, hardware, lockfile, profiles,
linker, cache state, and repeated samples. Distinguish dependency compilation,
application compilation, linking, process startup, and test execution. Do not
compare a warmed candidate with a cold candidate or call one sample a benchmark.

Deliverable: a short evidence-based choice or a clearly scoped remaining
question, not a blanket declaration that one engine is faster or more reliable.

## Stage 3 — Design and implement the application experience

Sketch what a user writes before defining framework internals:

```rust
// Illustrative shape only; these names and types are not an implemented API.
async fn accept_invitation(
    context: &ActionContext,
    input: AcceptInvitation,
) -> Result<Membership, AcceptInvitationError>
```

Determine where actor context, database access, time, authorization,
transactions, and errors belong. Make the action callable without an HTTP
request. Explicitly bind selected actions to routes; do not publish every action
automatically.

Then connect:

```text
React -> generated TypeScript client -> HTTP binding -> application action -> database
                      ^                       |
                      |                       v
                      +------ OpenAPI contract
```

Compare utoipa/utoipa-axum and aide on actual generated schemas and client
behavior. Check error unions, nullable fields, stable operation IDs,
authentication requirements, and actual response bodies. Select and pin a client
generator only after exercising it. Define regeneration/drift checks and ensure
generated output does not silently become an independently edited source of
truth.

If notification email joins the slice, specify commit/enqueue failure behavior.
An outbox is a candidate when durable delivery matters; it is not a promise of
exactly-once email. Test retries and duplicate-delivery handling explicitly.

## Verification model to grow with the framework

| Layer                  | What it checks                                                   | What it does not establish              |
| ---------------------- | ---------------------------------------------------------------- | --------------------------------------- |
| Compiler and types     | Structural invariants encoded in Rust                            | Correct business rules or authorization |
| API contract tests     | Requests/responses and generated clients agree with the contract | Contract matches user intent            |
| Business scenarios     | Independently stated examples and negative cases                 | All possible executions                 |
| Property tests         | Invariants across meaningful generated inputs/sequences          | Correctness outside modeled behavior    |
| Database/failure tests | Real constraints, transactions, contention, recovery             | Another engine's behavior               |
| End-to-end workflows   | Real client can complete representative tasks                    | Exhaustive coverage                     |
| Mutation testing       | Selected injected defects are caught                             | Proof that no defects remain            |

Use asymmetric users/projects and both sides of boundaries. For properties,
prefer state-transition sequences that can expose unauthorized or duplicate
membership over random inputs that only test parsing rejection. Expected
outcomes must not be derived from the function under test.

A future `verify` command could orchestrate checks and report their scope,
environment, failures, skipped checks, and artifacts. It must not label an app
universally correct. This command is an idea, not an existing feature.

## Fast loop versus clean verification

Start with existing watch/build tools and supervised React/backend processes.
Measure before experimenting with crate boundaries, linking, caching, or
Subsecond. Keep a clean build plus fresh-process verification path independent
of any state-preserving development mechanism.

After the slice works, ask an AI assistant to add a role or authorization rule.
Record which mistakes the compiler catches, which tests catch, which escape, how
much guidance was required, and how many independent edits were necessary. Use
that evidence to decide what generators, conventions, or framework primitives
are worth building next.
