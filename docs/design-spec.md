# Iris living design specification

Last updated: September 27, 2026.

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
S16 compares two authoring paths with one annotated membership vertical slice.
S17 proposes the next bounded step: a reference application that generalizes S16
across operations and adds the first reads. S18 sets that application's
development lifecycle: storage, seeds, journal mode, task supervision and one
development command. S10 distinguishes implementation from proposals. Use the
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

| Capability                                                              | Status and evidence                                                                                                                                                                                       |
| ----------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| SQLite/Turso comparison and feedback measurements                       | Implemented experiment; [findings](embedded-db-findings.md)                                                                                                                                               |
| Axum APIs, two OpenAPI exporters, generated TS and React                | Implemented experiment; [API guide](../experiments/api-slice/README.md)                                                                                                                                   |
| Local OIDC/session authentication                                       | Implemented protocol experiment, not real-provider identity assurance; [auth guide](../experiments/api-slice/authentication.md)                                                                           |
| Atomic invitation/outbox and local mail recovery                        | Implemented experiment; [delivery guide](../experiments/api-slice/delivery.md)                                                                                                                            |
| CLI/MCP verification interface                                          | Implemented pilot; [guide](../experiments/agent-interface/README.md)                                                                                                                                      |
| Membership role/removal workflow                                        | Implemented experiment, now on main; [API guide](../experiments/api-slice/README.md); verification reported below                                                                                         |
| Domain layout and redesigned result model                               | Proposed; no framework API released                                                                                                                                                                       |
| Rejection metadata, precise per-code schemas and shared contract export | Isolated S16 experiment implemented; [verification matrix](../experiments/api-slice/s16.md); no existing API migration                                                                                    |
| Reference application, multi-operation contracts and first reads        | S17 checkpoints A and B implemented, each with its client and browser workflow, and the mutations' declared current-state read; [guide](../apps/reference/README.md)                                      |
| Reference application lifecycle                                         | Decided in S18; storage, initialization, the journal-mode check, reset, the migration refusal, shutdown and the session cleanup task implemented; the development command authorized, not yet implemented |
| Execution context, causal correlation and bounded evidence collection   | Proposed in S13; no context API, trace persistence or collector implemented                                                                                                                               |
| Runtime evidence, durable receipts, idempotency, performance inspector  | Design ideas, not implemented                                                                                                                                                                             |
| Controlled AI repair/productivity comparison                            | Deferred by owner                                                                                                                                                                                         |

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
- Read conventions: visibility predicates, pagination, GET contracts and field
  disclosure. S17 proposes a first slice.
- Actor construction, job identity and explicit system authority.
- Database strategy and transaction composition when a concrete need appears.
- Receipt persistence, replay authorization, retention and idempotency
  semantics.
- Runtime evidence collection, correlation, bounded retention and disclosure.
- Configuration, startup, migrations, jobs and service lifecycle conventions.
  S18 decides a first slice for the reference application; storage, reset, the
  migration refusal, shutdown and session cleanup are implemented, the
  development command authorized and pending.
- Packaging, generators and compile-loop improvements justified by experience.

Do not interpret this agenda as approval to implement all items. Generic action
traits, resource DSLs, broad macros, policy engines, universal repositories,
automatic retries and production integrations remain deferred or undecided.

## S12 — Static contract declarations for action authors

**Proposed reference design; no implementation authorized by this section.** The
separately authorized S16 experiment now tests a bounded subset with ecosystem
enumeration and a runtime/export bridge; see its dated evidence below. Recommend
explicit Rust types and exhaustive mappings first. Keep a narrowly scoped
metadata derive as an alternative, not a selected dependency or API. The goal is
reliable machine-readable contracts with useful omission diagnostics, not the
fewest lines of application code.

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

These were proposed checks, not productivity studies. The October 2, 2026
observable-boundary evidence below partially addresses items 2 and 3; it does
not establish in-commit behavior or a new ownership policy.

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

### Observable caller-loss boundaries — October 2, 2026

ROB-1110 adds four authenticated reference `change_role` tests, with only
`cfg(test)` instrumentation. On Axum 0.8.9, Hyper 1.11.1, Tokio 1.53.1 and SQLx
0.9.0 on this Linux orb:

- Aborting the owned `app().oneshot` future after domain validation but before
  the write gives a cancelled join. The hold remains armed; tracked connection
  closure is acknowledged, and a fresh independent connection reads `editor`.
- A complete raw HTTP/1.1 request held after its write but before commit has
  exactly one outstanding tracked connection; an independent read sees `editor`.
  Closing both socket directions and dropping the sole caller handle produces an
  explicit outer request-service `Dropped` observation while the hold remains
  armed. Closure is then acknowledged and the role stays `editor`.
- Holding a successful inner response before exposure to Hyper allows an
  independent read of `viewer`, but no response byte reaches the caller during
  the bounded hold check. Full socket loss produces `Dropped` while this hold
  remains armed; acknowledged closure and `viewer` are checked independently.
- The loss-free control uses identical framing and both holds, releases them,
  validates the complete success envelope, observes `Returned`, and checks
  acknowledged closure and `viewer`.

Every final-state check is followed by an independent SQL transaction writing
the distinct third role `owner`, committing, and reading it back. Canonical
database-path and phase-scoped gate registrations are removed by RAII. Holds
release on failure; server cleanup signals graceful shutdown to accepted
connections as well as the accept loop, and normal termination is awaited.
Socket, SQL, phase and join waits are bounded. The existing guarded issuer
fixture and lifecycle before-commit gate behavior are retained.

