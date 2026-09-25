# Iris living design specification

Last updated: September 25, 2026.

This is the current design entry point for Iris: what we want to build, the
conventions we are considering, and most importantly why. It is a living spec,
not documentation of a released framework. Read this before proposing framework
abstractions or implementing a design discussed in the conversation.

## How to interpret and maintain this spec

- **Accepted direction:** agreement on a principle or boundary; not necessarily
  implemented, and not an irreversible API commitment.
- **Proposed:** a concrete candidate to explore; examples are not existing APIs.
- **Implemented experiment:** executable evidence with deliberately limited
  scope.
- **Open:** a choice still requiring discussion or evidence.
- **Deferred:** intentionally not current work; not permanently rejected.

This document owns the current design synthesis. [Decisions](decisions.md)
preserves the chronological record and superseded choices. Experiment guides own
commands and measured results. If code and the spec differ, report the gap: do
not silently treat a proposal as implemented or change code just to match it.

When updating a design, preserve its stable section ID, explain the reason and
tradeoff, update its status and affected examples, and link supporting evidence.
Record superseded decisions rather than erasing their rationale. Keep unresolved
questions explicit. Implemented status requires executable evidence, not only a
code sketch or an agent's assertion. Changes to direction need owner agreement;
agents may propose alternatives without presenting them as accepted.

For external model review: S01–S03 explain the goals, S04–S08 the action and
failure boundaries, S12 the static contracts, and S13 the proposed execution and
evidence lifecycle. S14 defines the caller-loss recommendation and failure
table; S15 gives a concrete result, response and recovery reference contract.
S10 distinguishes implementation from proposals. Use the
[independent review brief](design-review-brief.md) for assignments, three review
tracks, report format and synthesis instructions. S13 also supplies focused
runtime-evidence questions; assess the design, not just the example syntax.

## S01 — Purpose and constraints

**Accepted direction.** Iris is a personal learning project: assemble an
opinionated, API-first Rust framework from existing crates and own its
conventions and integrations. Designing it is part of the purpose; adopting or
forking a batteries-included framework would miss that goal. Original crates may
follow when a useful responsibility becomes clear.

Rails inspires coherent developer experience, not a translation of Ruby's
programming model. Borrow predictable structure, useful defaults, diagnostics,
testing, and eventually generators and lifecycle tooling. Study other
frameworks' tradeoffs, not only their surface APIs. Ash informs actions and
discoverability; Loco informs explicit Rust conventions; FastAPI informs typed
transport contracts; Dioxus informs tooling and feedback loops. React remains
the first client; JSON APIs should support other clients too. Inertia and
Rust-based frontend rendering are not the initial model.

Use standards such as OpenAPI to connect Rust contracts, documentation,
generated clients, and verification. Standards adoption does not eliminate
runtime checks or business tests. JSON:API adoption remains open and is distinct
from JSON APIs.

Embedded-first startup and disposable databases reduce setup friction. SQLite
and native Turso are distinct candidates; PostgreSQL remains a desired server
database direction. Do not infer production-engine correctness from another
engine's tests or invent a universal persistence layer before understanding the
differences. SQLite/SQLx and utoipa remain provisional experiment choices.

## S02 — Optimize for AI-authored applications

**Accepted direction.** Optimize time to an independently verified change, not
generated code volume. Strong conventions should make the correct path easy and
mistakes visible to compiler checks, runtime enforcement, or verification.
Document which kind of enforcement each convention actually has.

Agents need discoverable contracts, fast feedback, stable diagnostics, permitted
execution evidence, and reproducible checks. Compilation alone does not
establish business intent; generated tests can repeat an implementation's
misunderstanding. Humans/reviewers still own acceptance criteria and authority
to change them.

Treat compile/test latency as a design constraint. Measure before optimizing:
distinguish reduced compilation work, cached work, linking, and restart costs.
Dioxus/Subsecond is a reference to investigate, not a selected dependency or a
promise of hot patching. Fast loops must coexist with clean-build and
fresh-process verification. Do not claim AI productivity gains without a study.

**Deferred:** additional productivity tests and the controlled seeded-bug repair
comparison. The owner chose to focus on framework design instead. Independent
feature development supplied useful friction observations, not a controlled
measurement of repair speed or reliability.

## S03 — Explicit execution, shared declarations

**Accepted direction; precise registration API proposed.** Domain actions remain
ordinary Rust functions. Share transport declarations where they prevent drift;
do not introduce an execution engine just to remove OpenAPI boilerplate.

| Model considered                                       | Benefit                                                             | Cost / current disposition                                                    |
| ------------------------------------------------------ | ------------------------------------------------------------------- | ----------------------------------------------------------------------------- |
| A: functions plus separate route/contracts             | Transparent control flow and normal Rust tools                      | Registration can drift; remains a valid baseline                              |
| B: functions plus unified typed HTTP registration      | One discoverable contract feeds routes, schemas and operation index | Preferred direction; use existing integrations before adding an Iris registry |
| C: resource/action DSL with policies and change stages | Shared vocabulary and potentially reusable lifecycle                | Deferred: Iris would own ordering, state, hooks, transactions and composition |

The membership experiment already shares business logic. Repetition occurs in
response declarations and the two comparison exporters. Maintaining both utoipa
and aide indefinitely is not a requirement. Select one primary path when ready.

An HTTP operation index is not discovery of every domain function. Registration
does not prove authorization or locking correctness and must not automatically
expose mutations through MCP.

S12 develops the proposed static contract: shared response-contract data for
runtime rendering and documentation, with explicit Rust as the reference model.
That consumer integration does not exist merely because DTOs derive schemas.

## S04 — Domain-owned behavior, adapter-owned protocols

**Proposed authoring convention, consistent with accepted action boundaries.**
Start small:

```text
src/
  app.rs
  identity.rs
  domains/
    mod.rs
    memberships.rs
  http/
    mod.rs
    memberships.rs
```

The domain owns commands, outcomes, authorization and mutation. HTTP owns wire
DTOs, session extraction, response mappings and route contracts. Application
assembly owns shared services and middleware. Jobs and CLI callers invoke domain
actions directly, not through HTTP. A domain starts as one file; split internals
when needed without changing its public import path. This is not a required
crate layout or a mandate for many tiny modules.

Expose named intent: `change_role(ChangeRole)` and
`remove_member(RemoveMember)`. Both may share a private mutation implementation.
Keep optional-role-means-delete mechanics private. A proposed command:

```rust
pub struct ChangeRole {
    pub project_id: i64,
    pub user_id: i64,
    pub role: MemberRole,
}
```

Pass a trusted `Actor` separately from caller-supplied input. Keep HTTP string
IDs and domain IDs distinct where contracts differ; this is useful boundary
conversion, not duplication to remove automatically. Do not fabricate canonical
result records by echoing a request: return an acknowledgment, or read and
return the stored result with appropriate transactional guarantees.

Initially use explicit concrete database access, such as
`&mut SqliteConnection`, rather than a generic context or repository trait. The
caller acquires the connection; the action owns its transaction. This is a
SQLite experiment shape, not the final portable database API. The domain must
not depend on HTTP `AppState`.

HTTP assembly should expose a small function such as
`http::memberships::routes()` using the selected existing router/OpenAPI
integration. Root assembly should not manually repeat every DTO and response
registration. Exact APIs remain proposed.

## S05 — Identity, authority, transactions, and side effects

**Accepted direction.** Identity says who called; it does not establish current
permission. Obtain HTTP identity from validated sessions. Deferred commands that
perform new domain actions retain trusted initiating identity and recheck
current authority on execution. Distinguish those commands from delivery of an
already committed effect: S13 proposes narrow service authority and eligibility
checks for outbox delivery, not revival of the initiating user's credentials.
This clarifies the earlier overbroad statement that all jobs repeat actor
authorization. The delivery/revocation policy remains an explicit product
decision.

CLI identity needs an explicit trusted resolution path; an arbitrary actor-ID
flag is not authentication. System authority must be explicit, not a
missing-actor bypass. Rust constructor visibility discourages accidents but is
not a sandbox against other application code.

Authoritative mutable permission checks and invariants must run in the same
correctly serialized transaction as their mutation. The SQLite membership
example uses `BEGIN IMMEDIATE`, then checks ownership, loads the target, counts
owners when needed, mutates, and commits. A policy pre-check or merely declaring
an action transactional does not prevent concurrent last-owner violations. Other
database engines require their own locking/isolation design and tests.

Public actions initially own transactions and require a connection with no
active transaction. Do not silently nest transaction-owning actions. If a real
multi-operation atomic workflow appears, an outer action can reuse internal
transaction-bound operations. Generic composition and automatic retries remain
deferred until their semantics are justified.

