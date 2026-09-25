# Iris design review: `opus55-all-01`

Status: original independent review, preserved as returned on September
25, 2026. It assesses revision `f674991a93071c87b6c647ef7c3d005c51d296e7`. It is
not a synthesis, an owner decision or a change to the living spec, and none of
the proposals below are implemented. Most source paths are abbreviated and refer
to files under `experiments/`.

## 1. Review identity and scope

- **Review identifier and track.** `opus55-all-01` is a single reviewer covering
  Tracks A, B and C. Finding IDs use the form `O55-<track>-NN`.
- **Model.** Claude Opus 5.5 (`claude-opus-5-5`, 1M context) from Anthropic,
  running in Claude Code at maximum reasoning effort.
- **Revision.** `f674991a93071c87b6c647ef7c3d005c51d296e7` ("Add independent
  agent design review brief"). Local `HEAD` matched `origin/main` (checked with
  `git ls-remote`) and the working tree was clean. I used this one snapshot
  throughout.
- **Documents read in full.**
  - `docs/design-spec.md`
  - `docs/design-review-brief.md`
  - `docs/decisions.md`
  - `experiments/api-slice/delivery.md`
  - `experiments/agent-interface/README.md`
  - I skimmed `docs/research.md`.
- **Source read (not executed).**
  - API slice server: `api-slice/server/src/{lib.rs, invitations.rs,
    delivery.rs, main.rs, bin/export-contracts.rs}` and its `Cargo.toml`.
  - SQLite experiment: `embedded-db/sqlite/src/{issue.rs, lib.rs, outbox.rs}`,
    migrations 0001–0003 and `shared/domain.rs`.
  - Agent interface: `agent-interface/{catalog,schema,server,runner,cli}.mjs`
    and `evidence.rs`.
  - Web client: `web/src/api.ts`, `web/type-tests/clients.ts` and
    `web/scripts/{contracts,breaking-change}.mjs`.
  - Outlines only, via grep: `tests/contract.rs`, `sqlite/tests/delivery.rs`,
    `main.tsx`.
  - Resolved versions in `Cargo.lock`.
- **External sources.**
  - Upstream crate source from the local Cargo cache at the lockfile's exact
    versions: sqlx-core and sqlx-sqlite 0.9.0, hyper 1.11.1, tower-http 0.6.11.
  - Also cached but not in this lockfile: tokio-util 0.7.19 and
    opentelemetry_sdk 0.32.1.
  - docs.rs pages: aide 0.15.1, utoipa 6.0.0, strum 0.28.0.
  - sqlite.org "Transaction" page and RFC 9457.
- **Not inspected.**
  - The membership implementation. It is absent from this snapshot (confirmed
    with `git grep`), so I treat S05/S13's description as the assumption, as the
    brief instructs.
  - Auth module internals and full test bodies.
  - `embedded-db-findings.md`, `first-experiment.md`, the API and auth guides,
    and the linked Amp threads.
- **Executed verification: none.** I built nothing, ran no tests and started no
  application. Every failure sequence below is analysis of source and
  documentation, not a reproduction.

### The design as I understand it

Iris is a set of conventions over Axum, SQLx (SQLite for now), one OpenAPI
exporter, openapi-typescript and React.

- **Domain actions.** A feature is a domain module of named actions. Each action
  is an ordinary async function that takes an explicit connection, a trusted
  `Actor` and a command. It owns one `BEGIN IMMEDIATE` transaction, and inside
  it authorizes, checks invariants and mutates.
- **Results.** An action returns one of three things: typed success; a
  per-action rejection enum with stable codes and transport-neutral recovery
  metadata; or an execution failure.
- **HTTP adapter.** It owns wire DTOs, identity and CSRF extraction, status
  mapping and the route contract. One mapping function is meant to feed both
  runtime rendering and OpenAPI export.
- **External effects.** These go through an outbox row committed with the
  mutation. A leased, attempt-fenced worker delivers them under narrow service
  authority.
- **S13 proposals.**
  - An explicit execution context: invocation ID, deadline and cancellation
    signal.
  - Lossy `tracing`/OTel correlation.
  - Inspection output that separates observation, effect assessment and
    collection coverage.
  - Durable receipts and idempotency stay deferred.
- **Verification.** Action, contract, concurrency and browser tests, plus a
  local CLI/MCP runner that exposes checkpoints observed during tests.

### Where my assumptions differ from the spec's framing

1. **Effect certainty belongs to the transaction boundary.** The spec treats
   committed / not committed / unknown mainly as something evidence and adapters
   work out. I treat it as a typed fact produced where `BEGIN` and `COMMIT` are
   called.
2. **Server-owned execution comes first.** The spec defers server-owned
   execution of mutations as a separate design. I think S13's evidence model
   depends on it.
3. **Only client-held identifiers survive a lost response.** When a response is
   lost, only an identifier the client created itself (such as an idempotency
   key) or a read of current state can reconcile it. Server-generated IDs are
   lost along with the response.

---

## 2. Assessment

### Most important conclusion

The boundaries are right, and the spec is unusually careful not to overclaim. I
found no critical defect. The main weakness is where things are placed.

The design lets adapters and evidence *infer* effect certainty. In the current
stack, the facts that establish it exist only at the `BEGIN` and `COMMIT` call
sites, and cancellation near commit destroys even those. Two small runtime
pieces resolve most of the hard cases in S07, S08 and S13:

1. **A transaction helper.** One Iris transaction helper is the only code
   allowed to begin or commit. It rolls back every `Err`, so a rejection means
   nothing was applied, by construction. It returns a typed `effect`.
2. **Server-owned execution for mutations.** The server then always observes the
   terminal outcome.

With these two pieces, S13 can shrink to an evidence model that is
durable-first, closed-vocabulary and built around one terminal record per
invocation.

On the static side, S12's explicit Rust reference is sound. Its only step the
compiler doesn't check, enumerating the variants, is covered by an existing
crate. The remaining work is one generic bridge plus response declarations made
by the extractors that produce them. An Iris derive is unnecessary.

### Preserve

- Authorization, invariant checks and the mutation in one correctly serialized
  transaction (S05). Pre-checks are not enforcement.
- The `Actor` kept separate from the command. The session carries identity only,
  never cached roles.
- A domain that does not depend on HTTP; jobs and CLI call actions directly.
- Business rejection separated from execution failure. Stable, explicit codes
  that don't depend on variant names. Recovery metadata as constraints, not
  permission.
- The outbox committed with the mutation, and the attempt counter as the fence.
  Token reuse plus a stable Message-ID make duplicate mail harmless — a good
  choice to keep.
- "Unknown stays unknown." Lossy telemetry never blocks business execution. A
  new trace root at untrusted ingress; baggage dropped.
- The distinction between delivering committed intent and executing a new
  command.
- The pilot's evidence discipline: fixed event names and integers, source
  digest, `stale`, and `blocked` vs `failed`.
- The explicit non-goals: no generic executor, no resource DSL, no policy
  engine.

### Decide before implementing S12/S13

1. The transaction helper and a typed `effect` on `ExecutionFailure` (B-01).
2. Execution ownership for mutations (B-02).
3. The `after_unknown` contract, and the client-originated key as the only
   handle for a lost response (C-01).
4. The terminal record as the unit of evidence (C-04). This depends on items 1
   and 2.

### Refinements that can wait

- **Before choosing the "one primary" exporter:** re-run the utoipa/aide
  comparison with response inference enabled (A-02).
- **Cheap, any time:**
  - Outbox fixes (B-03).
  - Deriving the MCP enums from the catalog (C-05).
  - Typed IDs (A-03).
  - strum for enumeration (A-01).
- **Before the first external client:** add an `Unknown` decoding path (C-02).
- **Before freezing the error-code policy:** trim the identifier budget (C-03).
- **Before PostgreSQL:** the aggregate-lock convention (B-04).
- **Owner policy:** delivery eligibility (B-05).
- **An authoring spike:** context carried by the unit of work (A-04).
- **Spec documentation:** add one table mapping each convention to how it is
  enforced (compiler / contract check / test / review). S02 asks for this, and
  today it is scattered.

