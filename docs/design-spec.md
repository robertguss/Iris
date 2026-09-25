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
permission. Obtain HTTP identity from validated sessions. Jobs retain trusted
initiating identity and recheck authority on execution. CLI identity needs an
explicit trusted resolution path; an arbitrary actor-ID flag is not
authentication. System authority, if introduced, must be explicit, not a
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

| Capability                                                              | Status and evidence                                                                                                               |
| ----------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| SQLite/Turso comparison and feedback measurements                       | Implemented experiment; [findings](embedded-db-findings.md)                                                                       |
| Axum APIs, two OpenAPI exporters, generated TS and React                | Implemented experiment; [API guide](../experiments/api-slice/README.md)                                                           |
| Local OIDC/session authentication                                       | Implemented protocol experiment, not real-provider identity assurance; [auth guide](../experiments/api-slice/authentication.md)   |
| Atomic invitation/outbox and local mail recovery                        | Implemented experiment; [delivery guide](../experiments/api-slice/delivery.md)                                                    |
| CLI/MCP verification interface                                          | Implemented pilot; [guide](../experiments/agent-interface/README.md)                                                              |
| Membership role/removal workflow                                        | Independent agent reports local implementation and passing checks; not pushed or transferred to this checkout at this spec update |
| Domain layout and redesigned result model                               | Proposed; no framework API released                                                                                               |
| Rejection metadata, precise per-code schemas and shared contract export | Proposed in S12; no derive or runtime bridge implemented                                                                          |
| Runtime evidence, durable receipts, idempotency, performance inspector  | Design ideas, not implemented                                                                                                     |
| Controlled AI repair/productivity comparison                            | Deferred by owner                                                                                                                 |

The
[independent membership thread](https://ampcode.com/threads/T-01a0d5ff-d9a8-71dc-80a6-0bdb678bf916)
reported 38 workspace tests, 14 API tests, 5 MCP test groups and
browser/contract checks passing in its own checkout. This is reported evidence,
not an independent rerun here. Its real protocol test caught missing MCP enums
after adding a CLI scenario; those entries were fixed there. Dependency setup,
Cargo environment, duplicate exporter declarations and browser-fixture
allowlists remain reported friction. These observations motivate shared
definitions, not productivity claims.

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
implementation commitment or a claim of fully single-source contracts today.
Keep runtime telemetry and durable invocation receipts for a later design pass.
Static discovery describes allowed operations and contracts; making an MCP
mutation callable remains a separate explicit exposure and authorization
decision.

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
