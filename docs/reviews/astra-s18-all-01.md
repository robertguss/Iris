# Iris S18 design review: astra-s18-all-01

Status: original review, preserved as returned on September 27, 2026, apart from
this status note. It assesses revision
`f83688e0f8c0a0ac218cb31416eb713f276c135d`. It is not a synthesis, an owner
decision or a change to the living spec; S18 records the dispositions. Paths
under `/private/tmp` and `/var/folders` are the reviewer's temporary evidence
and may no longer exist.

## 1. Review identity and scope

- **Review identifier and tracks:** `astra-s18-all-01`; all three tracks: A,
  application authoring; B, correctness and authority; C, AI usability.
- **Model/provider:** OpenAI GPT-6 through Codex. The exact serving variant and
  build identifier are not exposed to this reviewer.
- **Snapshot:** `f83688e0f8c0a0ac218cb31416eb713f276c135d`, on local branch
  `lifecycle-design`. `git rev-parse f83688e` and `git rev-parse HEAD` agreed;
  the working tree was clean. Repository path-and-line citations below refer to
  this snapshot, including lines in the spec. No remote publication is implied.
- **Prior involvement:** I am not a fresh reviewer. In this same session I
  reviewed the step 1 plan, its uncommitted diff, and the corrections to three
  P2 and two P3 findings. All five were addressed in the section committed at
  this revision. This report is not an independent confirmation of that work;
  prior involvement creates an anchoring limitation. It is a separate design
  assessment by the same reviewer, not another vote or owner approval.
- **Scope:** S18 and its interactions with S13, S14, S17, and the invitations
  follow-up. The assignment narrows the brief's whole-spec reading instruction;
  I did not conduct a new review of the rest of the spec. No additional agents
  or reviewers were created, and no additional review reports were consulted for
  this assignment.

### Inputs and access

I read the design-review brief; S18; the spec's maintenance rules; S13's
delivery, execution-ownership and evidence boundaries; S14; and the relevant S17
ownership, operation-sequence, recovery and exclusion passages. I also inspected
the S18 entry in `docs/decisions.md` and the reference application's run
instructions.

Source inspection covered the reference binary; connection, migration and seed
assembly; membership transaction and cleanup classification; session-store
cleanup and expiry checks; Vite configuration and package commands; browser
runner process ownership, readiness, environment, restart and termination paths;
and the frozen delivery loop, its callers, and outbox claim/completion rules.
The earlier diff review also inspected the cited identity and membership test
fixtures.
`git diff --exit-code 8d2cfc7 f83688e -- apps experiments crates Cargo.lock Cargo.toml .github`
confirmed that these implementation inputs did not change between the earlier
source review and this snapshot.

Pinned upstream source inspected for this pass:

- `sqlx-core-0.9.0/src/migrate/migrator.rs:238-285`: migration bookkeeping and
  checksum refusal.
- `sqlx-sqlite-0.9.0/src/connection/mod.rs:220-234`: explicit close awaits
  worker termination.
- `axum-0.8.9/src/serve/mod.rs:267-303,389-414`: graceful shutdown stops
  accepting connections and waits for connection tasks; that path supplies no
  drain timer.

These sources were read from the local Cargo registry. Codebase graph and
CodeScent tools were not exposed in this session; inspection used the named
files, ast-grep, and bounded reads. No repository content was sent to an
external research service.

### Executed verification and limits

For this report I ran Git identity/scope checks and one scratch probe outside
the checkout. The probe evaluates the checked-in `vite.config.ts` in a Node VM
with `defineConfig` mocked as an identity function. It tests the configuration
expression, not Vite's server or a browser workflow. On Node **22.23.2**, the
observed `/api` targets were:

| Input                                      | Selected target         |
| ------------------------------------------ | ----------------------- |
| No `IRIS_API_TARGET`                       | `http://127.0.0.1:3003` |
| Inherited override `http://127.0.0.1:3999` | `http://127.0.0.1:3999` |
| Explicit supervisor override to port 3003  | `http://127.0.0.1:3003` |

Command and retained evidence:

```sh
node --experimental-vm-modules /private/tmp/astra-s18-all-01-ew4q3nzy/evidence/proxy-config-probe.mjs /Users/robertguss/Projects/startups/Iris/apps/reference/web/vite.config.ts
```