---

## 3. Findings

### Summary

| ID       | Title                                                                                       | Kind                 | Impact              | Confidence                            |
| -------- | ------------------------------------------------------------------------------------------- | -------------------- | ------------------- | ------------------------------------- |
| O55-B-01 | Make the transaction boundary the only source of effect facts                               | contradiction + risk | significant         | supported                             |
| O55-B-02 | Cancellation near commit is unobservable; give mutations server-owned execution             | risk                 | significant         | supported                             |
| O55-C-01 | After `unknown`, only client-held facts reconcile; resends can return misleading rejections | risk + open question | significant         | supported                             |
| O55-C-04 | Evidence: terminal record as the unit, closed vocabulary, on-disk dev sink                  | simplification       | significant         | conditional                           |
| O55-A-02 | Extractors should declare their own responses; re-run the exporter comparison               | alternative          | significant         | supported (merge behavior unverified) |
| O55-A-01 | Close the enumeration gap with strum; drop the Iris derive                                  | simplification       | significant         | supported (costs unmeasured)          |
| O55-C-02 | Generated client types assume a closed world                                                | risk                 | significant         | supported                             |
| O55-C-03 | Rejection metadata and identifiers: one identity per fact                                   | simplification       | limited–significant | supported                             |
| O55-B-03 | Outbox: fence acknowledgments on the attempt counter only                                   | risk                 | limited             | supported                             |
| O55-B-04 | Last-owner rule: make serialization explicit and independent of path                        | alternative          | limited now         | supported / conditional               |
| O55-A-04 | Carry invocation context with the unit of work                                              | simplification       | limited–significant | conditional                           |
| O55-B-05 | Deferred effects: make the authority distinction enforceable                                | alternative + policy | limited             | supported                             |
| O55-C-05 | Discovery drift already exists in the MCP pilot                                             | contradiction        | limited             | supported                             |
| O55-A-03 | Typed IDs; one rejection enum per distinct rejection set                                    | simplification       | limited             | supported                             |

### Shared scenarios examined

| Scenario                                  | Depth                                      | Conclusion                                                                                                                                                                                                                                                                                                                         | IDs              |
| ----------------------------------------- | ------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------- |
| Add a rejection variant                   | In depth                                   | Missing descriptor or status match arms fail compilation. A handwritten `ALL` list omits the variant silently, so OpenAPI and TypeScript lack the branch and today's client shows the raw code. With `VariantArray` and a derived catalog, every static step is enforced. Correct behavior still needs a test owned by a reviewer. | A-01, C-02, C-05 |
| Two owners depart concurrently            | In depth (algorithm from S05; code absent) | On SQLite, `BEGIN IMMEDIATE` puts the owner count and the write in one serialized step. Every operation that can reduce owners must use that path. PostgreSQL at READ COMMITTED needs an explicit lock or SERIALIZABLE.                                                                                                            | B-04             |
| Authority changes after enqueue           | In depth                                   | Delivery of committed intent under service authority, with eligibility checked in the claim transaction. Any further mutation is a new command. Revocation behavior is product policy.                                                                                                                                             | B-05             |
| Initial lock acquisition times out        | In depth                                   | BUSY at `BEGIN` supports `not_started` for *this action's transaction* only (see limits under B-01). Retry safety only needs `not_committed`, which holds for every SQLite BUSY inside a helper-owned transaction.                                                                                                                 | B-01             |
| Commit succeeds; response disappears      | In depth                                   | Without a receipt, the caller knows only its own key and a read of current state, never which request caused it. A resend can return a rejection caused by its own earlier success.                                                                                                                                                | C-01             |
| Caller cancels near commit                | In depth                                   | Today nobody owns completion: hyper drops the handler, and sqlx still commits a `COMMIT` it has already dispatched. No terminal record is produced.                                                                                                                                                                                | B-02             |
| SMTP may accept; lease is replaced        | In depth                                   | The old worker can still send. A `false` acknowledgment proves only that this call made no transition. Today that includes a lease that merely expired with no other worker involved.                                                                                                                                              | B-03             |
| Telemetry disabled, dropped or evicted    | Moderate                                   | Durable rows and client-held keys survive. Evidence can report whether a terminal record exists, drops since boot and `boot_id` — not per-invocation completeness.                                                                                                                                                                 | C-04             |
| Forged propagation or unrelated inspector | Moderate                                   | The spec's default (new root, untrusted link, baggage dropped) is right. The SDK's default `ParentBased` sampler would hand sampling to the caller. Keep terminal records out of sampling, and keep inspection local-only.                                                                                                         | C-04             |
| Old client or stale producer              | Moderate                                   | An unknown code or shape decodes to `Unknown`, with no inference. Evidence carries build, digest and `boot_id`, plus a `stale` flag as the runner already does.                                                                                                                                                                    | C-02, C-04       |

---

### O55-B-01: Make the transaction boundary the only source of effect facts

- **Kind:** contradiction (S13's example vs S07's principle) plus risk.
- **Impact:** significant. As sketched, three things can go wrong, and all three
  feed retry decisions:
  - inspection can report `not_applied` for a change that committed;
  - adapters keep deriving retry advice from error codes instead of the stage
    that failed;
  - a composed workflow can commit after an inner rejection.

  I don't rate it critical: triggering it needs an authoring bug or a future
  database engine.
- **Confidence:** supported for current code and the S13 text; conditional for
  PostgreSQL, which is not implemented.
- **Spec sections and excerpts:**
  - S07: "Preserve `unknown` across layers… never substitute a confident
    default."
  - S08: "`mutation=not_started` **only because that stage establishes these
    facts**… later-stage failures need their own classification."
  - S13 flow A step 4: "Last-owner rejection before mutation supports
    `membership_mutation=not_applied`."
  - S13's example JSON pairs `assessment: not_applied, basis:
    rejection_before_mutation` with `coverage: unknown, reason:
    terminal_record_unavailable`.
  - S06 leaves `ExecutionFailure` open.
- **Evidence:**
  - `api-slice/server/src/lib.rs:224-230` maps *every* busy error to
    `Unavailable` (503, "Try again shortly"). Actions return `Result<Outcome,
    sqlx::Error>`, so by the time the adapter sees an error, it no longer knows
    which stage failed.
  - `sqlite/src/issue.rs:72-81` and `sqlite/src/lib.rs:86-95` repeat the same
    commit/rollback `match`, and both **commit** when the result is
    `Ok(rejection)`.
  - sqlx-sqlite 0.9.0 leaves `journal_mode` unset (`options/mod.rs:179-183`), so
    the experiment runs in SQLite's default rollback-journal mode. In that mode
    `COMMIT` can return `SQLITE_BUSY` while readers hold locks, and "the
    transaction remains active" (sqlite.org/lang_transaction.html §2.3).
- **Failure sequences:**
  1. *A checkpoint overclaims.*
     1. An agent refactors `change_role`. The last-owner branch still records
        `membership.invariant_rejected`, but the `return` is lost, or a later
        statement writes on that path.
     2. The update commits, and the terminal record is then evicted or the
        process exits.
     3. Following S13, inspection reports `not_applied` with basis
        `rejection_before_mutation`. That is false, and it happens exactly while
        someone is diagnosing that bug.
  2. *Classification ignores the stage.*
     1. A BUSY at `BEGIN IMMEDIATE` and a BUSY at `COMMIT` both become "503, try
        again".
     2. On SQLite neither committed (the second transaction is rolled back when
        it is dropped), so today's advice is safe — but only because of an
        engine property no code states.
     3. The same mapping on PostgreSQL would advise retrying after a connection
        error *during* `COMMIT`, which may have committed.
  3. *Composition loses a rejection.*
     1. `transfer_ownership` calls `set_role(tx, …).await?` twice.
     2. If the first call returns `Ok(Outcome::LastOwner)`, the `?` succeeds and
        the outcome is silently dropped. The enum is not `#[must_use]`.
     3. The outer action then commits the first half.