The four behavioral tests passed before any production change; none was needed.
Four separately seeded runtime mutants were caught with green isolated controls
before and after restoration: premature commit, skipped update, rollback in
place of commit while still reporting success, and suppressed tracked-close
acknowledgment. See the
[dated decision](decisions.md#observable-membership-caller-loss--october-2-2026)
for commands and counts.

These observations cover only the stated `change_role` boundaries and stack.
They do not distinguish FIN from RST, interrupt a commit, establish continuation
after caller loss, cover removal, or provide a receipt. Without a validated
terminal response, the caller's outcome remains unknown; a current-state read
does not identify this attempt. In-commit cancellation remains untested here.

## S15 — Reference result and recovery contract for agents

**Proposed general contract; partially tested, not a wire/API migration.** This
reference is now exercised in part by the isolated S16 experiment; the general
contract and sketches below remain proposals, not changes to the existing API.
It applies S06–S14 to membership role changes. Oracle critique informed
finalized rejection, cleanup preservation and the distinction between action
failure and request-response failure. Types and JSON below are design sketches;
the current API still returns `MemberChange` or `{code,message}`. No receipt,
inspector, executor, automatic retry or new MCP mutation capability is
introduced.

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
_Status:_ S17's reference application now exposes such a read and declares
`listProjectMembers` as the current-state read of both its mutations
([evidence](#current-state-read-evidence)).

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

## S16 — Authoring one action and publishing its HTTP contract

**Bounded alternative-A experiment implemented; no approved wire migration.**
The September 26 [execution record](../experiments/api-slice/s16.md) reports
real SQLx/Axum, export, client and omission checks. The authoring sketches and
alternative B below retain their design provenance; they are not released APIs.
Keep ordinary transaction-owning functions; try a shared response mapping at the
existing utoipa registration boundary before creating an Iris endpoint API. This
narrows S03's preferred typed-registration direction: share semantic data first,
add a typed registration wrapper only if the experiment demonstrates a useful
check it cannot otherwise provide. S12 owns rejection metadata and S15 owns
result/recovery semantics; this section connects them, not a third model.

Source inspection used the
[S15 baseline](https://github.com/robertguss/Iris/commit/d960fce9f84388da560864ffa978f391ccf65208).
The [original review synthesis](reviews/opus55-all-01-synthesis.md) remains
unchanged; this is a follow-up design, not another independent review or
consensus. The
[decision record](decisions.md#action-authoring-vertical-slice--september-25-2026)
records alternatives and outstanding choices.

### One author-owned action, regardless of registration choice

Proposed home: `domains/memberships.rs`, with HTTP in `http/memberships.rs`
following S04. The current files are instead
[SQLite members](../experiments/embedded-db/sqlite/src/members.rs) and
[HTTP members](../experiments/api-slice/server/src/members.rs). Current
`ChangeMember` includes `actor_id` and optional role/removal, returns
`MemberOutcome`, commits business rejections, and can replace a primary error
with a rollback error. The following is a **pseudocode migration sketch**, not
compilable Rust or a claim that those issues have been changed. The SQL ordering
comes from current code; stage classification/cleanup handling is proposed.

```rust
// Application-owned types. Actor comes from trusted identity, never the body.
struct ChangeRole { project_id: i64, user_id: i64, role: MemberRole }
struct RoleChangeAcknowledged;
enum ChangeRoleRejection { Forbidden, MemberNotFound, LastOwner }

async fn change_role(
    conn: &mut SqliteConnection, // No active transaction on entry.
    actor: &Actor,
    cmd: ChangeRole,
) -> ActionResult<RoleChangeAcknowledged, ChangeRoleRejection> {
    // Explicitly handle begin errors at this boundary; classify only facts known.
    let mut tx = match conn.begin_with("BEGIN IMMEDIATE").await {
        Ok(tx) => tx,
        Err(e) => return begin_failure(e), // Proposed S15 classification, not API.
    };
    let body = async {
        // All helpers here mean ordinary application SQL on this same tx.
        if !is_current_owner(&mut tx, cmd.project_id, actor.user_id()).await? {
            return Err(StopReason::Rejected(Forbidden));
        }
        let target = load_member(&mut tx, cmd.project_id, cmd.user_id).await?
            .ok_or(StopReason::Rejected(MemberNotFound))?;
        if target.role == Owner && cmd.role != Owner
            && owner_count(&mut tx, cmd.project_id).await? == 1 {
            return Err(StopReason::Rejected(LastOwner));
        }
        update_role(&mut tx, cmd.project_id, cmd.user_id, cmd.role).await?;
        Ok(RoleChangeAcknowledged)
    }.await; // Body SQL errors become StopReason::Execution at stage=body.
    match body {
        Ok(ack) => match tx.commit().await {
            Ok(()) => Ok(ack),
            Err(e) => commit_failure(e), // No universal rollback conclusion.
        },
        Err(primary) => match tx.rollback().await {
            Ok(()) => finalize_stop(primary, Cleanup::RollbackAcknowledged),
            Err(e) => failed_with_cleanup(primary, e), // Preserve BOTH causes.
        },
    }
}
```

The sketch's classification/finalization names abbreviate S15 cases; they do not
propose a generic transaction runner. Acknowledged rollback finalizes a business
rejection, but an execution fault remains failure. Failed rollback always yields
`Failed`, even with `primary=Rejected(LastOwner)`. Driver-specific commit-error
cleanup/disposal remains unresolved; a consumed transaction is not proof of
`NotRequired`. Dropped futures may return nothing. No `?` bypasses finalization
outside the body. No HTTP registration can verify these properties.

Owner checks precede target lookup to avoid unauthorized existence disclosure.
SQLite's writer serialization covers checks and mutation; other engines and
other mutation paths require their own proof/tests. Same-role success
acknowledges commit, not a permanently current record. Actor construction
belongs to trusted identity code, not a deserializable command or a registration
default.

### HTTP adapter: author conversion, shared rendering

For both approaches the author supplies the ordinary handler below. All
`contract`, `reply`, and envelope names are **proposed pseudocode**. Axum's
`State`, `Json`, `JsonRejection` and `IntoResponse` are existing APIs; they do
not implement the missing bridge.

```rust
async fn endpoint(State(state), actor: Actor, body: Result<Json<ChangeRoleRequest>, JsonRejection>) {
    let request = parse_or_refuse(body)?;
    // Wire string IDs -> canonical positive i64 with overflow check; role enum.
    // Unknown fields rejected; identity is not accepted in the request.
    let command = request.try_into_command()?;
    let mut conn = connect_or_public_failure(&state.database).await?;
    let result = change_role(&mut conn, &actor, command).await;
    CHANGE_ROLE.reply(result) // Finalized rejection only; failures never unwrap it.
}
```

An actual signature must carry trusted request identity into refusal/failure
rendering as well. Connection failure is a request failure before action
dispatch, not a fabricated action result. Iris supplies generic envelope
rendering, literal-code schema construction, shared refusal/failure descriptors,
and bounded diagnostic conventions. The author owns safe projections and what
statuses mean. Public rendering never serializes `ExecutionFailure<R>` or its
private rejection; diagnostic projection is separately authorized and
allowlisted under S13/S15. No inspector is implemented by declaring this
contract.

Declare each semantic fact once, rather than requiring one giant macro:

| Declaration / owner          | Single source and consumers in the proposed design                                                                                                                                                          |
| ---------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Operation / application      | One pair: domain `memberships.change_role`, existing OpenAPI `changeMemberRole`; renderer, export extension and discovery consume it. Preserve the existing operation ID for client continuity.             |
| Rejections / domain          | S12's enum and exhaustive descriptor match: stable codes, safe summaries, rule and recovery constraints. Unit enumeration uses the candidate `strum::VariantArray` derive, not a handwritten list.          |
| Public projection / HTTP     | One exhaustive match yields status plus allowlisted descriptor view; runtime renderer and exporter consume it. For this unit enum there are no rejection payloads. Private rules need not enter OpenAPI.    |
| Success / HTTP               | One projection maps acknowledgment to `{completion:"acknowledged"}`, with one 200 response declaration and schema. Do not echo requested IDs/role as stored state.                                          |
| Shared HTTP responses / Iris | One versioned envelope/profile describes invalid request, authentication, CSRF and safe 500/503. Registration selects it; middleware and extractors must actually render it.                                |
| Recovery / application       | Operation declares inspection, read and keyed replay unsupported for this slice; new submission requires current authority and intent. Last-owner prerequisite comes from its descriptor. No retry program. |

The operation pair would be exported as OpenAPI `operationId=changeMemberRole`
and an Iris extension containing the domain name, wire version and public
recovery constraints. Extension syntax is not selected. Standard TypeScript
generation must not be assumed to interpret it: the client boundary needs a
small generated manifest from the same declaration or an explicitly tested
extension consumer. Privileged metadata never goes in that public manifest.

### A: ordinary functions with explicit registration

Keep the existing `#[utoipa::path]` and
`OpenApiRouter::routes(utoipa_axum::routes!(endpoint))` pattern. Let the
attribute own method/path/request schema; remove independently authored response
tables only when a response bridge replaces them. A domain-local `routes()`
assembles the route and applies its response mapping to the collected operation
before returning the router/document. The bridge must fail export if the target
operation is absent, ambiguous or has the wrong identity; never silently skip
it. The root registers that module once. No function trait, action object or
executor.

Keep the operation-name pair in the response declaration, not duplicated in an
`operation_id` attribute. For this one-route module, the bridge selects its sole
collected POST operation, assigns the public ID from that pair (replacing any
inferred handler name), and attaches the domain-name extension. Assert the
expected path/method independently in contract tests. A larger module needs an
explicit association with its collected route metadata; do not introduce a
second handwritten path catalog to call it single-source.

The bridge did not exist at the design-pass baseline; the bounded implementation
is recorded below. Its narrow job is to enumerate mapped rejections, add success
and the selected middleware profile, register referenced schemas, group branches
per status, and install responses on the existing OpenAPI operation. Runtime
uses those same mappings. Request DTO schema derives remain ecosystem-owned.
Route identity linkage remains a contract check rather than a compiler
guarantee. Export/assembly must run even if the server starts without serving
interactive API docs.

### B: a small typed registration value on existing crates

Alternative **invented API**, not valid utoipa/axum syntax:

```rust
let role_http: HttpOperation<ChangeRoleRequest, RoleChangeAcknowledged, ChangeRoleRejection> =
    HttpOperation::post("/api/memberships/role")
        .identity("memberships.change_role", "changeMemberRole")
        .success(200, acknowledge_publicly)
        .rejections(rejection_contract) // S12 exhaustive match, not another code list.
        .boundary(browser_mutation_profile())
        .recovery(no_inspect_read_or_replay());
router.register(role_http, endpoint);
```

Here the value owns route identity as well as response data. A future handler
return type such as `ContractReply<RoleHttp>` could connect the handler's output
to this registration at compile time. Merely accepting an Axum `Handler` and a
descriptor side by side does **not** establish that connection. Designing those
type bounds, early-return handling and state/extractor compatibility is real
Iris work, not something supplied by the sketch's generic parameters. Raw
responses and middleware can still bypass it. Execution stays in the ordinary
handler/action; registration must never start a transaction or supply authority.

B can remove the path/identity lookup and reject mismatched reply types, but
requires a maintained registration API and escape-hatch policy. A supplies a
smaller first experiment with weaker assembly guarantees. Both still need the
same response/schema bridge, disclosure review and behavioral tests. Neither
discovers unregistered domain actions or exposes MCP mutations.

### Full request contract, not just the handler result

Use S15's candidate envelope for the experiment only; no existing endpoint is
migrated by this document. Every JSON branch requires version, operation, local
request ID and its literal tag; rejected/refused/failure branches require their
literal code and safe message. Success requires its data schema.

| Producer / condition                                         | Proposed status, kind and code                                                                                                    | Scope                                                                                         |
| ------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------- |
| JSON/media/body-limit rejection, invalid ID/role/extra field | 400 `refused`, `http.invalid_request`                                                                                             | Normalize caught extractor failures, including oversize bodies, to this profile; no dispatch. |
| Missing trusted actor                                        | 401 `refused`, `http.unauthenticated`                                                                                             | No dispatch; session middleware may refuse CSRF first.                                        |
| Origin/CSRF boundary                                         | 403 `refused`, `http.csrf_refused`                                                                                                | No dispatch; do not call this domain forbidden.                                               |
| Finalized domain rejection                                   | 403 `rejected`, `memberships.forbidden`; 404 `rejected`, `memberships.member_not_found`; 409 `rejected`, `memberships.last_owner` | Only after required cleanup.                                                                  |
| Acknowledged commit                                          | 200 `success`                                                                                                                     | `{completion:"acknowledged"}`; no action permanence claim.                                    |
| Connection/action/session/response failure                   | 500 `failure`, `iris.internal`; 503 `failure`, `iris.unavailable` where classified                                                | May occur before dispatch or after commit; no retry/effect inference.                         |
| Unmatched route/method, external gateway, connection loss    | Separate transport/routing contract; otherwise client unknown                                                                     | Do not invent this operation's envelope, code or server request ID.                           |

403 has **one response object with a `oneOf` of two literal tag/code branches**,
not last-write-wins registration. Codes permitted at 409 do not include every
global error. Reject mismatched status/tag/code tuples. Current auth can fail
after `next.run`; its normalization only handles some bodyless server errors.
The proposed profile must include actual session-layer failures and outer
normalization, not infer them from handler types. Bind operation identity at a
route-aware boundary before fallible operation middleware; if unavailable, use
the separate unclassified path. Do not label arbitrary 4xx as pre-dispatch.
Timeout/rate-limit layers are not added here; adding one later requires an
explicit profile update, effect policy and tests.

For the candidate version 1, allow unknown additive object fields but require
all declared fields and literal values. Unknown version/code, malformed 200,
empty/HTML body or thrown fetch/body-read error produces `ClientUnknown`,
outside the known server union. This additive-field policy is a recommendation
to test, not current client behavior. Wrap request execution and validation, not
only `openapi-fetch`'s resolved value. Generate TS with the existing
`openapi-typescript` pipeline; choose/test a runtime OpenAPI 3.1-compatible
schema validator separately. Types and schema examples do not perform
validation.

### Maintenance paths and honest feedback

Paths below use the proposed S04 layout; current counterparts are the two
`members.rs` files and `server/src/lib.rs`. Generated files are outputs, not
additional places to hand-edit semantic facts.

| Change                                                                                         | A: explicit registration + response bridge                                                                                                                                                                               | B: typed registration value                                                                                                                                  |
| ---------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Add a unit `ProjectArchived` rejection (hypothetical policy, not proposed membership behavior) | `domains/memberships.rs`: variant, body check, descriptor code/summary/recovery. `http/memberships.rs`: exhaustive status/public projection arm. Enumeration adds export branch automatically. No route edit.            | Same domain and projection edits; no registration edit because it refers to the rejection type. Typed registration does not remove these semantic decisions. |
| Change LastOwner 409 to 422                                                                    | One HTTP mapping edit, then regenerate OpenAPI/TS/manifest; independently update reviewed compatibility expectations and client handling.                                                                                | Same. A status change is not caught by Rust's type checker.                                                                                                  |
| Replace acknowledgment with required stored `role`                                             | Domain success/body must obtain stored state if that is the promise; HTTP projection/DTO/schema change; A's success declaration must reference the new type if renamed. Regenerate, then compile/check affected clients. | Same domain/wire work; tied success types can reject an outdated projector/registration, but cannot prove the value is stored state.                         |
| Add/refine a middleware response                                                               | Change shared profile AND actual producer; regenerate affected operations and test through real middleware.                                                                                                              | Same; handler reply typing cannot enforce middleware coverage.                                                                                               |

For comparison, **today** adding a rejection touches domain outcome/body,
`members::apply`, global `ErrorCode`/`ApiError`, endpoint attributes if statuses
change, and aide declarations in `lib.rs`, then both snapshots/generated
clients. Even without a new status, the global `Problem` schema permits
unrelated codes at that status. The proposed bridge removes that duplication,
not business work. Do not keep two exporters as a permanent author obligation;
leave the current comparison intact until a separately authorized migration.

| Feedback layer          | What a later experiment must establish                                                                                                                                                                   | Remaining gap                                                                                                          |
| ----------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| Rust compiler           | Exhaustive descriptor/projection matches; command/result types; derive enumerates unit variants. For B only, demonstrate a mismatched handler reply compile failure.                                     | Strings can collide; an action can return the wrong rejection or lie about cleanup. A's route link is not type-proven. |
| Contract tests          | Exact independent allowed tuples; unique operation identities; 403 union retains both branches; references resolve; emitted responses validate; generated TS narrows; unknowns reject.                   | A generated fixture using the same mapping can reproduce its mistake. Schema validity is not authorization.            |
| Behavioral tests        | Authority before disclosure; two-owner concurrent self-demotions leave an owner; same-role success; primary plus rollback failure; middleware failure after commit; response loss never triggers resend. | Driver/task-loss coverage and other mutation paths remain engine-specific; no durable recovery.                        |
| Disclosure review/tests | Public/private projection separation with secret canaries; missing evidence stays unknown; no raw bodies/error chains in client diagnostics.                                                             | A type or allowlist alone does not establish all nested instrumentation is safe.                                       |

An agent encountering validated `memberships.last_owner` can explain the
necessary prerequisite and stop. After lost response it retains a distinct
unknown attempt, advertises no inspection/read/replay capability, and does not
promote another member or resend. These are design examples, not an executed
agent study or a claim of faster repairs.

### Verified seams and the smallest later experiment

Librarian source research for this pass verified:

- [utoipa-axum 0.3.0 `routes!`](https://docs.rs/utoipa-axum/0.3.0/utoipa_axum/macro.routes.html)
  consumes annotated handler paths, not arbitrary descriptor callbacks.
  [Mutable document access / splitting](https://github.com/juhaku/utoipa/blob/utoipa-6.0.0/utoipa-axum/src/router.rs#L366-L390)
  permits a response post-processor without replacing execution.
- [utoipa 6.0.0 responses](https://github.com/juhaku/utoipa/blob/utoipa-6.0.0/utoipa/src/openapi/response.rs#L20-L77)
  use a status-keyed map; insertion replaces a duplicate status, not a union.
- [aide 0.15.1 response inference](https://docs.rs/aide/0.15.1/src/aide/operation.rs.html)
  has input/output hooks, but
  [layers](https://docs.rs/aide/0.15.1/src/aide/axum/mod.rs.html) do not infer
  middleware responses. The current experiment explicitly disables response
  inference; that does not establish that aide is inferior.
- [strum 0.28.0 `VariantArray`](https://docs.rs/strum/0.28.0/strum/trait.VariantArray.html)
  supplies enumeration; its derive supports unit variants. Payload-bearing
  rejection projections remain a separate problem. No dependency was added.

**Recommend A plus the narrow response bridge as the next experiment**,
retaining utoipa as provisional primary. Do not build B until its stronger
linkage pays for its new author-facing types. This is a recommendation, not
implementation authorization. Smallest useful experiment after approval:

1. One isolated membership route using the existing SQLx/Axum stack, S15 result
   finalization and one chosen envelope. Keep the public demo unchanged.
   Implement only the descriptor-to-runtime/OpenAPI seam and unit enumeration;
   no registry, receipts, collector, executor or custom macro.
2. Register through real `routes!`, attach response data to the collected
   operation, and export using the existing exporter pattern. Validate actual
   in-process HTTP responses through real identity/CSRF/extractor middleware,
   not just calls to the renderer. Independently assert 403's two branches and
   that a 409 body with an unrelated code fails schema validation.
3. Run real Rust checks/tests, generate TypeScript from that document, compile
   known-code narrowing and negative fixtures, then execute a whole-request
   runtime decoder with real responses plus malformed 200, wrong operation,
   unsupported version/code, HTML, empty body and fetch rejection. Validator
   dialect/reference support is an acceptance condition, not an assumption.
4. Make a temporary extra rejection and a response change, then deliberately
   omit each required edit. Record which compiler/contract/client check fails
   and where, including the route-link omission. Use independently written
   business expectations and failure injection for cleanup/post-commit cases.
   Remove the probes. No productivity benchmark or claim that generated tests
   prove intent.

Pass means the small bridge works with real dependency versions and catches its
stated drift cases, not that Iris's future API is proven. If A leaves routine
route/reply mismatches invisible until runtime, try B's smallest typed linkage
against that same fixture. If the bridge cannot preserve literal-code schemas
and per-status unions, revisit the library seam before adding abstraction. Wire
migration, exact failure/disposal classification, and runtime validator
selection remain open. This documentation pass ran no application tests and
compiled none of the pseudocode.

### September 26 execution evidence and remaining choices

The preceding recommendation and proposed gates describe the September 25 design
pass. The later authorized experiment implements A in
`experiments/api-slice/server/src/s16`, separately assembled from both existing
demos. A small response bridge consumes exhaustive domain/HTTP mappings for
runtime rendering and real utoipa export. Strum 0.28.0 enumerates unit variants;
the generated 403 schema retains both literal branches. Ajv 8.20.0 validates the
export's OpenAPI-3.1/JSON-Schema-2020-12 response schemas at a whole-request
boundary; openapi-typescript supplies static narrowing, not validation.

**Executed:** ten focused Rust tests, 44 runtime client cases, and 16 detected
omission/drift probes with positive controls. Broader API/SQLite verification
passed 43 tests with the existing Mailpit test ignored. Existing exporter/client
snapshots, web build, Clippy and formatting passed. The
[matrix and diagnostic record](../experiments/api-slice/s16.md) owns exact
commands, expected facts and limits. Temporary policy/response edits were
removed. Librarian research confirmed crate seams; oracle review refined
acceptance criteria before implementation. Neither is an independent final code
review.

The evidence supports the smaller bridge: adding a unit rejection needs no
handwritten enumeration or route edit. It also confirms A's limitation: Rust
accepts wrong paths and changed statuses; independent contracts and regenerated
client checks must catch drift. Raw responses can bypass the bridge. Do not call
route/reply linkage compiler-enforced or introduce B solely to erase this
caveat.

The real post-commit session-save failure produced public 500 while readback
confirmed the committed role. Failed cleanup preserves primary cause privately
and renders failure, not finalized rejection. Cleanup-failure observations were
injected at finalization, not real driver rollback I/O failures. Real SQLite
begin/body/deferred-constraint commit errors were exercised; begin/commit
cleanup stays conservatively unconfirmed. No `NotRequired`, safe-reuse,
task-loss or cross-engine guarantee follows. Neither public failure nor client
unknown grants resend authority. Recovery discovery declares inspect/read/replay
unsupported.

Open: wire migration, larger-module route association, richer failure
vocabulary, driver disposal under other faults, bounded client body reads,
process/future loss and portability. No receipt, executor, typed registration
wrapper, custom macro, resource DSL, runtime inspector or productivity benchmark
was added.

## S17 — Reference application and first reads

**Proposed; checkpoints A and B, and the mutations' current-state read, are
implemented experiments.** The owner settled its seven open choices on September
26, 2026, then authorized the CI prerequisite and checkpoint A, and on September
27 checkpoint B. Checkpoint A's
[server-side](#checkpoint-a-server-side-evidence) and
[client](#checkpoint-a-client-evidence) evidence, and checkpoint B's
[server-side](#checkpoint-b-server-side-evidence) and
[client](#checkpoint-b-client-evidence) evidence, record what was built. The
owner then approved declaring `listProjectMembers` as the mutations'
current-state read, and its [evidence](#current-state-read-evidence) records
that step. This section was first written as a proposal authorizing no
implementation, dependency, CI or wire change. It was drafted after independent
assessments of `9235c6e` by Claude (Opus 5.5) and Astra (GPT-6-Astra through
Codex), then revised after Astra's [review](#review-of-this-proposal). The
[owner decisions](#owner-decisions) are recorded below; the assessments and the
review are not owner decisions.

### Why a reference application now

The four legacy operations share older conventions (`{code,message}` errors and
one global error enum), but the S15/S16 contract under active design has one
instance: S16's bridge is verified for one route, and its assembly asserts a
single collected path. All four domain operations are POST mutations. The only
GET routes are `/api/openapi.json`, `/api/auth/session` and
`/api/auth/callback`, so React cannot discover authorized domain state and
remains an ID-based console. S13–S16 deepened failure and recovery semantics for
one action; the larger remaining risk is breadth. A small reference application
tests whether the S16 path generalizes across operations and supplies the first
read conventions before any of it becomes framework API.

A local rerun at `9235c6e` (macOS, Node 24 rather than the pinned 26) passed the
default workspace tests (48), the dev-identity API tests (24, one Mailpit test
ignored), Clippy, rustfmt, web `verify`, `verify:s16` (10 Rust tests, 44 client
cases) and `probe:s16`. The
[decision record](decisions.md#reference-application-and-first-reads--september-26-2026)
lists the commands. The owner then authorized the prerequisite: the CI workflow
now runs `verify:s16` and `probe:s16`, whose commands passed locally under Node
26.8.1 ([record](decisions.md#s16-checks-in-ci--september-26-2026)). The
workflow runs on pull requests and pushes to `main`; its first observed run, on
draft pull request #1 at `a0c25ff`, passed every step
([record](decisions.md#reference-application-checkpoint-b-server-side--september-27-2026)).
Checkpoint B's commits then ran at `06ac967` (run 36329466284). The first
attempt failed only in the frozen agent-interface check, at its `members`
scenario reproduction (`runner.test.mjs:79`), and skipped the later steps; a
rerun of the same commit passed every step, including both browser workflows.
The failure is assessed as a flake; its cause was not diagnosed
([record](decisions.md#reference-application-current-state-read--september-27-2026)).
The current-state read's commits then ran at `12227aa` (run 36345360713) and
passed every step on the first attempt, including all three browser workflows
([record](decisions.md#documentation-hygiene--september-27-2026)).

### Ownership boundaries

| Location                                                                                                     | Owns                                                                                                                                                                                                                       | Must not contain                                                                     |
| ------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| `apps/reference/`, S04 layout: `app.rs`, `identity.rs`, `domains/`, `http/`, `migrations/`, `tests/`, `web/` | Commands, rejection types and descriptors, transactions and cleanup classification, SQL and visibility predicates, wire DTOs and projections, operation declarations, session/OIDC code, seeds and bootstrap, React client | Code presented as a framework API                                                    |
| `crates/iris/` (`publish = false`; name provisional)                                                         | Only behavior that two application operations demonstrably share. Likely candidates: envelope rendering, the per-operation response bridge, the shared refusal/failure profile, the request-ID boundary                    | Transactions, cleanup classification, authorization, SQL, domain types, session/OIDC |
| `experiments/`                                                                                               | Frozen historical evidence, kept green in CI until the reference application carries equivalent evidence; each retirement is recorded in the decision record                                                               | New features, or migration to the new envelope                                       |

**Extraction rule:** move code into `crates/iris` only when a second operation
needs the same behavior, and name both operations when doing so. Two operations
sharing SQLite mechanics does not establish a framework failure model, so S15
result and cleanup types stay application-owned in this slice. The crate is
private and provisional; presence there is not a stability promise, and
packaging and naming remain open (S11).

- **Registration:** utoipa is primary; aide remains only in the frozen
  comparison. The reference application's build is the first single-exporter
  baseline; measure it (D07) rather than infer it.
- **Identity:** session/OIDC is the only identity path, using the existing local
  issuer fixture for development and tests; the development identity header is
  not ported. The session module is copied as application code, similar to
  authentication generated into an application; whether Iris later provides it
  is open. Session, login, callback and logout keep their existing non-envelope
  contracts.
- **Schema and bootstrap:** fresh consolidated migrations own the application
  schema rather than replaying experimental history. The owner chose to add
  `users.display_name` and `projects.name`, so reads disclose something beyond
  already-known IDs; `user_contacts.email` stays delivery-only. As in the
  experiments, tests and the development binary create a disposable database,
  apply migrations and load seed fixtures at startup.
- **Deferred to a lifecycle pass:** persistent storage, seed policy, SQLite
  journal mode, worker supervision, and one development command for the issuer,
  API and Vite. The delivery worker is not ported.
  [S18](#s18--reference-application-lifecycle) now decides that pass; storage,
  reset, the migration refusal, shutdown and session cleanup are implemented,
  the development command authorized and pending.

### Operation sequence

Each checkpoint has its own acceptance checks and stop gate. If a checkpoint
contradicts an earlier assumption, stop and record it; do not silently widen an
abstraction to absorb it.

| Checkpoint | Operations (domain / OpenAPI ID)                                                             | Routes                                                        | What it tests                                                                                                                                                  |
| ---------- | -------------------------------------------------------------------------------------------- | ------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A          | `memberships.change_role` / `changeMemberRole`; `memberships.remove_member` / `removeMember` | `POST /api/memberships/role`; `POST /api/memberships/remove`  | S16 ported unchanged into the S04 layout, then a control operation with an identical rejection set; multi-operation association; the first extraction decision |
| B          | `projects.list_mine` / `listMyProjects`; `memberships.list` / `listProjectMembers`           | `GET /api/projects`; `GET /api/projects/{project_id}/members` | Filter-style and precondition-style read authorization, pagination, GET contracts, a React member directory                                                    |
| Follow-up  | `invitations.issue` / `issueInvitation`; `invitations.accept` / `acceptInvitation`           | `POST /api/invitations`; `POST /api/invitations/accept`       | 201 success, several literal branches per status, a possibly credential-bearing body, outbox enqueue, existence-hiding 404; completes the React workflow       |

Checkpoints A and B can be authorized separately. Invitation policies and
checks, including whether issuance returns its token, get their own design after
A and B supply evidence. Reads come before invitations because reads are the
largest undesigned area, and member management over seeded fixtures is a
coherent application without invitations. Mutations keep their command-style
paths and S16's operation IDs; reads use resource paths. Resource-verb routing
for mutations is a separate, deferred convention question. Listing pending
invitations waits for explicit visibility rules and must never expose tokens.

**October 2 design update:** [S19](#s19--reference-invitations-and-delivery)
settles that follow-up as accepted future design, not implementation. Issuance
returns no credential; invitation listing remains excluded. The earlier
checkpoint reasoning above is preserved as provenance.

### Multi-operation assembly

Generalize S16 alternative A without a second handwritten path catalog:

1. Collect each operation alone in its own `OpenApiRouter` with one `routes!`
   call. The bridge mutates that single collected operation through
   `get_openapi_mut`, preserving S16's exactly-one-operation assertion per
   declaration; `OpenApiRouter::merge` then adds it to the application router.
   Both methods exist in the pinned utoipa-axum 0.3.0. _Implemented as:_ one
   checked assembly merges each bridged document into the application's and
   keeps each operation's router separate, so the operation's boundary can wrap
   its own session layer and the export needs no identity provider (see
   [evidence](#checkpoint-a-server-side-evidence)).
2. Check components before each merge. utoipa 6.0.0's `OpenApi::merge` silently
   keeps the first same-named component, so a differing definition under an
   existing name must fail assembly, while identical shared definitions pass.
   Distinct DTOs need distinct names; S16's generic `SuccessData` becomes
   operation-specific.
3. Each declaration supplies the operation-name pair, expected method, success
   status and projection, exhaustive rejection mapping, applicable shared
   profile and recovery capabilities. CSRF refusal applies only to unsafe
   methods, matching the session layer, which skips GET, HEAD and OPTIONS.
4. After merging, catalog checks fail assembly on duplicate OpenAPI IDs or
   domain names, a surviving inferred handler ID, or one public code with
   differing descriptor metadata. Independent contract tests still assert every
   path and method.
5. The request-ID and envelope boundary applies per operation for its declared
   method. Axum 0.8.9 dispatches HEAD to GET handlers, so a GET operation's
   boundary also establishes request context for HEAD. The owner chose to serve
   HEAD as its GET operation: same status and headers, no body, covered by tests
   rather than a separate OpenAPI declaration. Other unmatched routes and
   methods stay outside operation contracts, as in S16. This slice avoids two
   operations on one path. _Status:_ implemented: POST method gating, and for
   both reads a GET boundary that also establishes request context for HEAD,
   with tests ([evidence](#checkpoint-b-server-side-evidence)).

`change_role` and `remove_member` share one rejection type because their
permitted sets coincide (S12; review finding O55-A-03). The owner chose this
over per-operation types; the shared type splits as soon as their permitted sets
diverge. Operations with different sets keep their own types, and a code used by
several types delegates to one descriptor definition. Codes are namespaced by
the operation's domain even when the fact concerns another entity, for example
`invitations.already_member`.

### Public response policy

Every domain operation adopts the S16-tested subset of S15: a version-1 envelope
with `schema_version`, `operation`, `request_id` and a literal `kind`; `code`
and `message` for non-success; `data` for success. Additive fields are
tolerated; unknown versions and codes remain `ClientUnknown`. The remainder of
S15 stays proposed, and the frozen `{code,message}` API is not migrated.

| Operation                  | Success                                   | Domain rejections                                                                             |
| -------------------------- | ----------------------------------------- | --------------------------------------------------------------------------------------------- |
| change role, remove member | 200 `{completion:"acknowledged"}`         | 403 `memberships.forbidden`; 404 `memberships.member_not_found`; 409 `memberships.last_owner` |
| list my projects           | 200 page of `{project_id,name,role}`      | None; rows are filtered by actor                                                              |
| list project members       | 200 page of `{user_id,display_name,role}` | 403 `memberships.forbidden` for an unknown project or a non-member                            |

Every domain operation also declares the shared profile: 400
`http.invalid_request`, 401 `http.unauthenticated`, 500 `iris.internal` and 503
`iris.unavailable`, plus 403 `http.csrf_refused` for POST. Reads need 503 too:
SQLx 0.9 leaves SQLite's journal mode unset unless requested, and the
experiments use a 100 ms busy timeout, so readers and committing writers can
contend.

**Existence hiding** is specified per concealed pair, not as a blanket rule.
Member listing gives an unknown project and a project the caller does not belong
to the same 403 response apart from `request_id`; the mutations keep their
existing concealment of unknown projects from non-owners. Authorized callers may
still learn absence: an owner receives `memberships.member_not_found`. The owner
chose this uniform 403, matching the mutations, over a uniform 404 for reads.

After checkpoint B, `change_role` and `remove_member` may declare
`listProjectMembers` as their current-state read (S15). A page describes present
state only: absence from a page or a traversal does not prove removal, and a 403
after a lost self-removal response does not resolve that earlier invocation
(S14). Read operations declare no recovery capabilities; repeating a read is a
new observation, not a replay. _Status:_ both reads omit `recovery` from
`x-iris`. On September 27 the owner approved the declaration, and both mutations
now make it: `recovery.read` names `listProjectMembers` and binds its
`project_id` path parameter to the request's `project_id` field. A caller sends
it under their current authorization as a fresh first page, with no cursor. A
returned page, a member's absence or a 403 resolves nothing about the attempt:
the read is no receipt and no concurrency fence, and it authorizes no new
submission. In the member directory, the readback and manual reloads provide new
state observations and never alter an attempt's outcome
([evidence](#current-state-read-evidence)).

### Read conventions

Reads are ordinary application functions. They have no mutation effects to
report, but they still own cleanup:

```rust
// Proposed shape; not an Iris API.
pub async fn list_members(
    conn: &mut SqliteConnection,
    actor: &Actor,
    query: ListMembers, // project_id, limit, position after a user_id
) -> Result<Page<MemberSummary>, ReadError<ListMembersRejection>>;
```

- One deferred read transaction covers the visibility check and the page query,
  so both observe the same database state under SQLite's locking; other engines
  need their own proof. Finalize it explicitly. In SQLite's default
  rollback-journal mode an unfinished reader delays committing writers, and
  SQLx's rollback on drop is queued, not acknowledged.
- Visibility is explicit, application-owned SQL. `list_mine` filters on the
  actor's memberships. `list_members` first establishes that the actor is a
  member of the project, in any role, then selects. Authorization is
  re-evaluated for every page. There is no policy engine or policy-to-SQL
  translation.
- Pages use keyset pagination on a unique ascending key (`user_id` or
  `project_id`). `limit` is 1–100 with a default of 50. `cursor` is an opaque,
  versioned, length-bounded string that clients echo verbatim. It carries
  position only: authority comes from the actor, so a forged or foreign cursor
  can reposition within visible rows but never widen them. No signing is needed
  for this slice.
- A page is not a snapshot of the collection. On an unchanged dataset, forward
  traversal returns each visible row once in key order. With concurrent changes,
  rows can be missed; the unique key prevents duplicates. `next_cursor: null`
  means no further rows when that page was read.
- Unknown and duplicate query parameters are rejected as `http.invalid_request`,
  matching bodies. A scratch probe of the serde_urlencoded 0.7.1 path behind
  axum 0.8.9's `Query` confirmed both under `deny_unknown_fields`; the handler
  still checks ranges such as `limit=0`, and assembled routes need their own
  contract tests.
- Candidate runtime enforcement: run reads on a dedicated connection with
  SQLite's `PRAGMA query_only` enabled and drop it afterwards, so an accidental
  data change fails and the setting cannot leak into mutations. This guards
  mistakes; it is not a security boundary or compile-time guarantee.
- ETags, conditional requests, sort and filter options, total counts and a
  generic pagination helper are deferred.

`ReadError` reuses S15's rejection, failure and cleanup vocabulary without a
mutation-effect assessment, which does not imply cleanup certainty. A connection
that returned an error is dropped rather than reused, as in S16.

### React client

A new client in `apps/reference/web` covers session bootstrap, my projects,
members, and role change or removal. Domain operations go through one
whole-request boundary generalized from S16: it owns request execution and body
reading, including thrown request and body-read errors, and validates responses
against the exported document. The session endpoints keep their existing
contracts. Query parameters are validated by the server adapter, not the client
boundary. Body reads are bounded, so an oversize body becomes `ClientUnknown`.
Measure the bundle and startup cost of runtime validation. Generated TypeScript
remains static assistance, not validation. _Status:_ implemented for all four
operations: checkpoint A's two ([evidence](#checkpoint-a-client-evidence)), and
both reads with my projects, members and the member directory
([evidence](#checkpoint-b-client-evidence)). After an unconfirmed attempt, the
console offers the mutations' declared current-state read
([evidence](#current-state-read-evidence)).

### Acceptance checks

Expectations are written independently of descriptor-generated fixtures (S12).

| Checkpoint             | Independent checks                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Prerequisite           | CI runs `verify:s16` and `probe:s16`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| A                      | Ported S16 suites pass with unchanged expectations; exact status/kind/code tuples for both operations; shared-code consistency; frozen experiments stay green                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| A probes               | An omitted bridge on the second operation, a duplicate OpenAPI ID, conflicting metadata for a shared code, a wrong path, and a same-named but different component schema each fail; an identical shared schema passes                                                                                                                                                                                                                                                                                                                                                                                               |
| B authorization        | Unknown and non-member projects give the same 403 response apart from `request_id`, while permitted disclosures stay distinct; a member removed between pages receives 403 on the next page; `list_mine` returns only the actor's memberships; email canaries never appear; GET needs no CSRF token while POST still does                                                                                                                                                                                                                                                                                           |
| B pagination and input | On an unchanged dataset, forward traversal returns every member once in key order; foreign cursors stay within the caller's visible rows; limit 0, 101 or non-integer, a malformed cursor, and unknown or duplicate parameters each return 400; authenticated and unauthenticated HEAD requests behave deliberately, with no missing-context 500                                                                                                                                                                                                                                                                    |
| B cleanup              | Rejection, query failure and cancellation each finalize or drop the read transaction, after which a writer can commit; `query_only` fails an accidental write and never reaches a mutation connection                                                                                                                                                                                                                                                                                                                                                                                                               |
| React                  | Typed narrowing and runtime decoder cases for every domain operation, including oversize bodies; session bootstrap unchanged; one browser workflow per checkpoint against the local issuer                                                                                                                                                                                                                                                                                                                                                                                                                          |
| Probes                 | Extend the omitted-edit probes with a changed GET parameter bound and an omitted visibility predicate that an independent test, not the compiler, catches                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| Current-state read     | Assembly fails on each unsupported declaration: the descriptor's shape; an unresolved or non-Iris target; a non-GET target or one with recovery; a missing or extra binding; a required parameter outside the path; a body field that is undeclared, optional or of a different schema. A hand-written recovery contract for both mutations; the client refuses any recovery shape it does not support; exact unconfirmed wording. Committed mutations whose responses are withheld, read back showing a matching role, an absent member and a 403 after self-removal, each resolving nothing and resending nothing |

### Explicit exclusions

No generic Action trait or executor, typed registration wrapper (S16 alternative
B), custom derive, resource or query DSL, policy engine or automatic
policy-to-SQL translation, generic pagination framework, receipts or idempotency
keys, evidence collector or tracing, bounded server-owned execution, delivery
worker, lifecycle conventions, envelope migration of the session endpoints,
PostgreSQL or Turso support, real OIDC provider, generators or CLI, migration of
the frozen experiments, or productivity claims.

### Owner decisions

On September 26, 2026, the owner settled each choice as recommended. The
alternatives stay recorded for their rationale.

| Choice                             | Decision                                                                           | Alternative not chosen                                                                |
| ---------------------------------- | ---------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| Existence hiding for reads         | Uniform 403, as the mutations already use                                          | Uniform 404 for reads                                                                 |
| Who can list members; which fields | Any project member, in any role; `user_id`, `display_name`, `role`; never email    | Owners only; IDs and roles only                                                       |
| HEAD on GET operations             | Serve it as its GET operation, without a body                                      | Reject HEAD explicitly                                                                |
| Rejection-type sharing             | Share where permitted sets coincide; split when they diverge                       | Per-operation types everywhere, with shared descriptor values                         |
| Unknown query parameters           | Reject, as bodies do; duplicates too                                               | Ignore                                                                                |
| Step order                         | Reads before invitations                                                           | Invitations before reads                                                              |
| Frozen experiments in CI           | Keep until the reference application covers their evidence; record each retirement | Path-filtered or scheduled runs; removal from CI once the reference application lands |

### Review of this proposal

Astra reviewed the first draft read-only on September 26, 2026, across all three
review tracks, against `9235c6e`, the uncommitted diff, application source and
pinned crates. It found no blockers, confirmed the API, CSRF, journal-mode,
GET-inventory and response-shape claims, and judged reads before invitations
justified. This revision incorporates all four major findings (silent schema
loss on merge, HEAD dispatch to GET handlers, read-transaction cleanup and
`query_only` leakage, over-broad decoder scope) and all three minor ones
(existence-hiding scope, pagination and readback overclaims, and scope, which
moved invitations to a follow-up). Claude verified the merge and HEAD findings
against the pinned sources. Astra's verdict was to revise, then proceed with the
first tranche, now checkpoints A and B. Its sign-off pass on the revised draft,
before the owner decisions were recorded, found all seven resolved and approved
it with one wording nit, since applied: the client boundary owns thrown request
and body-read errors, not just decoding. A later diff review signed off on the
recorded owner decisions. One model's review is not owner approval or consensus.

### Checkpoint A server-side evidence

**Implemented experiment for the server side, September 26, 2026.** The
[client evidence](#checkpoint-a-client-evidence) completes checkpoint A. Each
step was plan- and diff-reviewed by Astra before commit. The
[reference application guide](../apps/reference/README.md) owns the commands and
measured results; the
[decision record](decisions.md#reference-application-checkpoint-a-server-side--september-26-2026)
records the choices and limits.

- **Built:** `apps/reference` in the S04 layout, with the session/OIDC module
  copied as application code, one consolidated migration (including
  `users.display_name` and `projects.name`) and disposable seeds;
  `memberships.change_role` ported from S16; and `memberships.remove_member`,
  which shares its rejection type and a private mutation with `change_role`.
- **Assembly as implemented:** each operation is still collected alone and
  bridged through `get_openapi_mut`. One checked assembly, used by both the
  application and its export, merges each document only after rejecting a
  conflicting path or same-named component, then checks the catalog. Operation
  routers stay separate rather than going through `OpenApiRouter::merge`, so
  each operation's boundary wraps its own session layer, preserving S16's
  middleware order, and the export needs no identity provider.
- **Extraction decision:** both operations use envelope rendering, the
  per-operation response bridge, the shared refusal/failure profile, the
  request-ID boundary and the assembly checks, so these moved into
  `crates/iris`. Its `Operation` declares a response contract for the bridge;
  registration stays an explicit `routes!` call in the application, so this is
  not alternative B. The application keeps transactions, cleanup classification,
  authorization, SQL, domain types, session code, wire-ID parsing and mounting.
  It injects a classifier for its session refusal markers and owns the session
  security scheme. The CSRF method exemption has one definition, in `iris`, used
  by both the session layer and the operation profile.
- **Deviations from the S16 source:** the session error enum keeps only the
  codes identity emits; S16's `SuccessData` became `ChangeRoleSuccess` and
  `RemoveMemberSuccess` over a shared `Completion`; unmatched routes return a
  plain 404 outside the session layers; per-operation documents no longer carry
  the security scheme; and fixture capture for the TypeScript harness, restored
  with the client, writes one file per operation.

| Acceptance row | Status                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| -------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Prerequisite   | Workflow runs `verify:s16` and `probe:s16`; commands pass locally and passed on GitHub Actions at `a0c25ff`                                                                                                                                                                                                                                                                                                                              |
| A              | Ported S16 Rust suites pass with unchanged expectations, except one marked-response case adapted from `Forbidden` (403) to `LoginFailed` (401) after the session enum narrowed; whole-request fixtures run against the assembled application. Exact status/kind/code tuples for both operations; shared-code metadata checked in memory and in the export; frozen experiments green. The S16 client cases are ported for both operations |
| A probes       | Omitted second bridge, duplicate OpenAPI ID, conflicting shared-code metadata, wrong path and a same-named different component each fail; an identical shared component assembles; an omitted operation collection also fails                                                                                                                                                                                                            |
| React (A)      | Implemented; see the [client evidence](#checkpoint-a-client-evidence): decoder cases for both operations, including oversize and unusable bodies; typed narrowing; unchanged session bootstrap; one browser workflow against the local issuer                                                                                                                                                                                            |

Not established: the busy classification of a failed connection open is
source-inspected rather than tested; local runs used macOS with Node 24.20.0 and
26.8.1. GitHub Actions later passed at `a0c25ff` on Ubuntu with Node 26.10.0. No
productivity claim follows.

### Checkpoint A client evidence

**Implemented experiment, September 27, 2026.** Each step was plan- and
diff-reviewed by Astra before commit. The
[reference application guide](../apps/reference/README.md) owns the commands and
measurements; the
[decision record](decisions.md#reference-application-checkpoint-a-client--september-27-2026)
records the choices and limits.

- **Boundary:** `apps/reference/web` generalizes S16's whole-request boundary to
  both operations. Construction fails unless the export's `x-iris` operations
  are exactly `changeMemberRole` and `removeMember`, each a POST with a JSON
  schema for every status. Each call executes its request once and validates the
  complete status and body against that operation's schemas, so one operation's
  envelope never passes as the other's. Bodies are read once, as sent: past 64
  KiB, declared or streamed, a body is cancelled and becomes `client_unknown`
  (`body_oversize`); a locked, consumed or partly read body is
  `body_unreadable`; invalid UTF-8 is `non_json`.
- **Contract source:** the client bundles the committed `openapi.json` rather
  than fetching a document, so no wire route was added. The Rust drift test
  guards the snapshot and a generated-type drift check guards the client, so a
  client and server built from one commit agree.
- **Evidence:** real responses captured from both Rust whole-request tests,
  counted per status, kind and code, then independent, cross-operation,
  malformed, oversize and unusable-body cases; compile-time narrowing for both
  operations; and seeded mutations of each check.
- **Console and session:** session bootstrap keeps the existing contracts over
  plain `fetch`. Role change and removal are by ID until checkpoint B's reads.
  Outcome wording is exhaustive over both operations' codes: a 500, a 503 and
  `client_unknown` say the outcome is unconfirmed, claim no effect and send no
  retry (S14).
- **Development binary and browser workflow:** `reference-dev --local-oidc-demo`
  serves disposable data against the local issuer, with one demo editor so
  success paths are reachable without invitations. One browser workflow drives
  sign-in, bootstrap recovery, both operations' success, refusal, absence and
  last-owner outcomes, and one unconfirmed attempt, against the production
  build.
- **Cost of runtime validation:** the production bundle is 374.32 kB (111.87 kB
  gzip), 130.62 kB (38.24 kB gzip) more than the same build with stub validators
  and no Ajv. Compiling both operations' validators took 13.3–24.1 ms (median
  13.9 ms) across nine production-build runs in headless Chrome 154, and 41.5 ms
  cold or about 10 ms warm in Node 24.20.0. Ajv compiles with `new Function`, so
  a strict content security policy would need precompiled validators.

Not established: local browser runs used macOS and headless Chrome 154; GitHub
Actions ran the workflow once, on Ubuntu, at `a0c25ff`. The workflow checks
selected paths, not every code. No frozen experiment was retired: S16's client
harness is now reproduced, but its other omission probes are not.

### Checkpoint B server-side evidence

**Implemented experiment for the server side, September 27, 2026.** The
[client evidence](#checkpoint-b-client-evidence) completes checkpoint B. Each
step was plan- and diff-reviewed by Astra before commit. The
[reference application guide](../apps/reference/README.md) owns the commands and
measured results; the
[decision record](decisions.md#reference-application-checkpoint-b-server-side--september-27-2026)
records the choices and limits.

- **Built:** `memberships.list` / `listProjectMembers`
  (`GET /api/projects/{project_id}/members`) and `projects.list_mine` /
  `listMyProjects` (`GET /api/projects`). Member listing first establishes that
  the actor is a member, in any role, then selects the page; own-project listing
  selects only the actor's memberships and declares no domain rejection.
  `user_contacts` now exists in the consolidated migration, so email canaries
  can show that no read discloses contacts.
- **Read conventions as implemented:** `read::run` takes the fresh connection it
  is given, enables `PRAGMA query_only`, runs the visibility check and the page
  in one deferred transaction, then awaits `COMMIT` or `ROLLBACK` and classifies
  cleanup with S15's vocabulary, which moved from `memberships` to `domains` so
  mutations and reads share it. It drops the connection, so neither the setting
  nor an unfinished transaction can reach a mutation. `ReadError` stays
  application-owned.
- **Pages:** `{items, next_cursor}`, with IDs as canonical decimal strings and
  `next_cursor` always present, null when no further rows existed. `limit` is a
  canonical decimal from 1 to 100, so `05` is refused, and defaults to 50.
  Cursors are `c1.` followed by a key, at most 32 bytes, and carry position
  only. Unknown or duplicate parameters and invalid values return 400, after
  authentication.
- **`crates/iris`:** a GET operation's boundary also establishes request context
  for HEAD, which axum answers with GET's status and headers and no body, and
  `Operation.recovery` is optional so reads omit `recovery` from `x-iris`. Both
  reads use these; the mutations' export is unchanged.
- **Deviations from the proposed shape:** reads take ownership of their
  connection rather than borrowing it; `memberships.forbidden` has one
  descriptor definition, used by both rejection types; the session scheme's
  description now says CSRF applies to unsafe methods only.
- **Client linkage:** the client boundary accepts the four `x-iris` operations
  by a hand-written method map and decodes both reads' captured responses;
  mutation-only code is typed as such. There is no read UI yet.

| Acceptance row         | Status                                                                                                                                                                                                                                                                                                                                        |
| ---------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| B authorization        | Unknown and non-member projects give the same 403 apart from `request_id`, headers included; owners, editors and viewers list alike; a member removed between pages gets 403 on the next; `list_mine` returns exactly each actor's memberships and roles, or `[]`; email canaries never appear; GET needs no CSRF token while POST still does |
| B pagination and input | Forward traversal returns every row once in key order for both reads; the default of 50, 1 and 100 are accepted; foreign and forged cursors stay within the caller's rows; limit 0, 101, `05` or non-integer, malformed, overlong or wrong-version cursors, and unknown or duplicate parameters return 400; HEAD mirrors GET without a body   |
| B cleanup              | A rejection and a page-query failure finalize with an acknowledged rollback, after which a writer commits; dropping a `read::run` future that holds SQLite's shared lock releases a writer it had blocked; `query_only` fails an attempted write, and the next mutation commits                                                               |
| Probes                 | A widened runtime or exported GET limit bound, and each read's omitted visibility predicate, pass the compiler and fail a named independent test; checkpoint A's probes are unchanged                                                                                                                                                         |
| React (B)              | Implemented; see the [client evidence](#checkpoint-b-client-evidence): decoder, narrowing and presentation cases for both reads, including oversize and unusable bodies; the member directory; unchanged session bootstrap; one checkpoint B browser workflow against the local issuer                                                        |

Not established: these steps ran on macOS with Node 24.20.0 and have not run on
GitHub Actions. The 503 captures use the operation's own mount with an injected
actor and no cookie, not an ordinary cookie-authenticated request. Cancellation
is shown for the `read::run` future, not for an HTTP disconnect. `query_only`
guards mistakes and is not a security boundary. The explicit 32-byte cursor
check cannot change behavior while keys are canonical i64 values, so that bound
is documented rather than observed. Only SQLite's default rollback-journal mode
was exercised. No productivity claim follows.

### Checkpoint B client evidence

**Implemented experiment, September 27, 2026.** This completes checkpoint B.
Each step was plan- and diff-reviewed by Astra before commit. The
[reference application guide](../apps/reference/README.md) owns the commands and
measurements; the
[decision record](decisions.md#reference-application-checkpoint-b-client--september-27-2026)
records the choices and limits.

- **Reads in the client:** `read` sends one GET with no body and no CSRF header,
  through the same whole-request boundary as the mutations. It has one overload
  per read, so a caller must narrow the operation first. The project ID is
  encoded into its path segment, and `limit` and `cursor` are sent only when
  given, the cursor exactly as received. Read presentation is exhaustive over
  both reads' codes. It shows only declared fields, and every sentence is scoped
  to the page as read. A 403 is one response for an unknown project and a
  non-member. A failed read says it was not loaded and that no retry was sent.
- **Member directory:** the console lists the caller's projects, then a
  project's members, page by page (1, 10 or 50 rows), with forward navigation
  and reload. A role change or removal starts from a listed member and sends
  checkpoint A's request. The directory's state is pure, tested transitions: a
  result is accepted only under the token it was requested with. When an attempt
  is sent, a shown listing of its project is marked as possibly predating it.
  That records ordering only, and only a later read clears it. No read follows a
  mutation automatically. An attempt's operation and target are captured when it
  is sent and its outcome is added when it completes; all three then stay
  unchanged until the next attempt or a session change. Confirmation belongs to
  one member and action. Session bootstrap is unchanged.
- **Evidence:** decoder cases for both reads, including additive fields,
  mismatches, undeclared statuses, and oversize and unusable bodies, with every
  body-handling case tallied per operation; compile-time narrowing and read
  parameters; read presentation, including an empty continuation page; request
  construction; directory transitions; and seeded mutations of each.
- **Browser workflows:** checkpoint A's workflow now runs from the directory.
  The runner then restarts the API, so checkpoint B's workflow starts from the
  seed data. That workflow covers: a lost initial projects read and a lost
  members read, each shown as not loaded and sent once; member and own-project
  traversal one row per page to the end; and a member who, after leaving the
  project, is refused their next page.
- **Cost of runtime validation:** the production bundle is 389.42 kB (114.41 kB
  gzip), 7.03 kB (1.86 kB gzip) more than at checkpoint B's server side.
  Compiling four operations' validators took 18.0–20.0 ms (median 19.1 ms)
  across five runs in headless Chrome 154.

Not established: local runs used macOS, Node 24.20.0 and 26.8.1, and headless
Chrome 154; GitHub Actions has not run these commits. The workflows check
selected paths, not every code. The API restart's failure window was probed with
an injected delay in a copy of the runner. The refused next page shows
authorization on a new read; it resolves nothing about the removal, which has
its own outcome. The mutations still declare no current-state read. No frozen
experiment was retired, and no productivity claim follows.

### Current-state read evidence

**Implemented experiment, September 27, 2026.** The owner approved declaring
`listProjectMembers` as the current-state read of `change_role` and
`remove_member`, on the terms Astra recommended. Two further steps on
`s17-checkpoint-a` built it: `cfb4d18`, the contract, and `8b7e68a`, the console
and its browser evidence. Astra reviewed each step's plan and diff before its
commit. The [reference application guide](../apps/reference/README.md) owns the
commands and measurements; the
[decision record](decisions.md#reference-application-current-state-read--september-27-2026)
records the choices and limits.

- **Declaration:** `x-iris` `recovery.read` is `false` where no read is
  declared, never `true`. Both mutations declare
  `{"operation_id": "listProjectMembers", "path_inputs": {"project_id": {"request_body_field": "project_id"}}}`.
  Inspection and replay stay unsupported, the new-submission constraint is
  unchanged, and the reads still omit `recovery`.
- **Linkage checks:** assembly checks each declared read against the assembled
  document. The target must be an Iris GET without recovery that requires no
  parameter outside its path. The bindings must cover exactly its path
  parameters, each from a required request-body field with an identical schema.
  Equality is deliberately conservative and rejects some compatible schemas.
  Structure cannot show which field is right: a probe binds `user_id` instead of
  `project_id`, and assembly accepts the binding while the hand-written recovery
  test rejects it.
- **Client:** construction parses each mutation's recovery into an explicit type
  and refuses any shape it does not support, and any recovery on a read.
  Assembly checks compatibility; the client checks only the shape it relies on;
  hand-written tests pin which field feeds which parameter.
- **Console:** an unconfirmed outcome (no usable response, 500 or 503) adds:
  "Reading the project’s members again is a new read. A returned page describes
  members when it was read and neither confirms nor rules out this attempt." It
  offers "Read members of" the attempt's project: that project's first page,
  with no cursor, its input taken from the attempt's recorded request body
  through the declared binding. No read automatically follows a mutation; the
  readback needs a user action and never changes the attempt.
- **Browser evidence:** to withhold a response, the workflow lets the request
  reach the server. A wrapper records the response and throws, so the page gets
  a network error, and the runner then checks independently that the recorded
  response is a 200 acknowledgment. The runner restarts the API a second time,
  so a third workflow starts from the seed data:
  - a withheld role change is read back showing the new role;
  - a withheld removal, sent while page 1 had a next cursor, is read back from
    page 1 showing the member absent;
  - in checkpoint B, Bob's withheld self-removal is refused its next page and,
    read back from another project, refused again.

  Each attempt sent one mutation request and each readback one GET, and the
  outcome and target stayed unchanged. Checkpoint A now also shows a reload
  keeping an acknowledged outcome, with no readback offered.

- **Counts:** 34 `crates/iris` tests, 21 of them new, and 127 in the workspace.
  16 negative client documents; 51 presentations; 13 request constructions and
  readbacks. Two new omission probes. 45 seeded mutations across both steps,
  each caught. The three restart ownership probes, aimed at the second window.
  The production bundle is 391.75 kB (115.10 kB gzip), and compiling the
  validators took 17.3–29.4 ms (median 19.0 ms) across five runs.

Not established: local runs used macOS, Node 24.20.0 and 26.8.1, and headless
Chrome 154. GitHub Actions has not run `cfb4d18` or `8b7e68a`, which are not
pushed. The runner receives the acknowledgment before withholding it from the
page. The cases therefore show that client uncertainty is kept after an
acknowledged commit. They do not test a real disconnect, cancellation, or server
work continuing after the caller is lost. The runner's copy of the response is
never application recovery evidence. The workflows check selected paths, and the
restart windows were probed with injected delays in copies of the runner. One
validator compile measured 29.4 ms; it was not investigated. No frozen
experiment was retired, and no productivity claim follows.

## S18 — Reference application lifecycle

**Decided and implemented: storage, initialization, reset, the migration
refusal, shutdown, session cleanup and the development command.** On September
27, 2026, the owner chose this lifecycle pass as the chunk after the pull
request #1 merge. It covers the five items that S17
[deferred](#ownership-boundaries): persistent storage, seed policy, SQLite
journal mode, worker supervision, and one development command for the issuer,
API and Vite. It was first written as a proposal authorizing no implementation,
dependency, CI, migration or wire change. The recommendations are the driver's
(Claude, Opus 5.5). The same day, the owner settled the first of the ten
[choices](#lifecycle-owner-decisions) and delegated the other nine to the oracle
(Astra, GPT-6-Astra through Codex), which chose each recommended option. The
oracle is also this section's design reviewer and reviewed its plans and diffs,
so its nine choices are not an independent approval. The owner then authorized
implementation in reviewed steps: storage and initialization, reset and
migrations, shutdown and session cleanup, then the development command. The
first step's [evidence](#storage-and-initialization-evidence), the second's
[evidence](#reset-and-migration-evidence), the third's
[evidence](#shutdown-and-session-cleanup-evidence) and the fourth's
[evidence](#development-command-evidence) record what was built. Source
citations in the sections before it are to `8d2cfc7`.

### Current behavior

Every process that uses the reference application's schema starts from a fresh,
disposable database, so no data carries over to a restart. Its temporary
directory is removed when dropped, which happens on a normal return; tempfile
relies on destructors, so a process ended by a signal can leave the directory
and its data behind.

| Concern               | Current behavior                                                                                                                                                                                                                                                                                                                                                                         | Source                                                                                                                                                                                                                                              |
| --------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Development database  | `reference-dev` creates `reference.db` in a fresh temporary directory, removed on a normal return but possibly left behind after a signal; a restart starts from fresh data                                                                                                                                                                                                              | [`reference-dev.rs:16-17`](../apps/reference/src/bin/reference-dev.rs)                                                                                                                                                                              |
| Test databases        | Each test fixture owns a temporary directory; so does each read-module test                                                                                                                                                                                                                                                                                                              | [`tests/identity.rs:59`](../apps/reference/tests/identity.rs), [`memberships/tests.rs:65`](../apps/reference/src/http/memberships/tests.rs), [`read.rs:107-108`](../apps/reference/src/read.rs)                                                     |
| Migration             | One consolidated migration, applied by SQLx's migrator at every startup to a new file                                                                                                                                                                                                                                                                                                    | [`app.rs:49`](../apps/reference/src/app.rs), [`reference-dev.rs:19`](../apps/reference/src/bin/reference-dev.rs), [`0001_initial.sql`](../apps/reference/migrations/0001_initial.sql)                                                               |
| Seeds                 | `seed` inserts Alice (11) and Bob (29), projects 41 and 43 with one owner each, and both external identities bound to the configured issuer; its comment restricts it to disposable fixtures. The development binary then adds Bob as an editor of 41, as the membership tests do                                                                                                        | [`app.rs:51-71`](../apps/reference/src/app.rs), [`reference-dev.rs:20-25`](../apps/reference/src/bin/reference-dev.rs)                                                                                                                              |
| Journal mode          | Never set. SQLx 0.9.0 leaves `journal_mode` unset unless requested, because WAL persists in the database file and switching needs an exclusive lock that the busy timeout cannot wait for. Every file database therefore uses SQLite's default rollback journal; the identity test's in-memory database uses SQLite's `MEMORY` journal                                                   | `sqlx-sqlite-0.9.0/src/options/mod.rs:179-183`, [`tests/identity.rs:573`](../apps/reference/tests/identity.rs)                                                                                                                                      |
| Busy timeouts         | At runtime, domain connections from `app::connect` wait 100 ms and the session store's pool of up to four connections waits 1 s, both overriding SQLx's 5 s default. The membership test fixture's session pool keeps the 5 s default                                                                                                                                                    | [`app.rs:30-37`](../apps/reference/src/app.rs), [`reference-dev.rs:27-35`](../apps/reference/src/bin/reference-dev.rs), [`memberships/tests.rs:74-80`](../apps/reference/src/http/memberships/tests.rs), `sqlx-sqlite-0.9.0/src/options/mod.rs:203` |
| Reads                 | Each read owns a fresh connection with `query_only` enabled and one deferred transaction, and drops the connection afterwards                                                                                                                                                                                                                                                            | [`read.rs:50-68`](../apps/reference/src/read.rs)                                                                                                                                                                                                    |
| Session expiry        | `Store::cleanup` deletes expired sessions and login attempts, but the running application never schedules it; only a test calls it, on an in-memory database. The frozen `auth-demo` runs it every 60 s and logs database errors, but nothing supervises that task's termination                                                                                                         | [`store.rs:19-22`](../apps/reference/src/identity/store.rs), [`tests/identity.rs:601`](../apps/reference/tests/identity.rs), [`auth-demo.rs:54-62`](../experiments/api-slice/server/src/bin/auth-demo.rs)                                           |
| Workers               | None. The delivery worker is not ported                                                                                                                                                                                                                                                                                                                                                  | [S17](#ownership-boundaries)                                                                                                                                                                                                                        |
| Shutdown              | `axum::serve` runs without graceful shutdown or signal handling; the process ends on a signal with requests in flight                                                                                                                                                                                                                                                                    | [`reference-dev.rs:48`](../apps/reference/src/bin/reference-dev.rs)                                                                                                                                                                                 |
| Starting it by hand   | Three commands in three terminals: the issuer fixture on 4001, the development server on 3003 (`IRIS_LISTEN` overrides it) and Vite on 5175, which proxies `/api` to 3003 or `IRIS_API_TARGET`                                                                                                                                                                                           | [Reference guide](../apps/reference/README.md#run-it), [`vite.config.ts`](../apps/reference/web/vite.config.ts)                                                                                                                                     |
| Processes under tests | The browser runner refuses taken ports, treats a server as ready only when its own child reports the bound address, runs each child in its own process group, stops everything on an unexpected exit or SIGINT/SIGTERM, and escalates SIGTERM to SIGKILL after 5 s. It restarts the API to reset data, and passes its whole environment to the API. The binary's smoke test binds port 0 | [`browser.mjs:9-18`](../apps/reference/scripts/browser.mjs), [`browser.mjs:206-245`](../apps/reference/scripts/browser.mjs), [`dev_binary.rs:56-72`](../apps/reference/tests/dev_binary.rs)                                                         |

The frozen API slice is the only worker evidence. Its delivery worker runs as a
spawned task that ticks every second, claims one outbox row under a 30 s lease,
sends with a 10 s timeout, and logs and retries any database error forever
([`delivery.rs:54-85`](../experiments/api-slice/server/src/delivery.rs),
[`outbox.rs:3-4`](../experiments/embedded-db/sqlite/src/outbox.rs)). Both entry
points race the server against the worker's join handle and exit with "delivery
worker stopped" if the task ends, so a worker panic stops the process
([`main.rs:19`](../experiments/api-slice/server/src/main.rs),
[`main.rs:34-37`](../experiments/api-slice/server/src/main.rs),
[`auth-demo.rs:41`](../experiments/api-slice/server/src/bin/auth-demo.rs),
[`auth-demo.rs:67-70`](../experiments/api-slice/server/src/bin/auth-demo.rs)).
Neither stops claiming before exit or waits for an in-flight send. Lease expiry
permits another attempt only if the database survives and the row stays
eligible, and the frozen entry points create a fresh database on every restart.
Claims interrupted before sending still count toward the five-attempt budget,
and exhaustion, expiry or acceptance ends eligibility
([`outbox.rs:24-27`](../experiments/embedded-db/sqlite/src/outbox.rs)). Neither
transmission nor delivery is guaranteed, and duplicates remain possible (S13's
[delivery table](#worked-flow-b-invitation-followed-by-outbox-delivery)). The
session cleanup task is not raced against the server, so its termination would
go unnoticed.

### Journal-mode evidence

A scratch probe held a deferred read transaction open after a `SELECT`, then
tried to commit an insert from a second connection with a 100 ms busy timeout,
once per mode:

| Mode                | Writer's commit while the reader is open           | Reader | After reopening                              |
| ------------------- | -------------------------------------------------- | ------ | -------------------------------------------- |
| Rollback (`DELETE`) | Fails with "database is locked" after about 120 ms | 1 row  | Still `delete`; only the database file       |
| WAL                 | Succeeds at once                                   | 1 row  | Still `wal`; `-wal` and `-shm` files present |

It ran on macOS through Python's SQLite 3.53.4, not the libsqlite3-sys 0.37.0
build the application links, and not through SQLx; it confirms documented SQLite
behavior rather than the application's. The `-wal` and `-shm` files were
observed while connections were still open; SQLite removes them when the last
connection closes cleanly, which the oracle's own probe observed. It matches
S17's reasoning: in rollback mode an unfinished reader blocks committing
writers, which the reads bound by finalizing explicitly. WAL removes that
contention for readers, but changes the file set a reset must delete. WAL for
persistent development storage alone would leave the disposable tests in
rollback mode, so adopting it would need its own contention and cleanup
evidence.

### Recommendations

Accepted direction through the [owner decisions](#lifecycle-owner-decisions);
recommendations 1 to 6, the startup check of 7, and 8's supervision, session
cleanup and shutdown timeline are implemented. Each names the choice it depends
on.

1. **Storage stays disposable by default; persistence is an explicit path.** The
   development binary gains an explicit database path argument, for example
   `--database PATH`, and keeps its temporary database when none is given.
   Tests, the smoke test and the browser runner keep passing nothing, so they
   stay disposable. A command-line argument rather than an environment variable,
   because the browser runner passes its whole environment to the API: an
   exported persistent path would silently reach it. The development command
   (recommendation 6) supplies a gitignored default such as
   `apps/reference/.dev/reference.db`.
2. **One API owns a persistent database, and initialization is atomic.** The
   Rust application owns the database lifecycle: initialization, migration,
   validation, reset, and closing connections and tasks at shutdown. While it
   runs, it holds an ownership lock on its database path, so a second start or a
   reset against the same path is refused before touching the database. A
   crashed owner's lock must not block a later start, and external SQLite tools
   do not honor the lock. No workflow here needs several API processes sharing
   one persistent database; if one appears, this choice is revisited. A new
   database is built at a temporary sibling path: create, migrate, seed, close.
   It then takes the target name only if that name is still free (for example a
   hard link, which fails if the target exists), and the temporary name is
   removed. SQLx connections are closed and their closure awaited before
   linking, and nothing reopens the temporary name. An initialization
   interrupted before the link leaves no database at the target; one interrupted
   between link and removal leaves a complete database under two names, and the
   next startup removes the leftover temporary name. The no-clobber link still
   protects the target if two initializations race despite the lock. Hard
   linking is a candidate mechanism, not a proven implementation. An existing
   database is migrated but never seeded, so a restart keeps changed
   memberships.
3. **Seeds run only inside initialization.** `seed` and the development binary's
   extra editor row form one development fixture set, applied only to a database
   this initialization created. There is no separate seed command and no
   reseeding of an existing file. Seeded external identities are bound to the
   issuer URL in use at creation, so the development command fixes the issuer at
   `http://127.0.0.1:4001`; a database created against another issuer needs a
   reset.
4. **Reset deletes the database, with exclusive ownership.** Every user of the
   database is stopped and its connections closed first, and startup and reset
   exclude each other, for example with a lock file. The development command
   refuses a reset while its API is running. A connection left open keeps using
   the deleted file while new connections use its replacement, and SQLite warns
   that deleting a live database's files risks corruption through reused sidecar
   names. A reset then removes the database file and any `-journal`, `-wal` and
   `-shm` files, and initializes again. It removes the session, login-attempt
   and external-identity tables with the domain data, so every browser session
   ends. It is never an in-place delete of rows.
5. **Migrations become append-only once a persistent database exists.** SQLx's
   migrator records each applied version with a checksum and refuses a modified
   migration. After persistence lands, schema changes add a migration rather
   than editing `0001_initial.sql`; a checksum refusal stops startup. Startup
   never resets. Its message names the database path and the migration version,
   describes the mismatch, and suggests restoring the applied migration's source
   (adding a new migration for the intended change) before the reset, which
   discards the data.
6. **One Node supervisor script starts the issuer, API and Vite.** A script such
   as `apps/reference/scripts/dev.mjs`, possibly also exposed as an npm script,
   starts the issuer on 4001, then the API on 3003 with the persistent path,
   then Vite on 5175. It sets the addresses it owns explicitly: Vite's proxy
   target (`IRIS_API_TARGET`) is the API it started, and the API's listen
   address, public origin and issuer match the other two children, overriding
   any inherited values, so the console cannot reach another API and database.
   Its reset invokes the Rust application's reset rather than deleting files
   itself. It adopts the browser runner's process rules: refuse taken ports,
   readiness from each child's reported address, one process group per child,
   stop everything and exit non-zero when any child exits unexpectedly, and
   SIGTERM then SIGKILL after 5 s. Invocation (a script, an npm script or both)
   is separate from process ownership. Code shared with the browser runner moves
   into a common module only when the second script needs it, mirroring S17's
   extraction rule. It does not regenerate the contract in watch mode; the drift
   checks already name the regeneration commands.
7. **Journal mode stays rollback for now, and startup reports it.** Rollback
   mode is the only mode checkpoint B's read evidence exercised
   ([evidence](#checkpoint-b-server-side-evidence)), and a single developer's
   database is assumed to rarely contend. Startup reads `PRAGMA journal_mode`
   and refuses a database whose mode differs from the configured one, rather
   than switching it silently, since a file opened elsewhere in WAL mode keeps
   WAL. WAL for the persistent database stays the alternative, set once during
   initialization. Busy timeouts stay as they are and are documented.
8. **Periodic tasks and workers are supervised when they arrive.** With
   persistence, expired sessions accumulate, so session cleanup becomes the
   first periodic task. Any task, including a future delivery worker, is owned
   by the process: its handle is kept, an unexpected exit or panic is reported
   and stops the development process (as the frozen entry points do for the
   worker), and shutdown follows one timeline: on SIGINT or SIGTERM the server
   stops accepting connections, tasks stop starting new work, in-flight requests
   and tasks drain until an inner deadline, then pools and connections close and
   the ownership lock is released. Axum 0.8.9's graceful shutdown waits for
   connections without a timer of its own, so the inner deadline is the
   application's. It fits inside the supervisor's outer 5 s kill with a margin,
   or the design states that forced termination is the limit. Neither a clean
   exit nor an expired deadline creates a receipt, a rollback acknowledgment or
   permission to resend: a mutation interrupted by shutdown stays unconfirmed
   for its caller, as S14 requires. These are constraints for the invitations
   design, not a worker implementation. That design must still choose restart
   versus stop on worker failure, and state that a send interrupted by shutdown
   stays uncertain: SMTP may have accepted it. After the lease expires the row
   may be sent again, or never, if the attempt budget, expiry or acceptance ends
   its eligibility.

### Acceptance checks for a later implementation

| Area            | Independent checks                                                                                                                                                                                                                                                                                                                                                      |
| --------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Storage         | Without the path argument, data is disposable, as today; with it, a restart keeps a changed role and a removal and does not reseed; a persistent path set in the environment does not reach the browser runner or the tests; a second start on the same path is refused before touching the database, and a crashed owner does not block the next start                 |
| Initialization  | Interrupting initialization after migration and after seeding leaves no database at the target; interrupting it after the link leaves a complete database at the target, and the next startup removes the leftover temporary name; two concurrent initializations leave one seeded database with one set of fixtures; an existing database is migrated and never seeded |
| Reset           | Reset is refused while the API or another initialization holds the database; otherwise it removes every database-owned file, including `-wal` and `-shm` when present, and a browser session from before the reset is no longer valid afterwards                                                                                                                        |
| Migrations      | A modified applied migration stops startup without resetting, with a message naming the path and version and offering restoration before the reset; after restoring the migration, the retained data is intact; an added migration applies to an existing database                                                                                                      |
| Journal mode    | A newly created database reports the configured mode; an existing database in another mode, including one switched to WAL elsewhere, is refused, not converted                                                                                                                                                                                                          |
| Tasks, shutdown | Session cleanup removes expired rows on a persistent database; a task that panics stops the process with a message; cooperative completion and inner-deadline expiry are tested separately, each within the outer kill bound; no task starts new work after the signal; a mutation interrupted by shutdown is reported as unconfirmed, never acknowledged               |
| Command         | A taken port refuses startup before anything starts; with a conflicting `IRIS_API_TARGET` inherited, a sentinel at that address receives no requests; readiness waits for all three; any child's unexpected exit stops the others and exits non-zero; SIGINT stops all three process groups; the frozen experiments and the browser runner behave as before             |

### Lifecycle owner decisions

On September 27, 2026, the owner settled the first choice as recommended and
asked the oracle to settle the rest; it chose each recommended option and
flagged none for the owner. The alternatives stay recorded for their rationale.

| Choice                                   | Decision                                                           | Alternative not chosen                                                        |
| ---------------------------------------- | ------------------------------------------------------------------ | ----------------------------------------------------------------------------- |
| Default for the development binary       | Disposable unless given an explicit path                           | Persistent by default, disposable by flag                                     |
| How the path is given                    | Command-line argument                                              | Environment variable, removed from the browser runner's environment           |
| Where the development command keeps data | Gitignored `apps/reference/.dev/`                                  | The operating system's per-user data directory                                |
| When seeds run                           | Only during atomic initialization of a new database                | An explicit seed command, idempotent against an existing database             |
| Migration policy after persistence       | Append-only; a checksum refusal names the reset                    | Keep editing `0001_initial.sql` and reset on every schema change              |
| Journal mode                             | Rollback everywhere, checked at startup                            | WAL for the persistent database, set once at initialization                   |
| Development command                      | A Node supervisor script, optionally also an npm script            | A process-runner dependency, a Rust binary, or a Makefile without supervision |
| Session cleanup                          | A supervised periodic task once storage persists                   | None while development data stays small                                       |
| Owners of a persistent database          | One API at a time; a second start is refused                       | Concurrent starts converge on one initialized database                        |
| Shutdown budget                          | An inner drain deadline inside the outer kill bound, with a margin | A coarse process-stop bound that promises no drain                            |

### Storage and initialization evidence

The first implementation step, under the owner's authorization, covers
recommendations 1, 2 and 3 and the startup check of 7.
[`storage.rs`](../apps/reference/src/storage.rs) owns the development database;
[`reference-dev`](../apps/reference/src/bin/reference-dev.rs) takes
`--database PATH` after `--local-oidc-demo`, refuses any other argument, and
stays disposable without it through the same code path. No dependency was added:
the ownership lock is Rust 1.98.1's `File::try_lock` on a never-deleted sibling,
`<name>.iris-lock`, which the operating system releases when the process ends.

The plan review added the protocol's safety rules. A path is resolved to one
identity first: the parent must exist and is canonicalized, and a symbolic link
as the database, a name ending in `.iris-lock` or `.iris-init` in any case (with
or without a SQLite sidecar suffix), or a parent inside a staging directory is
refused before the lock file is created. A new database is built in a staging
directory, `<name>.iris-init/`, claimed with `create_dir` under the lock and
holding an `owner` record with the canonical target path; the database is
migrated, seeded with the development fixture set (`seed` plus the editor row),
closed with its closure awaited, then published with a hard link that fails if
the target exists. A later start reclaims a staging directory only if it is
empty or holds a matching `owner` record and nothing but the initializer's own
files, deleting the database files before the record and never recursively; any
other occupant is refused and kept. An existing target with two links is
accepted only when the second is the owned staged database with the same device
and inode, and must have one link after reclamation. A missing target whose
`-journal`, `-wal` or `-shm` file remains is refused, since a hot journal could
replay old content into a new database. An existing database is checked for the
rollback journal (`delete`) before any migration write, migrated, and never
seeded. Paths, the staging directory and link counts are validated before any
staging file is deleted; the journal check follows reclamation of a valid
staging directory. Connections and the server's session pool that were
successfully opened are closed, their closure awaited, before the lock is
released on the handled return paths; a failure while opening one is not
covered.

Local run on macOS (APFS), Rust 1.98.1, Node 24.20.0 for the browser workflow:

```sh
cargo test --workspace --locked              # 150 passed, 0 failed
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo fmt --all --check
npm --prefix apps/reference/web run verify   # passed
mise exec node@24.20.0 -- node apps/reference/scripts/browser.mjs --artifacts <dir>   # passed
```

| Acceptance row | Test                                                                                                                                                                                                                                                                                                                                        |
| -------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Storage        | Without `--database` the smoke tests, the browser workflow and a disposable `Storage` behave as before; a restart keeps a changed role and a removal without reseeding; a second owner, in-process or a second `reference-dev` process, is refused before the database changes; after `SIGKILL` of a running server the next start succeeds |
| Initialization | A child process of the test binary is killed at barriers after migration, after seeding and after publication; the first two leave no target and the next start reclaims the staging directory and seeds once; the third leaves two links, and the next start restores one without reseeding; concurrent opens leave one seeded database    |
| Aliases, names | A directory alias contends for the same lock; a symbolic link, a foreign hard link, and a publication leftover beside a foreign alias are refused with every name kept; reserved names in mixed case are refused; unrelated siblings survive reclamation                                                                                    |
| Journal mode   | A new database reports `delete`; a database switched to WAL elsewhere is refused and still reports `wal`; a missing database with a stale sidecar is refused                                                                                                                                                                                |
| Migration      | A valid SQLite file without the schema is migrated to version 1, keeps its unrelated table and receives no fixtures                                                                                                                                                                                                                         |

The barriers and an injected failure after migration exist only in the library's
test build. Twelve mutations, each on a disposable copy with its own Cargo
target directory, were each caught by at least one storage test: dropping the
journal check, the lock, the migration of an existing database, the sidecar,
symbolic-link, link-count or owner-record check, or the reclamation before
initialization or beside an existing database; seeding an existing database;
publishing with `rename` instead of a hard link; and case-sensitive reserved
names.

Limits: closing before publication is source-reviewed, not tested, because an
idle connection in rollback mode leaves no observable file state; the failed
initialization test shows lock reacquisition and recovery, not that no
descriptor lingered. A torn `owner` write is refused, never reclaimed, and needs
removal by hand. External SQLite tools and processes that ignore the lock are
not excluded; only the no-clobber link protects the target. A database created
against another issuer is not detected. Linux and GitHub Actions had not run
when this was written; the step was later pushed to `main`, and GitHub Actions
run 36364999959 (Verify, on Ubuntu) passed on `c810f91` on its first attempt,
including these storage tests. Reset and the migration refusal followed in the
[next step](#reset-and-migration-evidence); graceful shutdown, session cleanup
and the development command are later steps.

### Reset and migration evidence

The second implementation step, on September 30, 2026, covers recommendations 4
and 5. `Storage::reset` and
`reference-dev --local-oidc-demo --database PATH --reset` replace a database
with a newly seeded one; a modified applied migration stops startup with a
refusal that names the path, the version, restoration and then the reset. No
dependency was added.

`open` and `reset` share one front half: the path is resolved, the ownership
lock is taken, and the staging directory, the three sidecar names and the target
are validated before anything is deleted or opened with SQLite. Reset then
requires the target, if present, to be empty or to start with SQLite's 16-byte
header; reclaims a valid staging directory; deletes the target, then its
`-journal`, `-wal` and `-shm`; and initializes as a first start does. It never
opens the old database, so it also works on one in WAL mode or with a modified
migration. The database is deleted before its sidecars: an interruption then
leaves a missing database with stale sidecars, which a start refuses, naming the
reset, and a second reset completes. The other order could leave a database
without its hot journal. The command needs `IRIS_OIDC_ISSUER`, since the seeded
identities are bound to it, and reads nothing else: it does not contact the
issuer or listen. The lock file is kept.

The plan review found that a database could have been created at another
database's sidecar name (`dev.db-wal` beside `dev.db`), each with its own lock,
so that a reset of `dev.db`, or SQLite opening it, would treat the other
database as a sidecar. The oracle reproduced the deletion for all three
suffixes. A database name ending in `-journal`, `-wal` or `-shm`, in any case,
is now refused. For databases created before that rule, a start or reset is
refused, leaving the database and its neighbors as they are apart from possibly
creating the target's empty lock file, when a sidecar name has an `.iris-lock`
or `.iris-init` sibling (whether or not the sidecar itself exists, which covers
an initializer that has not published yet), or when its occupant is not a
regular file with one link or starts with SQLite's database header. The
[guide](../apps/reference/README.md#a-database-at-a-sidecar-name) gives the
manual procedure for moving such a database as a complete file set; three
further review rounds corrected that procedure (recovery files must move with
the database, a staging directory is removed only after the checks the code
makes, and a symbolic link is not a staging directory).

Local run on macOS (APFS), Rust 1.98.1, Node 24.20.0 for the browser workflow:

```sh
cargo test --workspace --locked              # 171 passed, 0 failed
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo fmt --all --check
npm --prefix apps/reference/web run verify   # passed
mise exec node@24.20.0 -- node apps/reference/scripts/browser.mjs --artifacts <dir>   # passed
```

| Acceptance row      | Test                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| ------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Reset               | Refused with the in-use error while another owner holds the path, while a child process is blocked inside initialization, and, through the binary, while a server runs, which keeps serving; otherwise it discards changed memberships, sessions and login attempts and leaves the seeded fixtures, the rollback journal, no staging directory and the same lock file. It removes `-wal` and `-shm` left by a killed WAL-mode process, both asserted present immediately before the reset with nothing opening the database in between, and each sidecar planted alone. The reset command printed in a refusal, run as printed through `sh`, resets the database for names with spaces, quotes and shell metacharacters. Through the binary, a session signed in as Alice survives a restart and reports no user after a reset, for the same cookie |
| Reset, interruption | A child killed between the database's deletion and the sidecars' leaves no database; a start refuses, naming the reset, and a second reset completes. A child killed while the reset initializes leaves no database, and the next start initializes. A reset reclaims the staging directory of an initialization killed before or after publication                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| Reset, refusals     | A symbolic link, a reserved name, a missing parent, a foreign hard link, a file with a damaged header, a non-SQLite file, a directory and an unrecognized staging occupant are refused with every file unchanged; an empty file is replaced                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| Sidecar names       | For each suffix, for both a start and a reset, with the whole directory compared before and after: a live database created by the earlier rules, a SQLite database with no lock sibling, an empty file with a lock sibling, a directory, a symbolic link and a two-link file at the sidecar name; and, with the sidecar absent, a lock sibling, a staging sibling and a live initializer blocked before publication. The three suffixes are refused as database names in mixed case without creating a lock file                                                                                                                                                                                                                                                                                                                                    |
| Migrations          | A migrator whose version 1 differs is refused with a message naming the path and "migration 1", restoration before the reset command, and the file unchanged; with the original migrator the changed role and removal are intact and nothing is reseeded; the reset then succeeds. An added migration applies to an existing database and keeps its data. Another migration failure names the database                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| Guide               | A WAL-mode database at `dev.db-wal`, left by a killed process with a committed change only in its write-ahead log, shows that change after the whole file set is moved, and the older value when the database is moved alone                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |

The run-time migrators are built from copies of `0001_initial.sql` in a
temporary directory; `migrations/` is unchanged. The test-only barrier
`after-remove` joins the earlier three. Forty-three mutations, each on a
disposable copy with its own Cargo target directory, were each caught by at
least one test. Twelve repeat the first step's against the refactored source.
Nine are on reset: leaving each sidecar, deleting sidecars before the database,
dropping the header check, refusing an empty file, not initializing, not
reclaiming staging, and not deleting the database. Twelve are on the sidecar
names: dropping the checks for a reset only, for a start only, or running a
reset's after the database is deleted; ignoring the file type, the link count,
the header, the ownership siblings, either sibling alone, or the siblings of an
absent sidecar; and admitting the suffixes as names, or only in lower case. Five
are on migrations and their message: leaving the bare error, resetting at
startup, offering the reset first, dropping the path, and printing the reset
command's path unquoted. Five are on the binary: serving after a reset, reading
the public origin, opening instead of resetting, printing a refusal whose
command is not the reset, and accepting `--reset` without `--database`. The
tests and the implementation were written together, so these mutations, not an
observed failing run, are the evidence that the tests detect the behavior.

Limits: the target check is a plausibility check, so a reset deletes an
unrelated empty file, or one starting with SQLite's header, at the path it is
given; a database whose header is damaged is refused and needs removal by hand.
The sidecar checks detect recognizable database evidence; a foreign regular
single-link file at a sidecar name with no ownership sibling and no database
header is deleted by a reset. They run once, under the target's lock: only a
binary from before this step can create a database at a sidecar name, and one
started after the checks have passed is not excluded. The session evidence is
one signed-in session through the issuer fixture, not the browser. The ownership
lock belongs to the open file, so a child process spawned while it is held
shares it until the child execs. The development binary spawns none. In the
multi-threaded storage tests a spawn in one test made another test's path look
in use for that instant, which the oracle reproduced as an intermittent failure;
the storage tests that spawn a child now run alone for their whole length, under
a lock the other storage tests share, and the membership fixture takes the same
lock around its issuer spawn, the only other spawn in the library's tests. Forty
consecutive runs of the library's tests passed. A process that did spawn
children while owning a database would extend its ownership to them in the same
way. The guide's `stat` command for Linux and its manual steps were not
executed; the move of the file set was, through SQLx on macOS. Linux and GitHub
Actions had not run when this was written; the step was later pushed to `main`,
and GitHub Actions run 36721898213 (Verify, on Ubuntu) passed on `66d0608` on
its first attempt. Shutdown and session cleanup followed in the
[next step](#shutdown-and-session-cleanup-evidence); the development command is
a later step.

### Shutdown and session cleanup evidence

The third implementation step, on September 30, 2026, covers recommendation 8
for the process itself: supervised tasks, session cleanup and the shutdown
timeline. [`lifecycle.rs`](../apps/reference/src/lifecycle.rs) owns everything
after the listener is bound, and
[`reference-dev`](../apps/reference/src/bin/reference-dev.rs) builds its runtime
by hand so that the module can end it. One dependency changed: Tokio's `signal`
feature, which adds `signal-hook-registry` 1.4.8 to the lockfile.

The handlers for SIGINT and SIGTERM are registered before the readiness lines
are printed. Serving ends at the first of a signal, a task ending, or the server
ending, and one timeline follows. The stop is published: the server stops
accepting, and a periodic task starts no new unit, with the stop winning over a
tick that is ready at the same time. Requests and units already in flight may
finish until the drain deadline, 3 s after that first cause; a later signal does
not move it. After a complete drain, the session pool is closed and every domain
connection's closure awaited under one close deadline of 1 s, with the runtime
still running. The outcome keeps the cause, the drain, the failures and the
closure apart: a panic before or during the drain is kept, a task's return
counts as normal only if the stop had been requested when it returned (read
inside the task, so a return that races a signal is still an early return), and
the exit code is 0 only for a stop that a signal requested, that drained, that
had no failure and whose closure was acknowledged. Session cleanup runs
`Store::cleanup` at start and every 60 s; a database error is printed and the
next tick tries again.

The plan review found that the first plan released the ownership lock after
shutting the runtime down, which establishes nothing about SQLx: its SQLite
connections run on their own threads, and the oracle's probe showed a second
owner taking the lock while a worker was still inside an update. Closure is
therefore acknowledged, not assumed. `http::open` returns a tracked connection:
a ticket is taken before the opening is awaited, and only an awaited `close`
that succeeds releases it. Dropping the connection queues that close on the
runtime, so no handler can forget it; the two reads pass the tracked connection
through `list_members`, `list_mine` and `read::run` without unwrapping it. When
closure is not established, the process prints its diagnostics and calls
`std::process::exit(1)` with the storage never dropped, so the lock lasts until
the operating system ends the process and every connection with it. That is the
forced-termination limit recommendation 8 allows. Forced termination happens
after an expired drain, where no close is attempted because handlers still hold
connections, and after an expired close deadline. The diff review found that a
task's factory ran before its handle existed, so a panic there escaped
supervision; the factory now runs inside the watched task.

Tests were written against stub bodies first; that run failed all 19 library
tests and all 4 binary tests then written, and is kept with the step's evidence;
three library tests were added afterwards, one split out and two from the diff
review. Local run on macOS (APFS), Rust 1.98.1, Node 24.20.0 for the browser
workflow:

```sh
cargo test --workspace --locked              # 198 passed, 0 failed
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo fmt --all --check
npm --prefix apps/reference/web run verify   # passed
mise exec node@24.20.0 -- node apps/reference/scripts/browser.mjs --artifacts <dir>   # passed
```

| Acceptance clause                                             | Test                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| ------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Session cleanup removes expired rows on a persistent database | In the library, on a database opened through `Storage::open` with an injected clock: expired sessions and login attempts go at the first tick and unexpired ones stay; rows that expire later go at a later tick; ticks that fail while a writer holds the database past the pool's busy timeout are reported and the task keeps running. Through the binary: rows planted while the server is stopped are gone after the next start, and live ones remain                                                                                                                                                                                                                                                   |
| A task that panics stops the process with a message           | In the library: a panic, an early return, and a panic in a task's factory before it returns its future each stop the server and are recorded as that task's failure, and the other task stops. As a process (a child of the library's test binary running the same `serve` and `finish`): exit code 1 with `task doomed panicked`, and the database can be owned again                                                                                                                                                                                                                                                                                                                                       |
| Cooperative completion, within the outer kill bound           | In the library: a request held inside its handler when the signal arrives completes with its response once released, new connections fail, and the stop is drained. A role change held between its write and its commit (a gate that exists only in the test build) is signalled, released, commits, answers 200, and the pool and tracker close inside the close deadline. Through the binary: SIGTERM and SIGINT on an idle server exit 0 in under 2 s, with the path free at once and a changed role kept; a role change admitted by the server (its `100 Continue` read) and completed after the signal is answered 200 and the process then exits 0                                                     |
| Inner-deadline expiry, within the outer kill bound            | In the library: a request never released expires the drain at the deadline and not before, and is never answered; a task that ignores the stop does the same. Through the binary: a role change whose body is never completed ends with exit code 1 between 2.9 s and 4.5 s after the signal, a second SIGTERM one second in neither ending nor extending the drain, with no final response after the interim one and the role unchanged after a restart                                                                                                                                                                                                                                                     |
| No task starts new work after the signal                      | A counting task starts no unit after the signal and the unit in flight finishes; a real cleanup blocked on a writer at the signal runs both its statements after the writer leaves; with the stop and a tick ready together at its select, no unit starts; a task started after the stop runs none; a subscriber created after the stop still sees it                                                                                                                                                                                                                                                                                                                                                        |
| A mutation interrupted by shutdown is unconfirmed             | As a process: a role change is held between its write and its commit, the process is signalled, and at the drain deadline it terminates. While it waits at a test-only barrier just before exiting, the path is still in use for another owner and the request is unanswered; its own exit code is then 1 with the unconfirmed and unestablished-closure lines, the caller's request fails, and reopening shows the old role. The binary case above shows the same for a request abandoned before its transaction. The caller's side is the existing client case that a failed request decodes to `client_unknown` / `request_failed` ([`client.test.ts:333-335`](../apps/reference/web/src/client.test.ts)) |
| Closure before the lock is released (from the plan review)    | A ticket is released only by an awaited close: in WAL mode the `-wal` file is present while the ticket is outstanding and gone once it is acknowledged; while the connection's own worker thread is held inside an update (through SQLx's update hook) the ticket stays outstanding, and it is acknowledged once the worker is released; a drop outside a runtime and a failed open keep their tickets. As processes: a session-pool connection that is never returned, and a tracked connection that is never dropped, each end in a forced exit with code 1 and the close-timeout line after the close deadline, the path in use until then and free afterwards                                            |
| Outcomes kept apart (from the plan review)                    | A panic after the signal, a panic followed by an expired drain (both lines), a normal return after the signal, an early return in the same poll as the signal, and the message for each combination of drain and closure                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |

The browser workflow's runner stops the API with SIGTERM three times; each API
log shows the drain and `stopped after draining`, and each restart found port
3003 free. The library and binary suites passed 25 consecutive runs each on the
final tree.

Thirty-nine mutations, each on a disposable copy with its own Cargo target
directory and with both suites green before and after, were each caught by at
least one test. Four are on the timeline: no graceful shutdown, tasks never told
to stop, no drain deadline, and a deadline a quarter as long. Two leave SIGINT
or SIGTERM unhandled. Ten are on tasks: the factory run outside the watched
task, handles not watched, a panic or an early return taken as a normal return,
a panic during the drain discarded, completion classified when the handle is
read, no first tick, a ready tick winning over the stop, a unit cut short by the
stop, and a late subscriber missing the stop. Three are on cleanup: a database
error ending the task, and either deletion dropped. Five are on closure: the
close deadline removed, the pool not closed, the tracker not awaited, a close
attempted after an expired drain, and a tracker wait that ignores the current
count. Five are on the tracker: a ticket released without a close, released by a
close that does not wait for the worker, released by a drop outside a runtime,
released by a failed open, and never taken. Seven are on the exit: storage
dropped before a forced termination, a forced termination exiting 0, a stop
counted clean without acknowledged closure, with a failure or without a signal,
a failed but closed stop exiting 0, and a clean stop exiting 1. Two are on the
messages, and one leaves the binary's cleanup task unstarted. The previous
step's 43 mutations, one adjusted for the binary's changed return, were run
again on this tree and each caught.

Limits:

- The budgets are configured values with a measured margin, not a guarantee
  covering output and the operating system: the expired drain through the binary
  ended between 2.9 s and 4.5 s after the signal, inside the supervisor's 5 s.
- A transaction interrupted by a forced exit is left to SQLite's rollback
  journal, as after a kill. The unchanged role after reopening is evidence for
  that gated transaction on that run, not for every forced exit. At that stage,
  response loss on either side of commit and cancellation during commit remained
  open. S14's October 2 observable-boundary checks now cover selected loss
  boundaries; cancellation during commit remains open.
- `100 Continue` shows that hyper 1.11.1 had begun reading the request's body.
  It shows nothing about authentication or the transaction. That hyper finishes
  an admitted request and refuses new connections during a graceful shutdown is
  observed for axum 0.8.9 and that hyper, not taken from documentation.
- An opening that fails while serving keeps its ticket, because SQLx can create
  a connection before `connect` returns an error and nothing is left to
  acknowledge its closure. Every later stop of that process is then a forced one
  with exit code 1 after the close deadline. The message says closure was not
  acknowledged, not that a connection is known to be open. This applies to the
  tracked domain opening only, not to a busy statement on an open connection or
  to the session pool.
- A forced exit leaves a disposable database's temporary directory behind.
- That the handlers are registered before the readiness lines is
  source-reviewed; no test hits the window. A signal before registration, during
  storage opening or issuer discovery, ends the process as before.
- A second signal is not a faster exit.
- Linux and GitHub Actions had not run when this was written; the step was later
  pushed to `main`, and GitHub Actions run 36758026534 (Verify, on Ubuntu)
  passed on `a1d51da` on its first attempt, including these tests. The
  development command followed in the
  [next step](#development-command-evidence).

### Development command evidence

**October 2, 2026 follow-up (ROB-1112):** CI now runs the full
development-command suite in the foreground immediately after
`Verify reference client`, before the later browser workflows in the same serial
job. Normal failure propagation applies; no conditions, skips, retries,
deadlines or version pins changed. The September 30 evidence and S18's original
CI exclusion below are historical.

The relative-database test now checks contents rather than inode identity: Alice
reads Bob's seeded editor role, changes it to viewer with an acknowledged
authenticated request, and reads viewer after a fresh sign-in across restart
without rewriting the sentinel. A refused live reset must leave viewer intact; a
successful stopped reset must restore editor after another fresh sign-in.
Refusal, success, log, proxy-session and clean-stop checks remain. Lead's actual
Linux baseline was 21 passed and 1 failed: a successful reset reused
inode 1579119. Deletion and recreation permit inode reuse; this did not
establish a runtime defect. The earlier shell tail masked Node's failure status.
New runs capture Node's exit status before reading log tails. See the dated
[verification record](decisions.md#development-command-ci-and-content-regression--october-2-2026)
for local acceptance and isolated mutation evidence; Actions acceptance remains
for Lead to observe, not a claim from these local checks.

**October 2, 2026 colored-output correction:** The first
[Actions run](https://github.com/robertguss/Iris/actions/runs/36961151968) at
[`a4112b682438821ece8a46c7b7a53387715544d7`](https://github.com/robertguss/Iris/commit/a4112b682438821ece8a46c7b7a53387715544d7)
passed 21 of 22 tests, including the content-based reset test. Readiness and the
proxy request succeeded, but Vite's ANSI sequences split the readiness test's
`Local:` label and URL. That test now searches a local
`stripVTControlCharacters` snapshot; raw capture, diagnostics, all seven
patterns, presence/order checks and the exact final summary check remain. No
supervisor, runtime, color configuration, pin or deadline changed. The dated
[decision record](decisions.md#colored-readiness-correction--october-2-2026)
distinguishes the colored reproduction and correction checks from the earlier
local results; fresh Actions acceptance is still pending.

The fourth implementation step, on September 30, 2026, covers recommendation 6.
[`dev.mjs`](../apps/reference/scripts/dev.mjs) starts the issuer, the API and
Vite on the persistent database `apps/reference/.dev/reference.db`, which is
gitignored, or on the file given by `--database PATH`; `--reset` runs
`reference-dev --reset` on that database and starts nothing else. The process
ownership it shares with the browser runner moved into
[`supervise.mjs`](../apps/reference/scripts/supervise.mjs), as recommendation
6's extraction rule asks; the browser runner keeps its own signal handlers, its
retirement of replaced APIs and its logs. No dependency, npm script, Rust,
contract, client, migration, CI or frozen-experiment change.

The command installs its signal handlers before its first child, refuses before
building when any of 4001, 3003 or 5175 accepts a connection, builds the server,
and starts the three in order, each only once the one before reported its own
address, with 60 s for each. It sets the API's listen address, public origin and
issuer and Vite's proxy target itself, overriding inherited values. A child's
exit before a stop, a readiness timeout or a failed build stops the rest and
exits 1. SIGINT or SIGTERM begins a stop: SIGTERM to every process group,
SIGKILL 5 s later to any group with a process left, and every member's exit
awaited. A group stays owned until its last member has exited, not only its
leader. The first cause of a stop is kept, and later signals are only reported.
A stop that a signal requested exits 0 whatever the children's exit codes, which
it prints: the API's exit 1 after an expired drain is its own outcome, not a
failure of the command. A child that needed SIGKILL makes the stop exit 1, and
so does a signal that interrupts a reset.

The plan review found that the browser runner's one-shot handlers let a second
Ctrl-C end the supervisor during its cleanup, which the oracle reproduced under
Node 24.20.0 and 26.8.1; that handlers installed after the build would leave a
signalled build running; and that a readiness check on a healthy run could not
show that each readiness is waited for. The command's handlers therefore persist
from its start, and readiness is tested with stand-in servers held at gates
([`gated-supervisor.mjs`](../apps/reference/scripts/test/gated-supervisor.mjs))
under the command's own supervision.

The diff review found three defects and reproduced each. An empty
`--database ""` fell back to the default path, so `--database "" --reset` reset
the default database; an empty path is now a usage error, and only an omitted
argument selects the default. Ownership of a process group ended when its leader
exited, so a member that outlived its leader, or ignored SIGTERM after the
leader exited, was left running and the stop reported clean; groups are now
owned, signalled and escalated until they are empty. And only a start created
the default database's directory, so a first `--reset` on a fresh checkout
failed; a reset now creates it too, while an explicit path's directory must
still exist.

Tests were written first; against stubs, all 18 then written failed, and that
run is kept with the step's evidence. One test gained a reset case afterwards,
and the diff review added four. The last change resolves a relative
`CARGO_TARGET_DIR` against the checkout, in `dev.mjs` and in the tests'
throwaway checkout. Ten consecutive runs of the suite, the browser workflow and
the mutations below were made just before it; afterwards the suite passed again
on both Node versions and with `CARGO_TARGET_DIR=target`. Local run on macOS
(APFS), Rust 1.98.1:

```sh
node --test apps/reference/scripts/test/dev.test.mjs   # 22 passed on Node 24.20.0 and 26.8.1
mise exec node@24.20.0 -- node apps/reference/scripts/browser.mjs --artifacts <dir>   # passed
cargo test --workspace --locked              # 198 passed, 0 failed
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo fmt --all --check
npm --prefix apps/reference/web run verify   # passed
node apps/reference/scripts/probes.mjs       # passed
```

| Acceptance clause                                    | Test                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| ---------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A taken port refuses startup before anything starts  | For each of the three ports held by a listener: exit 1 naming it, no build line, no child, no database or lock file, and the other two ports unbound. With a `cargo` stand-in on `PATH` holding the build, SIGINT or SIGTERM ends the build within 5 s without SIGKILL and starts no server                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| A conflicting inherited `IRIS_API_TARGET` is ignored | Two sentinel servers count requests: `IRIS_API_TARGET` and `IRIS_OIDC_ISSUER` are inherited as one, `IRIS_LISTEN` and `IRIS_PUBLIC_ORIGIN` as the other. A session read through Vite's proxy and a sign-in as Alice from the console's origin succeed, the issuer on 4001 serves its discovery document, and neither sentinel receives a request, including the API's discovery at startup. A reset with another `IRIS_OIDC_ISSUER` inherited still binds the seeded identities to 4001: a later sign-in as Alice succeeds                                                                                                                                                                                                   |
| Readiness waits for all three                        | With gated stand-ins: only the first runs until it is ready, then only the second, and the ready line follows the third's readiness. A readiness timeout, a signal, and an unexpected exit while the second is pending each stop the first two and never start the third. With the real children, each start follows the previous child's address line, the ready line comes last, and a request through Vite succeeds at once. The API refusing its database before readiness stops the issuer and never starts Vite                                                                                                                                                                                                        |
| Any child's unexpected exit stops the others         | For each child, SIGKILL of its group after readiness: exit 1 naming it, every other group gone, the ports free. A signal after the command reported that exit is only reported, and the exit names the first failure                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| SIGINT stops all three process groups                | SIGINT and SIGTERM each: every group gone, the ports free, the API's `stopped after draining`, exit 0, and the database can be reset at once. With a role change admitted by the API and never finished, the API exits 1 at its drain deadline and the command prints that and exits 0 within 5.5 s of the signal. A child held with SIGSTOP is killed 5 s after SIGTERM and the stop exits 1 naming it; second and third signals during that stop neither end nor shorten it. With gated stand-ins, each keeping a helper in its group: a leader killed alone leaves no member of its group or any other behind, and a member that ignores SIGTERM after its leader exits is killed 5 s later, the stop exiting 1 naming it |
| Persistence and reset                                | A relative `--database` lands in the working directory and keeps its inode across a restart; a reset through the command is refused while that database is served, which keeps serving, and afterwards replaces the file. In a throwaway checkout holding the two scripts, a first reset and a first start each create `apps/reference/.dev/reference.db` there; an empty `--database`, with or without `--reset`, is a usage error that prints no data path and creates nothing                                                                                                                                                                                                                                             |
| The frozen experiments and the browser runner        | The browser runner, moved onto the shared module, passes its workflow, and each of its three API logs shows the drain; the workspace suite, the omission probes and the web `verify` pass; nothing in `experiments/` changed                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |

Thirty-nine mutations of `dev.mjs` and `supervise.mjs` were run on the tree just
before its last change, each on a disposable copy with its own Cargo target
directory and with the suite green before and after; 38 were caught by at least
one test. Sixteen are on the command: no port check, or the check after the
build; the API's target, listen address, public origin or issuer left inherited,
and the reset's issuer; the handlers installed after the build; the build
outside cleanup; a failed step ignored; a reset that deletes the file itself;
`--database` ignored, resolved against the checkout, or falling back to the
default when empty; a reset that does not create the default directory; and an
interrupted reset exiting 0. Twenty-three are on the shared module: the ready
line before readiness; no readiness awaited, or only the first's, second's or
last's skipped; no readiness timeout; children not detached, or signalled alone
rather than as a group; a group released when its leader exits, a stop that
returns at the leader's exit, or no escalation once the leader has exited; an
unexpected exit not stopping; a failure exiting 0; a child's exit code during a
stop counted as a failure; no SIGKILL escalation, escalation not counted, or
after 10 s; cleanup not awaiting exits; one-shot handlers; a later signal
exiting at once or replacing the first cause; and the first signal ignored. The
one that survived removes the stop check just before a spawn. In the current
callers readiness resolves inside the output handler and the next spawn follows
in the same microtask queue, so no stop can arrive in between, and the browser
runner checks before it calls; the check is kept for other callers. The browser
workflow also ran against each of the twenty-three shared-module mutants and
passed every time, so it is a regression check of the runner, not a sensitive
check of the module. The gated stand-in keeps a helper in its process group that
outlives it, which is what catches a signal sent to the leader alone.

Limits:

- The tests use the command's fixed ports and run only locally: S18 excludes a
  CI change. They refuse to start while any of the ports is taken and must not
  run beside the browser workflow.
- The tests never touch a developer's data: they pass `--database`, or run the
  default path in a throwaway checkout with a `cargo` that builds nothing. The
  default path in this checkout was run once by hand.
- Cleanup reaches process groups: a descendant that starts its own session or
  group escapes it, and a group that refuses a probe (EPERM) is treated as gone,
  since it cannot be signalled either.
- The port check is not a reservation. A port taken between the check and a
  child's bind makes that child exit, which stops the command.
- Readiness is each child's own address line; the command does not probe the
  addresses.
- A child's final output line is printed only if it ends with a newline.
- The browser workflow ran once, just before the last change, which touches
  neither the runner nor the shared module; on macOS only.

### Review of the lifecycle proposal

The oracle, Astra (GPT-6 through Codex), reviewed S18 at `f83688e` under the
[design review brief](design-review-brief.md), covering all three tracks, as
[`astra-s18-all-01`](reviews/astra-s18-all-01.md). It was not a fresh reviewer:
it had reviewed the step's plan and diff, whose three P2 and two P3 findings
were addressed before that commit, and its report says so. It found no critical
contradiction and recommended keeping the direction. The driver accepted all
four findings and revised this section:

| Finding                                                        | Disposition                                                                                                                                                            |
| -------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AS18-B-01, significant: bind the proxy to the owned API        | Accepted. Recommendation 6 sets Vite's proxy target and the API's addresses explicitly, overriding inherited values; a sentinel acceptance check covers it             |
| AS18-B-02, significant: compose the shutdown deadlines         | Accepted. Recommendation 8 gives one shutdown timeline with the inner deadline inside the outer kill bound; interrupted mutations stay unconfirmed; a new owner choice |
| AS18-A-01, limited: one owner per development database         | Accepted. Recommendation 2 names the Rust application as lifecycle owner and refuses a second start on the same path; concurrent convergence becomes the alternative   |
| AS18-C-01, limited: explain non-destructive migration recovery | Accepted. Recommendation 5's refusal names the path and version and offers restoring the migration before the reset; startup never resets                              |

These dispositions are the driver's. They became accepted direction with the
[owner decisions](#lifecycle-owner-decisions), in which the owner delegated nine
choices to this same reviewer; one model's review and delegated choices are not
an independent approval or consensus.

### Explicit exclusions

No production deployment or configuration system, environments beyond local
development, backups, data migration between databases, PostgreSQL or Turso,
real OIDC provider, hot reloading of Rust code, watch-mode contract
regeneration, delivery worker or invitations, CI change, change to the frozen
experiments, or productivity claims.

## S19 — Reference invitations and delivery

**Accepted future design — October 2, 2026; not implemented or verified.**
ROB-1113 settles S17's invitation follow-up within S18's lifecycle. The reviewed
source base is
[89864373](https://github.com/robertguss/Iris/commit/89864373c59b434599346bc65a5689a99776008d).
The reference application currently has membership commands, reads, session
identity, persistent development storage and supervised session cleanup, but no
invitation routes or delivery worker. Its contacts table exists; its seed does
not populate contacts. The
[frozen delivery experiment](../experiments/api-slice/delivery.md) is precedent,
not evidence that this reference design works. Dated rationale and approval
provenance belong in the
[decision record](decisions.md#reference-invitation-design--october-2-2026); the
[application guide](../apps/reference/README.md#future-invitations-not-runnable-yet)
owns the runnable boundary. Linear owns implementation scope and sequencing.

### Invitation product and authority decisions

Invitations target existing accounts by user ID, grant only `editor`, and expire
one hour after issuance; equality with `expires_at` is expired. The server
generates a cryptographically random 32-byte credential encoded as hex. Email is
delivery metadata, never an identity or account-linking key. Issuance does not
change membership. Acceptance requires both the credential and the matching
session recipient; a bearer credential alone is insufficient.

| Decision point                        | Required future behavior                                                                                                                                                                               |
| ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Issue authorization                   | In the write transaction, check project owner first; absent project and non-owner both yield `invitations.forbidden`.                                                                                  |
| Remaining issue checks, in order      | Recipient exists; recipient is not already a member; no unexpired, unaccepted invitation is pending; recipient has a usable local `.test` contact. Only then persist invitation and outbox atomically. |
| Missing or unusable contact           | Return 409 `invitations.recipient_unavailable`, without disclosing an address or treating delivery as optional. Earlier checks still win.                                                              |
| Issuer loses authority after commit   | Do not revoke the invitation or suppress delivery merely because the issuer is no longer an owner. Fresh issuance needs current authority.                                                             |
| Unknown credential or wrong recipient | Uniform 404 `invitations.not_found` regardless of acceptance or expiry; do not expose the invitation or its recipient.                                                                                 |
| Invitation already accepted           | After credential and recipient match, return 409 `invitations.already_accepted` before checking expiry, even when `now >= expires_at`; no second membership effect.                                    |
| Unaccepted invitation expiry          | After credential and recipient match and the accepted check, return 409 `invitations.expired` when `now >= expires_at`; delivery retries never extend expiry.                                          |
| Recipient became a member meanwhile   | Acceptance consumes the invitation but preserves the existing membership's role; never promote or downgrade it.                                                                                        |
| Accepted recipient later removed      | The consumed invitation cannot restore membership on replay.                                                                                                                                           |
| New invitation after expiry           | A fresh authorized submission may issue a new credential if the checks pass; no automatic reissue.                                                                                                     |

Acceptance and its membership effect are one serialized transaction. Preserve
the application's failure/finalization classification: a failed rollback or
uncertain commit cannot be reported as a known rejection or acknowledged
success. No automatic transaction or client mutation retry is introduced.

Future fresh initialization and explicit reset add local test contacts for the
seeded accounts; existing databases are never reseeded or contact-backfilled.
Append-only schema migrations must leave old databases usable. Missing contacts
cause only issuance's explicit conflict, not a startup failure or a forced
reset. The happy demonstration is **Bob (`29`) inviting Alice (`11`) to project
`43`**, where Bob is the seeded owner and Alice is not already a member. Do not
use project 41's already-member path as the happy case.

### Public invitation wire and recovery contract

Use the existing public v1 envelope, session authentication, CSRF boundary,
canonical string IDs and shared refusal/failure profile. Do not port the frozen
experiment's token-preview response. The future declarations are:

| Domain operation / OpenAPI ID             | Route and input                                                                       | Success status and `data`                                                |
| ----------------------------------------- | ------------------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
| `invitations.issue` / `issueInvitation`   | `POST /api/invitations`, body `{project_id, recipient_id}`                            | 201 `{completion: "acknowledged", project_id, recipient_id, expires_at}` |
| `invitations.accept` / `acceptInvitation` | `POST /api/invitations/accept`, body `{token}`; recipient comes only from the session | 200 `{completion: "acknowledged", project_id}`                           |

`expires_at` is Unix seconds serialized as a string, following the frozen wire
precedent. Issue acknowledgment means invitation and enqueue committed, not mail
sent. Accept acknowledgment means acceptance committed, not necessarily a new
membership row. Neither response returns a token, contact address, receipt,
delivery status or lookup capability.

| Operation | Domain rejections                                                                                                                                                           |
| --------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Issue     | 403 `invitations.forbidden`; 404 `invitations.recipient_not_found`; 409 `invitations.already_member`, `invitations.invitation_pending`, `invitations.recipient_unavailable` |
| Accept    | 404 `invitations.not_found`; 409 `invitations.already_accepted`, `invitations.expired`                                                                                      |

Both also use the existing shared 400 invalid-request, 401 unauthenticated, 403
CSRF, 500 internal and 503 unavailable responses. Body validation must not leak
whether a credential belongs to another account. Domain 403 and CSRF 403 remain
distinct codes in the same envelope.

Both operations declare recovery `inspect: false`, `read: false` and
`replay: false`. A fresh submission requires current authority and intent; it is
not a retry authorized by a lost response. No invitation listing, preview,
status or receipt endpoint is included. Manually observing membership later
never retroactively confirms an unknown issuance or acceptance attempt.

### Outbox, local capture and credential lifetime

The invitation stores a hash; the pending outbox necessarily retains the
plaintext credential and a snapshot of the recipient contact. Invitation and
outbox commit or roll back together. Later contact edits cannot redirect an
existing job. Retries reuse the credential and stable Message-ID, without
extending expiry or promising receiver deduplication.

Clear the sensitive payload on terminal completion, or in an eligibility sweep
for expired, accepted or exhausted jobs when the lease permits. Do not clear a
live lease's payload as though its send had stopped. A stopped worker can leave
expired plaintext credentials in storage until cleanup resumes. Clearing columns
is not secure erasure of database pages, journals, backups, process memory or
captured messages. Resetting the reference database does not clear the separate
capture inbox; old links can remain visible and invalid.

Delivery is only to a dedicated loopback-bound local `.test` mail capture with
no relay or real SMTP configuration. Do not reuse a shared historical inbox as
proof of isolation. The frozen guide records a 24-hour age limit and 500-message
cap. For the reference application, retain the 24-hour precedent and propose a
500-message cap, but pin and verify the capture version, configuration and
retention behavior in the later delivery stage; these are not measurements of
the current reference app. No capture dependency or service is added now.

Diagnostics are bounded IDs, attempt numbers, stages and result categories.
Never log tokens, hashes, Message-IDs, addresses, message bodies or raw SMTP
errors; avoid credential-bearing debug output and attempt snapshots. An SMTP
acceptance means the capture server accepted the message, not that a user
received or read it.

### Worker claims, fencing and lifecycle

Use application-owned persistence and existing tracked connections and task
supervision. Claim in a short write transaction, then perform SMTP outside any
database transaction. Each claim consumes one of **five claims, not five
transmissions**, including claims interrupted before send. Leases last 30
seconds. Retryable failures use 5, 10, 20 and 40 second backoffs after claims
one through four; exhausted jobs become terminal. Malformed payloads and
permanent SMTP failures are terminal. Expired or accepted invitations prevent
future claims, not a send already active. A final crashed claim becomes eligible
for terminal cleanup after its lease expires.

Completion is fenced by the claimed attempt and a still-live lease. A returned
`false` means this call made no transition: even an expired lease without a
replacement claimant can cause it. It is not evidence of another worker or of
SMTP failure. A database error instead leaves completion acknowledgment unknown;
do not reinterpret it as `false`. Database errors get bounded diagnostics and
the next normal tick, not an unbounded retry loop. Unexpected worker exit or
panic stops the process through existing supervision; do not restart the task
automatically.

After observed shutdown, start no new claim. A claim admitted before the stop
may finish, but if shutdown is observed before SMTP begins, do not begin SMTP.
Do not refund that claim. Preserve S18's 3-second drain, 1-second acknowledged
connection-close window and the supervisor's outer 5-second kill bound. Neither
a longer send timeout nor a 30-second lease extends shutdown. If draining or
closure is not established, preserve termination with the ownership lock held
until process exit. Interrupted sends remain uncertain; process restart and
lease recovery can duplicate SMTP acceptance, not supply a request receipt.

| Failure window or intervening event          | Required observation and later behavior                                                                                       |
| -------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| Issuance fails before atomic commit          | No acknowledged invitation or enqueue; finalization uncertainty remains a failure, not a known rejection.                     |
| Issuance commits but response is lost        | Caller outcome unknown; worker may deliver. No inspect/read/replay capability or automatic resubmission.                      |
| Issuer authority revoked after commit        | Existing invitation remains usable and deliverable; later new submission rechecks authority.                                  |
| Expired or accepted before claim             | Suppress new claims; clear payload when no live lease prevents cleanup.                                                       |
| Expiry or acceptance during SMTP             | Active send may finish; acceptance still checks expiry and consumption.                                                       |
| Shutdown or crash after claim, before SMTP   | Claim budget is spent even without transmission; observed shutdown inhibits SMTP. Lease recovery may later retry if eligible. |
| Timeout, interruption or restart during SMTP | SMTP acceptance may already have occurred; retry can duplicate or never occur if expiry, acceptance or budget prevents it.    |
| SMTP accepted, completion lost or errors     | Database acknowledgment unknown; do not claim durable `sent`. Eligibility and lease recovery govern another claim.            |
| Stale completion returns `false`             | No transition by this call, not proof of replacement or a database error; no unfenced corrective write.                       |
| Duplicate mail arrives                       | Same invitation/credential; no exactly-once delivery promise and no repeat acceptance effect.                                 |
| Acceptance commits but response is lost      | Caller remains unknown even if later membership is visible; consumed token cannot be replayed after removal.                  |

### Browser credential handling

Provide ID-based issuance and explicit acceptance only. Consume and scrub the
invitation fragment on initial load and every fragment change, including
malformed values; never accept on navigation or GET. Keep a valid credential
only in memory, outside attempt snapshots, logs, browser storage and OIDC
parameters. Clear it after completion (including unknown outcome), dismissal,
account change, logout or authentication navigation. A full login loses it: sign
in first, then reopen the capture link. A lost token is not recoverable from a
status endpoint. Browser extensions, history synchronization and the capture
system are outside this in-memory privacy guarantee.

### Bounded later implementation candidates

These are constraints on separately scoped Linear issues, not authorization or a
second backlog. Each candidate must be green within its boundary:

1. **Private persistence and domain tests:** append-only invitation/outbox
   migrations, fresh-only contact seeds, atomic issue/accept and eligibility
   rules. Test accepted plus expired returns `invitations.already_accepted`,
   unaccepted at expiry equality returns `invitations.expired`, and unknown
   credentials or wrong recipients return `invitations.not_found` regardless of
   acceptance or expiry. Also test issue-check ordering, concurrent issue and
   acceptance, retained roles, removal after acceptance, enqueue rollback and
   old databases without contacts. No public partial feature.
2. **Private delivery and lifecycle:** tracked connections, claim budget,
   fencing, backoff, terminal cleanup, local capture isolation and retention,
   bounded diagnostics and supervised shutdown. Test the failure-window table,
   distinguishing a stale `false` from a completion database error and a claim
   from a send. Verify duplicates and interruption without claiming delivery.
3. **Both public operations and complete client together:** issue and accept
   HTTP declarations and tests, OpenAPI export followed by generated TypeScript,
   client declarations, decoder/type/recovery tests, presentation, views and
   browser coverage in the **same green candidate**. Do not split the public API
   from its client or hand-edit generated TypeScript. Exercise old-database
   conflict, Bob-to-Alice success, signed-out link reopening, malformed fragment
   scrubbing, account changes, replay and unknown outcomes.

This design does not implement those stages. No production mail, signup,
email-based linking, resend, revoke, manual retry, receipts, framework queue
abstraction or new feature in a frozen experiment is included. No CI, version
pin or lifecycle deadline changes are authorized. Extraction into `crates/iris`
still requires two concrete consumers. Future tests must retain
storage-exclusive guards for child spawns and isolated mutation source/build
directories with passing controls, and must not overlap fixed-port or in-place
browser runs.

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

- **2026-10-02, reference invitation design:** Added accepted future S19 for
  ROB-1113: product and wire decisions, disabled recovery capabilities,
  credential handling, fresh-only contacts, outbox and worker failure windows,
  lifecycle constraints and three bounded later green candidates. Documentation
  only; invitations and delivery are neither implemented nor verified here.

- **2026-10-02, S16 mutation build isolation:** The frozen `probe:s16` runner
  now builds into its disposable copy's own `target`. One `run` function covers
  the direct cargo commands; the regression exercises the nested verifier's
  inherited target and the removal of the runner copy when a probe throws. The
  pristine verifier runs before the first mutation and after the final reset. No
  dependency, contract, client, CI or schema change. The experiment is not
  retired.

- **2026-09-30, lifecycle shutdown and session cleanup:** Implemented S18's
  third step in the reference application: SIGINT and SIGTERM handling, one
  drain and close timeline, a supervised session cleanup task, and tracked
  domain connections whose acknowledged closure decides whether the ownership
  lock is released or the process terminates holding it. Recorded the evidence
  in S18, updated S10, S11 and S17's pointers, and corrected the second step's
  statement that CI had not run. One dependency change: Tokio's `signal`
  feature, which adds `signal-hook-registry` to the lockfile. No contract,
  client, CI, migration or frozen-experiment change.

- **2026-09-30, lifecycle reset and migrations:** Implemented S18's second step
  in the reference application: `--reset`, which replaces a database under the
  ownership lock, and the refusal of a modified applied migration. Reserved
  SQLite's sidecar names and added the checks that keep a reset, or SQLite, from
  treating another database as a sidecar. Recorded the evidence in S18, updated
  S10, S11 and S17's pointers, and corrected the first step's statement that CI
  had not run. No dependency, contract, CI or frozen-experiment change.

- **2026-09-27, lifecycle storage and initialization:** Implemented S18's first
  step in the reference application: `--database PATH`, one owner per path,
  atomic initialization with an owned staging directory, seeds only for a new
  database, and the rollback-journal check. Recorded the evidence in S18 and
  updated S10. No dependency, contract, CI or frozen-experiment change.
- **2026-09-27, lifecycle decisions:** Recorded S18's ten choices as settled:
  the first by the owner, the other nine by the oracle at the owner's request,
  each as recommended. Replaced the choices table with "Lifecycle owner
  decisions", marked the recommendations as accepted direction, and recorded the
  owner's authorization to implement S18 in reviewed steps. Updated the S18
  pointers in the introduction, S10, S11 and S17. Documentation only; no
  implementation yet.
- **2026-09-27, lifecycle pass:** Added proposed S18 for the reference
  application's lifecycle: current behavior with source citations, a
  journal-mode probe, recommendations for storage, initialization, seeds, reset,
  migrations, journal mode, task supervision and one development command,
  acceptance checks and ten choices for the owner, revised after the oracle's
  design review, preserved as `astra-s18-all-01`. Pointed S10, S11 and S17 at it
  and corrected the "Last updated" date. Documentation only; no implementation,
  dependency, CI, migration or wire change.
- **2026-09-27, documentation hygiene:** Recorded the current-state read's first
  GitHub Actions run, which passed every step, in S17's status paragraph.
  Documentation only; no design, wire or code change. The top-level README, the
  API guide, the reference guide and the decision record's open decisions were
  corrected alongside.
- **2026-09-27, current-state read:** With the owner's approval, `change_role`
  and `remove_member` now declare `listProjectMembers` as their current-state
  read. Assembly checks the linkage, the client parses the declaration, and the
  console offers a manual readback after an unconfirmed attempt, worded so that
  no page resolves the attempt. A second API restart gives the browser runner a
  third workflow on fresh data. Recorded the evidence in S17, a status note in
  S15 and checkpoint B's CI run. The wire change is the `recovery.read` value.
  No frozen experiment was retired.
- **2026-09-27, checkpoint B client:** Under the owner's authorization of
  checkpoint B, added both reads to the reference client with page-scoped
  presentation, replaced the console's ID form with a member directory whose
  paging and pairing rules are pure tested transitions, and added checkpoint B's
  browser workflow after an API restart. Recorded the client evidence and
  validation cost in S17; checkpoint B is complete. The mutations still declare
  no current-state read. No frozen experiment was retired; no wire change.
- **2026-09-27, checkpoint B server side:** With the owner's authorization,
  added `listProjectMembers` and `listMyProjects` to the reference application:
  owned `query_only` read transactions finalized explicitly, keyset pages with a
  versioned cursor, strict query parameters, HEAD served as GET, and reads
  without recovery metadata. Extended the omission probes to a GET parameter
  bound and both visibility predicates, recorded the evidence in S17, and noted
  the first passing GitHub Actions run, for checkpoint A. The client accepts the
  reads without read UI; checkpoint B stays partial. No frozen experiment was
  retired.
- **2026-09-27, checkpoint A client:** Added the reference client in
  `apps/reference/web`: a whole-request boundary for both operations with
  bounded single reads, typed narrowing, captured and independent decoder cases,
  a checkpoint A console over unchanged session bootstrap, a development binary,
  and one browser workflow against the local issuer, which CI now runs. Recorded
  the client evidence and validation cost in S17; checkpoint A is complete. No
  frozen experiment was retired; no wire change.
- **2026-09-26, checkpoint A server side:** Implemented `apps/reference` with
  session identity, the ported `change_role`, `remove_member`, one checked
  multi-operation assembly and a private provisional `crates/iris` holding what
  both operations share. Added checkpoint A omission probes and recorded the
  split assembly, extraction decision, deviations and acceptance status in S17.
  Client and browser acceptance remain outstanding; no frozen experiment was
  retired.
- **2026-09-26, S16 checks in CI:** With the owner's authorization, the CI
  workflow runs `verify:s16` in its web verification step and `probe:s16` as its
  own step. Both passed locally under Node 26.8.1; no GitHub Actions run has
  executed them yet. No application, dependency or wire change.
- **2026-09-26, S17 owner decisions:** The owner settled S17's seven open
  choices as recommended: uniform 403 existence hiding for reads, member lists
  visible to any member with display names, HEAD served as GET, shared rejection
  types where permitted sets coincide, rejection of unknown query parameters,
  reads before invitations, and frozen experiments kept in CI until covered.
  Implementation is not yet authorized.
- **2026-09-26, reference application proposal:** Added proposed S17 after
  independent Claude and Astra assessments of `9235c6e` and a local rerun of the
  workspace, S16 and web checks. Recommends a fresh reference application in the
  S04 layout, frozen experiments, utoipa as primary, a private provisional
  `crates/iris` with an evidence-based extraction rule, two checkpoints (a
  second mutation, then the first reads) with independent acceptance checks, and
  a follow-up design for invitations. Revised after Astra's review, which S17
  records. Documentation only; no implementation, dependency, CI or wire change.
- **2026-09-26, bounded S16 integration:** Implemented and verified alternative
  A in an isolated route. Added the runtime/export bridge, generated TypeScript,
  whole-request Ajv validation, behavioral/failure controls and omitted-edit
  probes. Existing API unchanged. Recorded executable findings, injection limits
  and remaining choices in the experiment matrix; no productivity claim.
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
- **2026-09-25, action-authoring vertical slice:** Added S16 comparing explicit
  ecosystem registration with a proposed typed wrapper, exact maintenance paths,
  full HTTP/client boundaries and a bounded later integration experiment. Source
  inspection and versioned librarian research informed the recommendation; no
  new independent review, runtime implementation or application test run.
