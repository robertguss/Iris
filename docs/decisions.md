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

- SQLite versus Turso as default; precise driver, release, features, and
  migration tools.
- Authentication mechanism, credential storage, and CSRF defenses. Same-origin
  HttpOnly-cookie authentication was suggested, not accepted as the final
  design.
- Project/tenant model and where tenant authorization is enforced; RLS is not
  selected.
- Invitation identity: verified account email, user ID, or another policy; how
  email changes and normalization affect acceptance.
- Whether repeat acceptance is idempotent success or a documented conflict.
- Which errors intentionally hide resource or token existence.
- Clock semantics at expiration and under transaction retries.
- OpenAPI version/library, client generator, and how runtime contract checks
  run.
- Whether durable email delivery enters the first slice or the next one; outbox
  design if committing database state must reliably result in delivery.
- Packaging, crate layout, CLI name, framework name, license, and release
  strategy.

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

## Maintaining this record

When a proposal is tested, record the exact commands, dependency versions,
observations, decision, and limitations. Keep superseded decisions visible
rather than silently rewriting the reasoning. Separate preferences from measured
results.

Update the living spec's current status and rationale alongside new decisions;
retain superseded reasoning here rather than maintaining competing current
specs.