- **Limits of `not_started`** (the scenario question). A BUSY at `BEGIN`
  supports `not_started` only for this action's transaction. The inference stops
  holding if:
  - anything had an effect before `BEGIN`;
  - the error came from a later statement;
  - the request fanned out to several actions;
  - an automatic retry wrapper hides an earlier attempt.
- **Why it matters for Iris:** retry safety and "unknown stays unknown" are the
  central promises of S06–S08. The facts that establish them exist only where
  `BEGIN` and `COMMIT` are called.
- **Smallest recommended correction:** one Iris-owned transaction helper that is
  the only code allowed to begin or commit, used as `begin_immediate(db)`
  followed by `tx.finish(result)`. It does three things:
  - Commits only when the result is `Ok`.
  - Rolls back every `Err`. A rejection then means nothing was applied, by
    construction, and `?` propagates inner rejections.
  - Returns `ExecutionFailure { effect, cause }` with `effect ∈ {NotStarted,
    NotCommitted, Unknown}`, classified at each call site using per-engine
    rules. For SQLite:
    - BUSY at `BEGIN` → `NotStarted`;
    - any error before `COMMIT`, or BUSY at `COMMIT` → `NotCommitted`;
    - any other `COMMIT` error → `Unknown`.

  Adapters and evidence read `effect`. Checkpoints explain *why* something
  happened, never *what committed*. Public failure bodies should carry the
  caller's own `effect` (`not_committed` or `unknown`). That discloses nothing
  about other resources, and it is exactly what a client needs to choose between
  backing off and reconciling.
- **Alternatives:**
  - Keep per-action commit code and classification in the adapter, as today.
    Fewer types, but blind to the stage and duplicated per action.
  - A generic action executor. S03 and S11 rightly reject this. The helper is
    not one: actions still own ordering, SQL and authorization.
- **Tradeoffs and new obligations:**
  - The helper is a small module, but its engine-specific rules must be tested
    for each engine.
  - "Record-then-reject" patterns (audit logs of denied attempts, lockout
    counters) need an explicit escape hatch, for example
    `finish_committing_rejection`. Evidence must report it as a distinct
    terminal state.
  - Separately, the experiment's rollback-journal default makes a BUSY at
    `COMMIT` more likely than WAL mode would.
- **Validation:** fault-injection tests for each stage. Each asserts the
  classified effect and the final row state.
  - Hold a write lock to force BUSY at `BEGIN`.
  - Hold a read transaction in rollback-journal mode to force BUSY at `COMMIT`.
  - Inject an I/O error at `COMMIT` with a test VFS.
  - Remove the rejection `return` (a mutant) and assert that evidence no longer
    claims `not_applied`.

### O55-B-02: Cancellation near commit is unobservable; give mutations server-owned execution now

- **Kind:** risk (a gap in S13).
- **Impact:** significant. How often this happens is unmeasured.
- **Confidence:** supported by upstream source for each mechanism, at the
  lockfile's versions.
- **Spec:** S13: "Client disconnect, timeout and dropping a future do not
  establish rollback… If accepted commands must finish after disconnect,
  introduce explicit execution ownership as a separate design."
- **Evidence:**
  - hyper 1.11.1, `proto/h1/conn.rs:431-505`. Once the request has been read and
    the handler is still running, `mid_message_detect_eof` detects the client
    closing the connection (half-close is off by default). hyper then closes the
    connection, which drops the in-flight handler future.
  - tower-http 0.6.11, `timeout/service.rs:143`. When the deadline passes, it
    returns the timeout response and drops the inner future.
  - sqlx-sqlite 0.9.0, `connection/worker.rs:264-283`. A `COMMIT` already handed
    to the connection's worker thread still runs. In the source's words: "the
    `Transaction` doesn't know it was committed… we need to ignore that
    rollback."
- **Failure sequence:**
  1. Alice's demotion reaches `tx.commit().await`, and the commit command is
     queued to the worker thread.
  2. The browser tab closes, or a timeout layer fires.
  3. hyper drops the handler.
  4. The worker commits and ignores the rollback that the drop requested.
  5. No action code runs after the commit, so there is no terminal record, no
     response and no error.

  If the drop happens one statement earlier, the transaction rolls back instead.
  The evidence looks identical; the effect is the opposite.
- **Why it matters for Iris:** S13 exists to tell these two outcomes apart. With
  drop-on-disconnect, the most ambiguous case leaves the server knowing nothing.
- **Smallest recommended correction:** mutating handlers spawn the action on a
  server-owned task and await its `JoinHandle`. In tokio, dropping a
  `JoinHandle` detaches the task rather than cancelling it.
  - Track these tasks with `tokio_util::task::TaskTracker` so shutdown can drain
    them.
  - Bound concurrency with a semaphore before spawning.
  - Check the cancellation signal only before `BEGIN`.
  - Reads keep drop-on-disconnect.

  "Caller cancels near commit" and "response lost after commit" then become the
  same case: known to the server, unknown to the client. C-01 covers that case.
- **Alternatives:**
  - Keep drop semantics and document "no terminal record" as a routine outcome.
    Simplest, weakest evidence.
  - Durable execution for all commands. Too large; I agree with deferring it.
- **Tradeoffs:**
  - Abandoned requests still consume work, which is why the semaphore bound
    matters.
  - Shutdown needs a drain deadline.
  - Tracing spans outlive their connections.
  - Adds tokio-util, a small dependency.
- **Validation:** add a test hook in the helper that pauses after `COMMIT` is
  dispatched, then close the client socket.
  - Today: the row is committed and there is no terminal record.
  - With server-owned execution: the row is committed and there is a `committed`
    terminal record.
  - Repeat with a tower-http timeout layer instead of a disconnect.

### O55-C-01: After `unknown`, only client-held facts reconcile; resends can return misleading rejections

- **Kind:** risk plus an open policy question.
- **Impact:** significant. It covers how every mutation is retried, by React and
  by agents.
- **Confidence:** supported. The self-demotion sequence assumes the check order
  in S13 flow A and the owner check that S08 itself raises.
- **Spec:**
  - S08: "Current state can help reconcile… does not prove which request caused
    that state."
  - S08: "A replay of a completed self-demotion must not simply rerun the
    original owner check."
  - S08: "Agents reconcile via an authorized receipt or retry the same key."
  - The S13 identifier table.
- **Evidence:**
  - Request, invocation and trace IDs are all generated by the server. They
    reach the client only in the response that was lost.
  - Current operations are safe to resend in terms of effect, because uniqueness
    checks prevent duplicates (`issue.rs:50-55`, `sqlite/lib.rs:47-49`). But
    after an earlier success, a resend answers with a *rejection*.
- **Failure sequence:**
  1. The owners are Alice and Bob. Alice sends "set Alice → editor".
  2. `COMMIT` succeeds, but the response is lost. The client records
     `outcome=unknown`.
  3. The agent resends. The new transaction checks authority first. Alice is no
     longer an owner, so the answer is `memberships.forbidden`.
  4. The agent concludes that Alice lacked permission and that nothing changed.
     Both conclusions are false.

  A resent accept behaves the same way (`already_accepted`), and so does a
  resent issuance (`invitation_pending`).
- **Why it matters for Iris:** for today's operations, the hazard is not a
  duplicate effect but a misread second answer. That is exactly "inventing
  missing facts".
- **Smallest recommended correction (receipts stay deferred):**
  1. Each operation declares and exports an `after_unknown` contract, with one
     of four values:
     - `resend_is_effect_safe`
     - `reconcile_by_read(<operation>)`
     - `resend_requires_key`
     - `do_not_resend`
  2. A convention: after `unknown`, read state before interpreting any
     rejection, because the rejection may be the echo of the earlier attempt.
  3. Successes of state-setting operations return what was read inside the
     transaction, including `changed: bool`, rather than a bare acknowledgment.
     This also exposes S12's same-role no-op.

  When receipts arrive, the helper writes them inside the mutation's
  transaction:
  - keyed by (actor, operation, key), with a hash of the domain command;
  - looked up *before* authorization and authorized by actor identity alone,
    which answers S08's open replay question for self-demotion.

  Because Iris puts external effects after commit, an atomic receipt needs no
  `pending` state. Concurrent duplicates wait on SQLite's write lock or on a
  PostgreSQL unique index.
