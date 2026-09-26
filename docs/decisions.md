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

## Maintaining this record

When a proposal is tested, record the exact commands, dependency versions,
observations, decision, and limitations. Keep superseded decisions visible
rather than silently rewriting the reasoning. Separate preferences from measured
results.

Update the living spec's current status and rationale alongside new decisions;
retain superseded reasoning here rather than maintaining competing current
specs.