External effects follow the outermost commit. Work requiring reliable delivery
uses an outbox persisted with the business mutation; an after-commit callback
alone cannot survive a crash. SMTP acceptance does not mean exactly-once
delivery.

## S06 — Results are contracts for agents, not prose to parse

**Accepted direction; exact Rust types and wire fields proposed.** Distinguish:

1. Success: the action completed according to its contract.
2. Business rejection: a rule prevented the requested operation.
3. Execution failure: execution could not complete normally.

The earlier sketch retained
`MemberOutcome::{Changed, Forbidden, MemberNotFound, LastOwner}`. The later
AI-first discussion favors typed success and action-specific rejections with a
separate execution-failure path:

```rust
// Conceptual only; these are not implemented Iris types.
pub enum ActionError<R> {
    Rejected(R),
    Failed(ExecutionFailure),
}
pub type ActionResult<T, R> = Result<T, ActionError<R>>;
```

This supersedes retaining `MemberOutcome` as the preferred future design sketch,
not the current code. A generic result type is not a generic action execution
trait. The representation, failure taxonomy, and mapping mechanisms remain open.
S15 refines this sketch to `ExecutionFailure<R>` so failed cleanup can preserve
a typed rejection without returning a misleading normal rejection.

Every public rejection needs a stable machine-readable identity, for example
`memberships.last_owner`. Prose supplements it. Renaming a public error code is
an API compatibility change. Central definitions should connect Rust variants,
codes, safe schemas, descriptions and adapter mappings; compiler-exhaustive
mappings and contract checks should catch omissions. Macros are not required to
start, and schema generation cannot prove the right rejection occurs.

Recovery metadata describes constraints, not permission. A last-owner rejection
means another owner is required and unchanged repetition is ineffective. It is
not a bug to repair by weakening policy, nor authorization to promote someone.
Transient failure is not automatically safe retry: safety depends on effect
certainty and the operation's idempotency contract.

## S07 — Separate action results, evidence, and invocation receipts

**Accepted direction.** Do not put every concern in one enormous error object:

| Contract           | Responsibility                                                    |
| ------------------ | ----------------------------------------------------------------- |
| Action result      | Typed success, rejection, or execution failure                    |
| Execution evidence | Observed stage, failure details, mutation certainty, completeness |
| Invocation receipt | Reconcile or safely replay after losing contact, where supported  |

Public responses expose only safe codes, statuses, operation identifiers,
correlation handles and permitted validation details. Privileged inspection may
expose allowed checkpoints, stages, source references and failure
classification. Enforce authorization and data minimization on inspection
itself. Do not expose tokens, cookies, raw SQL, sensitive parameters, or
inaccessible resource facts.

Facts and interpretations stay distinct. Missing logs do not prove an operation
did not run. Reaching the commit stage does not prove commit succeeded. Preserve
`unknown` across layers when certainty is unavailable; never substitute a
confident default. Correlation IDs are neither idempotency keys nor commit
proof.

S13 proposes how to collect and inspect these observations without treating
sampled telemetry as a durable receipt. S14 separates caller-loss guarantees
from execution ownership. S08's receipt facility remains deferred.

The following Problem-style JSON is a proposal, **not the current wire format**:

```json
{
  "type": "urn:iris:problem:memberships:last-owner",
  "title": "Last owner must remain",
  "status": 409,
  "code": "memberships.last_owner",
  "operation": "memberships.change_role",
  "request_id": "req_123",
  "rule": "memberships.at_least_one_owner"
}
```

The current experiment uses `{code,message}`; a type named `Problem` does not
make it RFC 9457-compliant. Problem Details adoption, exact extensions, code
namespace, and compatibility policy still need selection and migration design.

## S08 — Three failure scenarios define the intended behavior

**Accepted semantics; diagnostic field names proposed.** Alice requests her own
demotion from owner to editor:

### Last-owner rejection

Inside the serialized transaction, Alice is the only owner. Return business
rejection `memberships.last_owner`, mapped to HTTP 409. Permitted inspection can
report `stage=enforce_invariant`, `outcome=rejected`, `mutation=not_applied`,
and the observed owner count. Claim only evidence actually collected and
authorized for disclosure. The agent should explain the prerequisite, not retry
unchanged or automatically change access control.

### Contention before execution

SQLite cannot acquire its write lock within the bounded wait while starting
`BEGIN IMMEDIATE`. Map to a safe HTTP 503 infrastructure response. Inspection
can report `stage=begin_transaction`, `failure=database_lock_timeout`, and
`mutation=not_started` **only because that stage establishes these facts**.
Bounded backoff is safe with respect to duplicating this attempt, not a promise
of eventual success. Retry rechecks permissions and business invariants. Do not
give every database-busy error this same guarantee; later-stage failures need
their own classification.

### Response lost after commit

The database commits but the response is lost. The caller receives no Iris
response; a client-created diagnostic must say `origin=client`,
`failure=response_not_received`, `outcome=unknown`. It is not a server claim
that execution failed. Current state can help reconcile the desired result but
does not prove which request caused that state.

**Proposed optional capability:** selected commands accept idempotency keys and
store durable completion records atomically with the mutation. Bind a key to the
caller, operation and request content; reject incompatible reuse. Define record
retention, pending attempts, lookup authorization and replay behavior. A replay
of a completed self-demotion must not simply rerun the original owner check and
lose access to its own receipt, nor disclose results without authorization. The
exact replay access policy remains open.

Agents reconcile via an authorized receipt or retry the same key under an
established contract, rather than blindly submitting a new operation. Outbox
delivery state remains separate from business commit. Durable receipts and
idempotency are not implemented and should not silently alter every action.

## S09 — Verification and agent tooling

**Accepted direction.** Action tests call typed operations against disposable
databases. HTTP tests check identity, CSRF, wire validation, outcome mappings
and schema agreement. Concurrent tests assert the invariant, not only a favored
error ordering. Reviewers define expectations independently of implementation.

The implemented local CLI/MCP pilot discovers conventions, runs fixed checks,
reproduces controlled scenarios and exposes structured test checkpoints. It uses
a shared runner and the official SDK, not a separate business execution model.
Reports distinguish missing evidence, mismatches and stale source state. They
are not protected from repository authors editing their own checks.

Future ideas include authorized runtime inspection and performance diagnosis.
These are not present capabilities, and test checkpoints are not production
traces or deterministic replay. No arbitrary command/SQL tool, production
access, automatic application generation, or generic mutation exposure is
authorized by this design. External permissions must enforce stronger safety
boundaries; a tool allowlist is not a sandbox.

## S10 — Implementation evidence and scope

| Capability                                                              | Status and evidence                                                                                                             |
| ----------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| SQLite/Turso comparison and feedback measurements                       | Implemented experiment; [findings](embedded-db-findings.md)                                                                     |
| Axum APIs, two OpenAPI exporters, generated TS and React                | Implemented experiment; [API guide](../experiments/api-slice/README.md)                                                         |
| Local OIDC/session authentication                                       | Implemented protocol experiment, not real-provider identity assurance; [auth guide](../experiments/api-slice/authentication.md) |
| Atomic invitation/outbox and local mail recovery                        | Implemented experiment; [delivery guide](../experiments/api-slice/delivery.md)                                                  |
| CLI/MCP verification interface                                          | Implemented pilot; [guide](../experiments/agent-interface/README.md)                                                            |
| Membership role/removal workflow                                        | Implemented experiment, now on main; [API guide](../experiments/api-slice/README.md); verification reported below               |
| Domain layout and redesigned result model                               | Proposed; no framework API released                                                                                             |
| Rejection metadata, precise per-code schemas and shared contract export | Proposed in S12; no derive or runtime bridge implemented                                                                        |
| Execution context, causal correlation and bounded evidence collection   | Proposed in S13; no context API, trace persistence or collector implemented                                                     |
| Runtime evidence, durable receipts, idempotency, performance inspector  | Design ideas, not implemented                                                                                                   |
| Controlled AI repair/productivity comparison                            | Deferred by owner                                                                                                               |