- **Alternatives:**
  - Reconcile by reading state only, with no declarations. That works for
    today's operations, but fails for future operations that create something
    with a new identity.
  - A self-targeted no-op rule: setting *your own* role to its current value
    returns success before the authorization check. This fixes self-demotion
    without receipts, but it is an exception to authorization order that agents
    could over-generalize into a disclosure leak for other targets.
- **Tradeoffs:** `after_unknown` is a claim that must be reviewed and tested per
  operation. Receipts add a table, a retention policy and a key-reuse rejection.
- **Validation:** for each operation, commit, discard the response, resend, and
  assert the declared contract holds: the effect is applied at most once, and
  the second answer is the documented rejection or `changed:false`.

### O55-C-04: Evidence: terminal record as the unit, closed vocabulary, on-disk dev sink

- **Kind:** simplification plus alternative.
- **Impact:** significant for S13's scope and usefulness.
- **Confidence:** conditional, since this is design judgment. The sampling fact
  below is supported.
- **Spec:**
  - S13 collection options: "Bounded local ring buffer… first candidate".
  - The per-invocation coverage rules.
  - "Treat runtime strings as data."
- **Evidence:**
  - opentelemetry_sdk 0.32.1 defaults to `ParentBased(AlwaysOn)`
    (`trace/config.rs:33`; this crate is not in the lockfile). Any middleware
    that installs an extracted remote parent therefore lets the caller force
    sampling on or off.
  - The pilot already shows the right discipline: fixed event names, integers, a
    source digest, a `stale` flag, and `blocked` distinguished from `failed`
    (`runner.mjs:66-100`, `schema.mjs`).
- **Agent example:**
  1. The agent sends a request and gets a 500.
  2. It edits code, and `cargo run` restarts the server — as happens on nearly
     every edit in this loop.
  3. It asks what happened to the first request. An in-memory ring buffer has
     already lost it.
- **Smallest recommended correction:**
  1. **One terminal record per invocation.** The helper or the spawned task
     writes it with: operation, invocation, outcome (`success` /
     `rejected(code)` / `failed(effect)`), duration buckets and `producer{build,
     source_digest, boot_id}`. It has a reserved slot so checkpoint overflow
     can't evict it. It is not subject to trace sampling: a separate
     tracing-subscriber layer receives events regardless of what the OTel layer
     decides.
  2. **Simpler coverage.** Coverage becomes three facts: whether the terminal
     record is present, collector drops since boot, and `boot_id`.
     - Absent, with a newer `boot_id` → "the process ended before a terminal
       record".
     - Absent, with drops since boot → "possibly dropped".
     - Never interpret absence as "failed" or "rolled back".
     - Drop per-invocation completeness accounting.
  3. **On-disk dev evidence.** Store it as bounded, rotated JSONL under
     `target/`, as the runner already does for its reports.
  4. **A closed vocabulary enforced by the schema.** Only codes, checkpoint
     names, integers, IDs and enums; no free-text field anywhere. This rules out
     prompt injection and most disclosure by construction, which is stronger
     than "treat strings as data".
  5. **Durable facts before telemetry:** outbox columns (B-03) now, receipts
     later.
  6. **Keep inspection local-only** until there is an authorization model for
     it.
- **Alternatives:**
  - S13's ring buffer with three-dimension coverage: richer, but lost on every
    restart, and more machinery.
  - An OTLP backend: defer.