Output is retained beside the script as `proxy-config-output.txt`; the VM's
experimental-feature warning is in `proxy-config-stderr.txt`.

Earlier in this session, outside the checkout, I ran Python SQLite 3.53.4 probes
for closed-file hard-link publication, interruption after publication, reset
with a live connection, and WAL sidecar cleanup. I also ran a standalone
**tempfile 3.27.0** normal-return/SIGTERM probe. Their results informed the
already-resolved diff findings. They were **not rerun for this report**, and
neither the Python probes nor the tempfile executable prove the application's
SQLx lifecycle. Their scripts/results remain under
`/var/folders/f8/ft7ygqg92pj8qh0rwplbw2x80000gn/T/iris-s18-oracle-whe2dgpk/`.

I did not start the reference application, run its test suites, exercise real
SMTP, reproduce browser misrouting, or validate a proposed shutdown/locking
implementation. No implementation of S18 exists to accept. No productivity or
performance claim follows from this review.

### Design as understood

S18 proposes persistent local development state behind an explicit database-path
argument, while keeping ordinary tests and browser workflows disposable. A new
database receives migrations and one fixture set before publication; subsequent
starts migrate without reseeding. Reset is destructive and requires exclusive
ownership. Rollback journaling remains the recommended baseline, with mismatched
mode refused. A Node command would own issuer/API/Vite processes; Rust would
supervise periodic application tasks. Invitations and delivery remain a later
design. None of this selects a framework API, changes wire contracts, or
promises completion after caller loss.

## 2. Assessment

The proposed direction is coherent and worth retaining. I found no demonstrated
critical contradiction in the corrected S18. Preserve explicit persistence,
seeding only at initialization, truthful delivery uncertainty, disposable test
isolation, application-owned transactions, and the separation of proposal from
owner decision.

Two matters need resolution before their corresponding implementation:

- The development command must own the **effective proxy routing**, as well as
  the processes and ports. Existing configuration can otherwise direct the UI to
  another API while every owned process reports readiness (AS18-B-01).
- Bounded shutdown needs a composed deadline and an explicit interpretation of
  forced termination. The outer five-second kill and an unspecified inner drain
  are separate policies today (AS18-B-02).

The single-owner alternative and migration-refusal wording are refinements, not
reasons to reject the proposal (AS18-A-01 and AS18-C-01). Neither requires a
framework lifecycle abstraction. These are design recommendations for owner
consideration, not authorization to implement them.

### Shared scenarios considered

| Brief scenario                            | Depth and conclusion within this scope                                                                                                                                                                 |
| ----------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Add a rejection variant                   | Boundary check only. S18 should add no lifecycle edits to the existing domain/HTTP/client authoring path; no new omission analysis was run.                                                            |
| Two owners concurrently depart            | Source boundary checked. `BEGIN IMMEDIATE` precedes authority and owner-count checks in `domains/memberships.rs:190-209`; lifecycle ownership must not replace that transaction. No concurrency rerun. |
| Authority changes after enqueue           | S13 interaction reviewed. Delivery of committed intent remains distinct from a new command; S18 must not choose invitation revocation policy.                                                          |
| Initial lock acquisition times out        | Considered in depth. A database-lifecycle ownership refusal is not a membership transaction's `not_started` result; keep their diagnostics separate.                                                   |
| Commit succeeds; response disappears      | Considered in depth with process shutdown. Persistent state and a clean process exit provide no client-held invocation receipt.                                                                        |
| Caller cancels near commit                | Considered in depth in AS18-B-02. A drain timeout or process kill must retain S14's uncertainty.                                                                                                       |
| SMTP may accept; lease is replaced        | Considered in depth against `outbox.rs` and S13. The corrected S18 permits duplicates or no delivery, and a completion fence does not cancel a send.                                                   |
| Telemetry disabled, dropped or evicted    | Boundary check only. S18 creates no collector; lifecycle logs cannot establish missing mutation outcomes.                                                                                              |
| Forged propagation or unrelated inspector | Outside the proposed mechanism. S18 adds neither propagation nor an inspector; preserve S13's boundary. No new security proof claimed.                                                                 |
| Old client or stale diagnostic producer   | Considered for startup diagnostics and revision mismatch in AS18-C-01. Existing wire/client compatibility was not re-audited.                                                                          |