The
[independent membership thread](https://ampcode.com/threads/T-01a0d5ff-d9a8-71dc-80a6-0bdb678bf916)
reported 38 workspace tests, 14 API tests, 5 MCP test groups and
browser/contract checks passing in its own checkout. The implementation is now
[pushed](https://github.com/robertguss/Iris/commit/3121dd6264fcc6861d5c355c72472288abdee7a1)
and pulled into this checkout; the agent also reported successful
[GitHub Verify](https://github.com/robertguss/Iris/actions/runs/36182606800).
This is reported verification, not an independent rerun here. Its real protocol
test caught missing MCP enums after adding a CLI scenario; those entries were
fixed there. Dependency setup, Cargo environment, duplicate exporter
declarations and browser-fixture allowlists remain reported friction. These
observations motivate shared definitions, not productivity claims.

## S11 — Open design agenda

- Exact result/rejection/failure types and stable error-code policy.
- Safe public error schemas, existence hiding, and privileged inspection access.
- Minimal route/contract registration using the selected ecosystem integration.
- Actor construction, job identity and explicit system authority.
- Database strategy and transaction composition when a concrete need appears.
- Receipt persistence, replay authorization, retention and idempotency
  semantics.
- Runtime evidence collection, correlation, bounded retention and disclosure.
- Configuration, startup, migrations, jobs and service lifecycle conventions.
- Packaging, generators and compile-loop improvements justified by experience.

Do not interpret this agenda as approval to implement all items. Generic action
traits, resource DSLs, broad macros, policy engines, universal repositories,
automatic retries and production integrations remain deferred or undecided.

## S12 — Static contract declarations for action authors

**Proposed reference design; no implementation authorized by this section.**
Recommend explicit Rust types and exhaustive mappings first. Keep a narrowly
scoped metadata derive as an alternative, not a selected dependency or API. The
goal is reliable machine-readable contracts with useful omission diagnostics,
not the fewest lines of application code.

### What is defined where

| Definition                                                        | Owner                              | Consumers                                          |
| ----------------------------------------------------------------- | ---------------------------------- | -------------------------------------------------- |
| Command, success and business rejection types                     | Domain                             | HTTP, jobs, CLI, action tests                      |
| Stable rejection code, safe summary, rule and recovery constraint | Domain rejection metadata          | Adapters and permitted discovery                   |
| Wire DTOs and conversion/validation                               | HTTP adapter                       | Request parsing and wire schemas                   |
| HTTP status and permitted public error projection                 | HTTP adapter                       | Response renderer and contract exporter            |
| Route, operation ID, success and middleware responses             | HTTP assembly                      | Router, OpenAPI and operation catalog              |
| Invocation outcome, effect certainty, safe retry evidence         | Execution, not static declarations | Authorized diagnostics; outside this static design |

"Define once" means once per semantic fact, not one giant declaration for every
layer. HTTP string IDs and domain integer IDs intentionally differ. A static
rule identifier does not prove the rule was evaluated during a particular
invocation.

### Explicit Rust reference: domain contract

All Iris-specific types and functions below are illustrative. They are not
compiling examples or the current membership implementation. The command and
trusted actor remain separate; the action still implements its own transaction.

```rust
pub struct ChangeRole {
    pub project_id: i64,
    pub user_id: i64,
    pub role: MemberRole,
}

// Success acknowledges completion, including a same-role no-op.
// It does not pretend to be a canonical stored membership record.
pub struct RoleChangeAcknowledged;

pub enum ChangeRoleRejection {
    Forbidden,
    MemberNotFound,
    LastOwner,
}

// Signature only; authorization, SQL and commit remain ordinary Rust.
pub async fn change_role(
    conn: &mut SqliteConnection,
    actor: &Actor,
    input: ChangeRole,
) -> ActionResult<RoleChangeAcknowledged, ChangeRoleRejection>;
```

Transport-neutral metadata has no HTTP status and no automatic retry flag:

```rust
pub enum RecoveryConstraint {
    Unspecified,
    RequiresStateChange { prerequisite: &'static str },
}

pub struct RejectionDescriptor {
    pub code: &'static str,
    pub summary: &'static str,
    pub rule: Option<&'static str>,
    pub recovery: RecoveryConstraint,
}

impl ChangeRoleRejection {
    pub fn descriptor(&self) -> RejectionDescriptor {
        match self {
            Self::Forbidden => RejectionDescriptor {
                code: "memberships.forbidden",
                summary: "This operation is not permitted.",
                rule: None,
                recovery: RecoveryConstraint::Unspecified,
            },
            Self::MemberNotFound => RejectionDescriptor {
                code: "memberships.member_not_found",
                summary: "Member not found.",
                rule: None,
                recovery: RecoveryConstraint::Unspecified,
            },
            Self::LastOwner => RejectionDescriptor {
                code: "memberships.last_owner",
                summary: "The project must retain an owner.",
                rule: Some("memberships.at_least_one_owner"),
                recovery: RecoveryConstraint::RequiresStateChange {
                    prerequisite: "memberships.another_owner_required",
                },
            },
        }
    }
}
```

Adding a variant makes this wildcard-free match incomplete. Stable codes are
explicit: renaming a Rust variant must not rename its public code. Summaries are
display text, never parser input. `Unspecified` means no recovery conclusion,
not "retryable" or "permanent". A prerequisite is necessary, not sufficient for
success; it grants no permission to satisfy it automatically. This metadata does
not assert `not_committed`, prescribe a retry count, or classify a live database
failure. Those depend on execution evidence (S07–S08).

### HTTP maps once, then rendering and export consume the mapping

For the illustrative `POST /api/memberships/role` adapter:

```rust
fn rejection_contract(r: &ChangeRoleRejection) -> HttpRejectionContract {
    let status = match r {
        ChangeRoleRejection::Forbidden => StatusCode::FORBIDDEN,
        ChangeRoleRejection::MemberNotFound => StatusCode::NOT_FOUND,
        ChangeRoleRejection::LastOwner => StatusCode::CONFLICT,
    };
    HttpRejectionContract { status, descriptor: r.descriptor() }
}
```

Runtime rendering consumes `rejection_contract(actual_rejection)`. Export
enumerates permitted rejections and consumes that same function's data. This is
the intended integration boundary, not another separately handwritten table of
status/code strings. An alternative job/CLI adapter does not import HTTP.

The full operation also declares success, invalid-request 400, authentication
401, CSRF 403, and infrastructure 500/503 responses. Domain `Forbidden` and CSRF
share status 403 but are different codes. Group alternatives per status rather
than overwrite one with another. Success wire shape remains open; the example
acknowledgment is not an API migration from the existing response.

The 403/404 distinction assumes authorized disclosure of membership absence. If
a policy instead conceals existence, apply that public projection consistently
before rendering and export. Do not export an internal rule merely because it
exists in the domain metadata. Public OpenAPI and privileged agent discovery may
have different allowed views; sharing definitions does not bypass disclosure
policy. Keep detailed invocation evidence separate.

Existing serde/schema derives can supply serialization and DTO schemas. Current
utoipa endpoint attributes do not automatically consume arbitrary Rust
descriptor functions. A handwritten response-export bridge is a possible first
integration; it still needs schema registration and per-status grouping.
Selecting the exact library mechanism requires an implementation experiment, not
a spec assertion.

For machine clients, describe permitted errors as branches with a required
literal `code`, rather than one unconstrained global error enum under every
response. An example or discriminator label alone is not a literal-code
constraint. If rules or recovery data appear on the wire, preserve their
code-specific relationships in the schema. Check generated TypeScript narrowing
against real responses. Schema annotations are not runtime validation: canonical
positive 64-bit string IDs still need bounds and format checks at the adapter.

Old clients need a safe unknown-code path. Keep an unrecognized response
separate from the known typed union, rather than cast it to a known error, parse
prose, or infer retryability. Adding an unconstrained `code: string` branch to
the known union can undermine narrowing. Exact compatibility policy remains
open.

The decoder boundary must also cover request failure, malformed successful JSON,
empty responses and non-JSON gateway errors. Wrapping only the value of
`await client.POST(...)` misses exceptions thrown before decoding. Preserve
transport/protocol observations separately from validated business outcomes;
neither a decoding failure nor an unknown code proves no effects occurred. Do
not expose raw response bodies as agent diagnostics by default.

### Existing enumeration before an Iris derive

Prefer an ecosystem enumeration derive plus exhaustive descriptor/status matches
for unit rejection enums. `strum::VariantArray` is a candidate identified by the
independent review and oracle assessment, not an added dependency. This closes
the handwritten enumeration gap without an Iris attribute language. It still
requires an explicit dependency/feature selection, schema registration, safe
public projections and contract tests. Enumeration of payload discriminants
would enumerate kinds, not every payload or its disclosure policy.

The custom derive below remains a deferred alternative, not the default next
step. Reconsider it only if existing derives and ordinary matches leave a
demonstrated authoring or diagnostic problem.

### Narrow derive alternative

Instead of writing `descriptor()` and maintaining enumeration, the author could
write metadata alongside each variant. Illustrative syntax for one variant:

```rust
#[derive(RejectionContract)] // Hypothetical; no such Iris macro exists.
pub enum ChangeRoleRejection {
    #[rejection(
        code = "memberships.last_owner",
        summary = "The project must retain an owner.",
        rule = "memberships.at_least_one_owner",
        recovery = requires_state_change(
            prerequisite = "memberships.another_owner_required"
        )
    )]
    LastOwner,
    // Forbidden and MemberNotFound would each require explicit metadata too.
}
```

The derive would generate only descriptors and enumeration. It would not infer
codes from names, add HTTP status attributes to the domain, generate
transactions, implement policies or expose executable MCP tools. Initially
limiting it to unit variants avoids pretending arbitrary error payloads can be
serialized safely. Payload-bearing rejections would need an explicit design and
disclosure rules.

| Work                                                  | Explicit reference                     | Narrow derive alternative                 |
| ----------------------------------------------------- | -------------------------------------- | ----------------------------------------- |
| Commands, action body, authorization and transactions | Author                                 | Author                                    |
| Rejection meaning, stable code, summary and recovery  | Author supplies match data             | Author supplies attributes                |
| Descriptor implementation                             | Author, compiler checks match coverage | Generated                                 |
| Variant enumeration                                   | Author; omission risk                  | Generated from enum                       |
| Wire conversion, public projection and HTTP mapping   | Author                                 | Author                                    |
| DTO serialization/schema and client generation        | Existing ecosystem tools               | Same tools                                |
| Shared runtime/export/catalog bridge                  | Future integration work                | Still required; derive does not supply it |

The derive's concrete advantage is complete enumeration and missing-metadata
diagnostics, not automatically better business correctness. Its cost is a new
attribute language, compile work, expansion/debugging behavior, and maintained
diagnostics. Revisit after the explicit reference exposes recurring friction.
Require source-local errors and compile-fail cases for missing metadata, invalid
recovery syntax, duplicate codes within the enum and unsupported shapes. Cross-
operation collisions still need catalog checks and a deliberate code-reuse
policy.

### What verification can actually establish

- **Compiler today, with explicit matches:** missing descriptor/status match
  arms and type-invalid conversions. Avoid wildcard arms that hide new variants.
- **Not compiler-proven:** completeness of a handwritten `ALL` list used for
  export. A test iterating only `ALL` cannot discover variants omitted from it.
  Independent expected-code fixtures and business cases help, but do not prove
  coverage of every future variant. A derive could close the enumeration gap.
- **Contract checks:** independently expected codes/statuses, actual response
  schema validation, middleware responses, stable code compatibility, operation
  ID uniqueness and consistent metadata when a code is intentionally shared. Do
  not derive every expected result from the same descriptor under test.
- **Client checks:** generated types narrow correctly on known codes; unknown
  responses remain untrusted and do not trigger speculative recovery.
- **Business tests and review:** the correct rejection occurs, permission is
  enforced, concurrent last-owner mutations remain safe, and metadata is
  truthful. Neither a derive nor an OpenAPI document proves these properties.

This pass recommends explicit Rust as the reference specification, not an
implementation commitment or a claim of fully single-source contracts today. S13
develops the runtime evidence proposal; durable invocation receipts remain
deferred rather than becoming an implicit telemetry feature. Static discovery
describes allowed operations and contracts; making an MCP mutation callable
remains a separate explicit exposure and authorization decision.

## S13 — Execution context and evidence lifecycle

**Proposed for review, not implemented or a settled framework API.** This
section connects S12's static declarations to observations of actual execution.
It does not introduce a generic action executor, durable audit subsystem,
production inspection service, or claim that telemetry establishes business
correctness. Librarian research and oracle critique informed the proposal;
reviewers should challenge the recommendations and failure cases below.

### Ecosystem capabilities versus Iris responsibilities

Use existing Rust `tracing` spans/events for in-process instrumentation and
consider `tracing-opentelemetry` plus an OpenTelemetry SDK/exporter for
distributed correlation. No versions, exporter, collector backend or dependency
additions are selected. Check compatible releases before implementation.

- `Instrument` enters a span when an async future is polled. Do not hold a
  manual span-enter guard across `.await`; it can associate unrelated work with
  the wrong span. Spawned tasks need deliberate context propagation.
- `#[instrument]` captures arguments by default. Prefer `skip_all` and explicit
  allowlisted fields at sensitive boundaries, but also review nested events,
  return recording and formatted errors. `skip_all` alone is not redaction.
- W3C trace propagation provides correlation; syntax validation does not
  authenticate the sender. Parent context and links express relationships, not
  authorization, transaction membership or durable completion.
- Sampling, finite queues, export failures and process death can lose evidence.
  Force-flush/shutdown are bounded best-effort operations, not a durable audit
  guarantee. Tail sampling cannot recover data already dropped at the source.

Iris still owns checkpoint meaning, safe fields, failure/effect interpretation,
coverage reporting, source/build association, inspection authorization and the
relationship between business records and telemetry. Do not build a tracing
backend to replace ecosystem tools just to obtain those conventions.

### Small explicit context, not a service container

A conceptual extension of the action signature is:

```rust
// Proposed only. Actor and database remain separate explicit dependencies.
change_role(conn, actor, input, &execution).await

// Illustrative contents, not a public construction or persistence API:
struct ExecutionContext {
    invocation_id: InvocationId,
    deadline: Option<Instant>,
    cancellation: CancellationSignal,
}
```

Context creation belongs at trusted invocation boundaries. An action that opts
into this convention receives a normal borrowed context; no deadline and no
cancellation request are ordinary values, not a reason for pervasive optional
context parameters. Generate new invocation identity explicitly for a new
invocation; do not silently copy identity into unrelated work.

Do not put database handles, actor privileges, arbitrary request data,
exporters, or a generic service registry in this context. Spans are
instrumentation details, not business authority. The context itself neither
starts transactions nor guarantees cancellation safety. Whether every public
action eventually needs it remains open; the signature is not a current
migration requirement.

### Distinguish identifiers without creating one for every event

| Identifier                | Meaning and proposed lifetime                                                                                |
| ------------------------- | ------------------------------------------------------------------------------------------------------------ |
| Operation name            | Stable declaration, e.g. `memberships.change_role`                                                           |
| Local request ID          | One adapter interaction, including requests rejected before an action                                        |
| Invocation ID             | One action execution; exists even if tracing is disabled                                                     |
| Durable job identity      | Existing outbox row identity within its database/deployment scope; this experiment can reuse `invitation_id` |
| Attempt                   | Committed claim counter paired with job identity; not proof that SMTP ran                                    |
| Trace/span IDs            | Optional instrumentation and cross-service correlation                                                       |
| Idempotency key / receipt | Separate S08 capability; neither request ID nor trace ID substitutes for it                                  |

A strictly one-request/one-action adapter may reuse a generated value for
request and invocation IDs while keeping their meanings distinct. Fan-out
requires distinct invocations. There is no need for an additional lease UUID:
the existing job identity and claim counter supply the attempt fence.
Database-local integer IDs need trusted deployment/database scope when joining
observations across restarts, disposable resets or multiple environments.

Default recommendation at untrusted ingress: generate local correlation and a
local trace root. An optional remote-context link is explicitly untrusted;
continuing a remote parent requires a configured trusted-ingress policy. Never
use caller-supplied correlation as actor identity, tenant authorization, or an
unbounded sampling request. Drop baggage by default; any allowed keys require
size, value, onward-propagation and disclosure rules. Do not place credentials
or authority in baggage.

### Evidence separates observation, effects, and collection coverage

Keep three dimensions separate:

1. **Observed action result:** success, rejection, failure, or no observed
   terminal result. An ended/dropped span alone is not a successful action.
2. **Scoped effect assessment:** what is known about a membership mutation,
   invitation/enqueue commit, SMTP exchange, or fenced acknowledgment.
3. **Collection coverage:** which records this collector retained or cannot
   establish. This is not the outcome of the business operation.

An effect statement needs a basis, scope and observing source. Illustrative
inspection output, not a selected wire schema:

```json
{
  "schema_version": 1,
  "invocation_id": "inv_123",
  "operation": "memberships.change_role",
  "observation": {
    "checkpoint": "membership.invariant_rejected",
    "rule": "memberships.at_least_one_owner",
    "observer": "application"
  },
  "effect": {
    "scope": "membership_mutation",
    "assessment": "not_applied",
    "basis": "rejection_before_mutation"
  },
  "collection": {
    "scope": "this_invocation",
    "coverage": "unknown",
    "reason": "terminal_record_unavailable"
  }
}
```

A retained checkpoint may support a narrow fact despite incomplete overall
coverage. `observer=application` identifies the source, not independent proof.
Evidence should carry schema and producer/build versions and safe source
references when available, so an agent does not diagnose old behavior as current
code. Missing or mismatched provenance stays explicit. Do not automatically
export full paths or source excerpts.

Avoid unqualified `complete=true`. A collector may know its aggregate queue
overflowed without knowing which invocation lost records. Report that limited
knowledge; do not fabricate per-invocation loss counts. Conversely, no observed
drop does not prove completeness. Every completeness claim needs a bounded scope
and collection boundary. Missing terminal records must not silently mean
success, failure, rollback, or still-running.

Commit success observed by the application is useful evidence, but a crash can
occur before that event is emitted/exported. Trace presence is not a durable
receipt. Durable domain state can corroborate some facts without reconstructing
which invocation caused them. A same-role success may commit without changing
the role; a terminal outbox `dead` row does not prove earlier SMTP attempts had
no effect. Preserve these distinctions in summaries and automated advice.

### Worked flow A: changing a membership role

The independently implemented SQLite algorithm supplies the business sequence;
the following runtime observations are proposed, not present instrumentation:

1. HTTP establishes trusted identity and local correlation. Optional request and
   action spans describe the work; the action gets its invocation context.
2. After `BEGIN IMMEDIATE` succeeds, record acquisition of the write
   transaction. A known lock-acquisition failure before this point supports
   `not_started`.
3. Inside the transaction, check current actor authority, load the target, and
   enforce the owner invariant when necessary. Explicit business checkpoints
   describe only checks actually executed. Do not disclose target existence to a
   caller who failed authorization; owner count is privileged evidence.
4. Last-owner rejection before mutation supports
   `membership_mutation=not_applied`. Successful mutation SQL alone describes an
   uncommitted change, not success.
5. Record commit success only after the commit call returns success. A commit
   error or dropped future does not justify a universal rollback assertion.
6. A successful action and a successfully received HTTP response are separate
   facts. Lost response leaves the client's outcome unknown, as in S08.

Process death after commit but before emission leaves an evidence gap. Readback
may reconcile desired state but cannot identify its cause without a suitable
durable record. Rule checks and SQL statements stay action-owned; generic
instrumentation cannot discover them by reading function signatures.

### Worked flow B: invitation followed by outbox delivery

Current code atomically persists the invitation and outbox payload. The worker
commits a claim with a 30-second lease and incremented attempt, performs SMTP
outside the transaction, then updates state with an attempt-and-lease fence. The
limit is five claimed attempts, not proof of five actual transmissions.

Proposed observations and correlation:

1. After issuance commit returns, observe `invitation_and_enqueue_committed`.
   Any future persisted enqueue correlation must be bounded and written with
   that transaction. Missing trace context must not prevent business enqueueing.
2. Start a new delivery-attempt invocation after claim commit, associated with
   job identity and attempt number. Prefer a fresh attempt trace with a link to
   enqueue context where available. Continuing the original trace is technically
   valid, but long queues/retries have separate lifetimes and sampling needs.
3. Never resurrect the HTTP deadline, cancellation signal or credentials. The
   worker applies its own attempt budget and scoped service authority.
4. Observe `smtp_acceptance_observed` only when SMTP returns success. A timeout
   can mean acceptance is unknown, not definitely rejected. Acceptance is not
   inbox delivery. Do not export recipient, token, body, or token-derived
   Message-ID as correlation metadata.
5. Observe completion independently: `completion_update_applied` means the
   fenced database update returned true, with the recorded state (`sent`,
   `pending`, or `dead`). False means this call applied no transition; it does
   not alone reveal why. An error means resolution needs investigation, not a
   fabricated state.

Important failure windows:

| Window                                          | Permitted conclusion                                                         |
| ----------------------------------------------- | ---------------------------------------------------------------------------- |
| Crash after claim commit, before send           | Attempt consumed; no proof a send occurred                                   |
| Timeout during SMTP exchange                    | Acceptance may be unknown; retries may duplicate                             |
| SMTP accepted, then crash before acknowledgment | Possible duplicate on recovery; pending row does not establish no acceptance |
| Lease expired/replaced; old worker sends        | Fence protects the database acknowledgment, not the SMTP side effect         |
| Fenced completion returns false                 | No state transition by this completion call, regardless of SMTP observation  |
| Terminal row inspected later                    | Current durable state, not a complete history of attempts                    |

Current observation gaps: `send` collapses several outcomes into `Retry`; the
worker's `tick` currently ignores the boolean returned by `complete`. A future
evidence adapter must observe these distinctions before claiming them. This spec
does not fix that code or claim those records exist.

**Authority policy for review:** distinguish delivering committed intent from
executing a new user command. Recommend preserving current semantics: revoking
issuer ownership does not retract an already-issued notification. A service
worker checks delivery eligibility, not the original owner's present authority.
Acceptance/expiry suppress future claims; they do not cancel an in-flight send.
If revocation must suppress delivery, define a separate explicit revocation
rule. Stored initiating identity, if later added, is provenance rather than
reusable credentials. The current outbox does not persist that initiating actor.

### Cancellation, deadlines, and task ownership

Cancellation is cooperative signaling at action-defined safe boundaries. Do not
automatically race every SQL operation or commit against a cancellation token.
Client disconnect, timeout and dropping a future do not establish rollback or
absence of external effects. Ignoring a cancellation token does not keep a
future alive if its owner drops it.

Cleanup should have a separate bounded policy rather than immediately aborting
because the caller's signal is set. A cleanup budget still cannot guarantee
completion after task termination or process crash. If accepted commands must
finish after disconnect, introduce explicit execution ownership as a separate
design; this context does not supply a supervisor or durable job executor.

Use monotonic time for local durations and deadlines. It cannot be persisted as
a cross-process deadline. Workers create their own budget; business expiry uses
the application's explicit durable clock semantics. Cross-host wall timestamps
are presentation aids, not a total causal order. Links, local ordering and
durable attempt counters provide only the relationships they actually establish.

### Collection, privacy, and performance boundaries

Recommend ordinary telemetry be bounded, best-effort, and fail-open relative to
business execution. Exporter outages must not change authorization or business
results. A required durable audit trail would need separate availability and
transaction guarantees, not a silent change to this policy.

| Collection option                         | Benefit                                         | Limitation / disposition                                                                           |
| ----------------------------------------- | ----------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| Bounded local ring buffer / local sink    | Low-friction future dev inspection              | Eviction and restart lose data; single-process scope; first candidate, not selected implementation |
| OTLP to existing collector/backend        | Cross-process correlation and established tools | Extra operations, privacy and cost; sampling/export still lose data                                |
| Transactional durable audit/receipt store | Stronger business-linked history where designed | Schema, retention, failure and authorization costs; separate deferred design                       |

Choose limits explicitly before implementing a collector: field lengths, record
sizes, events per invocation, queue capacity, retention/eviction, query pages
and time windows. No numerical defaults are selected in this design pass. Expose
collector health/drop/eviction information with its real scope. Bounded flush on
shutdown reduces loss but does not establish durability. Inspection must
disclose its own unavailable or degraded state without disrupting domain
execution.

Use safe operation/query identifiers, error categories and typed checkpoints. Do
not capture raw SQL/parameters, HTTP bodies, cookies, headers wholesale,
invitation tokens, email addresses/bodies, environment values or full error
Debug chains. Existing token-bearing success results make indiscriminate return
recording dangerous. Inspect nested instrumentation and use canary-secret tests
before any production use; type derives do not provide a security boundary.

Inspection is separately authenticated and scoped to permitted resources and
fields. Possession of an invocation/trace handle grants no access. Do not reveal
whether a forbidden record exists. Treat runtime strings as data, not
instructions for an agent; no suggested shell commands or arbitrary SQL from
event payloads. Public error responses should expose only safe correlation, not
internal logs.

Per-invocation IDs belong in bounded diagnostic records, not metric labels.
Metrics use bounded operation/outcome/stage categories. Durations can separate
lock wait, transaction work, SMTP and export overhead, but a slow span is an
observation, not a proven root cause. Sampling biases and absent intervals must
remain visible; do not add overlapping span durations as if they were serial.

### Review decisions, alternatives, and validation criteria

These are recommendations to challenge, not assertions of owner-approved APIs:

- Keep invocation correlation independent of tracing. Alternative: reuse action
  span identity, accepting that disabled tracing can remove the handle.
- Keep context narrow and explicit. Alternative: purely ambient context, with
  less signature noise but less visible propagation and cancellation ownership.
- Prefer new attempt traces plus links. Alternative: one long trace, accepting
  queue lifetimes, retention and sampling coupling; both are valid OTel models.
- Preserve committed-notification service authority. Alternative: add explicit
  revocation semantics; never silently reuse the initiating user's credentials.
- Make ordinary telemetry lossy without blocking business execution.
  Alternative: required durable audit, with explicit availability and
  persistence costs.

Future acceptance criteria, **not executed tests in this design pass**:

1. Disabled/sampled/overflowing/unavailable telemetry preserves business
   behavior and local invocation identity; coverage does not pretend all events
   survived.
2. Interleaved async actions retain correct span association. Forged propagation
   and baggage cannot change identity, authority, inspection scope or
   boundedness.
3. Canary secrets in inputs, token-bearing results and nested errors do not
   appear in exported fields, metrics, propagated context or inspection
   responses.
4. Lock failure, invariant rejection, commit ambiguity, dropped futures and lost
   responses produce only the effect certainty supported by actual observations.
5. Crash windows around enqueue, claim, SMTP and fenced completion retain
   ambiguity; stale attempts cannot acknowledge a newer attempt, but diagnostics
   still allow the possibility of duplicate external delivery.
6. Eviction, exporter drops, process restart and schema/build mismatch do not
   create fabricated causal history or per-invocation completeness claims.
7. Revoked/unrelated inspectors cannot read records or distinguish forbidden
   existence. Retention, query limits and multi-environment ID scope are tested.

### Brief for other LLM reviewers

These questions focus on S13. For a whole-framework review, use the
[independent review brief](design-review-brief.md), which supplies the shared
assignment, authoring/security/AI-usability tracks, and findings format.

Review this self-contained spec as a design for a personal API-first Rust
framework assembled from existing crates, with React and AI-authored
applications. The authoring model is ordinary transaction-owning actions plus
shared transport contracts, not a resource DSL. The current CLI/MCP tool exposes
controlled test evidence only. Runtime context, tracing, inspection and durable
invocation receipts described here are proposals; do not assess them as deployed
features.

Evaluate S13 against S05–S08 and S12. In particular:

- Identify any place a proposed observation overclaims commit, rollback,
  delivery, retry safety or completeness. Give a concrete counterexample
  interleaving.
- Challenge the context and identifier budget. Which concepts can be removed
  without losing useful agent feedback or confusing durable and ephemeral state?
- Assess whether trusted-ingress, baggage, redaction and inspection
  authorization prevent disclosure and false attribution; distinguish trust from
  correlation.
- Test the job-authority distinction against self-demotion, issuer revocation,
  accepted/expired invitations and stale workers. Name product choices versus
  bugs.
- Compare the proposed ecosystem use with simpler existing capabilities. Flag
  mechanisms that require original framework code or stronger database
  guarantees.
- Explain what evidence could actually help an agent distinguish failure cases,
  and what still needs human intent or independent behavioral verification.

Return findings with section ID, severity, assumption, failing sequence,
smallest correction, tradeoff, and what would validate it. Separate confirmed
contradictions from questions and optional enhancements. Suggest alternatives
when useful; do not turn every uncertainty into a platform subsystem. No
implementation, production access, or productivity study is requested by this
review brief.

## S14 — Caller loss, execution ownership, and recovery

**Recommended baseline; stronger guarantees and exact APIs remain proposed.**
This section refines S07–S08 and S13 after the first independent review and
oracle assessment. It does not install an executor, transaction helper, receipt
store or retry policy. See the
[review synthesis](reviews/opus55-all-01-synthesis.md) for dispositions and
snapshot limits. Further independent reviews remain welcome.

### Baseline: loss of contact permits uncertainty

Recommend that ordinary actions promise truthful, scoped observations, not
completion after caller loss. If a client has no validated terminal response or
authorized durable receipt, it retains `outcome=unknown`. The server may have
more evidence than the client; neither side invents the other's knowledge.
Unknown is a useful machine result, not a generic error to retry away.

Execution ownership and durable reconciliation are independent capabilities:

| Capability                                     | What it adds                                                                                 | What it does not establish                                                           |
| ---------------------------------------------- | -------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| Baseline truthful uncertainty                  | Explicit limits on result/effect claims after contact is lost                                | Completion, rollback or safe retry                                                   |
| Bounded server-owned execution                 | An admitted task can outlive its request waiter while the server retains ownership           | Survival of abort, panic or process death; delivery of a terminal record             |
| Durable reconciliation for selected operations | A suitably committed record can identify an invocation's recorded result after response loss | Survival of uncommitted work, indefinite retention or exactly-once external delivery |

A receipt does not require detached execution, and detached execution does not
create a receipt. Keep request-associated execution as the minimal reference; do
not promise a specific Axum disconnect/drop behavior without testing it. Add
bounded server-owned execution only for a concrete operation requirement, with
admission limits, task supervision, shutdown and cleanup policies. Long work
that must survive process loss needs a separately designed durable job contract.
S13 context propagation alone supplies none of these mechanisms.

This recommendation accepts ambiguity rather than imposing persistence and
supervision costs on every action. An application that requires a retrievable
answer must opt into a stronger contract; Iris must not advertise the baseline
as satisfying that requirement.

### Lifecycle and effect table

These are interpretation rules for future evidence, not records already emitted
by the experiment. Database facts concern the named transaction only; they do
not cover external calls, other connections, earlier attempts or buggy action
code. Cleanup and connection reuse require driver-specific validation.

| Observed event or failure window                                   | Permitted server-side conclusion                                                                           | Caller/agent recovery constraint                                                                                             |
| ------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| Known write-lock acquisition failure before the transaction begins | This transaction's writes did not start; not a classification for every begin error                        | If a validated response establishes this fact, a new authorized attempt may use bounded backoff; otherwise retain unknown    |
| Business rejection before mutation                                 | Named mutation was not applied, if that ordering is established; no proof the rule was evaluated correctly | Explain the rejection's prerequisite; unchanged repetition is not a repair                                                   |
| SQL writes succeed, commit not yet acknowledged                    | Writes executed in an uncommitted transaction; final effect not established                                | Do not report successful completion or safe replay                                                                           |
| Explicit rollback completes successfully                           | This transaction's writes were rolled back under the driver's contract                                     | Keep the original rejection/failure; retry still depends on authority and operation semantics                                |
| Rollback requested but cleanup fails, times out or is dropped      | Cleanup is unconfirmed; independently established facts such as rejection-before-write still hold          | Preserve primary failure and cleanup failure separately; do not infer reusable connection or upgrade uncertainty to rollback |
| Commit returns error, or its result is lost                        | No universal commit/rollback conclusion; classify only with engine-specific evidence                       | Reconcile under an existing receipt/idempotency contract, or retain unknown                                                  |
| Commit succeeds, response delivery is lost                         | Server observed this transaction commit; client may have no such evidence                                  | State readback may establish desired state, not which invocation caused it                                                   |
| Request waiter disappears while an action may be running           | Contact was lost, not necessarily execution; ownership policy determines who retains the future            | Cancellation request is not rollback or confirmed cancellation                                                               |
| Process dies, restarts, or terminal evidence is missing            | Collector lacks a terminal observation; missing evidence does not locate the crash relative to commit      | A new process identity plus an absent record proves neither non-execution nor non-commit                                     |
| SMTP acceptance observed, acknowledgment missing                   | External acceptance and durable outbox completion are distinct facts                                       | Retry may duplicate delivery; receipt of business commit is not a delivery receipt                                           |

Stage classification belongs where begin/commit/rollback results are observed,
not in a generic SQL-error-to-HTTP mapper. A future helper may centralize these
facts but cannot certify business checks or forbid effects outside its scope. Do
not replace an original failure with a rollback error through an unqualified
`?`. Exact error composition and whether an unhealthy connection is discarded
remain implementation questions, not claims about current behavior.

### Agent recovery must use facts held before the response is lost

For operations offering reconciliation, the client must hold the lookup/key
material before the uncertain response. It may be client-generated or previously
issued by the server. Validate size, namespace and request binding; a public
handle grants neither authority nor proof of execution. It is separate from the
server's locally generated invocation identity unless an explicit contract binds
them. Baseline operations do not acquire receipts by adding a header.

Describe recovery as separate capabilities, not a single `retryable` flag:
authorized lookup, desired-state readback, replay under a bound idempotency key,
and issuance of a new command have different semantics. Record the evidence
required for each, including retention and authorization limits. Not found or
expired receipts do not prove that an invocation never committed unless the
receipt contract explicitly establishes that conclusion.

Counterexamples agents and reviewers must preserve:

- Alice's self-demotion commits and its response is lost. A new call receives
  `Forbidden`. That rejection is about the new call, not proof the first failed.
- Another owner restores Alice before a resend. Sending the same desired role
  now performs a new mutation; an idempotent-looking setter is not proof that
  automatic resend is appropriate after intervening changes.
- An invitation expires between attempts. Reissuing can create a new token and
  outbox job rather than replay the earlier issuance.
- A read shows the desired state, then another writer changes it before retry.
  Read-before-retry is not a concurrency fence. A later `changed` flag does not
  identify the earlier invocation's effects.

Until a receipt contract exists, report unresolved outcome rather than invent a
lookup or automatically resend. Agents may explain what authorized evidence is
available; they may not weaken policy or mutate state merely to diagnose it.

### Focused validation before choosing implementation mechanisms

These are future checks, not executed tests or productivity studies:

1. Exercise begin failure, rejection-before-write, acknowledged rollback and
   failed cleanup separately; preserve stage and both primary/cleanup causes.
   Verify the connection's safe reuse or disposal under the chosen driver.
2. Lose a response on both sides of commit. Assert client uncertainty even when
   the server has a terminal result, and do not manufacture a terminal result
   from absent logs after restart.
3. Drop the waiter before mutation and during commit under each proposed
   ownership policy. Verify only its stated continuation/cleanup guarantees.
4. Replay the self-demotion, intervening role restoration and expired-invitation
   sequences above. Reject any universal safe-resend inference.
5. For a future receipt design, test atomicity, incompatible key reuse,
   concurrent duplicate submissions, self-demotion access, revoked/unrelated
   lookup, expiration and lost receipt-write acknowledgment.
6. Exercise malformed/unknown/empty/non-JSON responses and network exceptions at
   the client boundary. They must not become fabricated business outcomes or
   automatic retries, and raw sensitive bodies must not leak into diagnostics.

The next design step is an operation-specific receipt/replay contract if durable
reconciliation is required, or a small stage-aware transaction evidence design
if the baseline suffices. Do not select an executor merely to make telemetry
look complete. Implementation remains separate work.

## S15 — Reference result and recovery contract for agents

**Proposed, not implemented or a settled wire/API migration.** This reference
applies S06–S14 to membership role changes. Oracle critique informed finalized
rejection, cleanup preservation and the distinction between action failure and
request-response failure. Types and JSON below are design sketches; the current
API still returns `MemberChange` or `{code,message}`. No receipt, inspector,
executor, automatic retry or new MCP mutation capability is introduced.

### Action results preserve why execution stopped

```rust
// Conceptual single-transaction reference; not serializable wire DTOs.
pub type ActionResult<T, R> = Result<T, ActionError<R>>;

pub enum ActionError<R> {
    Rejected(R),
    Failed(ExecutionFailure<R>),
}

pub struct ExecutionFailure<R> {
    primary: StopReason<R>,
    cleanup: Cleanup,
}

enum StopReason<R> {
    Rejected(R),
    Execution { stage: Stage, kind: FailureKind },
}

enum Cleanup {
    NotRequired,
    RollbackAcknowledged,
    Unconfirmed { stage: Stage, kind: FailureKind },
}

pub struct RoleChangeAcknowledged;
```

`Stage` and `FailureKind` stand for bounded diagnostic vocabularies, not driver
messages. Initial stage candidates are connection, begin, body and commit;
cleanup reports rollback separately. Application checkpoints explain which
business rule ran without generating a stage for every source line. The exact
failure taxonomy and driver mapping remain open. These types model one owned
transaction, not arbitrary compensation or distributed transactions.

Recommend committing a successful body and rolling back a stopped body. The
transaction-owning function finalizes its result; no generic executor is needed.
This differs from the current experiment, which commits `Ok(MemberOutcome)`
including rejections. The proposed invariants are:

- `Rejected(r)` is final only after this boundary's cleanup obligations are
  satisfied, or none arose. It is not merely a body's intention to reject.
- Rejection followed by failed rollback becomes `Failed` with
  `primary=Rejected(r)` and `cleanup=Unconfirmed`. The original typed rejection
  survives privately; its usual public rejection status does not survive.
- An execution fault followed by successful rollback remains `Failed`, retaining
  the primary stage/kind and `RollbackAcknowledged`. Cleanup must not replace
  the primary reason via an unqualified `?`.
- `NotRequired` is a positively established absence of cleanup obligations, not
  a synonym for unobserved cleanup or a consumed transaction handle.
- A dropped task or process may produce no `ActionResult`. An observer reports
  missing evidence rather than fabricating a returned failure.
- Success acknowledges transaction completion, including a same-role logical
  no-op. It promises neither enduring current state nor zero SQL/database work.

Keep construction at the owning boundary. Types/private fields cannot prove
cleanup occurred. An internal body may return `Result<T, StopReason<R>>`, but a
blanket conversion encouraging rejection propagation past cleanup is unsuitable.
Converting rejection types must handle both top-level `Rejected` and a rejection
inside `Failed.primary`. This is the cost of preserving typed causes instead of
erasing them into strings; generic composition helpers remain deferred.

Effects stay separate: failed cleanup does not erase independently established
rejection-before-write evidence. Conversely, a rejection alone cannot prove that
an arbitrary action performed no effects. A successful rollback covers only its
transaction under the selected driver's contract.

### Public responses describe the request, not every internal fact

Recommend evaluating a small versioned tagged envelope. This is an alternative
to S07's Problem-style sketch, not a decision to adopt both. A future migration
must select the format and update rendering, export, clients and compatibility
checks together. Machine decisions use literal tags/codes, never message prose.

Candidate HTTP 200 success:

```json
{
  "schema_version": 1,
  "operation": "memberships.change_role",
  "request_id": "req_6f23d890a1b24c55b147d920cbe47810",
  "kind": "success",
  "data": { "completion": "acknowledged" }
}
```

Candidate HTTP 409 finalized rejection:

```json
{
  "schema_version": 1,
  "operation": "memberships.change_role",
  "request_id": "req_70d3a890a1b24c55b147d920cbe47811",
  "kind": "rejected",
  "code": "memberships.last_owner",
  "message": "The project must retain an owner."
}
```

Candidate HTTP 500 public failure, including rejection plus failed cleanup:

```json
{
  "schema_version": 1,
  "operation": "memberships.change_role",
  "request_id": "req_80d3a890a1b24c55b147d920cbe47812",
  "kind": "failure",
  "code": "iris.internal",
  "message": "A normal response could not be produced."
}
```

`failure` means a server-reported request failure, **not necessarily a failed
action**. Post-action middleware can fail after business commit. The current
[authentication boundary](../experiments/api-slice/server/src/auth/mod.rs)
contains fallible work after `next.run`; this is a real boundary to account for,
not a claim that every membership request triggers such a failure.

Pre-action refusals have a distinct `kind=refused`, for example HTTP 403 with
`code=http.csrf_refused`, using the same envelope fields. This means the named
action was not dispatched for this request, not that middleware did no work. Do
not infer refusal from HTTP status: domain forbidden and CSRF are distinct. Only
responses whose producer knows the operation may assert its identity; unrouted
or gateway errors need a separate contract or the unknown client path.

Use generic `iris.unavailable` at 503 without effect certainty. The current
[database error mapper](../experiments/api-slice/server/src/lib.rs) is not
stage-aware. A future separately declared `memberships.write_not_started` code
could expose positively established lock-before-begin evidence for this
invocation's membership writes only. It would not establish authorization,
absence of all request effects or permission to retry. That code and its
disclosure policy are optional and unimplemented; generic 503 never implies it.

### Privileged evidence is a separate authorized projection

Candidate diagnostic for a last-owner rejection followed by failed rollback:

```json
{
  "schema_version": 1,
  "producer": { "build": "example-build" },
  "operation": "memberships.change_role",
  "request_id": "req_80d3a890a1b24c55b147d920cbe47812",
  "observed_result": "failed",
  "primary": { "kind": "rejected", "code": "memberships.last_owner" },
  "cleanup": {
    "observation": "unconfirmed",
    "stage": "rollback",
    "kind": "io"
  },
  "effect": {
    "scope": "this_invocation.membership_mutation",
    "assessment": "not_applied",
    "basis": "rejection_before_mutation",
    "observer": "application"
  },
  "collection": { "scope": "this_invocation", "coverage": "unknown" }
}
```

This is a permitted interpretation only if the ordering was actually observed.
The example uses S13's one-request/one-action identity reuse; fan-out requires
separate invocation identity. Inspection enforces resource/field authorization;
public error rendering does not serialize this object. No raw SQL, parameters,
credentials, arbitrary rejection payloads or unrestricted error chains belong
here. Missing producer/build association remains explicitly unknown. Even
privileged observations are evidence, not proof the application is correct.

### Client validation distinguishes server facts from local uncertainty

The proposed client result has two outer cases:

```text
ValidatedServerResponse(Success | Rejected | Failure | PreActionRefusal)
ClientUnknown(reason, local_attempt_id)
```

`ClientUnknown` reasons distinguish transport failure, unreadable/malformed
body, unsupported version, unknown code and contract mismatch. It describes the
client's observation, not a fabricated server failure. Its origin is client and
the original invocation's outcome remains unknown. No raw body or guessed server
request ID is required. A valid server `Failure` is known as a response while
its action outcome/effects can still be unknown.

The boundary owns request execution, body reading, parsing and validation,
including thrown errors. Validate expected operation, supported schema version,
status/tag/code combination and branch-specific payload. A 2xx with malformed
success is unknown, not success. HTML, empty bodies where JSON is required, and
unknown codes stay outside the known business union. Define additive-field
compatibility when selecting the schema; do not cast arbitrary JSON to the
generated TypeScript type.

Wire, evidence and producer/build versions have different meanings. Generate
local request identity at trusted ingress, including refusals; bound its format
(candidate: `req_` plus 32 lowercase hex characters). Caller-supplied strings
cannot become trusted local identity. Validation of a response schema is not
authentication: ordinary transport/origin trust still applies. Handle possession
does not grant inspection access, and response-only IDs do not survive response
loss for the client. Client-local attempt identity cannot locate server work
without an explicit binding. S14's pre-held recovery material remains separate.

The current HTTP operation ID is `changeMemberRole`; the proposed domain name is
`memberships.change_role`. Assembly must define their relationship once if both
are retained, rather than creating independent catalogs with matching names
assumed. None of these proposed envelope fields exist in the current exporter.

### Recovery discovery supplies capabilities, not instructions to mutate

Keep stable recovery constraints in the operation/rejection catalog, rather than
embed a recovery program in every error. Discovery is not authority. Declare
support and preconditions explicitly; absence of a declaration is no capability
an agent may assume.

| Capability           | Minimum declaration when supported                                                                                       | Limit                                                               |
| -------------------- | ------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------- |
| Inspect evidence     | Authorized locator, required lookup material, scope and coverage semantics                                               | Observations can be missing; not a receipt                          |
| Read current state   | Authorized read operation and resource inputs                                                                            | State now, not invocation causality or a concurrency fence          |
| Replay by key        | Pre-held material, actor/operation/content binding, retention and pending/absent/expired semantics, replay authorization | Not a fresh submission; no exactly-once external-effect promise     |
| Submit a new command | Existing operation, current authorization, preconditions and caller intent                                               | New attempt with new effects; never an implicit fallback for replay |

For this reference, runtime inspection and keyed replay are **not implemented**;
no new membership-read endpoint or executable MCP mutation is declared. An
application may expose an authorized state read separately. A new submission
uses the existing HTTP operation only within the caller's authority and intent.
`memberships.another_owner_required` describes a necessary state for last-owner
recovery, not permission to promote someone and not a sufficient fix. Neither
`Failed`, HTTP 503, nor client unknown grants an automatic retry policy.

### Scenario matrix and future checks

All rows describe the proposal. Each should become an independently asserted
case before a future implementation claims this contract; no such tests were
executed for this design pass.

| Scenario                                               | Result/evidence                                                                           | Agent conclusion or constraint                                         |
| ------------------------------------------------------ | ----------------------------------------------------------------------------------------- | ---------------------------------------------------------------------- |
| Last owner, rollback acknowledged                      | `Rejected(LastOwner)`                                                                     | Explain prerequisite; no unchanged retry                               |
| Last owner before writes, rollback fails               | `Failed`, original rejection retained; named mutation not applied if ordering established | Public failure, not 409; inspect only if supported and authorized      |
| Writes then rejection, rollback unconfirmed            | `Failed`, rejection retained; effects unresolved                                          | Rejection alone does not establish no mutation                         |
| SQL fault, rollback acknowledged                       | `Failed`, primary fault and cleanup retained                                              | This transaction rolled back; new attempt still needs authority/intent |
| Lock failure before begin                              | Stage-aware failure, cleanup absent only if established                                   | Generic 503 supplies no not-started guarantee                          |
| Same role, commit acknowledged                         | Success, logical no-op                                                                    | Completion, not lasting state or zero SQL activity                     |
| Commit error or lost commit result                     | Failure if finalized; effects may remain unknown                                          | No universal rollback inference                                        |
| Commit succeeds, middleware fails                      | Action success plus request failure                                                       | A valid public failure does not prove action failure                   |
| Commit succeeds, response lost                         | Client unknown                                                                            | Readback cannot establish causality; no blind resend                   |
| Lost self-demotion response, later forbidden           | Original unknown; later rejection                                                         | Keep both attempts separate                                            |
| Another owner restores role before resend              | New invocation may mutate again                                                           | Setter syntax is not replay safety                                     |
| CSRF refusal before dispatch                           | No named action invocation for this request                                               | Distinguish adapter refusal from domain forbidden                      |
| HTML, unknown code/version, malformed or empty success | Client protocol unknown                                                                   | Never infer success from 2xx or parse message prose                    |
| Task disappears during cleanup                         | Possibly no returned action result                                                        | Missing evidence proves neither rollback nor commit                    |

Before implementation, choose wire format/migration and driver-specific failure
mapping; verify cleanup and connection disposal; test public-versus-privileged
disclosure with secret canaries; test the whole-request decoder and literal-code
schemas. Keep expected scenarios independent of descriptor-generated fixtures.
No receipt store or generic executor is necessary to specify these boundaries.

## References and design provenance

The
[original discussion](https://ampcode.com/threads/T-01a0d13e-b20f-73ee-bed5-747eb2d3346c)
contains owner preferences and librarian/oracle consultations. This spec is
self-contained; agents should not need the entire conversation to follow it.

- [Research map](research.md): broader Rails, Ash, Rust and tooling references.
- [Ash update actions](https://github.com/ash-project/ash/blob/main/documentation/topics/actions/update-actions.md):
  declarative actions, atomic updates and transaction lifecycle. Do not equate
  pre-transaction policy evaluation with serialized invariant enforcement.
- [Loco reference application](https://github.com/loco-rs/loco/tree/main/examples/reference_spa/src):
  explicit Rust controllers/model methods and transaction ownership.
- [FastAPI response model example](https://github.com/fastapi/fastapi/blob/master/docs_src/response_model/tutorial003_py310.py):
  typed transport contracts, not automatic business concurrency guarantees.
- [tracing async instrumentation](https://github.com/tokio-rs/tracing/blob/master/tracing/src/instrument.rs)
  and
  [span guards](https://github.com/tokio-rs/tracing/blob/master/tracing/src/span.rs):
  per-poll instrumentation and why enter guards must not cross awaits.
- [tracing attributes](https://github.com/tokio-rs/tracing/blob/master/tracing-attributes/src/lib.rs):
  automatic argument capture and `skip_all` limitations.
- [tracing-opentelemetry span extensions](https://github.com/tokio-rs/tracing-opentelemetry/blob/main/src/span_ext.rs):
  distributed parents and cross-trace links.
- [OpenTelemetry Rust propagation](https://github.com/open-telemetry/opentelemetry-rust/tree/main/opentelemetry-sdk/src/propagation),
  [span processing](https://github.com/open-telemetry/opentelemetry-rust/blob/main/opentelemetry-sdk/src/trace/span_processor.rs),
  and
  [metrics](https://github.com/open-telemetry/opentelemetry-rust/blob/main/docs/metrics.md):
  context extraction, sampling/export limitations and metric cardinality.
- Current Iris
  [delivery worker](../experiments/api-slice/server/src/delivery.rs) and
  [outbox operations](../experiments/embedded-db/sqlite/src/outbox.rs):
  inspected for S13; proposed evidence records do not yet exist in these
  modules.

External references explain influences, not dependencies or permanent API
contracts; upstream branches may change. Recheck them before copying an API.

## Change record

- **2026-09-25:** Created current synthesis from the design discussion. Captured
  authoring alternatives, domain boundaries, AI-oriented result/evidence/receipt
  separation, three failure scenarios and rationale. Marked productivity study
  deferred and kept illustrative APIs separate from implemented experiments.
- **2026-09-25, static-contract design pass:** Added S12 with an explicit Rust
  authoring example, narrow derive alternative, author/generated
  responsibilities, disclosure boundaries and precise verification limits.
  Oracle critique informed the enumeration caveat and shared-consumer boundary.
  APIs and integration remain proposed; no runtime changes, macro implementation
  or productivity study.
- **2026-09-25, execution/evidence design pass:** Added S13, its two worked
  flows, source references, alternatives and external-model review brief.
  Clarified deferred-command versus committed-effect authority in S05. Captured
  actual worker observation gaps without changing code. No dependencies,
  instrumentation, collectors, durable receipts or productivity experiments were
  added.
- **2026-09-25, first review synthesis:** Added S14's caller-loss
  recommendation, lifecycle/effect table, recovery counterexamples and future
  validation gates. Recorded selective review dispositions separately, retained
  the original report, preferred ecosystem enum enumeration, and clarified the
  runtime client boundary. Updated membership delivery status after pulling its
  published implementation. No runtime changes or new verification claims.
- **2026-09-25, result/recovery reference:** Added proposed S15 after oracle
  consultation. Preserved typed primary rejection across cleanup failure,
  distinguished finalized action results from public request failures and client
  uncertainty, and specified recovery discovery limits. Recorded the post-action
  middleware failure case against current source. Examples and validation
  scenarios are design-only; no runtime or wire migration performed.