- **Tradeoffs:** a small file sink plus a subscriber layer to maintain. There is
  no free-text escape hatch, so rare diagnostics need a trusted local terminal
  (as the runner's README already says).
- **Validation:**
  - Kill the dev server mid-request and restart it. The inspector must report
    "ended before terminal record (boot changed)", never a fabricated outcome.
  - Put canary strings in inputs and assert that no evidence field can carry
    them.

### O55-A-02: Extractors should declare their own responses; re-run the utoipa/aide comparison with inference on

- **Kind:** alternative (reuse the ecosystem) addressing recurring drift.
- **Impact:** significant. Every handler repeats eight status declarations, and
  S10 lists duplicated exporter declarations as friction.
- **Confidence:** supported for aide 0.15.1's API (docs.rs) and for the current
  code. How aide merges responses with the same status is **unverified**.
- **Spec:**
  - S12: "The full operation also declares… 400… 401… 403… 500/503… Group
    alternatives per status."
  - S03: "use existing integrations before adding an Iris registry."
- **Evidence:**
  - `lib.rs:178-192` and `invitations.rs:42-55` repeat the same response tuples
    for utoipa, and `lib.rs:264-306` repeats them again for aide.
  - `lib.rs:263` sets `infer_responses(false)` "for an apples-to-apples
    comparison". That turns off aide's distinguishing feature:
    `OperationInput::inferred_early_responses` ("early returns for extractors…
    JSON parsing errors, authentication failures") and
    `OperationOutput::inferred_responses`.
  - utoipa 6.0.0 has `IntoResponses` for output types. I found no
    extractor-driven equivalent; this needs checking.
- **Authoring example:** a new handler takes `Actor`.
  - With inference on, one `impl OperationInput for Actor` documents 401 for
    exactly the handlers that take `Actor`.
  - Without it, the author has to remember the 401 tuple in every handler —
    twice per handler in today's dual-exporter code.
- **Smallest recommended correction:**
  - Make each convention a type that declares its own failure response: `Actor`
    → 401, a `Csrf` guard → 403 `csrf`, `WireJson<T>` → 400.
  - The handler return type `HttpResult<T, R>` declares the rejection branches
    through one shared bridge, plus 500/503 with `effect`.
  - Re-run the comparison with inference on, then pick one exporter, as S03
    already intends.
- **Alternative:** utoipa's `IntoResponses` for rejections, plus a global
  `Modify` pass that adds the middleware responses based on each operation's
  security requirements. That needs fewer custom traits, but the documentation
  is no longer tied to the extractors a handler actually uses.
- **Tradeoffs:**
  - aide needs custom trait impls, which is the reason decisions.md prefers
    utoipa.
  - S12's per-status grouping (403 `csrf` and 403 `forbidden` on one operation)
    needs a `oneOf` merged from two sources. That may need Iris code.
- **Validation:**
  - A contract test that every documented status has a declared producer, and
    every producer is documented.
  - An extractor without a documentation impl must fail compilation.

### O55-A-01: Close the enumeration gap with strum; drop the Iris derive; use `#[diagnostic::on_unimplemented]`

- **Kind:** simplification.
- **Impact:** significant. It removes a planned Iris macro and the only step in
  adding a rejection that the compiler doesn't check.
- **Confidence:** supported for the capabilities. The build cost and the
  agent-fluency effect are unmeasured.
- **Spec:**
  - S12: "Not compiler-proven: completeness of a handwritten `ALL` list… A
    derive could close the enumeration gap."
  - S12 says the derive's "concrete advantage is complete enumeration and
    missing-metadata diagnostics".
- **Evidence:**
  - strum 0.28.0 `VariantArray` derives `const VARIANTS: &'static [Self]` for
    enums with only unit variants.
  - For enums with payloads, derive it on the generated discriminant enum with
    `#[strum_discriminants(derive(VariantArray))]`.
  - strum 0.28.0 is already in `Cargo.lock`, via the `jsonschema`
    dev-dependency.
  - Missing metadata is already a compile error through S12's wildcard-free
    matches.
  - `#[diagnostic::on_unimplemented]` has been stable since Rust 1.78; the
    repository pins 1.98.1.
- **Authoring example:** adding `TargetSuspended`.
  - *Explicit matches plus `VariantArray`:*
    1. The compiler reports E0004 at the descriptor match and the status match,
       naming the variant.
    2. Enumeration, export and TypeScript regeneration then follow
       automatically; CI's drift check forces the regeneration.
  - *S12's explicit reference:* `ALL` silently omits the variant. OpenAPI and
    TypeScript then lack the branch, and today's React client shows the raw code
    (C-02).
  - *Either way:* only a behavioral test proves the action returns the variant
    in the right situation.
- **Smallest recommended correction:**
  - Derive `VariantArray` on rejection enums.
  - Put `on_unimplemented` messages on the Iris traits (`Rejection`,
    `HttpRejection`) that name the file to edit.
  - Unit-test code format and uniqueness over `VARIANTS`.
  - Snapshot all codes for compatibility review.
  - Move the narrow derive from "alternative" to "deferred".
- **Alternative:** S12's derive, which keeps metadata next to each variant and
  detects duplicate codes at compile time. Under my proposal, duplicates are
  caught at `cargo test` instead.
- **Tradeoffs:** agents already know strum and serde attribute idioms; an Iris
  attribute grammar would be new vocabulary. That is a hypothesis, not a
  measurement.
- **Validation:**
  - In a scratch branch, add one variant under each approach and record which
    steps fail at compile time, at test time, or never.
  - Measure how much adding strum costs a normal build.

### O55-C-02: Generated client types assume a closed world; add a runtime decoder with an explicit `Unknown`

- **Kind:** risk in current code; a correction for S12's open compatibility
  policy.
- **Impact:** significant for React and for clients in other languages. Limited
  while this is a demo.
- **Confidence:** supported.
- **Spec:** S12: "Old clients need a safe unknown-code path… Exact compatibility
  policy remains open."
- **Evidence:**
  - `web/src/api.ts:27-47` makes `errorTitle` exhaustive with a `never` check.
    At runtime, an unknown code falls through to `default` and is returned as
    the title.
  - `main.tsx:80,88` stores error bodies as `Problem`, and `main.tsx:184`
    renders them.
  - openapi-fetch does not validate response bodies.
  - A body that isn't a `Problem` at all — a proxy's HTML 502, an empty timeout
    response — is still typed as `Problem`, with `code` undefined.
- **Failure sequence:**
  1. The server ships `memberships.target_suspended`.
  2. An old React bundle receives it. According to the types, this cannot
     happen.
  3. The UI shows the raw code as a heading.
  4. An agent working from the generated types gets no signal that it has left
     the contract.
- **Smallest recommended correction:**
  - Generate `decode(operationId, status, body) → Known<Op> | Unknown{status,
    code?, correlation?}` from the per-operation literal-code schemas.
  - Policy for `Unknown`: show a generic message and the correlation handle, and
    infer nothing about retry or recovery.
  - Exhaustive switches cover `Known<Op>` only.
  - Add `discriminator.mapping` alongside the literal `const` codes, for
    generators in other languages.
- **Alternative:** a runtime `default` guard in every switch. Cheap, but every
  call site has to remember it.
- **Tradeoffs:** some runtime validation cost on error paths. Which
  OpenAPI-to-validator generator handles 3.1 `const` and `oneOf` correctly is
  unverified.
- **Validation:** client tests that feed an unknown code, an HTML 502 and an
  empty 408, and expect `Unknown` with no exception.

### O55-C-03: Rejection metadata and identifiers: one identity per fact

- **Kind:** simplification.
- **Impact:** limited to significant. Each extra identifier is another string an
  agent must keep in sync.
- **Confidence:** supported.
- **Spec:**
  - S07's example JSON shows `type: urn:iris:problem:memberships:last-owner`,
    `code: memberships.last_owner` and a public `rule`.
  - S12 adds `rule` and `prerequisite` to the descriptor, and warns "Do not
    export an internal rule merely because it exists."
  - S13 distinguishes request IDs from invocation IDs.
- **Evidence:**
  - RFC 9457 §3.1.1: "Consumers MUST use the "type" URI… as the problem type's
    primary identifier."
  - S07's example spells one identity two ways (colons and hyphens vs dots and
    underscores), so no mechanical mapping between them exists.
  - RFC 9457 §3.2 lets generic consumers ignore unrecognized extensions such as
    `code`.
- **Authoring example:** adding `target_suspended` asks an agent to invent up to
  six strings: a code, a URN, a rule, a prerequisite, a summary and a title.
  Only the *presence* of the code is checked by the compiler, and nothing checks
  them against each other.
- **Smallest recommended correction:**
  - `code` is the single identity. If RFC 9457 is adopted, `type` is
    `"urn:iris:problem:" + code` verbatim, with a test that the mapping is
    one-to-one.
  - `rule` is optional. Use it only when several codes share a rule, or to link
    a rule to acceptance tests owned by reviewers. That link is new and useful:
    it lets an agent tell enforced policy apart from a bug.
  - `prerequisite` must refer to an existing code or rule, checked by a catalog
    test.
  - Responses carry one public correlation handle. For single-action requests
    the invocation ID equals the request ID, which S13 already allows.
- **Recovery vocabulary:** keep `Unspecified` and `RequiresStateChange`.
  Consider adding `RequiresDifferentActor` for `forbidden`, the most common
  rejection: it tells the client another principal must act, and never implies
  self-escalation.
- **Alternative:** the current proposal, which offers more descriptive surface
  but more strings to keep in sync.
- **Validation:** a catalog test that `type` and `code` map one-to-one, that
  every `prerequisite` resolves, and that no public view includes a `rule`
  without explicit opt-in.
- **Open policy:** shared cross-cutting codes (`forbidden`, `not_found`) mean
  fewer client handlers. Per-domain codes (`memberships.forbidden`) are more
  precise.

### O55-B-03: Outbox: fence acknowledgments on the attempt counter only; use the lease to gate starting a send

- **Kind:** risk in the implemented experiment that S13 generalizes; evidence
  quality.
- **Impact:** limited. Duplicate mail is harmless because the token and
  Message-ID are reused. But one rare terminal state misleads evidence.
- **Confidence:** supported by reading the source; not executed.
- **Spec:**
  - S13 flow B describes an "attempt-and-lease fence".
  - S13 also says: "no need for an additional lease UUID: the existing job
    identity and claim counter supply the attempt fence."
  - `delivery.md`: "Completion requires the same attempt and a still-live
    lease."
- **Evidence:**
  - `outbox.rs:70` fences completion on `state='pending' AND attempts=? AND
    lease_until>?`.
  - `outbox.rs:24-27` marks rows whose lease expired at `attempts>=5` as dead,
    with a single combined reason, `expired_accepted_or_exhausted`.
  - `delivery.rs:72` discards the boolean that `complete` returns.
  - The acknowledgment write has a 100 ms busy timeout and no retry
    (`sqlite/lib.rs:18`).
- **Failure sequences:**
  1. *Late acknowledgment with no competing claim.*
     1. At t=100 a worker claims attempt 5, with the lease running to t=130.
     2. SMTP accepts at t=105.
     3. The host sleeps or the wall clock jumps, so `complete(Sent)` runs at
        t=131.
     4. It returns `false` even though no other worker claimed the row.
     5. The next cleanup marks the row dead with reason
        `expired_accepted_or_exhausted`.

     Durable state now says "exhausted" although the mail was accepted. If
     attempts were below 5, the next claim would re-send instead.
  2. *Acknowledgment write is busy.*
     1. An HTTP write transaction holds the lock for more than 100 ms.
     2. `complete` errors, and `tick` gives up.
     3. The row is re-claimed after the lease expires, and the mail is sent
        twice.
  3. *Conflated reason.*
     1. The recipient accepts the invitation using the link from an earlier
        duplicate email.
     2. Cleanup records the same reason it uses for exhaustion.
     3. An agent reads "dead" as a delivery failure, although the business
        outcome succeeded.
- **Smallest recommended correction:**
  - Acknowledge on `state='pending' AND attempts=?`. The attempt counter already
    rejects every newer claim; the lease condition adds no protection there.
  - Before starting an SMTP send, check that the remaining lease covers the send
    timeout plus a margin.
  - Retry the acknowledgment (not the send) within the lease, with a longer busy
    timeout for background writes.
  - Return `Applied` or `Superseded{current_attempt, state}` instead of a
    boolean.
  - Split terminal reasons into `invitation_accepted`, `invitation_expired`,
    `attempts_exhausted` and `permanent_failure`.
  - Persist `last_attempt_outcome` as `accepted`, `permanent_reject`,
    `transient` or `timeout_unknown`. These are durable facts that need no
    telemetry.
  - Add a row to S13's failure-window table: "lease expired but uncontested; a
    `false` result does not imply another worker took over."
- **Alternative:** keep the lease condition as a "live lease holder only"
  discipline and accept the extra duplicates and mislabels.
- **Tradeoffs:** little code. On multi-host PostgreSQL, compare leases against
  the database clock, not each worker's `unix_time()`.
- **Validation:** extend `snapshot_backoff_crash_recovery_and_stale_ack` with a
  claim at t=100, an acknowledgment at t=131 and no competing worker, and assert
  the row ends as `sent`. Keep the existing assertion that attempt 1 cannot
  acknowledge attempt 2 — it shows the counter alone is enough to fence.

### O55-B-04: Last-owner rule: correct on SQLite; make the serialization point explicit and independent of path

- **Kind:** alternative / open question (portability across engines).
- **Impact:** limited now. Significant if PostgreSQL uses the same algorithm.
- **Confidence:**
  - SQLite: supported. After `BEGIN IMMEDIATE` there is a single writer.
  - PostgreSQL: conditional. This is standard READ COMMITTED behavior, and
    nothing is implemented yet.
- **Spec:** S05 describes the algorithm and adds: "Other database engines
  require their own locking/isolation design."
- **Failure sequence (PostgreSQL, READ COMMITTED, same algorithm):**
  1. The owners are A and B.
  2. Transaction T1 (A demotes themself) and T2 (B demotes themself) each count
     2 owners.
  3. Each updates a different row, so there is no row conflict.
  4. Both commit, and the project has zero owners. This is a write-skew anomaly.
- **Smallest recommended correction:**
  - An "aggregate lock" convention: the first statement of every
    membership-mutating transaction locks the project row. That is `FOR UPDATE`
    on PostgreSQL, and implied by `BEGIN IMMEDIATE` on SQLite.
  - Keep one private function as the only writer of `memberships.role`; the spec
    already plans a shared private mutation.
  - When a transaction touches several aggregates, lock them in ID order.
  - Optionally add a database trigger that raises if no owner would remain. It
    maps to an execution failure, i.e. a bug. It also protects paths the action
    never sees: a future admin CLI, cascading account deletion, one-off data
    fixes.
- **Alternative:** SERIALIZABLE isolation with retry on serialization failures.
  That needs the retry semantics the spec defers.
- **Validation:** a two-task barrier test, run on every supported engine and for
  every operation that can reduce owners. It should assert at least one owner
  remains, not a particular error ordering (as S09 says).

### O55-A-04: Carry invocation context with the unit of work, not as a fourth parameter

- **Kind:** simplification of S13's context.
- **Impact:** limited to significant; it affects every action signature.
- **Confidence:** conditional. It needs an authoring spike.
- **Spec:**
  - S13: `change_role(conn, actor, input, &execution)`.
  - S13: "Do not put database handles… in this context."
  - S13: "Whether every public action eventually needs it remains open."
- **Reasoning:** after B-01 and B-02, the context's consumers are the
  transaction helper and the spawned task. The helper needs the deadline to
  budget lock waits, the cancellation check before `BEGIN`, and the invocation
  ID for the terminal record. Action bodies rarely need any of it.
- **Smallest recommended correction:**
  - The trusted boundary creates `Invocation { id, deadline, cancel, conn }`.
    This is a unit-of-work handle, not a service container.
  - Actions take `&mut Invocation` where they now take `&mut SqliteConnection`.
  - Inner functions that take `&mut Tx` inherit the context automatically.
  - `Actor` stays a separate parameter.
- **Alternative:** S13's explicit `&ExecutionContext` parameter, which makes
  propagation more visible at the cost of an extra parameter on every action.
- **Tradeoffs:**
  - This inverts S13's "no database handle in the context" rule. The
    justification is that nothing else travels with it.
  - It ties invocation identity to database-backed actions. That is acceptable
    because Iris actions are database-centric and external effects go through
    the outbox.
- **Validation:** write `change_role`, `remove_member` and an atomic
  `transfer_ownership` both ways. Compare signatures and test setup, and check
  whether evidence gets IDs without threading them by hand.

### O55-B-05: Deferred effects: keep the distinction; make it enforceable and list the eligibility policy

- **Kind:** alternative plus open policy questions.
- **Impact:** limited.
- **Confidence:** supported, from the claim SQL.
- **Spec:** S05 and S13's authority policy for delivering committed effects.
- **Evidence:** `outbox.rs:24-33` treats a row as eligible when it is pending,
  unexpired, unaccepted and under five attempts. There is no check on the
  recipient's status, the project's status or the issuer. That matches the
  stated policy.
- **Points:**
  - **Enforce the split.** Delivering committed intent vs executing a new
    command is sound. Enforce it with a rule: a delivery handler may perform
    only its external effect and its own bookkeeping. Any other domain mutation
    is a new command, with explicit `System(reason)` authority or a fresh
    authorization of the initiating actor. Give the worker a capability that
    exposes only `complete`: no `Actor`, no general connection.
  - **Declare eligibility.** Eligibility is product policy. Declare it per
    effect type and evaluate it in the claim transaction. Open questions:
    - a recipient who is disabled or deleted;
    - a recipient who already became a member another way (they still get the
      invitation today);
    - an archived project;
    - an issuer removed after an account compromise.

    If owners expect removing an issuer to cancel their pending invitations,
    provide an explicit "revoke pending invitations" action rather than
    rechecking the issuer's authority at delivery.
  - **Document the in-flight bound.** An eligibility change cannot stop a send
    already in progress. The lag is at most the lease plus the send timeout,
    about 30 seconds here.
- **Alternative:** re-check the initiating user's current authority at delivery
  time. The spec rightly rejects this; I don't recommend it.
- **Validation:** for each eligibility predicate, a claim test that changes the
  condition between enqueue and claim and asserts the row is suppressed.

### O55-C-05: Discovery drift already exists in the MCP pilot; derive every enum from one catalog

- **Kind:** contradiction with decisions.md: "CLI and MCP use the same check
  catalog."
- **Impact:** limited, since this is a pilot, but it is exactly the drift class
  S10 reports.
- **Confidence:** supported.
- **Evidence:**
  - `server.mjs:26-30` hardcodes the enums for topic, profile, scenario and
    check.
  - `cli.mjs:10-11` derives the same values from `profiles` and `checks`, and
    `schema.mjs:4` derives `checkId`.
  - As a result, the CLI accepts `reproduce compile`, while MCP's
    `reproduce_scenario` accepts only `auth` or `outbox`.
  - S10 records that the protocol test "caught missing MCP enums after adding a
    CLI scenario".
- **Smallest recommended correction:** derive the enums, e.g.
  `z.enum(Object.keys(conventions))`. For application operations, make the
  exported OpenAPI document the single catalog for clients, docs and MCP
  discovery, with `x-iris-*` extensions for recovery metadata and
  `after_unknown`. Public and privileged views are filters of that one document,
  never separate lists.
- **Alternative:** keep hand-written enums and rely on the protocol test to
  catch drift. That is the current state; S10 shows it does catch drift, but
  only after the fact.
- **Validation:** add a check to the catalog and assert that the CLI and MCP
  accept the same set of values with no other edits.

### O55-A-03: Typed IDs; one rejection enum per distinct rejection set

- **Kind:** simplification and risk reduction.
- **Impact:** limited.
- **Confidence:** supported.
- **Spec:** S04/S12 define `ChangeRole { project_id: i64, user_id: i64, … }` and
  a separate `ChangeRoleRejection` per action.
- **Evidence:** `invitations.rs:62-67` converts two wire strings to `i64`
  through a local helper (`invitations.rs:34-40`). Swapping the two IDs still
  compiles.
- **Smallest recommended correction:**
  - Use `ProjectId` and `UserId` newtypes in the domain.
  - Use one `WireId` type that does canonical parsing, replacing per-module
    helpers.
  - When `change_role` and `remove_member` have the same rejection set, use one
    `MembershipRejection` enum, so the shared private mutation needs no mapping.
    Split it only when the sets diverge, because per-operation OpenAPI precision
    depends on it.
- **Alternative:** raw `i64` IDs and per-action rejection enums, as proposed.
  Simpler types, but swapped IDs still compile, and the shared private mutation
  needs enum mapping.
- **Validation:** a compile-fail test showing that swapped IDs no longer
  compile.
- **Where further simplification would erase guarantees:**
  - Keep wire DTOs separate from commands; that boundary does the
    string-to-`i64` conversion and validation.
  - Keep HTTP status out of the domain. Jobs and CLI then stay HTTP-free, and
    disclosure rules can differ by transport.

---

## 4. Preferred design and unresolved choices

### 4.1 Preferred design

Keep S04 and S05's shape. Add two small runtime pieces, derive the static
contracts from existing crates, use OpenAPI as the single catalog, and make
evidence durable-first.

1. **Actions.**
   - Signature: `async fn op(inv: &mut Invocation, actor: &Actor, cmd: Cmd) ->
     ActionResult<Out, Rej>`.
   - Private functions taking `&mut Tx` let actions compose inside one
     transaction.
   - Typed IDs, and one rejection enum per distinct rejection set.
2. **Transaction helper (Iris).**
   - The only code allowed to begin or commit.
   - Rejection means rollback, so nothing is applied.
   - Returns a typed `effect` and emits the terminal record.
   - Receipts can be added to it later.
3. **Execution ownership.**
   - Mutations run to completion on tracked server tasks.
   - Cancellation is checked only before `BEGIN`.
4. **Static contracts.**
   - Explicit descriptor and status matches.
   - `strum::VariantArray` for enumeration.
   - One generic bridge that both renders responses and documents them.
   - Extractors document their own early responses.
5. **Wire format.**
   - Error bodies are `{code, message, correlation, effect?}`.
   - RFC 9457 is optional; if adopted, `type` is derived from `code`.
   - Each operation has its own literal-code branches.
   - A generated client decoder has an `Unknown` branch.
6. **Catalog.**
   - OpenAPI plus `x-iris-*` extensions (recovery metadata, `after_unknown`).
   - MCP discovery reads a filtered view of it; there are no hand-written lists.
7. **Outbox.**
   - Acknowledgment is fenced on the attempt counter only; the lease gates
     starting a send.
   - The acknowledgment is retried within the lease.
   - Separate terminal reasons, plus a durable `last_attempt_outcome`.
   - Handlers may perform only their effect and their own bookkeeping.
8. **Evidence.**
   - Durable facts first.
   - One terminal record per invocation: never sampled, with a reserved slot,
     carrying build, digest and `boot_id`.
   - A closed vocabulary; on-disk storage in development; local-only inspection.

I checked how these pieces interact:

- **Server-owned execution + lossy telemetry.** The server learns more, and
  nothing new is promised to the client.
- **Receipts.** They live inside the mutation's transaction, so they don't
  depend on telemetry.
- **Rollback by default + audit of denied attempts.** Needs an explicit escape
  hatch, reported as a distinct terminal state.
- **A resend while the original is still running on its server task.** It waits
  for the original at the write lock.

### 4.2 Authoring sketch: `change_role`

APIs marked `(I)` are invented. Everything else is std, sqlx, strum or
axum/aide.

```rust
// src/domains/memberships.rs: command, outcomes, authorization, SQL
use iris::db::{Invocation, Tx};                 // (I) unit-of-work handle + transaction guard
use iris::action::{ActionResult, reject};       // (I) Result<T, ActionError<R>>; reject(r) = Err(Rejected(r))
use iris::rejection::{Rejection, Descriptor};   // (I) explicit trait impl; no derive

pub struct ChangeRole { pub project: ProjectId, pub user: UserId, pub role: MemberRole }
pub struct RoleSet { pub role: Option<MemberRole>, pub changed: bool } // read inside the tx, not echoed

#[derive(Debug, Clone, Copy, strum::VariantArray)]   // existing crate: complete enumeration
pub enum MembershipRejection { Forbidden, MemberNotFound, LastOwner } // shared with remove_member

impl Rejection for MembershipRejection {
    fn descriptor(&self) -> Descriptor {
        match self {                                  // no wildcard: a new variant is E0004 here
            Self::Forbidden => Descriptor::code("memberships.forbidden").requires_different_actor(),
            Self::MemberNotFound => Descriptor::code("memberships.member_not_found"),
            Self::LastOwner => Descriptor::code("memberships.last_owner")
                .requires_state_change("memberships.another_owner_required"),
        }
    }
}

pub async fn change_role(inv: &mut Invocation, actor: &Actor, cmd: ChangeRole)
    -> ActionResult<RoleSet, MembershipRejection>
{
    let mut tx = inv.begin_immediate().await?;   // (I) BUSY here => Failed { effect: NotStarted }
    let result = set_role(&mut tx, actor, cmd.project, cmd.user, Some(cmd.role)).await;
    tx.finish(result).await                       // (I) Ok => COMMIT (errors classified);
}                                                 //     Err => ROLLBACK, so a rejection applied nothing

// The only writer of memberships.role; remove_member and transfer_ownership reuse it.
pub(crate) async fn set_role(tx: &mut Tx<'_>, actor: &Actor, project: ProjectId,
    user: UserId, role: Option<MemberRole>) -> ActionResult<RoleSet, MembershipRejection>
{
    lock_project(tx, project).await?;             // aggregate lock (FOR UPDATE on PostgreSQL)
    if !is_owner(tx, project, actor.user()).await? {
        return reject(MembershipRejection::Forbidden);        // before any target lookup
    }
    let Some(current) = member_role(tx, project, user).await? else {
        return reject(MembershipRejection::MemberNotFound);
    };
    let demotes_owner = current == MemberRole::Owner && role != Some(MemberRole::Owner);
    if demotes_owner && owner_count(tx, project).await? == 1 {
        return reject(MembershipRejection::LastOwner);
    }
    write_role(tx, project, user, role).await?;   // ordinary sqlx
    Ok(RoleSet { role, changed: Some(current) != role })
}
```

```rust
// src/http/memberships.rs: wire DTOs, conversion, status mapping, route
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ChangeRoleBody { project_id: WireId, user_id: WireId, role: WireRole } // (I) WireId

#[derive(Serialize, JsonSchema)]
struct RoleSetBody { role: Option<WireRole>, changed: bool }

impl HttpRejection for MembershipRejection {      // (I) trait carrying #[diagnostic::on_unimplemented]
    fn status(&self) -> StatusCode {
        match self {
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::MemberNotFound => StatusCode::NOT_FOUND,
            Self::LastOwner => StatusCode::CONFLICT,
        }
    }
}

async fn change_role(
    State(app): State<App>,
    actor: Actor,                                  // documents 401 itself
    _csrf: Csrf,                                   // (I) documents 403 `csrf`
    WireJson(body): WireJson<ChangeRoleBody>,      // (I) documents 400
) -> HttpResult<RoleSetBody, MembershipRejection> {// (I) renders + documents branches; 500/503 carry `effect`
    let cmd = ChangeRole {
        project: body.project_id.parse()?, user: body.user_id.parse()?, role: body.role.into(),
    };
    let out = app.run_to_completion(async move |inv| {       // (I) spawned + TaskTracker
        memberships::change_role(inv, &actor, cmd).await
    }).await?;
    Ok(RoleSetBody { role: out.role.map(Into::into), changed: out.changed })
}

pub fn routes() -> ApiRouter<App> {
    ApiRouter::new().api_route("/api/memberships/role", post_with(change_role, |op| {
        op.id("changeMembershipRole").after_unknown(AfterUnknown::ResendIsEffectSafe) // (I)
    }))
}
```

```ts
// React: generated decoder (I); exhaustive only over this operation's known codes
const res = decode("changeMembershipRole", await api.POST("/api/memberships/role", { body }));
switch (res.kind) {
  case "ok":       notify(res.data.changed ? "Role updated" : "Already had that role"); break;
  case "rejected": /* exhaustive over forbidden | member_not_found | last_owner */ break;
  case "failed":   res.effect === "not_committed" ? offerRetry() : reconcile(); break;
  case "unknown":  showGeneric(res.correlation); break; // no recovery inference
}
```

### 4.3 What an author must understand

| Concern                     | S12/S13 proposal                                     | Preferred                              |
| --------------------------- | ---------------------------------------------------- | -------------------------------------- |
| Begin/commit/rollback       | A `match` in every action (already duplicated twice) | `begin_immediate` / `finish`           |
| Effect classification       | `is_busy` in the adapter, plus checkpoints           | Typed `effect` from the helper         |
| Invocation context          | Fourth parameter `&ExecutionContext`                 | Carried by `Invocation`                |
| Rejection enumeration       | Handwritten `ALL`                                    | `strum::VariantArray`                  |
| Descriptor / HTTP status    | Explicit matches                                     | Explicit matches (unchanged)           |
| 400/401/403/500/503 docs    | Listed per handler (per exporter)                    | Declared by extractor and return types |
| Rendering and export bridge | Future integration                                   | One generic Iris implementation        |
| Execution on disconnect     | Handler future (dropped)                             | `run_to_completion`                    |
| Retry after unknown         | Unspecified                                          | One declaration per operation          |
| Client errors               | Closed generated union                               | Generated decoder with `Unknown`       |
| MCP discovery               | Separate lists                                       | Filtered OpenAPI with `x-iris-*`       |

The feature's files stay the same: a domain file, an HTTP file, one registration
line, and the React call site.

**One simplification to adopt:** the transaction helper. It replaces three
things — the per-action commit/rollback block, the context parameter and
classification in the adapter — while *strengthening* guarantees: rejection now
implies rollback, and the effect is classified by the actual stage.

**Simplifications to avoid:**

- Putting HTTP status on domain variants. It couples jobs and CLI to HTTP and
  loses per-transport disclosure rules.
- Merging wire DTOs with commands. It loses validation at the string-to-`i64`
  boundary.

### 4.4 Ecosystem reuse and the Iris code that remains

| Capability                        | Existing crate or tool                                                                           | Iris-owned remainder                                     |
| --------------------------------- | ------------------------------------------------------------------------------------------------ | -------------------------------------------------------- |
| Rejection enumeration             | strum `VariantArray` / `EnumDiscriminants`                                                       | None                                                     |
| Per-status OpenAPI responses      | aide `OperationInput`/`OperationOutput` (or utoipa `IntoResponses`)                              | One generic bridge; merging same-status `oneOf` branches |
| Response validation in tests      | `jsonschema` (already used)                                                                      | None                                                     |
| TypeScript types and decoding     | openapi-typescript (used); a validator generator (to be selected)                                | A decode wrapper                                         |
| Request IDs                       | tower-http `SetRequestId` / `PropagateRequestId`                                                 | None                                                     |
| Server-owned execution and drain  | tokio spawn + tokio-util `TaskTracker`                                                           | `run_to_completion` (small)                              |
| Transaction effect classification | None: this is engine semantics                                                                   | Helper + per-engine rules + fault tests                  |
| Tracing and export                | tracing, tracing-subscriber, tracing-opentelemetry                                               | Evidence layer + JSONL sink                              |
| Convention diagnostics            | rustc `#[diagnostic::on_unimplemented]`                                                          | Attributes on Iris traits                                |
| Outbox                            | Unverified whether any maintained queue crate can enqueue inside the caller's SQLite transaction | Keep the ~75-line outbox if none can                     |

### 4.5 An agent interpreting outcomes

This is analysis, not an executed experiment.

**Sequence 1: lost response on self-demotion; receipts not yet implemented.**

- *Available:*
  - The client-side record `{origin: client, failure: response_not_received,
    outcome: unknown}`.
  - The operation's `after_unknown: resend_is_effect_safe`.
  - A read operation for membership. It is hypothetical — no such operation is
    in this snapshot.
- *Justified interpretation:* the demotion may or may not have committed. Any
  later `memberships.forbidden` may be *caused* by it.
- *Next safe action:* read Alice's membership.
  - If she is `editor`, report "the requested state holds; this request's effect
    is unconfirmed", and stop.
  - If she is still `owner`, resend once and interpret the new result normally.
- *Still missing:* proof of causation; changes others made in the meantime;
  whether Alice still wants the change (human intent).

**Sequence 2: last-owner rejection.**

- *Available:*
  - A 409 response with code `memberships.last_owner`.
  - Recovery metadata
    `requires_state_change(memberships.another_owner_required)`.
  - The rule that a rejection means rollback, by construction.
- *Interpretation:* a correctly enforced business rule, not a bug and not a
  transient failure. Nothing changed.
- *Next safe action:*
  - Explain the rule to the human.
  - Ask whether to promote a specific member. That is a new command under the
    human's current authority.
  - Do not resend unchanged, and do not modify policy, tests or roles on your
    own.
- *Still missing:* who should become owner, and whether the requester is allowed
  to promote anyone. That is only learned by making an authorized attempt.

**Sequence 3: infrastructure failures.**

- *Available:* a 503 or a 500 response carrying the caller's `effect`.
- *Interpretation and next action:*
  - `503` with `effect: not_committed`: resending with bounded backoff cannot
    duplicate this attempt. Permissions and invariants are rechecked on the
    retry. It is not a promise of success.
  - `500` with `effect: unknown`: handle like Sequence 1 and reconcile before
    resending.
- *Still missing:* the root cause (privileged evidence), and whether a retry
  will succeed.

### 4.6 Owner decisions

1. Server-owned execution as the default for mutations (B-02).
2. Rollback on rejection as the default, with an explicit escape hatch for
   record-then-reject (B-01).
3. A public `effect` field on failure bodies (B-01). This is a disclosure-policy
   choice.
4. The exporter: aide with inference vs utoipa with a `Modify` pass, after
   re-running the comparison (A-02).
5. The code namespace (cross-cutting vs per-domain), and RFC 9457 adoption with
   `type` derived from `code` (C-03).
6. `after_unknown` declarations now. Receipts when the first operation that
   creates a new identity appears, with replay authorized by actor identity
   (C-01).
7. Delivery eligibility policy (B-05).
8. Optionally, a self-targeted no-op that runs before authorization (C-01).

### 4.7 Explicitly deferred

- The narrow Iris derive.
- An OTLP collector or backend.
- Runtime and production inspection, and the authorization it would need.
- Per-invocation completeness accounting.
- A durable job executor.
- Automatic retries.
- A policy engine.
- PostgreSQL support — but record the aggregate-lock convention now.

### 4.8 Evidence that would change these recommendations

- **A-04:** the authoring spike shows that carrying context in `Invocation`
  complicates tests or composition. Then keep S13's explicit parameter.
- **A-02:** aide cannot merge same-status branches, or its trait burden exceeds
  per-handler lists across three or more domains. Then use utoipa with `Modify`.
- **A-01:** the measured build cost of strum is significant. Then use a
  handwritten `ALL` plus independent fixtures.
- **B-02:** server-owned execution measurably hurts load shedding, or the
  product wants cancel-on-disconnect. Then keep drop semantics and document "no
  terminal record" as routine.
- **B-03:** there is a real need for acknowledgments only from a live lease
  holder, such as an audit rule. Then keep the lease condition, but return
  `LeaseExpired` distinctly from `Superseded`.
- **C-01:** agents misuse `after_unknown`, or the declarations are often wrong.
  Then keep only the read-before-interpreting convention.
- **B-01:** most real actions turn out to need record-then-reject. Then
  reconsider rollback as the default.