## 3. Findings

### AS18-B-01 — Bind the frontend proxy to the API the command owns

**Kind:** risk.

**Impact:** significant. A developer can inspect or change a different local
database from the one the command just started, defeating the intended
relationship between the UI, owned API and persistent path.

**Confidence:** conditional. The configuration behavior is source-supported and
was observed in the narrow expression probe. The future supervisor is not
implemented; the failure depends on it inheriting the proxy override without
replacing or rejecting it. No end-to-end misrouting was executed.

**Spec section and short excerpt:** S18 recommendation 6,
`docs/design-spec.md:2528-2538`: "starts the issuer on 4001, then the API on
3003 with the persistent path, then Vite on 5175"; acceptance at line 2570
relies on process readiness.

**Assumptions and source evidence:** `apps/reference/web/vite.config.ts:3-6`
selects the proxy from `IRIS_API_TARGET`. `scripts/browser.mjs:61-62,880-885`
spawns Vite without an explicit environment override. The API receives explicit
origin/issuer/listen values at lines 214-218. The proxy check at lines 223-228
accepts an HTTP success; it does not establish which database-backed API
supplied it.

**Failure sequence:**

1. The shell retains `IRIS_API_TARGET=http://127.0.0.1:3999` from another local
   session, where a compatible reference API is still running.
2. A supervisor following the inherited configuration starts its own API on
   3003, its intended persistent path, and Vite on 5175. All readiness lines
   appear as expected.
3. Vite forwards `/api` to 3999. If that API is configured for the same public
   origin and the browser authenticates there, subsequent reads or mutations
   concern that API's database. Port ownership alone has not established
   routing.

**Why it matters for Iris's goals:** An agent cannot verify a
database-preserving restart or a role change if successful UI behavior may
belong to another instance. This is an execution-target problem, not an argument
for a new contract or telemetry subsystem.

**Smallest recommended correction:** Specify that the supervised command
explicitly sets Vite's proxy target to its owned API address, and sets the API's
listen address, public origin and issuer coherently. Override or reject
conflicting inherited values for those fields. Add an acceptance case with a
conflicting `IRIS_API_TARGET`; the other target must receive no requests. No new
wire endpoint is necessary.

**Alternative, including retaining the current design:** Keep arbitrary proxy
overrides for the existing manual workflow. If the supervised command also
supports an external API, make that a distinct mode whose ownership and storage
claims differ. Retaining unrestricted inheritance requires documenting that the
three-process command does not necessarily serve its own API, which I do not
recommend for this bounded slice.

**Tradeoffs, new obligations and dependencies:** Explicit overrides reduce
ad-hoc configuration in the supervised mode but add no package dependency. Any
later configurable port must update the effective proxy target from the same
chosen address.

**Validation that would distinguish alternatives:** Start a harmless local
sentinel at the conflicting target, run the future command with that override,
and verify that the sentinel receives no `/api` traffic while the expected
database receives the authorized workflow change. The VM probe in section 1
establishes only why this acceptance case is needed.

### AS18-B-02 — Define the shutdown budget and what expiry means

**Kind:** open question.

**Impact:** significant. Different interpretations of the two deadlines can make
graceful-drain acceptance meaningless or lead to claims of completion that S14
does not support.

**Confidence:** supported that the design leaves the composition unspecified;
the failure sequences are conditional on a later implementation. This is not a
claim that S18 currently promises every in-flight operation will succeed.

**Spec section and short excerpt:** S18 recommendation 6 at
`docs/design-spec.md:2531-2534` says "SIGTERM then SIGKILL after 5 s";
recommendation 8 at lines 2552-2558 gives in-flight work "a bounded deadline";
acceptance at line 2569 says SIGTERM ends requests within that deadline.

**Assumptions and source evidence:** `apps/reference/scripts/browser.mjs:87-92`
implements the outer five-second escalation. The current reference binary simply
awaits the server (`src/bin/reference-dev.rs:45-49`). Axum 0.8.9's
graceful-shutdown path awaits connection completion without its own timeout
(`src/serve/mod.rs:267-303`). The frozen mail send has a ten-second timeout
(`delivery.rs:58`), which is evidence for the later invitations decision, not a
requirement to port that number. S14's effect table
(`docs/design-spec.md:1082-1089`) distinguishes commit, response loss, dropped
waiters and process death.

