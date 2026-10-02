# Decisions and open questions

Initial record: September 24, 2026.

The [living design specification](design-spec.md) is now the current synthesis
and starting point for agents. This document retains the chronological record;
early proposals and open questions below may be superseded by later sections.

## Experiment follow-up — September 24, 2026

The [embedded database spike](embedded-db-findings.md) now supplies executable
evidence. Both SQLx/SQLite and native Turso passed ten shared scenarios. Three
isolated build samples per candidate favor SQLx/SQLite's feedback loop for this
workload; SQLx also supplied migration tooling directly. The recommendation is
to use SQLite for the next API slice, not to lock in Iris's default permanently.

The spike targets verified user IDs, uses a fixed timestamp per attempt, treats
expiration equality as expired, returns `AlreadyAccepted` on repeat acceptance,
and preserves existing membership roles. These settle experiment semantics only.
It introduces no runtime database abstraction and does not implement
authentication.

The proposals and open questions below preserve the initial design record; exact
experiment dependency pins are in Cargo.toml/Cargo.lock, not framework promises.

## API follow-up — September 24, 2026

The [API slice](../experiments/api-slice/README.md) now exercises Axum + SQLite,
both utoipa and aide exports, generated TypeScript, and a React consumer. Both
OpenAPI integrations pass the same runtime contract checks. Utoipa is the
provisional choice for the running demo because it needs fewer custom operation
traits here; no compile-time winner has been measured for these libraries.

The experiment keeps actions independent of HTTP, uses string IDs on the wire,
rejects unknown request fields, and maps explicit domain outcomes to documented
statuses and `{code,message}` errors. This is not an RFC 9457 implementation.
Contract drift checks and an intentional breaking-change probe complement
business and browser tests; generated types are not runtime validation.

Development identity is feature-gated and the demo binary requires explicit
opt-in. Ordinary builds reject the synthetic header. Real authentication remains
open. These are experiment conventions, not a released framework API. The
proposal table and open questions below retain the original research context;
the linked slice records which portions now have executable evidence.

## Second-action follow-up — September 24, 2026

Owner-authorized invitation creation now complements acceptance. The action
checks the owner's project membership inside the write transaction, generates a
random token, stores its hash, and grants no membership until acceptance. It
issues editor invitations only, with a fixed one-hour lifetime. Existing members
and unexpired pending invitations are conflicts; an expired invitation can be
replaced. Revocation of ownership prevents new issuance, not acceptance of
already-issued invitations. These choices apply to the experiment, not a general
authorization system.

Both actions reuse identity extraction, structured errors, database-error
mapping, OpenAPI/client tooling, and a contract-test request helper. No generic
Action trait or policy DSL was justified by this second action. The application
convention remains explicit input + actor + time → transactional domain function
→ outcome → HTTP mapping. The [API guide](../experiments/api-slice/README.md)
records the workflow, boundary cases, build-loop measurements, and limitations.

Creation is SQLite-only; the shared schema migration is exercised by both
database experiments. Development identity remains caller-controlled. Tokens are
returned directly only to make this demo testable; email, idempotency, recovery
after a lost response, and real authentication remain future work.

**Status vocabulary:** Accepted direction means agreement on the project's
direction, not an irreversible technical commitment. Proposed means a candidate
to test. Open means not yet decided. Deferred means intentionally outside the
first experiment, not permanently rejected.

## Accepted direction

### D01 — Build our own framework from ecosystem crates

This is a personal learning project. Designing conventions and assembling the
foundation is part of the enjoyment and purpose. Start with existing crates;
consider original crates later when experience reveals a worthwhile
responsibility.

Loco and other Rust frameworks are references, not the selected foundation. The
earlier recommendation to build a Loco profile was superseded by this choice.
There is no current plan to fork an existing framework.

Tradeoff: we take responsibility for integration, upgrades, diagnostics, and
test infrastructure even when other projects already provide those features.

### D02 — Borrow Rails DX, not Ruby's programming model

Value predictable structure, useful defaults, generators, a coherent CLI, easy
tests, and a path from a new app to deployment. Do not reproduce dynamic models,
runtime metaprogramming, or hidden callbacks simply because Rails uses them.

Study other frameworks' decisions and rejected alternatives, not just their
surface features. Favor readable application-owned code and explicit execution.

### D03 — API-first with React as the first consumer

The framework should power more than browser applications. React is the
preferred UI technology, not a reason to make the backend React-specific.

JSON HTTP APIs are the initial direction. Inertia was considered but is not the
initial transport model. Dioxus is a tooling and architecture reference, not a
replacement for React. Server rendering, Rust browser code, and multiple
frontend modes are not initial requirements.

### D04 — Reuse standards and generate useful artifacts

OpenAPI is a central candidate for connecting server contracts, documentation,
client generation, and verification. Do not invent a proprietary protocol
without a demonstrated benefit.

JSON:API is a separate standard, not a synonym for JSON APIs or OpenAPI.
Adoption of JSON:API has not been decided.

### D05 — Verification is a framework capability

Make it easy to state intended behavior independently of implementation and
obtain evidence about that behavior. Combine compiler checks, contract tests,
business scenarios, properties, real-database tests, concurrency/failure tests,
and selected end-to-end workflows. Investigate mutation testing once useful
tests exist.

Generated code and generated tests can share the same misunderstanding. Neither
coverage nor a green compiler is a correctness certificate. Reports should name
what was verified and what remains unverified.

### D06 — Embedded-first experience; engine choice stays explicit

Compare SQLite and local Turso for low-friction startup, isolated tests,
disposable experiments, and deployment options. Embedded databases are
legitimate application databases, not merely temporary substitutes for server
databases.

PostgreSQL is the owner's preferred server database and remains a future
direction, but the initial PostgreSQL-only proposal was superseded. Support has
not yet been implemented or promised for any backend.

Tests against SQLite/Turso do not prove PostgreSQL locking, isolation, type,
migration, or SQL behavior. An application must verify against its deployment
engine. Multiple supported engines require tests against each.

Native Turso and SQLx's SQLite driver are distinct implementations. File/SQL
compatibility does not make their drivers interchangeable. Avoid a universal
persistence interface before concrete differences are understood.

### D07 — Measure the feedback loop

Treat compile and test latency as design constraints. Distinguish reducing work,
reusing compiled work, cheaper linking, and avoiding restart/full-link work.

Study Dioxus's tooling and experimental Subsecond approach. A fast development
loop must coexist with clean-build and fresh-process verification. Do not infer
build speed from implementation language or assume additional crates improve it.

## Proposed architecture, not yet selected

| Proposal                                                                  | Reason to investigate                                                        | Evidence still needed                                                  |
| ------------------------------------------------------------------------- | ---------------------------------------------------------------------------- | ---------------------------------------------------------------------- |
| Named application actions independent of HTTP                             | Reuse behavior from APIs, jobs, CLI, and tests; borrow Ash's domain boundary | Sketch actual feature code before defining generic traits              |
| Axum + Tokio                                                              | Explicit HTTP integration and ecosystem middleware                           | Compile and exercise the first API slice                               |
| SQLx + SQLite baseline                                                    | Pooling, migrations, test support, optional checked SQL                      | Validate selected release/features and measure build cost              |
| Native local Turso comparison                                             | Embedded native-Rust async engine and an interesting integration experiment  | Exact released-driver behavior, migrations, concurrency, build costs   |
| Rust contracts plus explicit routes generate OpenAPI                      | Keep Rust tooling and derive useful client artifacts                         | Compare utoipa/utoipa-axum and aide on real output                     |
| Separate persistence types and external DTOs where their contracts differ | Avoid leaking storage shape or sensitive fields                              | Avoid multiplying identical structs without a concrete boundary        |
| Stable structured API errors, possibly RFC 9457 Problem Details           | Predictable clients and testable failure behavior                            | Choose error codes, statuses, disclosure policy, and generator support |
| One supervised development command                                        | Coordinate backend, React, contract generation, and diagnostics              | Start with existing tools; define lifecycle and error behavior         |

Status annotated September 27, 2026; the rows above are unchanged. Each status
names the later record that exercised, narrowed or deferred the proposal.
Settlements apply to the experiments or the reference application, not to a
framework default.