**Failure sequence:**

1. The implementation selects an inner drain duration longer than the outer five
   seconds, or only awaits Axum graceful completion.
2. SIGTERM arrives while a mutation is committing or a future worker is sending.
3. The supervisor kills the API first. Its process deadline is met, but the
   inner completion/cleanup promise was not met. A successful restart or absent
   log does not reveal the interrupted operation's outcome.

**Why it matters for Iris's goals:** S18 must preserve the useful distinction
between bounded process termination, acknowledged database cleanup, and a
validated response to the caller. Making all three appear as "shutdown passed"
would weaken S13/S14's evidence model.

**Smallest recommended correction:** Choose one shutdown timeline before
implementation: when admission stops, when periodic tasks stop taking new work,
which work is allowed to drain, the inner deadline, connection/pool closure and
ownership-release ordering, and the outer kill deadline. Keep the inner budget
within the outer one with an explicit margin, or state that forced termination
is the intended limit. Test graceful completion separately from deadline expiry.
Neither path invents a receipt, rollback acknowledgment or permission to resend.

**Alternative, including retaining the current design:** Retain five seconds as
a coarse process-stop bound and explicitly promise no drain completion. That is
simpler and consistent with S14, but it must not be described as proven graceful
cleanup. Alternatively increase the outer bound after choosing an inner policy;
no particular duration is established by this review.

**Tradeoffs, new obligations and dependencies:** Longer draining delays
shutdown; shorter draining increases unresolved outcomes. The design needs
ordering and failure classification, not a generic detached action executor.
Worker-specific send policy remains in the invitations design.

**Validation that would distinguish alternatives:** Inject barriers before
mutation, during commit, and after commit before response delivery in a later
implementation copy. Exercise both cooperative completion and deadline expiry;
assert process bounds separately from client uncertainty and observed cleanup.
When invitations are designed, add an interrupted-send case without treating
lease expiry or fenced completion as proof that SMTP did not accept.

### AS18-A-01 — Consider one owner per development database

**Kind:** simplification.

**Impact:** limited. This could reduce the application lifecycle's coordination
rules and authoring burden; no implementation-size or productivity reduction has
been measured.

**Confidence:** conditional on the owner wanting one development API process per
persistent database. S18 does not establish a need for several API processes to
share that path.

**Spec section and short excerpt:** S18 recommendation 2,
`docs/design-spec.md:2494-2505`: "Of two concurrent initializations one wins
while the other opens the winner's file." Recommendation 4 requires exclusive
reset ownership (lines 2513-2522), and recommendation 6 uses fixed ports.

**Assumptions and source evidence:** The browser runner already refuses occupied
ports (`scripts/browser.mjs:847-853`). The reference binary currently owns
bootstrap (`src/bin/reference-dev.rs:16-40`), and S17 assigns schema, seeds and
bootstrap to application code (`docs/design-spec.md:1855-1863`). Nothing in
these sources provides or requires a multi-process lifecycle protocol for a
persistent reference database.

**Concrete authoring example:** One implementation could let the Rust binary
publish the database, the Node script delete it on reset, and several binary
instances race initialization. An author then has to coordinate publication,
temporary aliases, reset exclusion and runtime ownership across two languages.
The proposal does not require this split, but leaves the ownership choice open.
A single Rust-owned lifecycle path can give the second startup a clear busy
refusal instead of requiring it to join the winner.

**Why it matters for Iris's goals:** The lifecycle should make ordinary domain
authoring easier without making every action understand startup locks or fixture
state. A smaller ownership contract is preferable if the development workflow
does not need shared access from multiple API processes.

**Smallest recommended correction:** Name the database lifecycle owner. Consider
one persistent API owner at a time, with the same application-owned ownership
protocol used by startup and offline reset. Let Node own child processes and
invoke Rust for database operations. A competing start can fail before touching
the database. Preserve atomic no-clobber initialization; this alternative does
not by itself prove publication, eliminate crash leftovers, or make external
SQLite tools honor the application's lock.

**Alternative, including retaining the current design:** Keep convergent
concurrent initialization if multiple processes sharing the database are an
intended requirement. Then define compatible runtime/reset ownership and
leftover-name recovery explicitly. That is defensible, but is additional scope
beyond making one local command convenient.

**Tradeoffs, new obligations and dependencies:** A single owner disallows two
reference APIs sharing one persistent path. It still needs a stable ownership
identity and crash-safe release, plus the existing offline-reset rule. Separate
process workers would require revisiting this choice; an in-process future
worker fits it. No lock API or new dependency is selected here.

**Validation that would distinguish alternatives:** Test two starts against the
same path, including different API ports, and an overlapping reset. Under the
single-owner alternative the second start/reset is refused before database
mutation; an owner crash permits a later start without manual lock-file deletion
being mistaken for proof of safety. Prefer the more complex protocol only if an
actual workflow needs simultaneous owners.

### AS18-C-01 — Explain non-destructive recovery before naming reset

**Kind:** alternative.

**Impact:** limited. A small diagnostic change would better protect
intentionally preserved development state and help an agent distinguish source
drift from a broken database.

**Confidence:** supported as a diagnostic opportunity, not an observed unsafe
agent action. S18 says the message names reset; it does not authorize automatic
reset, and this finding does not assert that it does.

**Spec section and short excerpt:** S18 recommendation 5,
`docs/design-spec.md:2523-2527`: "a checksum refusal stops startup with a
message naming the reset"; the migration acceptance row at line 2567 repeats
that test.

**Assumptions and source evidence:** SQLx 0.9.0 compares the stored and source
migration checksums and returns `VersionMismatch` for a difference
(`sqlx-core/src/migrate/migrator.rs:272-275`). That comparison does not
establish that the database is corrupt or that its contents should be discarded.
S18 recommendations 1-3 deliberately retain changed memberships. S13 separates
runtime data from instructions (`docs/design-spec.md:943-948`), and the spec's
maintenance rules keep owner authority distinct from recommendations (lines
25-30).

**Concrete agent example:** An author accidentally edits `0001_initial.sql` in
an otherwise usable persistent checkout. Startup refuses it. A message that only
points to reset makes destructive reinitialization the most visible next step,
even though restoring the prior migration and adding a new migration may
preserve the developer's data.

**Why it matters for Iris's goals:** Useful feedback should identify the
mismatched artifact and the available choices, without turning a diagnostic into
authority to erase the state that persistence was added to preserve.

**Smallest recommended correction:** State that startup leaves reset as an
explicit destructive operation. Identify the database path and migration
version, describe the mismatch, and mention restoring compatible migration
source before the option to reset and discard data. The reset operation itself
must still satisfy the exclusive-ownership rule. A concise ordinary message is
enough; no new diagnostic protocol is required.

**Alternative, including retaining the current design:** Keep reset as the
recommended recovery if the owner explicitly treats all persistent development
state as expendable across schema mistakes. Document that policy and the data
loss. Alternatively retain only the underlying SQLx error, at the cost of less
useful source-local guidance.

**Tradeoffs, new obligations and dependencies:** Slightly more diagnostic text
and an acceptance assertion. Avoid raw environment dumps, arbitrary commands
from runtime data, or a new result-envelope family for local startup errors.

**Validation that would distinguish alternatives:** With a retained membership
change, modify an applied migration in a disposable copy. Startup should refuse
without invoking reset; after restoring the original migration, the retained
state remains accessible. Review the message for clear alternatives and explicit
data-loss consequences. This is a proposed acceptance check, not an executed
agent-repair experiment.

## 4. Preferred design and unresolved choices

Keep S18 as a local application lifecycle proposal. My preferred implementation
shape, if the owner later approves it, is:

1. The Rust application owns database initialization, migration, validation,
   reset and connection/task shutdown. Consider one process owner per persistent
   database. Test fixtures continue to own separate disposable databases.
2. Node owns the issuer/API/Vite process tree and the effective addresses passed
   between them. The proxy is explicitly bound to the owned API. It does not
   acquire a second implementation of database reset or seed policy.