- Named application actions independent of HTTP. _Status:_ exercised as a
  convention, not a framework API. The API slice's original invitation actions
  are plain transaction-owning functions without HTTP types, and the second
  justified no generic trait
  ([second-action follow-up](#second-action-follow-up--september-24-2026)). S16
  and the reference application keep ordinary functions with explicit utoipa
  registration
  ([bounded S16 integration](#bounded-s16-integration--september-26-2026)). A
  generic `Action` trait stays [deferred](#deferred-infrastructure).
- Axum + Tokio. _Status:_ exercised: the API slice compiled and ran on them
  ([API follow-up](#api-follow-up--september-24-2026)), and S16 and the
  reference application use them. Exact versions live in `Cargo.lock`. This
  serves the experiments and the reference application, not a released
  framework.
- SQLx + SQLite baseline. _Status:_ narrowed. The embedded database spike
  measured its build cost beside Turso and found its migration tooling included
  ([findings](embedded-db-findings.md)); it serves the API slice, S16 and the
  reference application as a provisional baseline, not a permanent default
  ([experiment follow-up](#experiment-follow-up--september-24-2026)).
  Session-store pools and migrations are exercised; compile-time checked SQL is
  not used. Exact pins live in `Cargo.lock`.
- Native local Turso comparison. _Status:_ compared in the original spike: it
  passed the same ten scenarios, including the concurrent-acceptance race, while
  its clean test build took about three times as long
  ([findings](embedded-db-findings.md)). Its small custom migrator, since
  extended to the shared second migration
  ([second-action follow-up](#second-action-follow-up--september-24-2026)),
  lacks SQLx's checksum and history tooling. The API slice, S16 and the
  reference application use SQLite, and S17 excludes Turso support
  ([S17 exclusions](design-spec.md#explicit-exclusions)).
- Rust contracts plus explicit routes generate OpenAPI. _Status:_ narrowed.
  utoipa/utoipa-axum and aide were compared on the same handlers and passed the
  same checks, and utoipa became provisional
  ([API follow-up](#api-follow-up--september-24-2026)). The reference
  application exports with utoipa alone; aide remains only in the frozen
  comparison ([S17 ownership boundaries](design-spec.md#ownership-boundaries)).
- Separate persistence types and external DTOs where their contracts differ.
  _Status:_ exercised through domain/wire separation and explicit projections,
  not as a framework rule or a separate persistence-model convention. Domain
  types carry no HTTP types; the HTTP adapter owns wire DTOs and projections,
  with string IDs on the wire and integer IDs in the domain
  ([S12](design-spec.md#what-is-defined-where),
  [S17 ownership boundaries](design-spec.md#ownership-boundaries)).
  Project-member reads return `user_id`, `display_name` and `role`; stored email
  never appears
  ([S17 public response policy](design-spec.md#public-response-policy)).
- Stable structured API errors, possibly RFC 9457 Problem Details. _Status:_
  narrowed; RFC 9457 is not adopted. The API slice uses `{code,message}` errors
  ([API follow-up](#api-follow-up--september-24-2026)). The reference
  application's domain operations use the S16-tested subset of S15's versioned
  envelope with namespaced codes and a declared shared profile, while the
  session endpoints keep `{code,message}`
  ([S17 public response policy](design-spec.md#public-response-policy)).
  Existence hiding is decided per concealed pair (see the
  [open decisions](#open-decisions)); a framework-wide error-code policy remains
  open ([S11](design-spec.md#s11--open-design-agenda)).
- One supervised development command. _Status:_ decided for the reference
  application, not yet implemented: a Node supervisor script starts the issuer,
  API and Vite, with persistent storage, seed policy, journal mode and task
  supervision settled alongside it
  ([S18 owner decisions](design-spec.md#lifecycle-owner-decisions)). The
  reference application has a development server and a browser runner that owns
  its processes, not yet one command for the issuer, API and Vite.

## Deferred infrastructure

- Generic `Action` trait, resource DSL, broad procedural-macro API.
- Ash-style policy expression engine and automatic policy-to-query translation.
- Universal ORM/database/transaction interface and plugin system.
- Multiple frontend modes, Inertia integration, and SSR.
- Hot patching as a required development mechanism.
- Broad queue/storage/mail abstractions before the relevant feature exists.

These may become valuable. The initial constraint is to demonstrate their need
with ordinary code rather than design the extension system first.

## Open decisions

Status annotated September 27, 2026; the original questions are unchanged. Each
status names the later record that settled or narrowed the question, and how
far: most settlements apply to the experiments or the reference application, not
to a framework default.

- SQLite versus Turso as default; precise driver, release, features, and
  migration tools. _Status:_ narrowed. SQLx/SQLite serves the experiments and
  the reference application, not as a permanent default
  ([experiment follow-up](#experiment-follow-up--september-24-2026)); exact pins
  live in `Cargo.lock`, not framework promises. The default engine remains open.
- Authentication mechanism, credential storage, and CSRF defenses. Same-origin
  HttpOnly-cookie authentication was suggested, not accepted as the final
  design. _Status:_ selected for the experiment, not a released framework:
  same-origin server-side sessions with an HttpOnly cookie, OIDC authorization
  code with PKCE, and a session-bound CSRF token
  ([authentication experiment](#authentication-experiment--selected-september-24-2026)).
  The reference application copies the session module as application code
  ([S17 ownership boundaries](design-spec.md#ownership-boundaries)).
  Real-provider verification and a durable account lifecycle remain deferred.
- Project/tenant model and where tenant authorization is enforced; RLS is not
  selected. _Status:_ open as a framework question. The experiments and the
  reference application use project memberships checked by application code:
  authority checks run in the same serialized transaction as their mutation
  ([S05](design-spec.md#s05--identity-authority-transactions-and-side-effects)),
  and reads use explicit visibility SQL re-evaluated for every page
  ([S17 read conventions](design-spec.md#read-conventions)). RLS and a policy
  engine remain unselected.
- Invitation identity: verified account email, user ID, or another policy; how
  email changes and normalization affect acceptance. _Status:_ settled for the
  experiments. Acceptance is bound to user ID, not email; delivery snapshots the
  recipient's test address, so a contact change does not redirect an existing
  invitation
  ([local delivery follow-up](#local-delivery-follow-up--september-24-2026)).
  Email-based identity linking is not implemented.
- Whether repeat acceptance is idempotent success or a documented conflict.
  _Status:_ settled for the experiments as a documented conflict: repeat
  acceptance returns `AlreadyAccepted`, 409 `already_accepted` over HTTP
  ([experiment follow-up](#experiment-follow-up--september-24-2026)).
- Which errors intentionally hide resource or token existence. _Status:_ decided
  per concealed pair, not as a blanket rule. In the API slice, another
  recipient's token gets the same 404 `not_found` as an unknown one
  ([API guide](../experiments/api-slice/README.md#contract-choices-and-evidence)).
  In the reference application, member listing gives an unknown project and a
  non-member the same 403, the mutations conceal unknown projects from
  non-owners, and an authorized owner can still learn absence through
  `memberships.member_not_found`
  ([S17 public response policy](design-spec.md#public-response-policy)).
  Invitations in the reference application await their own design.
- Clock semantics at expiration and under transaction retries. _Status:_
  expiration settled for the experiments: one fixed timestamp per attempt, and
  equality counts as expired
  ([experiment follow-up](#experiment-follow-up--september-24-2026)). The
  invitation and membership actions have no automatic transaction-retry policy;
  clock semantics for such a policy remain open.
- OpenAPI version/library, client generator, and how runtime contract checks
  run. _Status:_ narrowed. utoipa is provisional after both exporters passed the
  same checks ([API follow-up](#api-follow-up--september-24-2026)), and the
  reference application exports OpenAPI 3.1 with utoipa alone.
  openapi-typescript generates static types, and the S16 and reference clients
  validate whole responses with Ajv against the export
  ([bounded S16 integration](#bounded-s16-integration--september-26-2026)).
- Whether durable email delivery enters the first slice or the next one; outbox
  design if committing database state must reliably result in delivery.
  _Status:_ settled: it entered the next slice as a SQLite transactional outbox
  with fenced leases and bounded retries. The demos use disposable databases, so
  this is not production durability
  ([local delivery follow-up](#local-delivery-follow-up--september-24-2026)).
- Packaging, crate layout, CLI name, framework name, license, and release
  strategy. _Status:_ open. The reference application's shared crate,
  `crates/iris`, is private and provisionally named; its presence is no
  packaging decision
  ([S17 ownership boundaries](design-spec.md#ownership-boundaries)).

## Authentication experiment — selected September 24, 2026

This supersedes the earlier open authentication proposal for the experiment, not
for a released framework. Use same-origin server-side sessions with an HttpOnly
cookie and OIDC authorization-code flow with PKCE. Rust owns identity
verification; React receives local user IDs and a session-bound CSRF token,
never provider tokens. Map `(issuer, subject)` to existing local users; do not
link accounts by email.

Use `openidconnect 4.0.1` and `tower-sessions 0.15.0`. Keep a small SQLx 0.9
SQLite session adapter because the published SQLx adapter and axum-login
versions use older, incompatible session/SQLx types. Its update-only saves also
prevent stale writes from recreating logged-out sessions. Keep protocol attempts
in their own table for atomic one-use consumption, not a session JSON map.

Select fixed eight-hour authenticated sessions, ten-minute anonymous sessions
and login attempts, and logout of this session only. Login promotion and logout
compete atomically for the old browser session row. A callback that loses to
logout fails; already-authorized domain actions are not cancelled. Provider SSO
and other browser sessions are unaffected.

Start with a credential-free local issuer and allowlisted Alice/Bob subjects.
This tests signed tokens and browser behavior without granting real identity
assurance. A real-provider smoke test, deployment, and durable account lifecycle
remain deferred. See the
[implementation and verification record](../experiments/api-slice/authentication.md).

## Local delivery follow-up — September 24, 2026

Real-provider authentication remains unverified and is deferred to a stable
callback environment. Instead, exercise delivery to existing local users using
Lettre 0.11.23, Mailpit 1.31.2, and a SQLite transactional outbox. No real
email, signup, email-based identity linking, generic queue framework, or resend
API.

Both HTTP demos now commit invitation and delivery payload together. A 201 means
queued, not sent. Delivery uses a snapshot of the user's test address; changing
a contact does not redirect an existing invitation. Automatic retries reuse its
token and Message-ID. Delivery is not exactly-once: SMTP acceptance followed by
lost acknowledgement can duplicate mail. Five attempts and a 30-second fenced
lease bound recovery; expired/accepted invitations are not newly claimed.
Pending plaintext credentials are cleared on terminal cleanup, which is not
secure disk erasure. Demo database restarts still discard data.

This supersedes earlier statements that HTTP issuance stores only hashes or has
no email delivery. Acceptance remains bound to user ID, not email. SQLite-only
migrations are combined with shared migrations in one SQLx ledger; Turso's
historical comparison is unchanged. See the
[delivery record](../experiments/api-slice/delivery.md).

## Agent verification interface — September 24, 2026

Optimize for time to an independently verified change, not generated code
volume. Conventions should distinguish compiler/database enforcement, CI checks,
and documented expectations. Rust types alone cannot establish business intent.

Start with a shared local verification runner and thin MCP adapter, using the
official TypeScript MCP SDK 1.30.1 and Zod 4.6.5. This is an experiment in
access to evidence, not a framework-language decision. CLI and MCP use the same
check catalog and report schema; no separate MCP implementation of application
logic.

Expose conventions, fixed check plans, controlled scenario reproduction, and
run/operation inspection. Begin with the existing logout/callback race and
outbox recovery tests. Include expected/observed checkpoints, source
fingerprints, versions, evidence completeness and reproduction commands. Do not
present these test checkpoints as production traces or claim causal diagnosis
from partial data.

Keep production access, arbitrary commands/SQL, application generation and broad
profiling out of scope. Allowlisted commands still execute trusted repository
code. Raw logs and credential-bearing payloads are not part of the evidence
format. Reviewers own intent and acceptance criteria; agent-editable tests and
reports are not a protected evaluator.

Next evaluate seeded regressions and valid-change controls against independent
acceptance checks before claiming faster or more reliable agent repairs. See the
[pilot guide](../experiments/agent-interface/README.md) for commands,
verification, limitations, and the proposed study. Librarian research informed
the SDK choice; oracle review informed the evidence and stale-report safeguards.

## Framework design synthesis — September 25, 2026

The owner deferred further productivity experiments in favor of designing
framework features and conventions. The earlier repair-study recommendation is
not the current next task. Independent membership development supplied friction
observations, not a controlled productivity result; its unpushed implementation
remains in a separate checkout at this update.

The [living spec](design-spec.md) records the subsequent discussion: ordinary
transaction-owning actions with shared transport declarations; domain, adapter,
and application boundaries; actor provenance; AI-oriented typed results and
stable error codes; and the separation of action results, execution evidence,
and optional durable invocation receipts. Three concrete failure scenarios
explain why unknown effects must remain unknown and why transient failures do
not automatically authorize retries.

Principles are agreed direction, while example APIs, wire fields, and the exact
Rust error representation remain proposed. In particular, Problem-style JSON is
not the current `{code,message}` response format. The spec records rationale,
alternatives, deferred features and evidence so other agents can critique and
extend the design without treating sketches as implemented guarantees.

## Action-authoring vertical slice — September 25, 2026

**Proposed recommendation; documentation only.** The owner requested one
`memberships.change_role` authoring/API design in a new thread, not framework
implementation.
[S16](design-spec.md#s16--authoring-one-action-and-publishing-its-http-contract)
contains the annotated slice, alternatives, exact edit paths and later checks.
Source inspection started at the pushed
[S15 revision](https://github.com/robertguss/Iris/commit/d960fce9f84388da560864ffa978f391ccf65208).
The [first review synthesis](reviews/opus55-all-01-synthesis.md) retains its
original provenance; this follow-up is not an independent review. Work and
versioned ecosystem research are in the
[authoring thread](https://ampcode.com/threads/T-01a0da64-dc00-76cb-abb5-efba1af7a00d),
following the
[source discussion](https://ampcode.com/threads/T-01a0d13e-b20f-73ee-bed5-747eb2d3346c).

Recommend ordinary transaction-owning functions and explicit utoipa registration
with a narrow shared response bridge first. This refines S03's preference for
typed registration without changing its execution boundary. Runtime rendering
and export should consume the same exhaustive mappings; operation identity,
literal rejection codes, safe projections and recovery constraints each have one
semantic owner. Preserve existing `changeMemberRole` as the OpenAPI ID paired
with the domain name. Trial S15's envelope in isolation, not as an implicit
migration of the existing API.

Alternative: a typed `HttpOperation` value could own routing metadata and
enforce handler/reply compatibility. It still needs the same schema/response
bridge and cannot prove middleware coverage, transaction correctness or
permission. Defer that author-facing API until a real compilation experiment
shows its added checks justify maintaining it. Retaining today's duplicate
response tables is the zero-integration baseline, but leaves recurring drift and
overly broad error schemas. Replacing utoipa with aide is not justified by this
source research; middleware still needs explicit contracts and same-status
branches need merging.

The smallest later experiment is one isolated real Rust membership route,
descriptor-driven runtime/OpenAPI responses, generated TS narrowing and an
executed whole-request validator. Probe an added rejection and changed response
with deliberately omitted edits; assert failures independently of generated
fixtures. Include CSRF/domain-forbidden 403 branches, malformed success, cleanup
failure and post-commit request failure. No experiment was run in this design
pass.

Open: owner agreement on this narrower first step, envelope compatibility and
migration, driver-specific failure/cleanup classification, and runtime validator
selection. Proposed additive-field tolerance does not permit unknown codes or
versions. Receipts, generic executors, resource DSLs, custom macros, runtime
inspectors, automatic retries and productivity studies remain deferred. No
runtime, dependency, public wire or deployment changes accompany this record.

## Bounded S16 integration — September 26, 2026

**Implemented experiment, not a public API migration.** The owner authorized the
small alternative-A experiment after the preceding design pass. The
[execution record](../experiments/api-slice/s16.md) includes commands, the
verification matrix, all omitted-edit diagnostics and limitations. The
[implementation thread](https://ampcode.com/threads/T-01a0dda8-c086-70ba-89e1-2fba483156d3)
contains librarian research and the requested pre-implementation oracle review.

Ordinary transaction-owning functions plus explicit utoipa registration work for
this bounded route. One exhaustive public mapping drives rendering and export;
strum 0.28.0 supplies unit enumeration. The shared 403 response preserves CSRF
refusal separately from finalized domain forbidden. Existing demo routers, wire
contracts, aide/utoipa comparison and React consumer remain unchanged. The only
shared renderer change adds nonserialized error provenance consumed by the
isolated boundary, avoiding inference from status or arbitrary error bodies.

Select Ajv 8.20.0's draft-2020-12 entry point for this experiment, with strict
schema checking and no payload mutation. Keep openapi-typescript 7.13.0 for
static generation. Real exported references, per-status alternatives, additive
fields and the extension's recovery capabilities are exercised. Unknown code,
version, operation, malformed success, empty/HTML bodies and request/body-read
exceptions remain outside the server union. No automatic retry is offered.

Evidence: ten focused Rust tests and 44 whole-request client cases pass. All 16
selected omitted-edit probes are detected and removed from the disposable source
copy. The compiler finds missing enum/mapping/projector edits; independent
contracts find business/route/status/profile errors; regenerated TypeScript
finds the stale 409 consumer after a temporary 422 change. Wrong route paths
still compile. These results support the narrow bridge, not stronger compiler
linkage or a claim that a typed wrapper is now necessary. They measure no
productivity.

The session-store SQL failure after commit is decisive: a valid public 500 can
coexist with a committed role change. Explicit rollback after a real body error
is distinguished from statement rollback using a FAIL-trigger control. Primary
rejection/execution and cleanup causes survive separately, but rollback-failure
observations are injected into finalization rather than actual driver I/O
faults. Real busy-begin and deferred-FK commit failures retain conservative
cleanup uncertainty. No general safe-disposal, pool-reuse, caller-loss or
cross-engine claim follows. Receipts, executors, inspectors, custom macros and B
remain deferred; wire migration and larger-module linkage remain open.

Combined API/SQLite verification passed 43 tests with one existing Mailpit test
ignored. Both previous contract snapshots/clients, web build/breaking-change
check, Clippy and formatting passed. No push, PR, deployment or shared-state
change was part of this experiment.

## Reference application and first reads — September 26, 2026

**Proposed recommendation; documentation only.** After S16, independent
assessments of `9235c6e` by Claude (Opus 5.5) and Astra (GPT-6-Astra through
Codex) found that the S15/S16 contract still has one instance, no domain read
endpoint exists, and both demos use disposable databases. Claude's local rerun
on macOS with Node 24 (CI pins 26) passed:

```sh
cargo test --workspace --locked                                # 48 passed
cargo test --locked -p iris-api-spike --features dev-identity  # 24 passed, 1 ignored
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo fmt --all --check
npm --prefix experiments/api-slice/web run verify
npm --prefix experiments/api-slice/web run verify:s16  # 10 Rust, 44 client cases
npm --prefix experiments/api-slice/web run probe:s16   # all probes detected
```

CI runs neither S16 web command; adding both is a separate prerequisite.

[S17](design-spec.md#s17--reference-application-and-first-reads) recommends a
fresh reference application in the S04 layout instead of extending the
experiments, which become frozen evidence kept green in CI. Utoipa becomes
primary. Framework code starts as one private, provisional `crates/iris` that
receives only behavior two operations demonstrably share; transactions and
cleanup classification stay application-owned. Checkpoint A ports `change_role`
and adds `remove_member` as a control. Checkpoint B adds the first reads (my
projects and project members, with explicit visibility SQL and keyset
pagination) and a React member directory. Invitation issuance and acceptance
follow in a separate design. Domain operations adopt the S16-tested subset of
S15's envelope; the session endpoints keep their contracts.

Astra's read-only review of the first draft found no blockers. Its four major
findings were incorporated: silent component-schema loss when merging OpenAPI
documents, HEAD dispatch to GET handlers, cleanup obligations of read
transactions, and an over-broad client decoder scope. S17 records the full
disposition.

Alternatives considered: migrating the existing demos in place (keeps two live
result models and a legacy React console), extracting a framework crate first
(premature stabilization), and invitations before reads (defers the largest
undesigned area). The owner then settled all seven open choices as recommended:
a uniform 403 hides whether an inaccessible project exists; any member may list
a project's members with display names, never email; HEAD is served as its GET
operation; `change_role` and `remove_member` share a rejection type until their
permitted sets diverge; unknown and duplicate query parameters are rejected;
reads come before invitations; and frozen experiments stay in CI until the
reference application covers their evidence, with each retirement recorded here.
Still open: authorization to implement checkpoint A or the CI prerequisite, the
invitation design, and naming and packaging. Persistence, seed policy, workers
and a development command remain for a separate lifecycle pass. No runtime,
dependency, CI or wire change accompanies this record.

## S16 checks in CI — September 26, 2026

**Implemented CI change; not yet observed on GitHub Actions.** The owner
authorized the next chunk: the CI prerequisite, then S17 checkpoint A, on a
branch from `docs/s17-reference-app`, without a push or a merge into `main`. The
workflow's web verification step now runs `verify:s16` after `verify`, and a new
step runs `probe:s16` before the agent-interface checks. Toolchain pins, caching
and triggers are unchanged.

Local run on macOS with Node 26.8.1 (CI pins 26.10.0) and npm 11.19.0 (CI pins
10.9.9), using the exact step commands:

```sh
npm --prefix experiments/api-slice/web run verify      # passed, 7 s
npm --prefix experiments/api-slice/web run verify:s16  # 10 Rust, 44 client cases, 6 s
npm --prefix experiments/api-slice/web run probe:s16   # all 16 probes caught, 34 s
```

`verify:s16` also passed under Node 24.20.0. The timings were taken with a warm
local `target/` and do not predict CI time. The workflow runs on pull requests
and pushes to `main`, so pushing this branch alone would not trigger it; an
observed Actions run remains outstanding. No application, dependency or wire
change accompanies this record.

## Reference application checkpoint A, server side — September 26, 2026

**Implemented experiment; checkpoint A remains partial.** Under the owner's
authorization for the CI prerequisite and checkpoint A, the branch
`s17-checkpoint-a` added `apps/reference` and `crates/iris` in five reviewed
steps. Astra reviewed each plan and diff before its commit. The
[S17 evidence](design-spec.md#checkpoint-a-server-side-evidence) summarizes the
result, and the [guide](../apps/reference/README.md) owns the commands.

Local results on macOS with Rust 1.98.1 and Node 24.20.0:

```sh
cargo test --workspace --locked                                # 81 passed
cargo test --locked -p iris                                    # 9 passed
cargo test --locked -p iris-reference                          # 24 passed
cargo test --locked -p iris-api-spike --features dev-identity  # 24 passed, 1 ignored
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo fmt --all --check
node apps/reference/scripts/probes.mjs  # 6 caught, 3 controls, 39 s
```

The frozen web `verify`, `verify:s16` and `probe:s16` stayed green throughout.
The probe time was measured locally with a cold, isolated target; the probes add
an independent build to CI whose duration there is unmeasured.

Decisions and the alternatives not taken:

- `change_role` and `remove_member` share a private mutation, as S04 permits,
  rather than duplicating S16's transaction for removal. Only the optional-role
  guard and the update-or-delete step differ.
- Each operation's response contract is an `Operation` declaration that
  rendering, export and assembly checks all read. Registration stays an explicit
  `routes!` call, so S16 alternative B remains unadopted.
- One checked assembly serves the application and its export. It merges
  documents after conflict checks and keeps operation routers separate, rather
  than using `OpenApiRouter::merge`, so each boundary wraps its own session
  layer and export needs no identity provider.
- Extraction: envelope rendering, the response bridge, the shared profile, the
  request-ID boundary and the assembly checks moved to `crates/iris` because
  both operations use them. Wire-ID parsing stays in the application: the rule
  permits moving it but does not require it.
- Unmatched routes return a plain 404 outside the session layers, and the
  session error enum keeps only the codes identity emits.

No frozen experiment was retired. S16's client harness is not yet ported, and
the reference application's checkpoint A browser workflow is outstanding, so S16
stays in CI.

The new probe runner builds in its own temporary target. S16's `probe:s16`
builds into the checkout's shared `target/`; Astra reproduced in scratch that
this can leave a mutated build artifact that a later run treats as current. The
experiment stays frozen, so this is recorded as a known risk rather than fixed.
The isolation was later authorized and made
([S16 mutation builds isolated](#s16-mutation-builds-isolated--october-2-2026)).

Limitations: the busy classification of a failed connection open is
source-inspected; checkpoint A's client and browser acceptance, HEAD and GET
behavior, Linux and GitHub Actions runs remain outstanding. No productivity
claim follows.

## Reference application checkpoint A, client — September 27, 2026

**Implemented experiment; checkpoint A is complete.** Under the same
authorization, three further steps on `s17-checkpoint-a` added the reference
client, a development server and one browser workflow. Astra reviewed each plan
and diff before its commit. The
[S17 client evidence](design-spec.md#checkpoint-a-client-evidence) summarizes
the result, and the [guide](../apps/reference/README.md) owns the commands.

Local results on macOS with Rust 1.98.1, Node 24.20.0, `agent-browser` 0.38.1
and headless Chrome 154:

```sh
cargo test --workspace --locked                   # 83 passed
cargo test --locked -p iris-reference             # 26 passed
npm --prefix apps/reference/web run verify        # 122 decoder, 24 presentation cases; build
node apps/reference/scripts/browser.mjs           # passed; owns its processes and ports
```

`verify` also passed under Node 26.8.1; `npm@10.9.9 ci` (CI's pin) installed the
lockfile. The frozen web `verify` and `verify:s16` stayed green. CI now runs the
client verification and the browser workflow; no GitHub Actions run has been
observed.

Decisions and the alternatives not taken:

- The client bundles the committed export instead of fetching
  `/api/openapi.json`, which the application does not serve; adding it would
  have been a wire change. Client and server must therefore ship together.
- The boundary accepts exactly the export's `x-iris` POST operations and
  validates each response against its own operation's schemas; a second
  handwritten operation list in the client is checked against the export rather
  than trusted.
- Bodies are read once, up to 64 KiB, as strict UTF-8. Oversize bodies get a new
  `client_unknown` reason, `body_oversize`, beside S16's five; locked, consumed
  or partly read bodies are `body_unreadable`, as S16's `text()` call implied.
- Session endpoints use plain `fetch` under their existing contracts, without
  `openapi-fetch` or runtime validation.
- A 500, a 503 and `client_unknown` are all presented as unconfirmed, with no
  effect claim, no retry and no readback, which this checkpoint lacks.
- The development server seeds one demo editor itself rather than changing the
  shared test seeds, which the membership tests already extend.
- The lockfile was resolved with `npm install --before=2026-09-24` so every
  package matches the experiment's lockfile, instead of taking a newer
  transitive `rolldown`.

Mutations seeded while building, each failing its intended check: no streaming
limit, no declared-length pre-check, no unusable-body check, either operation's
validator accepted, generated-type drift, widened response types, no linkage or
domain-schema check, and lost or short fixture capture (client); a missing flag
check, unmounted domain routes, a missing outcome arm and an overclaiming 500
(development server and presentation); and, in the browser, a missing demo
membership, overclaiming unconfirmed wording, a retry after a lost response and
a missing CSRF header. The workflow's own process handling was probed too: a
taken port, foreign servers on every port, a spawn error, SIGTERM, a child that
never reports ready, a child killed mid-run and a build that ignores SIGTERM
each stop it without a PASS or leftover processes, and a second invocation is
refused without disturbing the first.

No frozen experiment was retired. S16's client harness is now reproduced for
both operations, but its other omission probes (compile-time omissions,
compatibility and regeneration drift, the projector, and the CSRF profile
branches) are not, so S16 stays in CI.

Limitations: macOS and one headless browser only; the browser workflow checks
selected paths; Ajv's runtime compilation needs `unsafe-eval` under a strict
content security policy; no GitHub Actions run. No productivity claim follows.

## Reference application checkpoint B, server side — September 27, 2026

**Implemented experiment; checkpoint B is partial.** On September 27, 2026, the
owner authorized checkpoint B, and approved publishing `s17-checkpoint-a` and
opening a pull request so the workflow would run on GitHub Actions, without a
merge. Draft pull request #1 ran it at `a0c25ff`, with checkpoint A complete:
every step passed on Ubuntu with Node 26.10.0 in 7 min 37 s, including the S16
checks, the reference client verification (23 s) and the browser workflow (15
s). This is the first observed run of those checks. Three further steps on the
branch, each plan- and diff-reviewed by Astra before its commit, added
checkpoint B's server side; they have not been pushed. The
[S17 evidence](design-spec.md#checkpoint-b-server-side-evidence) summarizes the
result, and the [guide](../apps/reference/README.md) owns the commands.

Local results on macOS with Rust 1.98.1 and Node 24.20.0:

```sh
cargo test --workspace --locked                   # 106 passed; iris 13, iris-reference 45
npm --prefix apps/reference/web run verify        # 143 runtime cases across 4 operations; build
node apps/reference/scripts/probes.mjs            # 10 caught, 7 controls; 1 min 24 s cold
node apps/reference/scripts/browser.mjs           # checkpoint A workflow; nine passes at 5b16132
```

Clippy with `-D warnings`, rustfmt, the dev-identity suite (24 passed, 1
ignored), and the frozen web `verify` and `verify:s16` stayed green. With the
reads' schemas the production bundle is 382.39 kB (112.55 kB gzip), and
compiling four operations' validators took 17.5–21.0 ms (median 19.5 ms) across
those nine browser runs.

Decisions and the alternatives not taken:

- This chunk covers checkpoint B's server side, as checkpoint A was split; the
  previous handoff proposed all of B at once. Each export change still forces
  client linkage, so the client accepts the reads without read UI.
- Reads run through an application-owned `read::run` and `ReadError`, not
  `crates/iris`, because cleanup classification stays with the application
  (S17). A read owns its connection instead of borrowing it, so `query_only` and
  an unfinished transaction cannot reach a mutation.
- `crates/iris` changed only where both reads need it: HEAD on a GET operation,
  and reads omitting `recovery` rather than declaring an explicit "none".
- `user_contacts` joined the one consolidated migration instead of a second
  migration, since every database is disposable.
- `limit` is lexically strict (`05` is refused) and exported as a pattern. The
  cursor is an unsigned, versioned `c1.` position; authority comes from the
  actor, so signing adds nothing here.
- Member listing has its own one-refusal rejection type that reuses the single
  `memberships.forbidden` descriptor; own-project listing has an uninhabited
  rejection type and declares no 403.
- The mutations do not yet declare `listProjectMembers` as their current-state
  read (S15 says "may"); that choice is left to the client chunk.

Mutations seeded while building, each failing its intended check:

- `crates/iris`: no HEAD rule; `recovery` forced into a read.
- Read module: a rejection projected despite a failed rollback; `query_only`
  off.
- Member listing: visibility dropped or narrowed to owners; the page not scoped
  to its project; the limit widened to 101 or accepting `05`; the cursor version
  unchecked; the next position off by one; input checked before the session; no
  HEAD rule at the application level; a changed forbidden descriptor; a read
  failure reported as busy.
- Own-project listing: the actor filter dropped; the key position ignored; the
  order reversed; the role taken from another member's row; the limit parsed
  without the shared rule; input checked before the session; a failure reported
  as busy; a mismatched declared handler identity.

One mutation survived as equivalent: raising the cursor's 32-byte bound cannot
change behavior while keys are canonical i64 values, so the bound is documented
rather than observed. The probe harness was also checked against itself: a probe
whose edit changed nothing made it fail.

Limitations: these steps ran on macOS only and have not run on GitHub Actions;
the reads' 503 captures use an injected actor without a cookie; cancellation is
shown for the read future, not an HTTP disconnect; only SQLite's
rollback-journal mode was exercised; there is no read UI or browser acceptance
yet. No frozen experiment was retired. No productivity claim follows.

## Reference application checkpoint B, client — September 27, 2026

**Implemented experiment; checkpoint B is complete.** Under the owner's
authorization of checkpoint B, three further steps on `s17-checkpoint-a` added
the reads to the reference client, replaced the checkpoint A console's ID form
with a member directory, and added checkpoint B's browser workflow. Astra
reviewed each plan and diff before its commit. The
[S17 client evidence](design-spec.md#checkpoint-b-client-evidence) summarizes
the result, and the [guide](../apps/reference/README.md) owns the commands.

Local results on macOS with Rust 1.98.1, Node 24.20.0, `agent-browser` 0.38.1
and headless Chrome 154:

```sh
npm --prefix apps/reference/web run verify   # 231 runtime cases, 49 presentations, 9 requests, 11 directory transitions; build
node apps/reference/scripts/browser.mjs      # checkpoint A, API restart, checkpoint B; five consecutive passes, about 13 s each
```

`verify` also passed under Node 26.8.1. No Rust, export or seed changed, so the
workspace results in the server-side entry still describe the server (106
tests). The production bundle is 389.42 kB (114.41 kB gzip), and compiling the
four operations' validators took 18.0–20.0 ms (median 19.1 ms) across the final
runs of both workflows.

Decisions and the alternatives not taken:

- `read` sits beside `send` in `membership.ts`, with one overload per read. A
  caller holding either read must narrow it first, because a union would drop
  the members' path parameter and their 403. A generic signature, first
  proposed, accepted a union-typed caller without a project ID.
- Read presentation projects rows to their declared fields and scopes every
  sentence to the page as read. An empty page says "No members on this page when
  it was read", never "no members", since a continuation can be empty while
  earlier pages had rows. A failed read says it was not loaded and that no retry
  was sent.
- The directory's state is pure transitions in `directory.ts`, tested under
  Node, because the client has no DOM test harness. Tokens come from one counter
  that nothing resets, and a result is accepted only under its own token, so a
  late page never lands under another project, list or session.
- An attempt marks a shown listing of its project when it is sent, whatever its
  outcome, and an accepted page is marked when an attempt followed its request.
  This records ordering only, and the note says the listing "may predate your
  last attempt". "Listed before your last change", first proposed, implied a
  change even after an unconfirmed attempt. At send time only a shown members
  listing of the attempt's project is marked; either list whose pending request
  preceded an attempt is marked when that result arrives. The projects list's
  wording is scoped to its read.
- No read follows a mutation automatically, and no read starts while an attempt
  is in flight. A reload is a new observation and never rewrites an attempt's
  outcome.
- An attempt is recorded when sent: its operation, its target and later its
  outcome. Only the next attempt or a session change replaces it; a same-user or
  failed refresh keeps it. Removal confirmation belongs to one member and
  action.
- Next-page navigation uses a rows-per-page select (1, 10 or 50) rather than new
  seed rows.
- The browser runner restarts the API once between the two workflows, so
  checkpoint B starts from the seed data. Any other server exit still stops the
  run, and nothing starts once a stop has begun. Not taken: changing the
  development seed; a second API and preview; two full start and cleanup cycles.
- The mutations still do not declare `listProjectMembers` as their current-state
  read (S15 says "may"). The owner has not decided.

Mutations seeded while building, each failing its intended check:

- Reads (17): a read's 403 schema accepting CSRF refusal; `presentRead`
  spreading the raw item, claiming every member was listed, saying "No members."
  on an empty page, or naming non-membership in its 403 title; `read` sending
  POST, adding the CSRF header, dropping the cursor, leaving the project ID
  unencoded, or always sending `limit`; a member summary widened with `email`,
  the narrowing's CSRF literal widened, `ReadParams` making the project ID
  optional, or an overload admitting an unnarrowed read; `READS` omitting a
  read; body cases skipped for the reads, or one dropped for a mutation.
- Directory (10): the token check dropped; `reload` continuing the page count;
  an attempt issuing a read, or not marking the listing; a result clearing the
  mark unconditionally; a page-size change skipping the members; opening a
  project keeping the selection; tokens reset on sign-out; `next` while pending;
  a first page keeping the previous rows.
- Page, through the browser (13): a reload clearing the outcome; the stale note
  missing; the attempt marked after its response; confirmation following another
  member; the endpoint following the form; a refresh clearing the attempt;
  sign-out keeping it; a removal sending the wrong member; a second page
  ignoring its cursor; a lost members read or a lost initial projects read
  resent; a refused next page keeping the rows; the projects' page line
  overclaiming.

The runner's API restart was probed on disposable copies. A foreign listener
taking port 3003 inside the restart window fails the run and survives it; the
replacement API killed after it is ready stops the run, naming it; SIGTERM
inside the window stops the run with no replacement started. None printed a PASS
or left a process, port or browser session behind.

No frozen experiment was retired: S16's remaining omission probes are still not
reproduced, so S16 stays in CI.

Limitations: macOS and one headless browser only; GitHub Actions has not run the
checkpoint B commits, because pushing them awaits the owner; the workflows check
selected paths, not every code; the restart window was probed with an injected
delay in a copy of the runner; one step 2 browser run measured a 31.1 ms
compile, not investigated. Bob's refused next page shows authorization on a new
read; it is not evidence about the removal, which has its own outcome. No
productivity claim follows.

## Reference application current-state read — September 27, 2026

**Implemented experiment.** The owner answered the three questions the
checkpoint B client handoff left open, one at a time:

- The local commits may be pushed to draft PR #1.
- `change_role` and `remove_member` should declare `listProjectMembers` as their
  current-state read, on the terms Astra recommended when asked.
- That declaration is the next chunk.

Two steps on `s17-checkpoint-a` built it: `cfb4d18`, the contract, and
`8b7e68a`, the console and its browser evidence. Astra reviewed each plan and
diff before its commit. The
[S17 evidence](design-spec.md#current-state-read-evidence) summarizes the
result, and the [guide](../apps/reference/README.md) owns the commands.

The push brought draft PR #1 to `06ac967`. GitHub Actions run 36329466284 failed
on its first attempt only in the frozen agent-interface check, at the `members`
scenario reproduction (`runner.test.mjs:79`); the later steps were skipped. A
rerun of the same commit passed every step, including both browser workflows, so
checkpoint B has passed CI. The failure is assessed as a flake; its cause was
not diagnosed. That step passed in every other recorded run and locally at the
same commit, and nothing it depends on changed.

Local results on macOS with Rust 1.98.1, Node 24.20.0, `agent-browser` 0.38.1
and headless Chrome 154:

```sh
cargo test --workspace --locked                # 127 passed; 34 in crates/iris
node apps/reference/scripts/probes.mjs         # every probe, including two new ones
npm --prefix apps/reference/web run verify     # 231 runtime cases, 51 presentations, 13 requests and readbacks, 11 transitions; build
node apps/reference/scripts/browser.mjs        # A, restart, B, restart, C; five consecutive passes, 15 s each
```

Clippy, rustfmt, the dev-identity tests, `verify` under Node 26.8.1, the frozen
S16 checks, the agent-interface test and the OIDC fixture test also passed. The
production bundle is 391.75 kB (115.10 kB gzip), and compiling the validators
took 17.3–29.4 ms (median 19.0 ms) across the five browser runs.

Decisions and the alternatives not taken:

- `recovery.read` is `false` when no read is declared, and otherwise names the
  read's public operation ID with a binding from each of its path parameters to
  a request-body field. It is never `true`, which would name neither the read
  nor its inputs. There is no expression language, cursor or query binding: the
  read is a fresh first page.
- Linkage is checked in `check_catalog` over the assembled document, because it
  spans operations and consumers see that document. Extending `CatalogEntry`
  would have duplicated what the document already holds. The bridge still
  rejects a path parameter bound twice, which its map would otherwise collapse
  silently.
- A bound field's schema must equal the path parameter's. This is deliberately
  conservative: some compatible schemas fail, and no schema subsumption is
  implemented. A required parameter outside the path fails whatever its name. A
  first version matched required parameters by name only, so a required query,
  header or cookie `project_id` passed beside the bound path parameter.
- Structure cannot show which field is right. Binding `user_id` passes every
  check, so a hand-written test pins the relationship. A probe shows assembly
  accepting the swap and the hand-written recovery test rejecting it.
- The client parses recovery structurally and refuses what it does not support.
  Exact values are pinned only in tests. Not taken: hard-coding the expected
  declaration beside `METHODS`.
- An unconfirmed outcome points to reading the members again, in page-scoped
  wording, only when the declared read is one the page knows. A read the page
  has no wording for adds nothing.
- The readback reuses the directory's `openProject`, a first page with no
  cursor, with inputs taken from the attempt's recorded request body through the
  binding. It is offered only while the outcome is unconfirmed, it stays offered
  after a 403, and it never changes the attempt. No read automatically follows a
  mutation.
- To withhold a response, the workflow lets the request reach the server. A
  wrapper records the response and throws, so the page gets a network error, and
  the runner then checks the recorded acknowledgment independently. The existing
  lost-response case aborts the request instead, so it shows nothing about a
  commit.
- The browser runner restarts the API a second time, so a third workflow starts
  from the seed data. With two users and no invitations, "Alice removes Bob" and
  "Bob removes himself" exclude each other in one dataset. The absent-member
  case also could not share checkpoint A's removal without dropping A's 404 from
  a stale row. Not taken: dropping that check; changing the seed.
- Bob's self-removal in checkpoint B became the withheld case. Its next-page 403
  check stays. Checkpoint A now checks that a reload keeps an acknowledged
  outcome, the coverage B's acknowledged self-removal used to give.

Mutations seeded while building, each failing its intended check:

- Contract (34): each linkage predicate dropped or weakened, each caught by its
  own case. The target-read predicates are isolated from one another, and the
  name-only parameter match is caught by its regression case. Also: an
  undeclared read rendered `true`; a duplicate binding collapsing; the linkage
  never checked; the reference binding `user_id`, or declaring no read; each
  clause of the client's recovery parse; and `api.recovery` accepting a read.
- Console (11): the pointer dropped; wording claiming the read confirms; wording
  for any declared read; `readback` hard-coding `project_id` or ignoring the
  declaration. Through the browser: the readback reading the open project,
  clearing the attempt, continuing from the shown page's cursor, being hidden
  after a 403, or being offered for any outcome. And a withheld real 409, which
  must fail the runner's status check.

The second restart was probed on disposable copies, through the window after the
second API exits. A foreign listener on port 3003 fails the run and survives it.
The second replacement, killed once ready, stops the run, naming it. SIGTERM in
the window starts no third API. None printed a PASS or left a process, port or
browser session behind.

No frozen experiment was retired.

Limitations: macOS and one headless browser only. GitHub Actions has not run
`cfb4d18` or `8b7e68a`, which are not pushed. The runner receives the
acknowledgment before withholding it, so the cases show retained client
uncertainty after a commit, not a real disconnect, cancellation or server
continuation. The workflows check selected paths. One compile measured 29.4 ms,
not investigated. No productivity claim follows.

## Documentation hygiene — September 27, 2026

**Documentation only; no design decision changed.** The owner chose this pass as
the chunk after the current-state read.

GitHub Actions run 36345360713 tested `12227aa`, the current-state read's
documentation commit, on draft pull request #1. It passed every step on its
first attempt, including the reference browser workflow's three workflows. The
[current-state read entry](#reference-application-current-state-read--september-27-2026)
recorded that GitHub Actions had not yet run its commits; it stays as written,
and this run closes that gap.

The top-level README now lists the reference application and describes S16 and
S17 under its next milestone. The API guide's absent list names production email
delivery instead of durable email delivery; the delivery experiment provides
transactional local mail delivery on disposable demo databases. The owner's
choice of this pass authorized that one sentence in a frozen experiment. The
[open decisions](#open-decisions) above gain dated status notes; their questions
are unchanged. The reference guide's CI limit and S17's status paragraph record
the run.

## Architecture table status — September 27, 2026

**Documentation only; no design decision changed.** The owner chose this pass
after the documentation hygiene chunk. The
[proposed architecture table](#proposed-architecture-not-yet-selected) gains
dated status notes, one per row; its rows are unchanged.

## Reference application lifecycle — September 27, 2026

**Proposed recommendation; documentation only.** After pull request #1 merged,
the owner chose the lifecycle pass that S17 deferred, and a new local branch,
`lifecycle-design`, from the handoff commit `8d2cfc7`.
[S18](design-spec.md#s18--reference-application-lifecycle) records current
behavior from source: every database starts fresh and disposable, SQLx 0.9.0
leaves the journal mode unset, the running application never schedules the
session store's cleanup, the development server has no graceful shutdown, and
the frozen delivery worker's callers stop the process if the worker task ends. A
scratch probe outside the checkout, on Python's SQLite 3.53.4 rather than the
application's build, confirmed that in rollback mode an open reader makes a
committing writer fail after its 100 ms busy timeout, while WAL lets it commit.

S18 recommends that storage stay disposable unless the development binary is
given an explicit path argument, not an environment variable, which the browser
runner would pass along. A new database is initialized atomically at a temporary
sibling path and seeded only then, so a restart keeps changed data. A reset
deletes the database files. Migrations become append-only once a persistent
database exists. The journal mode stays rollback and is checked at startup. A
Node supervisor script with the browser runner's process rules starts the
issuer, API and Vite. Session cleanup, and later any worker, runs as a
supervised task with a bounded shutdown. Worker restart policy and send
uncertainty are left to the invitations design.

The oracle's design review of S18 under the design review brief,
[`astra-s18-all-01`](reviews/astra-s18-all-01.md), found no critical
contradiction; its reviewer had also reviewed the step's plan and diff. Its four
findings were accepted: the development command binds Vite's proxy to the API it
owns, shutdown follows one timeline inside the supervisor's kill bound, one API
owns a persistent database at a time, and a migration refusal offers restoration
before a reset. Alternatives are recorded with S18's ten owner choices. No
runtime, dependency, CI, migration or wire change accompanies this record.

The owner then settled the first choice as recommended: the development binary
stays disposable unless given an explicit path. At the owner's request, the
oracle settled the other nine, choosing each recommended option: a command-line
path argument; data in a gitignored `apps/reference/.dev/`; seeds only during
atomic initialization; append-only migrations with a checksum refusal that
offers restoration before a reset; the rollback journal everywhere, checked at
startup; a Node supervisor script as the development command; a supervised
session-cleanup task once storage persists; one API at a time per persistent
database; and an inner drain deadline inside the supervisor's kill bound. The
oracle also reviewed S18, so these nine are not an independent approval. The
owner then authorized implementing S18 in reviewed steps: storage and
initialization, reset and migrations, shutdown and session cleanup, then the
development command. Still not authorized: pushing `lifecycle-design`, a pull
request, and invitations. No runtime, dependency, CI, migration or wire change
accompanies this record.

## Reference application lifecycle storage — September 27, 2026

**Implemented experiment; pushed to `main` after this entry was written.** Under
the owner's authorization of S18, the first step gives the reference
application's development binary an explicit `--database PATH`; without it, data
stays disposable. One process owns a path through an operating-system lock on a
never-deleted sibling file. A new database is built in an owned staging
directory and published by a hard link that cannot replace an existing file;
only a new database is seeded, an existing one is migrated, and any journal mode
other than rollback is refused. The plan review added the ownership record for
staging, refusal of aliases, stale sidecars and reserved names, and closing
connections before the lock is released.

Locally on macOS, `cargo test --workspace --locked` passed 150 tests, including
kill-at-barrier recovery tests in subprocesses; clippy, rustfmt, the web
`verify` and the browser workflow passed, and twelve mutations were each caught.
[S18's evidence](design-spec.md#storage-and-initialization-evidence) lists the
checks and limits. No dependency, contract, CI or frozen-experiment change.
Linux and GitHub Actions had not run when this was written; GitHub Actions run
36364999959 (Verify, on Ubuntu) later passed on `c810f91` on its first attempt.

## Reference application lifecycle reset and migrations — September 30, 2026

**Implemented experiment; pushed to `main` after this entry was written.** The
second step of S18's implementation gives the development binary
`--database PATH --reset`: under the same ownership lock as a start, it deletes
the database and its SQLite sidecars and builds a newly seeded one, so every
session ends. It is refused while a server or an initialization holds the path,
validates everything before deleting anything, and never opens the old database.
A start against a modified applied migration stops with a refusal naming the
database, the version, restoration of the migration and then the reset; startup
never resets.

The plan review found, and reproduced, that a database created at another
database's sidecar name would be deleted by that database's reset. Such names
are now refused for a database, and a start or reset is refused when a sidecar
name shows recognizable evidence of being another database. The reference
guide's manual procedure for moving such a database went through three more
review rounds.

Locally on macOS, `cargo test --workspace --locked` passed 171 tests; clippy,
rustfmt, the web `verify` and the browser workflow passed. Forty-three
mutations, twelve of them the first step's repeated, were each caught.
[S18's evidence](design-spec.md#reset-and-migration-evidence) lists the checks
and limits. No dependency, contract, CI or frozen-experiment change. Linux and
GitHub Actions had not run when this was written; GitHub Actions run 36721898213
(Verify, on Ubuntu) later passed on `66d0608` on its first attempt.

## Reference application lifecycle shutdown and session cleanup — September 30, 2026

**Implemented experiment; pushed to `main` after this entry was written.** The
third step of S18's implementation gives the development binary one shutdown
timeline. On SIGINT or SIGTERM it stops accepting connections and starts no new
periodic work, lets requests and work in flight finish for up to 3 s, then
closes the session pool and waits up to 1 s for every domain connection's
closure to be acknowledged. Only then is the ownership lock released and the
exit code 0. Session cleanup is the first supervised task: it runs at start and
every 60 s, and its panic or unexpected end stops the process with a message and
exit code 1.

The plan review found, and reproduced, that shutting the runtime down does not
establish that SQLx's connections are closed: a second owner could take the lock
while a worker of the first was still inside an update. Domain connections are
therefore counted from before they are opened until an awaited close succeeds,
and when closure is not established (an expired drain, an expired close
deadline, or an opening that failed and left nothing to acknowledge) the process
terminates with exit code 1 while still holding the lock, leaving a disposable
directory behind. That is S18's forced-termination limit, chosen over promising
a closure that cannot be shown. The exit is never an acknowledgment or a
rollback receipt for a request in flight.

Tests were written first and their failing run kept. Locally on macOS,
`cargo test --workspace --locked` passed 198 tests; clippy, rustfmt, the web
`verify` and the browser workflow passed. Thirty-nine mutations were each
caught, as were the previous step's 43, run again.
[S18's evidence](design-spec.md#shutdown-and-session-cleanup-evidence) lists the
checks and limits. One dependency change: Tokio's `signal` feature, adding
`signal-hook-registry` 1.4.8 to the lockfile. No contract, client, CI, migration
or frozen-experiment change. Linux and GitHub Actions had not run when this was
written; GitHub Actions run 36758026534 (Verify, on Ubuntu) later passed on
`a1d51da` on its first attempt.

## Reference application lifecycle development command — September 30, 2026

**Implemented experiment; not pushed.** The fourth and last step of S18's
implementation adds one development command,
`node apps/reference/scripts/dev.mjs`, which starts the local issuer, the
development server on the persistent `apps/reference/.dev/reference.db` and
Vite, and with `--reset` runs the server's own reset. It refuses taken ports
before building, starts each child only once the one before has reported its
address, overrides every inherited address it owns, and stops everything when a
child exits unexpectedly. A signal-requested stop exits 0 whatever the
children's exit codes, so the server's documented exit 1 after an expired drain
is reported rather than treated as a failure; a child that needs SIGKILL makes
it exit 1. The process ownership it shares with the browser runner moved into a
common module.

The plan review found that the browser runner's one-shot signal handlers let a
second Ctrl-C end the supervisor during cleanup, which it reproduced; the
command's handlers persist from its start and cover the build. It also asked for
readiness to be tested with stand-in servers held at gates rather than inferred
from a healthy run. The diff review found, and reproduced, that an empty
`--database` fell back to the default path, so a reset could hit the default
database; that a process group was released when its leader exited, leaving
surviving members running; and that a first reset on a fresh checkout failed for
want of the default directory. All three are fixed and tested.

Tests were written first and their failing run kept. Locally on macOS, the
command's 22 tests passed on Node 24.20.0 and 26.8.1, and ten consecutive times
just before a final two-line change; the browser workflow,
`cargo test --workspace --locked` (198 passed), clippy, rustfmt, the web
`verify` and the omission probes passed. Thirty-eight of 39 mutations were
caught; the survivor removes a stop check that the current callers cannot reach.
[S18's evidence](design-spec.md#development-command-evidence) lists the checks
and limits. No dependency, npm script, Rust, contract, client, CI, migration or
frozen-experiment change; the command's tests are not in CI. Linux and GitHub
Actions have not run on this step.

## S16 mutation builds isolated — October 2, 2026

**Frozen-runner edit, authorized by the owner.** S16's `probe:s16` built into
the checkout's shared `target/`, recorded on September 26 as a known risk rather
than fixed. `run` now sets `CARGO_TARGET_DIR` to the runner's disposable
directory, overriding an inherited target. That one function covers the direct
cargo commands. The regression exercises the nested `s16.mjs` verifier, which
inherits the same environment, and the removal of the runner's copy when a probe
throws. The same private target runs the pristine verifier before the first
mutation and again after the final reset. The copy is still removed in `finally`
on success and on a thrown error. Forced termination is not covered.

The regression was written first. Its corrected run, against the old routing in
a disposable copy of the runner, failed because the nested verifier's cargo
received the fixture repository's `target` rather than the runner temp's. That
target stayed inside the disposable fixture. After the override, the same test
passed, and a copy with the runner's `rm` removed failed because the runner temp
survived the thrown probe. An earlier draft of the test, before this correction,
pointed the old runner at the real checkout and its shim wrote
`target/iris-s16-marker` there. Only that marker was removed. That draft is not
the recorded red run.

Observed on this orb, Node 26.10.0, Rust 1.98.1:

```sh
npm --prefix experiments/api-slice/web run probe:s16
# inherited sentinel target, dedicated temp parent
# 16 CAUGHT, 7 CONTROL, including both pristine verifier runs; exit 0
CARGO_TARGET_DIR=/tmp/rob-1116/healthy-target \
  npm --prefix experiments/api-slice/web run verify:s16
# before and after the probes: 10 Rust tests, 44 client cases; exit 0
```

The sentinel target, the checkout `target`, and the checkout source were
unchanged, and the runner temp was removed. These runs have not been repeated on
GitHub Actions. `probe:s16` runs the regression before the probes. No
dependency, contract, client, CI or schema change. The experiment is not
retired.

## Observable membership caller loss — October 2, 2026

ROB-1110 adds test-only observation at three authenticated `change_role`
boundaries, not a new execution owner. The owned future is aborted before
mutation; complete raw HTTP/1.1 caller sockets are fully closed while held
before commit or after a successful inner response but before exposure. The
outer guard distinguishes `Returned`, non-panicking `Dropped`, and `Panicked`.
Neither loss test releases its hold to obtain the required `Dropped` event;
timeout and a closed observer channel fail. Each then independently awaits
tracked closure, reads the expected role, and commits and reads back a third
role through direct SQL to prove writer progress. The identical-framing
loss-free control checks the entire success envelope.

The new tests passed against unchanged production behavior. Meaningful red
evidence came from exactly four isolated mutants, not missing-symbol failures.
Source was copied to `/tmp/rob-1110/source`, excluding `.git`, `target`,
`node_modules`, `.amp` and `.dev`. Every probe and control set
`CARGO_TARGET_DIR=/tmp/rob-1110/target`; no mutation touched checkout source or
its target. Each mutant was restored before the next; the complete four-test
control passed initially and after each of the four restorations (five green
controls, 20 test passes). Disposable paths describe this run, not an expected
future environment.

All probes used `cargo test --locked -p iris-reference --lib` with the named
`caller_loss::` filter below. Each compiled, ran one test and exited 101 with
one runtime failure; none hung:

| Mutant                                                               | Filter                                 | Decisive failure                                                              |
| -------------------------------------------------------------------- | -------------------------------------- | ----------------------------------------------------------------------------- |
| Move awaited commit before the pre-commit hold, retaining its result | `socket_loss_before_commit`            | `uncommitted write is invisible`: viewer instead of editor                    |
| Skip the role UPDATE                                                 | `same_framing_loss_free_control`       | `commit precedes response exposure`: editor instead of viewer                 |
| Replace commit with rollback, retaining apparent acknowledgment      | `socket_loss_before_response_exposure` | `commit precedes response exposure`: editor instead of viewer                 |
| Await close but suppress its tracked acknowledgment                  | `owned_future_abort_before_mutation`   | Ten-second bounded `Connections.closed()` wait expired; test ended in 10.26 s |

Checks on this Linux x64 orb:

```sh
cargo test --locked -p iris-reference --lib caller_loss
# 4 passed, 0 failed (95 filtered out)
npm --prefix experiments/agent-interface ci
# Dependency preparation only; frozen source unchanged.
cargo test --workspace --locked
# 202 passed, 0 failed, 0 ignored; reference library: 99 passed.
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
# Passed.
cargo fmt --all --check
# Passed.
npm --prefix apps/reference/web ci
npm --prefix apps/reference/web run verify
# 4 Rust captures; 231 runtime, 51 presentation, 13 request/readback,
# 11 directory cases; type checks, export drift checks and build passed.
node apps/reference/scripts/browser.mjs
# Exclusive fixed-port run: all three workflows passed.
npx --yes prettier@3.9.9 --print-width 80 --prose-wrap always --check \
  docs/design-spec.md docs/decisions.md apps/reference/README.md
# Passed.
```

Versions: Rust 1.98.1 (`48a229cea`), Cargo 1.98.1 (`797e8a9bc`), rustfmt
1.9.0-stable (`48a229ceae`), Node 26.10.0, npm 10.9.9, Prettier 3.9.9,
agent-browser 0.38.1; Axum 0.8.9, Hyper 1.11.1, Tokio 1.53.1, SQLx 0.9.0. The
browser run reported a 77.4 ms validator compile; this is one observation, not a
performance comparison.

Limits: these are held observable stages, never an interruption inside commit.
No FIN/RST distinction, removal coverage, continuation guarantee, receipt,
retry, recovery change or production ownership change is claimed. Caller outcome
remains unknown without a validated terminal response. Existing lifecycle tests
remain intact. No contract, dependency, migration, CI or frozen experiment
source changed. This records local Builder verification, not fresh Tester/Oracle
acceptance, GitHub Actions results, merge or deployment.

## Development-command CI and content regression — October 2, 2026

ROB-1112 adds the foreground command
`node --test apps/reference/scripts/test/dev.test.mjs` immediately after
`Verify reference client` in the existing serial CI job, before both browser
workflows. It uses normal failure propagation, with no condition, skip, retry,
background process, deadline or pin change. This supersedes S18's original
local-only CI exclusion; its September 30 observations and historical macOS
measurements remain unchanged.

The approved amendment corrects the relative-database test's inode assumption.
Lead's actual Linux baseline was 21 passed and 1 failed: successful reset reused
inode 1579119. Deletion and recreation permit reuse, so this was not evidence of
a runtime defect. A shell tail had masked the failing Node status; every run
below captured Node's exit code before reading its log tail.

Behavioral assertions were written first. Alice signs in, reads Bob (user 29) in
project 41 as editor, changes him to viewer using session CSRF and an
authenticated POST, requires HTTP 200 and `data.completion = acknowledged`, and
reads viewer. A fresh sign-in after restart reads viewer without rewriting it.
Live reset still fails with the original status/log checks and a working proxy
session; readback must remain viewer. After stopping, reset still succeeds with
the original log checks; a fresh start and sign-in must read editor. The test
explicitly stops successfully and checks that the ports are free. No runtime,
dependency, filter or deadline changes ship.

Mutation diagnostics used disposable source `/tmp/rob-1112-source`, absolute
private target `/tmp/rob-1112-target`, isolated test databases and exclusive
ports 4001, 3003 and 5175. Each mutant was independently restored, never
layered. The checkout's target was not used. The diagnostic command was:

```sh
CARGO_TARGET_DIR=/tmp/rob-1112-target node --test \
  --test-name-pattern='a relative database persists' \
  /tmp/rob-1112-source/apps/reference/scripts/test/dev.test.mjs
```

| Run                                                                                                                                                  | Result                                                                                                          |
| ---------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| Unchanged runtime control before mutations                                                                                                           | Exit 0; 1 passed, 0 failed                                                                                      |
| Persistent binary startup calls `Storage::reset` instead of `Storage::open`                                                                          | Exit 1 at `after restart`: actual editor, expected viewer; initial sentinel setup passed                        |
| Explicit reset calls `Storage::open` instead of `Storage::reset`                                                                                     | Exit 1 at `after stopped reset`: actual viewer, expected editor; live-refusal and success-log checks passed     |
| Reset's `InUse` branch updates Bob to editor with existing SQLx, requires one affected row, closes the connection, then returns the original refusal | Exit 1 at `after refused reset`: actual editor, expected viewer; refusal/status/log/proxy-session checks passed |
| Fully restored runtime control after mutations                                                                                                       | Exit 0; 1 passed, 0 failed                                                                                      |

All three were compiled runtime mutants killed by the intended content
assertions, not compilation, timeout, authentication or port failures. Raw
diffs, logs and exit statuses were retained for Tester review; no mutation
harness or runtime mutant is committed.

Local Linux x64 acceptance used Node 26.10.0, Rust 1.98.1 and npm 10.9.9:

```sh
npm --prefix apps/reference/web ci
# 57 packages installed; 0 vulnerabilities.
CARGO_TARGET_DIR=/tmp/rob-1112-acceptance-target \
  node --test apps/reference/scripts/test/dev.test.mjs
# Exit 0: 22 passed, 0 failed, 0 skipped, 0 cancelled; 138.36 s.
cargo fmt --all --check
git diff --check
npx --yes prettier@3.9.9 --print-width 80 --prose-wrap always --check \
  apps/reference/README.md docs/design-spec.md docs/decisions.md \
  apps/reference/scripts/test/dev.test.mjs
```

Ports were free before and after acceptance. YAML parsing and structural
comparison against the plan commit confirmed that the sole workflow change is
the unconditional foreground step immediately after client verification, with
both browser steps later and all existing configuration unchanged. Markdown/JS
formatting and whitespace checks passed. An additional Prettier check of the
workflow reports existing formatting differences on both the plan baseline and
candidate; that unrelated YAML formatting was left untouched. These are local
Builder results only; Lead must observe the actual Ubuntu Actions run, its
tested SHA, all 22 tests, both later browser steps and the full job before
recording Actions acceptance. No Actions result, independent Tester/Oracle
acceptance, merge or deployment is claimed.

### Colored readiness correction — October 2, 2026

The preceding local results remain valid, but
[Actions run 36961151968](https://github.com/robertguss/Iris/actions/runs/36961151968)
at
[`a4112b682438821ece8a46c7b7a53387715544d7`](https://github.com/robertguss/Iris/commit/a4112b682438821ece8a46c7b7a53387715544d7)
failed with 21 passed and 1 failed. The relative-database/reset test passed. The
readiness test had already observed `dev: ready` and HTTP 200 from the proxy,
but its presence assertion at line 373 searched raw Vite output. ANSI sequences
split both `Local:` and the URL, so the raw regex missed that line. The runtime
supervisor already strips color when scanning readiness.

The correction imports Node's `stripVTControlCharacters` and searches a local
`readinessOutput` snapshot immediately before the `at` helper. Raw capture and
diagnostics, all seven regexes, nonnegative indices, ordering, the final exact
summary assertion and the HTTP assertion remain unchanged. No runtime,
supervisor, `until`, color configuration, pins or deadlines changed.

Test-first reproduction used disposable source and absolute private target
directories. The first `FORCE_COLOR=1` run passed because this orb also exports
`NO_COLOR` and Vite emitted no ANSI bytes; that was not counted as red evidence.
Removing `NO_COLOR` from the diagnostic process environment, rather than setting
it empty, produced a genuine colored Vite `Local` line containing `\u001b[22m`
before the colon and `\u001b[1m` inside the URL. The old presence assertion then
failed with Node exit 1 after readiness and HTTP 200 had passed. Applying the
snapshot correction passed the same focused colored run with exit 0.

A disposable replay extracted the candidate's actual assertion block and seven
patterns. Both the genuine colored capture and its stripped form passed. For
each form, deleting each of the seven required lines independently failed the
presence assertion. Swapping web readiness and final `dev: ready` retained all
seven matches but failed the order assertion. Raw captures, replay source and
results, and direct Node statuses were retained; no diagnostic helper ships.

```sh
env -u NO_COLOR FORCE_COLOR=1 CARGO_TARGET_DIR=/tmp/rob-1112-color-target \
  node --test apps/reference/scripts/test/dev.test.mjs
# Exit 0: 22 passed, 0 failed, 0 skipped, 0 cancelled; 107.01 s.
```

This full acceptance run was unfiltered, with Node 26.10.0, Rust 1.98.1 and npm
10.9.9. Ports 4001, 3003 and 5175 were exclusive and free before and after. Node
statuses were saved before log tails. Rust formatting, whitespace and Prettier
3.9.9 Markdown/JS checks passed. Prior local facts and historical evidence are
unchanged. Fresh independent testing and an actual successful Actions run,
including both later browser steps and the full job, remain for Lead; this
correction does not claim Actions acceptance.

## Reference invitation design — October 2, 2026

ROB-1113 accepts the future reference invitation, outbox and worker contract in
[S19](design-spec.md#s19--reference-invitations-and-delivery). This is a
**design-only decision, not implemented or verified behavior**. The owner
delegated product choices to Lead and Oracle. The full approved plan is the
empty
[plan commit](https://github.com/robertguss/Iris/commit/1d8bd754a440e528107555bc37541fcd7b85ffba),
against
[89864373](https://github.com/robertguss/Iris/commit/89864373c59b434599346bc65a5689a99776008d);
approval and the one-task High fallback after Grok's pre-tool failure are
recorded in the
[owning thread](https://ampcode.com/threads/T-01a0f950-d46b-7216-9ae2-3d0233c84d3a).
That approval covers this documentation design, not later implementation or a
claim of completed runtime review. The spec owns contracts, this record owns
dated rationale and provenance, and the application README owns runnable
instructions and future prerequisites. Linear remains the work queue.

Existing accounts, editor-only grants, one-hour expiry and recipient-bound
acceptance keep the slice about authorization and delivery rather than signup or
identity linking. Server-generated credentials must leave through local capture
only, not the issue response. This deliberately rejects the frozen experiment's
demo token preview. The public v1 acknowledgment records the request's committed
effect, never delivery. No receipt/status endpoint or automatic retry is added:
both operations' inspect, read and replay recovery capabilities are false. Later
membership observations do not resolve an earlier unknown attempt.

Checking owner, recipient, existing membership, pending invitation and usable
contact in that order preserves authority and conflict precedence. The explicit
409 `invitations.recipient_unavailable` lets an old database without contacts
remain usable without silently reseeding it or treating delivery as optional.
Only future fresh initialization/reset adds contacts; schema changes remain
append-only. Bob-to-Alice on project 43 avoids project 41's existing membership.
Revoking the issuer's authority after commit does not revoke the invitation.
Acceptance retains a membership's existing role and cannot restore a removed
member by replaying a consumed credential.

The frozen [delivery guide](../experiments/api-slice/delivery.md),
[worker](../experiments/api-slice/server/src/delivery.rs) and
[outbox](../experiments/embedded-db/sqlite/src/outbox.rs) provide atomic
enqueue, contact snapshot, lease, backoff and duplicate-delivery precedent. They
are not the reference implementation. Five claims bound recovery, not five
actual sends; pre-send interruption spends a claim. A fenced completion
returning false makes no transition and does not establish a replacement worker,
whereas a database error leaves acknowledgment unknown. SMTP acceptance is not
user delivery, and retry after lost completion can duplicate it. Plaintext
pending payloads are necessary for sending and are cleared on terminal cleanup
or an eligible sweep, not securely erased; a stopped worker may retain expired
credentials. Database reset does not erase the separate inbox. The historical
24-hour retention and 500-message limit are not reference-app measurements; the
proposed reference cap and capture configuration require later pinning and
verification in an isolated, loopback-only, no-relay service.

Source inspection of the reference [seed](../apps/reference/src/app.rs),
[schema](../apps/reference/migrations/0001_initial.sql),
[HTTP modules](../apps/reference/src/http/mod.rs),
[lifecycle](../apps/reference/src/lifecycle.rs) and
[supervisor](../apps/reference/scripts/supervise.mjs) confirmed the design's
baseline: contacts are not seeded, invitations and delivery are absent, tracked
connection closure and task supervision exist, and the deadlines are 3 seconds
to drain, 1 to close, and 5 for the outer supervisor. Those deadlines stay
fixed. Worker database errors wait for the next normal tick with bounded
diagnostics; an unexpected exit or panic stops the process rather than
restarting the task. No claim starts after observed shutdown, and an admitted
claim cannot begin SMTP after observing shutdown. Active sends may outlive
invitation eligibility and become uncertain when interrupted. S19 records the
decision and failure-window tables that later checks must exercise.

Memory-only fragment handling, scrubbing malformed fragments as well as valid
ones, and clearing on unknown completion or authentication/account transitions
avoid turning recovery UI into credential storage. Full login deliberately loses
the token; sign in and reopen the capture link. No invitation preview, listing
or status workflow is implied.

The three later green boundaries are private persistence/domain tests, private
delivery/lifecycle, and **both public operations with their entire generated
contract/client/presentation/views/browser integration together**. Separating
the public server from its client would expose an incomplete contract. These are
bounded constraints for separate issues, not an implementation backlog or
authorization. No runtime, migration, generated file, dependency, frozen
experiment, CI, version pin or lifecycle deadline changes accompany this design.
Documentation validation compares factual claims to source and historical
evidence, checks local links/anchors and whitespace, and uses pinned Prettier
3.9.9 with width 80 and prose wrapping. No runtime tests or delivery/retention
measurements are claimed for ROB-1113; implementation evidence must be recorded
when those later stages actually run.

## Reference omission parity — October 2, 2026

ROB-1114 extends the existing reference runner rather than adding a probe
engine. The [complete mapping](design-spec.md#reference-omission-parity) covers
all frozen S16 omitted-edit families and their controls. The application,
library, committed OpenAPI/generated TypeScript, dependency manifests and frozen
experiments are unchanged. No experiment is retired. The sole CI change moves
the existing reference-probe step after reference-client verification and the
development-command tests, which supply installed web dependencies first.

The tests distinguish the suspected mistakes before accepting their evidence:
adding a shared refusal must reach both domain operations and actual exported
response branches; the independent compatibility expectations deliberately stay
unchanged. A status change must first fail Rust contracts, then stale generated
types, then obsolete TypeScript handling, with a passing two-branch repair.
Generation uses a temporary web-directory Node child and
`openapiTS`/`astToString` only: `npm generate` would run unchanged Rust
expectations before the intended TypeScript signal. Projector completion
compiles. An otherwise valid mounted request passes with 200 before a
router-fold-only omission produces raw 404; both contract integration tests
still pass. This is stronger than the frozen runner's empty-layer panic. CSRF
declaration removal filters only the mapping, not the method exemption, while
producer-linkage removal leaves export intact and fails the runtime envelope
assertion.

All touched baseline bytes, including generated files and temporary tests, are
registered before writes; reset restores and compares them. `Fixture::new`
preserves the storage-exclusive child-spawn guard. Named behavioral failures and
their diagnostics are required, including bare integration-test names; spawn
errors, signals and zero tests cannot satisfy them. A full fake-output emulator
was not added: actual Rust, export, TypeScript and web executions are the
discriminating checks. The runner copies dependencies because symlinking
`node_modules` lets Vite write its temporary configuration into the checkout.
Executable symlinks retain their relative targets inside that copy. Cargo uses
copied registry/git caches offline in a private home; nested verifiers inherit
the private target and temporary directory, not the caller's fixture path.

Verification used Linux, Rust 1.98.1, Node 26.10.0 and the locked reference web
dependencies, installed with `npm --prefix apps/reference/web ci` outside the
runner. The first separate healthy baseline command was:

```sh
env -u IRIS_REFERENCE_FIXTURES \
  CARGO_TARGET_DIR=/tmp/rob1114-evidence/healthy-target \
  cargo test --quiet --locked -p iris -p iris-reference
```

It failed before mutations: 98 reference library tests passed and
`lifecycle::tests::session_cleanup_deletes_expired_rows_on_a_persistent_database_and_outlives_a_busy_tick`
failed at `apps/reference/src/lifecycle/tests.rs:477:14` with:

```text
called `Result::unwrap()` on an `Err` value: Database(SqliteError { code: 5, message: "database is locked" })
```

An unchanged rerun passed. This does not diagnose or fix the lock failure; no
lifecycle test or runtime was changed. The first runner attempt also failed
closed when an older missing-variant diagnostic matcher did not match Rust
1.98.1's actual E0599 wording. The final matcher names the exact missing
`ProjectArchived` variant diagnostic. That failed attempt removed its owned copy
and left inherited target/fixture sentinels intact.

The full isolated command was run with conflicting inherited paths containing
sentinels, and an unrelated sibling sentinel beside the runner's owned copies:

```sh
/usr/bin/time -p env \
  TMPDIR=/tmp/rob1114-evidence/owned-tmp \
  CARGO_TARGET_DIR=/tmp/rob1114-evidence/inherited-target \
  IRIS_REFERENCE_FIXTURES=/tmp/rob1114-evidence/inherited-fixtures \
  node apps/reference/scripts/probes.mjs
```

One complete run passed **31 caught probes and 25 controls in 216.10 s**,
including the initial and final full Rust/web controls. Healthy Rust comprises
34 library tests, 99 reference library tests, 2 contract tests, 12 development
tests and 7 session tests (154 total). Web verification runs 4 captured-response
Rust tests, 231 client cases, 51 presentation cases, 13 request/readback cases,
11 directory cases, typechecking and the Vite build. The separate unchanged
healthy Rust rerun and `node apps/reference/web/scripts/verify.mjs` also passed
on `/tmp/rob1114-evidence/healthy-target` before that complete run.

The final version then changed only evidence printing, to show actual matched
multiline diagnostics rather than their regex text. Its repeat took 212.42 s,
caught all 31 negatives and passed 23 controls, but **failed the final restored
healthy Rust control**, with 98 reference library passes and the same lifecycle
test's SQLite code 5 error, this time at `lifecycle/tests.rs:472:10`. The exact
child command remained `cargo test --quiet --locked -p iris -p iris-reference`,
with its runner-private target and home. The web control after it did not run;
no PASS was emitted. Further retries stopped for separate diagnosis, with no
lifecycle changes or weakened checks. At that pause, a separate post-run healthy
Rust/web check, fresh-candidate independent verification and full CI remained
outstanding; that version was not a verified final candidate.

After both runs, 128 tracked source/generated/frozen-file SHA-256 hashes stayed
unchanged; after the final run, 3,036 installed dependency-file hashes and their
file inventory also matched. Both inherited directories contained only their
unchanged sentinels, the unrelated sibling survived, and each specifically named
owned copy was absent. The checkout's `target` remained absent. These are
owned-path checks, not a global temporary-directory scan or a forced-termination
cleanup claim. `node --check` and `git diff --check` passed. Documentation uses
pinned Prettier 3.9.9, width 80, prose wrapping always. No browser workflow,
workspace-wide suite, Clippy or frozen suite was rerun for this runner-only
change; none is claimed as evidence here.

### Resumed after the reviewed observer dependency

After ROB-1121 merged at pinned main
[eb0ecd9](https://github.com/robertguss/Iris/commit/eb0ecd916674f58d9bdf0be273d26b5daf11781a),
the preserved five-file diff and its SHA-256 were verified unchanged, committed
as an explicitly UNVERIFIED checkpoint, then integrated with a normal merge.
Only the shared decision record conflicted; both records were retained and the
ROB-1121 report below was corrected to distinguish the earlier 31/25 completed
run from the later 31/23 failure. The separate merged baseline confirmed 103
reference library tests, including four observer tests. At this integration, the
runner's only post-checkpoint code change updated its healthy count matcher from
99 to 103; all 31 probes, control commands, diagnostics, isolation and cleanup
remain. The historical race was not reproduced or assigned a precise
interleaving.

The integrated run passed **31 caught probes and 25 controls in 222.75 s**:

```sh
/usr/bin/time -p env \
  TMPDIR=/tmp/rob1114-integrated/owned-tmp \
  CARGO_TARGET_DIR=/tmp/rob1114-integrated/inherited-target \
  IRIS_REFERENCE_FIXTURES=/tmp/rob1114-integrated/inherited-fixtures \
  node apps/reference/scripts/probes.mjs
```

Both internal healthy Rust/web controls passed, and PASS followed successful
cleanup. Separately, before and after that run, these commands passed on a
healthy target that never held mutations:

```sh
env -u IRIS_REFERENCE_FIXTURES \
  CARGO_TARGET_DIR=/tmp/rob1114-integrated/healthy-target \
  cargo test --quiet --locked -p iris -p iris-reference
env -u IRIS_REFERENCE_FIXTURES \
  CARGO_TARGET_DIR=/tmp/rob1114-integrated/healthy-target \
  node apps/reference/web/scripts/verify.mjs
```

Each separate Rust suite passed 158 tests: 34 + 103 + 2 + 12 + 7. Web counts
remain 4 captured-response Rust tests and 231/51/13/11 client cases, with tsc
and Vite green. All 128 source/generated/frozen-file hashes and all 3,036
installed dependency hashes and their inventory remained unchanged against the
integrated baseline. The inherited target and fixture directories contained only
their unchanged sentinels; the unrelated sibling survived; the named owned copy
was removed; the checkout target stayed absent. Earlier failure logs and the
preserved patch were retained. Syntax, whitespace and pinned Markdown checks
passed. These are Builder checks; fresh independent verification and full CI for
the resulting candidate remain separate acceptance gates.

### Forced-color diagnostic matching

[CI 36974061992](https://github.com/robertguss/Iris/actions/runs/36974061992) at
[f3bef6a](https://github.com/robertguss/Iris/commit/f3bef6a748e37e9156ae887f9d90ef7fe7dbbb47)
failed the first missing-variant probe with the intended Cargo status 101 and
E0599 diagnostic. `CARGO_TERM_COLOR=always` placed ANSI sequences between
`error[E0599]` and its colon, so the exact pattern did not match raw output. The
earlier plain-output passes did not prove forced-color behavior. The saved
`gh run view --log-failed` output has literal caret notation and zero ESC bytes;
it was retained as CI evidence, not reused as an ANSI fixture.

Before changing the runner, its actual `added("variant")` mutation ran in a
disposable source copy with a private Cargo target, copied offline cache and
pristine/restored `cargo check` controls. Real Cargo stdout/stderr was captured
with `NO_COLOR` unset, `CARGO_TERM_COLOR=always` and `FORCE_COLOR=1`: status
101, three matching E0599 diagnostics and 143 genuine ESC bytes. Replaying the
actual old `expect` function rejected this colored capture but accepted the same
diagnostic without ANSI codes. Its thrown error retained the raw bytes.

The correction imports Node's `stripVTControlCharacters` and computes one
`plainOutput` snapshot inside `expect`, used only for pattern matching and
matched evidence. Thrown diagnostics still contain original stdout/stderr.
Patterns, status checks, spawn error/signal gates, environment and workflow are
unchanged; no helper or full-suite emulator was added. Replaying the actual new
function accepted the real colored E0599 and printed plain matched evidence. It
rejected an E0004 expectation, a capture with **all three** matching diagnostic
lines removed, and a status-0 negative. Every rejected case retained its raw
output verbatim. Literal caret syntax is not stripped in production.

The complete real forced-color run then passed **31 caught probes and 25
controls in 230.37 s**, including initial/final healthy Rust and web checks and
cleanup before PASS:

```sh
/usr/bin/time -p env -u NO_COLOR CARGO_TERM_COLOR=always FORCE_COLOR=1 \
  CARGO_HOME=/tmp/rob1114-color/cargo-home \
  TMPDIR=/tmp/rob1114-color/owned-tmp \
  CARGO_TARGET_DIR=/tmp/rob1114-color/inherited-target \
  IRIS_REFERENCE_FIXTURES=/tmp/rob1114-color/inherited-fixtures \
  node apps/reference/scripts/probes.mjs
```

Separate before/after `cargo test --quiet --locked -p iris -p iris-reference`
and `node apps/reference/web/scripts/verify.mjs` also passed with the same
forced-color flags and `NO_COLOR`/`IRIS_REFERENCE_FIXTURES` unset. They used an
unmutated source copy, a separate private `healthy-target`, copied offline Cargo
cache, copied web dependencies and private temporary/cache paths. Each Rust
suite passed 158 tests (34 + 103 + 2 + 12 + 7); web verification passed 4
captured-response Rust tests, 231/51/13/11 client cases, tsc and Vite.

The checkout's 128 source/generated/frozen-file hashes and 3,036 dependency-file
hashes and inventory stayed unchanged. The inherited target and fixture
directories retained only their sentinel files; the unrelated sibling survived;
the named runner-owned copy was absent; the checkout target remained absent. Raw
CI failure, real Cargo capture and replay evidence were retained separately from
successful forced-color logs. Node syntax, pinned Markdown formatting and diff
checks passed. The earlier plain passes remain historical evidence, not proof of
CI color handling. Fresh independent Tester, Oracle review and exact candidate
CI remain required; this local run does not claim those gates passed.

### Resumed after the separate startup observer dependency

[CI 36977253816](https://github.com/robertguss/Iris/actions/runs/36977253816) at
[f77e3e7](https://github.com/robertguss/Iris/commit/f77e3e7176d79e564238cb3bb6fbbaf04c08ed14)
caught all 31 probes and passed 23 controls, then failed the final pristine
`cargo test --quiet --locked -p iris -p iris-reference`. The 34/103/2 groups
passed; `dev_binary` passed 11 of 12 tests. Its
`a_start_deletes_expired_sessions_and_login_attempts_and_keeps_live_ones` failed
at `apps/reference/tests/dev_binary.rs:757:14`, the first SELECT unwrap, with:

```text
called `Result::unwrap()` on an `Err` value: Database(SqliteError { code: 5, message: "database is locked" })
```

The raw failed CI log was retained. The runner emitted no overall PASS and the
final web control did not run. This was a separate integration-test observer,
not ROB-1121's library helper. Work paused without retries or weakened controls
until separately reviewed ROB-1176 merged at pinned main
[38bdbf8](https://github.com/robertguss/Iris/commit/38bdbf81e160c911c16272fb10ae4f57dacf3788).
A normal merge integrated that dependency without conflicts, preserving both
observer records and the ROB-1121 provenance correction. The runner remained
byte-identical to the pushed candidate; no status, diagnostic, isolation or
cleanup check changed. No exact historical lock-holder or interleaving is
claimed reproduced.

The merged dependency passed the complete real forced-color runner: **31 caught
probes and 25 controls in 219.82 s**, with both initial/final healthy controls
and cleanup before PASS:

```sh
/usr/bin/time -p env -u NO_COLOR CARGO_TERM_COLOR=always FORCE_COLOR=1 \
  CARGO_HOME=/tmp/rob1114-1176/cargo-home \
  TMPDIR=/tmp/rob1114-1176/owned-tmp \
  CARGO_TARGET_DIR=/tmp/rob1114-1176/inherited-target \
  IRIS_REFERENCE_FIXTURES=/tmp/rob1114-1176/inherited-fixtures \
  node apps/reference/scripts/probes.mjs
```

Separate healthy before/after commands were
`cargo test --quiet --locked -p iris -p iris-reference` and
`node apps/reference/web/scripts/verify.mjs`, using an unmutated archive,
private copied dependencies/caches, a separate `healthy-target`, private
temporary paths, the same forced-color flags and inherited fixture output unset.
Each Rust suite passed **163 tests (34 + 103 + 2 + 17 + 7)**. The library count
remains 103; the separate dev-binary suite now has 17. Each web verifier passed
4 captured-response Rust tests, 231/51/13/11 client cases, tsc and Vite.

All 128 tracked source/generated/frozen-file hashes and 3,036 dependency-file
hashes and inventory matched the merged baseline. The inherited target and
fixture directories contained only unchanged sentinels; the unrelated sibling
survived; the named runner-owned copy was absent; the checkout target remained
absent. The runner's SHA-256 remained unchanged across integration and checks:
`3f2b96fd0a4cc0885022a077bb2282dd56176f8e7cdb0bd6019c10976135d123`. The prior CI
failure log was retained separately. Syntax, pinned Markdown and diff checks
passed. These are Builder results, not fresh independent Tester, Oracle or
exact-candidate CI acceptance; those remain required before merge.

## ROB-1121 bounded Busy observation — October 2, 2026

The Lead reported repeated pristine ROB-1114 controls failing in
`session_cleanup_deletes_expired_rows_on_a_persistent_database_and_outlives_a_busy_tick`:
`identifiers` unwrapped SQLite code 5, `database is locked`, at its two SELECT
sites (lines 472/477), with 98 library tests passing and one failing. An earlier
parity run completed with 31 caught mutants and 25 passed controls. The later
final-version run reached 31 caught mutants and 23 passed controls, then failed
its restored healthy Rust control and emitted no overall PASS. These are
retained reported failures, not new reproductions or evidence of a runtime
cleanup defect.

The approved
[plan](https://github.com/robertguss/Iris/commit/a69b98752ed7e80bd2c36fe9bffa607807a5bcee)
has the same source as base
[c3461b9](https://github.com/robertguss/Iris/commit/c3461b9d330aab93fb176a3926debf864ffa7501).
In the Builder's Linux x64 orb, a pristine `git archive` copy at
`/tmp/rob-1121/healthy` and private `healthy-target` ran 20 fresh-process
targeted invocations followed by one `cargo test -p iris-reference`, with
`RUST_BACKTRACE=1`. Every invocation exited 0; the batch took 35 s (15 s for the
targeted invocations). The cap was 20 plus one suite or 15 minutes, stopping at
the first failure; there was no failure or diagnostic retry. Each targeted
invocation used:

```sh
cargo test -p iris-reference --lib \
  session_cleanup_deletes_expired_rows_on_a_persistent_database_and_outlives_a_busy_tick \
  -- --exact lifecycle::tests::session_cleanup_deletes_expired_rows_on_a_persistent_database_and_outlives_a_busy_tick \
  --nocapture
```

The original failure was **not reproduced within budget**. Its exact phase and
acquisition-versus-SELECT interleaving remain unknown. Independently, the raw
observer cannot tolerate real Busy: it previously unwrapped either query's
error. Before adding retry behavior, the new regression tests ran against a
single-observation, error-propagating scaffold: transient Busy failed because
the observer returned before lock release, and persistent Busy failed because it
returned before the absolute deadline (exit 101, 1 passed / 2 failed). Non-Busy
SQL errors and wrong rows already failed immediately as required.

Both Busy tests first establish an actual Busy error from `identifiers` under
`BEGIN EXCLUSIVE`, not an injected error or a sleep-only success. After repair,
the transient observer remains pending while locked and obtains the exact
nonempty rows after rollback. Persistent Busy fails at its deadline with phase
and database error, even when expected rows are empty. Additional checks cover
wrong rows in strict mode, a missing second table, closed pool, actual pool
timeout without retry, and a phase deadline while all four pool connections are
held. Four observer tests pass. The lifecycle test still uses the existing 20 s
phase budget, 30 ms Busy timeout, four connections and 10 ms retry sleep; strict
held-writer observation, worker liveness, clean stop and closure remain. No
runtime, journal policy, dependency, runner or CI behavior changed.

Three semantic mutants ran sequentially in `/tmp/rob-1121/mutation`, with
`CARGO_TARGET_DIR=/tmp/rob-1121/mutation-target`, using the targeted command
above. No mutation touched checkout source or a shared target. The unchanged
control passed before and after (one test, exit 0, 0.37/0.38 s). Each mutant
compiled and failed the intended row assertion with exit 101:

| Mutation                                    | Assertion evidence                                                             | Test duration |
| ------------------------------------------- | ------------------------------------------------------------------------------ | ------------- |
| Both deletion predicates append `AND 0`     | Initial deletion deadline: all six rows, including expired rows, remain        | 20.05 s       |
| Both deletion predicates append `OR 1`      | Initial deletion deadline: empty rows instead of the four live/soon rows       | 20.18 s       |
| Cleanup awaits forever after its first Busy | Post-release deadline: all four live/soon rows remain instead of two live rows | 20.37 s       |

These are internal phase-deadline assertion failures, not compilation errors,
external process timeouts or unrelated failures. Scratch logs under
`/tmp/rob-1121/logs` are local execution evidence, not durable repository paths.

Final Builder verification on the repaired source, using private
`CARGO_TARGET_DIR=/tmp/rob-1121/candidate-target`:

```sh
cargo test -p iris-reference --lib observer_ # 4 passed
cargo test -p iris-reference --lib lifecycle::tests # 27 passed
cargo test -p iris-reference # 124 passed across library/integration suites
cargo test --workspace # 206 passed across test/doc-test targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
npx --yes prettier@3.9.9 --print-width 80 --prose-wrap always --check \
  docs/design-spec.md docs/decisions.md
git diff --check
```

All exited 0. No client/browser check is claimed for this test-only change. The
controlled exclusive lock demonstrates the observer defect, not the exact
historical scheduling sequence; other historical lifecycle/concurrency timing
cases are not claimed fixed. Fresh Tester, Lead/Oracle review and CI acceptance
remain separate delivery gates; Builder verification is not merge evidence.

## ROB-1176 bounded startup observation — October 2, 2026

The binary startup cleanup test has a separate observer from ROB-1121's library
test. CI run 36977253816 reported SQLite code 5 at its first SELECT's unwrap
after other groups passed (34/103/2 tests); the lock holder, interleaving and
eventual cleanup result are unknown. The approved plan is
[ff137478](https://github.com/robertguss/Iris/commit/ff1374785377f4d515921bef55b72e3c7e3a18f8),
on merged main
[eb0ecd91](https://github.com/robertguss/Iris/commit/eb0ecd916674f58d9bdf0be273d26b5daf11781a).
This repair changes only the integration-test observer and technical records.
Production cleanup, journal policy, connection timeouts, dependencies, runners,
CI and frozen experiments are unchanged.

`startup_rows` propagates both SELECT errors. `wait_for_startup_cleanup` retries
the entire observation only for `app::is_busy`, preserving the original
diagnostic for other errors. Wrong rows keep polling. One absolute 10 s deadline
from post-readiness covers both reads and asynchronous 50 ms sleeps; explicit
expiry checks prevent even valid rows from succeeding after expiry. Timeout
reports the last Busy or wrong-row observation. The exact live/live assertion,
seeding, both starts, first graceful stop, owned children and connection close
remain intact.

The Builder predeclared at most 20 targeted test processes plus one pristine
`dev_binary` suite, or 15 minutes, stopping on the first unexpected failure. The
actual allocation was **8 targeted processes plus 1 suite**, from
07:40:44–07:48:11 UTC (7 min 27 s including compilation and editing between
runs). No unexpected failure, diagnostic rerun or budget expansion occurred. One
baseline startup run passed; the historical CI race was not reproduced.

Test-first evidence used a compilable old-behavior scaffold, not absent symbols:
the raw reads propagated errors immediately and valid rows bypassed deadline
expiry. The five new regressions produced 2 passes and 3 expected failures (exit
101): transient Busy returned before explicit release, persistent Busy returned
before its deadline, and expired valid rows returned success. After repair, all
five passed. Both lock tests first establish actual SQLite Busy under
`BEGIN EXCLUSIVE` on a disposable migrated file. The transient waiter stays
pending for 250 ms before explicit rollback, then succeeds; the persistent case
exhausts a 250 ms deadline with the database diagnostic under an independent 2 s
watchdog. Other controls establish first-read success followed by a missing
second table, expired rows present, a live row missing, no rows, and an already
expired deadline with valid rows. No second-read lock injection is claimed;
source inspection establishes whole-observation restart.

Semantic negatives used a pristine archive plus the repaired integration test in
`/tmp/rob-1176/mutation`, with a separate
`CARGO_TARGET_DIR=/tmp/rob-1176/mutation-target`. Each run exercised the actual
binary startup test, disposable database and both starts. The pristine and
restored controls passed (0.30/0.27 s); restored source matched pristine bytes.
Both mutants compiled and failed internally with exit 101:

| Both cleanup SQL predicates | Deadline diagnostic                              | Test duration |
| --------------------------- | ------------------------------------------------ | ------------- |
| Append `AND 0`              | Unexpected `["live", "old", "live", "old"]` rows | 10.22 s       |
| Append `OR 1`               | Unexpected `[]` rows                             | 10.23 s       |

No mutant failed by external watchdog, compilation or unrelated assertion. The
other four targeted processes were baseline startup, old-red regressions,
new-green regressions and repaired startup. Commands used:

```sh
# Startup: baseline, repaired, pristine, both mutants and restored (6 processes).
cargo test -p iris-reference --test dev_binary \
  a_start_deletes_expired_sessions_and_login_attempts_and_keeps_live_ones \
  -- --exact --nocapture
# Old-red then new-green (2 processes).
cargo test -p iris-reference --test dev_binary startup_observer_ -- --nocapture
# One pristine suite: 17 passed, exit 0.
cargo test -p iris-reference --test dev_binary -- --nocapture
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
npx --yes prettier@3.9.9 --print-width 80 --prose-wrap always --check \
  docs/design-spec.md docs/decisions.md
git diff --check
```

Candidate checks use `/tmp/rob-1176/candidate-target`; no shared build target or
checkout production source was mutated. Raw logs, source hashes and scaffold,
candidate and mutation patches are retained in the Builder thread's evidence
archive. Scratch paths are local evidence, not durable repository interfaces.
The controlled lock proves the observer defect, not the historical interleaving
or any production cleanup repair. Full exact-candidate CI, fresh Tester and
Lead/Oracle review remain separate gates; no merge or broader runtime claim is
made here.

## Partial S16 CI retirement — October 2, 2026

ROB-1115 makes a bounded CI change after accepted ROB-1114 parity. The approved
plan is
[`62f67213a578f2b38a27d94724ff6c51f2fe059c`](https://github.com/robertguss/Iris/commit/62f67213a578f2b38a27d94724ff6c51f2fe059c),
whose parent is accepted base
[`493c00207cec72a0beda05e2c5e5e3185dceb614`](https://github.com/robertguss/Iris/commit/493c00207cec72a0beda05e2c5e5e3185dceb614).
The Builder required that exact remote plan tip before work. The
[Builder thread](https://ampcode.com/threads/T-01a0fbc9-4eff-71ae-8405-47444c454dfd)
owns this baseline/check evidence. The approved ROB-1115-only built-in High
Builder exception followed documented pre-tool Grok47 failures; it changes no
default or provider availability claim.

The [matrix](design-spec.md#partial-s16-ci-retirement) was written before the CI
edit. It audits all ten frozen Rust tests, the 19 captured and 25 independent
client cases, common one-call/no-diagnostic assertions, uncounted recovery and
compile-time narrowing, both drift edges, all 16 omissions and seven controls.
No unique unmapped obligation was found. The mapping is not based on 231 being
larger than 44: it names assertions and records marked Forbidden 403 becoming
LoginFailed 401 (raw 403 retained), and the maintained current-state-read
declaration with independent metadata expectations and `read: false` acceptance.
These are semantic adaptations, not wire identity.

Only the direct `verify:s16` line leaves the mixed web verification step. Normal
web `verify` remains. The compound `probe:s16` step becomes explicit
`node --test experiments/api-slice/web/scripts/test/s16-probes.test.mjs` in the
same serial position after dependency installation. That regression retains
ROB-1116's nested-target and exception-cleanup evidence, independently of real
contract/omission parity. Every other workflow field, step, pin and ordering is
preserved. Only the workflow, application README, spec and this record change;
no frozen source, artifact, package script, manifest or dependency changes.

Consequently, direct automatic frozen export/snapshot/generated-TS
synchronization, frozen client/narrowing executions and frozen omission
mutations stop. Frozen Rust still executes in workspace/default and API
dev-identity suites. Reference equivalents and explicit frozen mechanics remain
automatic. Historical manual commands and evidence remain available, without a
permanent manual-green promise. This supersedes earlier statements that all S16
entrypoints stay in CI; those dated decisions remain historical records.

### Accepted-base baseline and configuration evidence

One standalone verifier and one **full compound** probe baseline ran against a
`git archive` of the exact accepted base above, not the edited checkout. Tools:
Linux x64, Rust/Cargo 1.98.1, Node 26.10.0, npm 10.9.9. The source copy was an
independently initialized Git inventory so the omission runner copied only its
owned source. Dependencies were installed before either runner. With `scratch`
set to an owned `mktemp -d /tmp/rob1115-baseline.XXXXXX` directory, the commands
were:

```sh
mkdir "$scratch/source" "$scratch/tmp" "$scratch/inherited-target"
printf 'untouched\n' > "$scratch/inherited-target/sentinel"
git archive 493c00207cec72a0beda05e2c5e5e3185dceb614 | tar -x -C "$scratch/source"
git -C "$scratch/source" init -q
git -C "$scratch/source" add .
npm --prefix "$scratch/source/experiments/api-slice/web" ci
TMPDIR="$scratch/tmp" CARGO_TARGET_DIR="$scratch/private-target" \
  npm --prefix "$scratch/source/experiments/api-slice/web" run verify:s16
TMPDIR="$scratch/tmp" CARGO_TARGET_DIR="$scratch/inherited-target" \
  npm --prefix "$scratch/source/experiments/api-slice/web" run probe:s16
```

The standalone command passed **10 Rust tests and 44 whole-request runtime
cases**, with one attempt each and no raw diagnostics, plus export/type drift
and narrowing. The compound command passed **one mechanics regression, 16 caught
omissions and seven controls**, including both real pristine verifiers, and
exited 0. Export success and the separate exported-content assertion are two
controls, not one. The exact omission/control names and equivalences are in the
spec matrix.

Safety checks compared SHA-256 hashes of all protected tracked experiment,
reference and library files and Cargo manifests/lockfile before/after (the
authorized reference README edit excluded). All matched. The checkout target
remained absent; the inherited target contained only its unchanged `untouched`
sentinel. No `iris-*` source/target/fixture directories survived under the owned
temporary parent. Node's compile cache did remain there; it was not a runner
leak and was removed with the externally owned source/private target after
evidence collection. No checkout/shared-target mutation or fixed-port suite
overlap occurred. Standalone `verify:s16` does not isolate Cargo itself;
external disposable source and `CARGO_TARGET_DIR` were essential here.

An ephemeral Node check using installed `js-yaml` parsed both the plan workflow
and candidate, preserved mapping-key and step order, and allowed only the two
labels, direct-line removal and compound-step replacement. It passed the
candidate and rejected three discriminating fixtures: unchanged baseline,
candidate without normal web `verify`, and candidate without mechanics. These
fixtures stayed outside the committed patch. Configuration/documentation needs
these structural checks, not invented runtime TDD. Formatting/check commands:

```sh
npx --yes prettier@3.9.9 --print-width 80 --prose-wrap always --check \
  apps/reference/README.md docs/design-spec.md docs/decisions.md
git diff --check
```

Candidate runtime evidence is deliberately a separate gate: the fresh Tester
owns the real forced-color reference **31 caught / 25 controls** run with actual
initial/final healthy Rust and web, retained explicit mechanics, hashes,
sentinels and cleanup. The Builder does not duplicate that run, rerun the full
frozen baseline on the candidate, or claim it passed here. Exact-candidate
Tester, Lead/Oracle review and full CI remain required before merge. No PR,
merge, deployment or release is claimed by this record.

## Maintaining this record

When a proposal is tested, record the exact commands, dependency versions,
observations, decision, and limitations. Keep superseded decisions visible
rather than silently rewriting the reasoning. Separate preferences from measured
results.

Update the living spec's current status and rationale alongside new decisions;
retain superseded reasoning here rather than maintaining competing current
specs.