3. Use no-reseed persistence, append-only migrations and checked rollback mode
   initially. Keep hard-link publication a candidate until its interruption and
   recovery checks pass through the actual SQLx path. WAL for persistent storage
   remains a fair alternative requiring its own evidence.
4. Add the cleanup task without changing membership action ownership. Choose a
   composed process-shutdown policy and retain unknown outcomes where S14
   requires them. Do not add detached command execution or receipts merely to
   make shutdown look complete.
5. Make refusals explain the affected path/configuration/migration and the safe
   choices. Reset remains a deliberate operation that discards state.

### Track A: annotated change-role authoring sketch

This is an illustrative call-site sketch using existing names, not a patch or a
new API. Error-to-HTTP conversion is omitted; the current adapter continues to
own it.

```rust
// Bootstrap has already selected the database path and established its
// lifecycle ownership. No migration, seeding, reset or supervisor API here.
let mut conn = app::connect(&state.database).await?;

// Actor construction and input decoding remain at the existing HTTP boundary.
let result = memberships::change_role(&mut conn, &actor, input).await;
drop(conn);

// Existing mapping renders result. Process exit, a drain timeout or restart
// cannot replace this result with Acknowledged or authorize a retry.
```

The ordinary business action remains the code at
`apps/reference/src/domains/memberships.rs:157-170,183-232`: start
`BEGIN IMMEDIATE`, check owner authority, find the membership, protect the last
owner, update the role, and explicitly classify commit or rollback. The proposed
lifecycle ownership is not that transaction lock.

An author changing `change_role` still works with the domain action/descriptors,
HTTP mapping/declaration and the affected client contract/tests. They should not
have to update the Node supervisor or bootstrap merely to add a rejection. A
lifecycle author works with Rust bootstrap, its checks, the Node command and run
documentation. The preferred alternative clarifies those responsibilities; it
adds no generic action trait, context container or framework lifecycle registry.

### Track C: hypothetical agent decision sequence

This is analysis, not an executed agent experiment.

| Step             | Information, justified interpretation and action                                                                                                                                                                                                                                                         |
| ---------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Observe          | A proposed startup diagnostic identifies migration 1 and a checksum mismatch for the chosen persistent path. A previous development membership change is meant to survive.                                                                                                                               |
| Interpret        | This checkout disagrees with recorded migration history. The diagnostic is not proof of corruption, permission to reset, or a domain-operation result.                                                                                                                                                   |
| Next safe action | Inspect the source diff and expected migration revision read-only. Propose restoring the applied migration and adding an append-only migration if that matches the intended change. Use existing authorization for any edit; seek the owner's choice if preserving versus discarding data is unresolved. |
| Still missing    | Whether the source edit was intentional, which migration history the owner wants, and whether discarding this database is authorized. Do not infer those answers from an error string.                                                                                                                   |

For shutdown during a mutation, the corresponding boundary remains S14's: a
client without its validated response keeps the attempt unconfirmed. S17's
current-state read is a new observation, not a receipt or permission to resend
(`docs/design-spec.md:1979-1992`). Persistent storage does not alter that rule.

### Owner choices, deferrals and reversing evidence

All eight choices listed by S18 remain owner choices. This report adds a
recommended decision about one API owner per persistent database and calls for a
concrete shutdown budget before implementing that part. Resolving proxy routing
does not require opening a general configuration-system project.

Keep invitation issuance/acceptance policy, service authority, revocation,
worker restart strategy and delivery-specific drain behavior in the invitations
follow-up. Preserve S13's distinction between delivery of committed intent and a
new user command. Leave real providers, production deployment, backup design,
framework APIs, durable receipts, a collector, watch-mode contract generation
and productivity claims outside this chunk.

Evidence that would change these recommendations includes a real need for
several API/worker processes sharing a development database, which would justify
more ownership coordination; representative contention through the application's
pinned SQLx path, which could favor WAL; or a concrete requirement for admitted
commands to outlive their request waiter, which would require a separate S14
execution-ownership design. A clean shutdown test alone would establish none of
those requirements.

The original five diff findings remain resolved. The findings in this report are
scoped risks, an open policy question and alternatives for disposition. Neither
this report nor agreement with it settles the owner's choices or authorizes
implementation, dependencies, CI, wire changes, a push or a pull request.
