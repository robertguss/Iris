# Handoff

Written October 3, 2026, by the outgoing driver (Claude) at the end of its
session. The owner has decided that Iris switches from the `driver` skill to the
new `crew` skill (an Opus driver, a Sonnet builder and a Fable oracle), and that
invitations stage 2 is not built in this session. This replaces the earlier
October 3 handoff, which git history keeps at `763d61e:HANDOFF.md`.

## 1. State

Observed October 3, 2026, by the outgoing driver.

- Repository: `/Users/robertguss/Projects/startups/Iris` (GitHub
  `robertguss/Iris`).
- When this was written, the checkout was on branch
  `driver-handoff-2026-10-03b`, created from `main` at `763d61e`, with this
  `HANDOFF.md` uncommitted. `origin/main` was at `763d61e`, the merge of PR #18
  (ROB-1236).
- Reviewed through `763d61e`.
- Pushed through `763d61e`. Still pending at the time of writing: this handoff's
  commit, the push of its branch, its PR (with the PR text reviewed before
  posting), and its merge. Check `gh pr list --state all --limit 3` for the
  outcome rather than assuming it.
- CI is disabled: `gh workflow list --all` shows `Verify disabled_manually`
  (ROB-1229). No push or PR runs any check. The last CI run on `main` is
  37124637619 at `a6e9586`, and the later merges (PRs #17 and #18) have no run.
- Apart from this handoff, the working tree was clean, and no pull request was
  open.

Re-check HEAD, the working tree and the remote (`git status`, `git log -5`,
`git ls-remote origin`, `gh workflow list --all`) before relying on any of this.

## 2. Queue

Linear team `ROB`, project `Iris`. Snapshot October 3, 2026:

- Nothing is in `Ready`, `Planning`, `Building`, `In Review` or `Needs Input`.
- No split issue has sub-issues left.
- `Backlog`:
  - **ROB-1123**, CSP-safe runtime validators. It is parked until the owner
    chooses a restrictive CSP.
  - **ROB-1227**, intermittent agent-interface MCP focused-check failures in CI.
    With CI disabled it blocks nothing. The driver's comment recommends
    cancelling it, or parking it until CI returns, and the owner has not
    decided.

### Proposed next work: invitations stage 2 (not in Linear yet)

Stage 2 is the second of S19's "Bounded later implementation candidates":
private delivery and lifecycle. The owner deferred it on October 3 and has not
written or released it, so a new driver proposes it to the owner and does not
start it. This is everything known about it.

**Where stage 1 left off (ROB-1236, PR #18):**

- Migration `apps/reference/migrations/0002_invitations.sql` created:
  - `invitations`: `id`, `project_id`, `recipient_id`, `issuer_id`, `role`
    (always `editor`), `token_hash` (unique lowercase hex SHA-256),
    `created_at`, `expires_at` (`created_at + 3600`), and `accepted_at`, which
    is null until acceptance.
  - `invitation_outbox`: `id`, `invitation_id` (unique), `recipient_email`
    (nullable snapshot), `token` (nullable plaintext credential), `message_id`
    (required, unique, `<32 hex@reference.iris.test>`), and `created_at`.
  - The outbox has no claim, lease, attempt or status column. Stage 2 adds them
    in a new append-only migration (0003); 0002 is never edited.
- `apps/reference/src/domains/invitations.rs` holds `issue` and `accept`, each
  in one `BEGIN IMMEDIATE` transaction, with a generic `ActionError<R>` local to
  that module, plus the `finalize` and `unconfirmed` helpers.
  `domains/memberships.rs` was deliberately left unchanged. Tests are in
  `domains/invitations/tests.rs`.
- Fresh initialization and reset seed `alice@example.test` and
  `bob@example.test` (`storage::seed_development`). `app::seed`, the shared test
  fixture, has no contacts. Existing databases are never backfilled.
- `apps/reference/scripts/probes.mjs:92` pins the reference test count, now 132.
  Every stage that adds or removes reference tests must update it.

**S19 requirements for stage 2** (`docs/design-spec.md` S19, "Outbox, local
capture and credential lifetime" and "Worker claims, fencing and lifecycle";
read them directly):

- **Claims.** Claim in a short write transaction, then send SMTP outside any
  database transaction.
  - Budget: five claims, not five transmissions. A claim interrupted before the
    send still counts.
  - Leases last 30 s.
  - Backoff after claims 1 to 4: 5, 10, 20 and 40 s. An exhausted job becomes
    terminal, and so does a malformed payload or a permanent SMTP failure.
  - An expired or accepted invitation prevents future claims, but not a send
    already active.
- **Completion and errors.** Completion is fenced by the claimed attempt and a
  still-live lease.
  - A returned `false` means only "no transition by this call". It is not
    evidence of another worker or of an SMTP failure.
  - A database error leaves acknowledgment unknown. Never reinterpret it as
    `false`.
  - Database errors get bounded diagnostics and wait for the next normal tick,
    with no unbounded retry.
- **Clearing.** Clear the payload (the credential and the address snapshot) on
  terminal completion, or in an eligibility sweep for expired, accepted or
  exhausted jobs once no live lease prevents it. A successful, fenced terminal
  completion clears its own payload, but a sweep must never clear the payload of
  another send that is still active. Clearing columns is not secure erasure.
- **Lifecycle.** Use the existing tracked connections (`lifecycle::Connections`)
  and task supervision, as the S18 session-cleanup task does in `lifecycle.rs`.
  - After an observed shutdown, start no new claim. If a claim admitted before
    the stop observes shutdown before SMTP begins, it must not begin SMTP, and
    the claim is not refunded.
  - Keep S18's bounds: `lifecycle::DRAIN` (3 s), `lifecycle::CLOSE` (1 s), and
    the supervisor's outer 5 s kill. Neither a send timeout nor a lease extends
    shutdown.
  - An unexpected worker exit or panic stops the process through existing
    supervision, with no automatic restart.
  - If draining or connection closure is not established, the process terminates
    with the ownership lock held until it exits.
- **Mail.** Deliver only to a dedicated, loopback-bound local `.test` mail
  capture, with no relay or real SMTP configuration.
  - Pin and verify the capture's version, configuration and retention in this
    stage. S19 retains the frozen experiment's 24-hour age limit as a
    requirement, and only proposes a 500-message cap.
  - The frozen experiment (`experiments/api-slice/delivery.md`) used lettre's
    SMTP transport and Mailpit 1.31.2, installed by
    `experiments/api-slice/checks/install-mailpit.sh`. That is precedent, not
    evidence. Never reuse a shared historical inbox as proof of isolation.
  - Adding a mail dependency and a capture service needs the plan to justify it.
- **Continuity.** Retries reuse the same credential, contact snapshot and
  Message-ID, and never extend expiry. An issuer's later loss of authority
  neither revokes the invitation nor suppresses its delivery. An interrupted
  send stays uncertain, and restart or lease recovery can duplicate an SMTP
  acceptance; neither supplies a receipt.
- **Diagnostics.** Bounded IDs, attempt numbers, stages and result categories
  only. Never log a token, hash, Message-ID, address, message body or raw SMTP
  error.
- **Tests.** Cover S19's failure-window table. Distinguish a stale `false` from
  a completion database error, and a claim from a send. Verify duplicates and
  interruptions without claiming delivery.

**Risks to raise in planning:**

- It is timing-sensitive: leases, backoff and the shutdown windows. Use an
  injectable clock, like the `now` parameter in `domains::invitations`.
- It brings a new runtime dependency (an SMTP client) and an external capture
  process. Tests that start capture must use fixed or owned ports without
  overlapping other fixed-port suites (4001, 3003, 5175), and must follow the
  storage-exclusive guard for child spawns.
- It changes lifecycle code that S18 verified carefully.

Consider splitting it: the schema, claims and fencing first, then SMTP with
capture, then shutdown integration.

**Stage 3, after stage 2:** both public operations and the complete client in
one green candidate: HTTP declarations, the OpenAPI export, generated
TypeScript, decoder, type and recovery tests, presentation, views and browser
coverage. See S19 for the wire contract and browser credential handling.

## 3. Read these first

- `AGENTS.md`, which still describes the `driver` skill and a Pi worker (see
  section 6); its "Scope and safety" gates still apply.
- `docs/design-spec.md` S19, all of it.
- `docs/decisions.md`, its last three entries: "CI disabled — October 3, 2026"
  (ROB-1229), "Invitation persistence and domain rules — October 3, 2026"
  (ROB-1236), and the ROB-1119 output-tails entry before them.
- The Linear completion comments on ROB-1119, ROB-1120, ROB-1229 and ROB-1236.
  Each records its brief, every oracle finding with its disposition, and its
  verification.
- `apps/reference/src/domains/invitations.rs` and `lifecycle.rs`.

## 4. Context

- **Workflow history.** October 2: an Amp workflow merged PRs #2 to #13. October
  2–3: the `driver` loop, with a Codex oracle and a Pi worker, merged ROB-1119
  (#14), ROB-1120 (#15), the previous handoff (#16), ROB-1229 (#17) and ROB-1236
  (#18). Partway through ROB-1229 the owner told the driver to do all the work
  itself; Pi built ROB-1229's first version, and the driver built ROB-1236. Now
  the switch to `crew`.
- **Why CI is off.** The owner asked on October 3 to disable CI entirely.
  ROB-1229 replaced the merge gate in `AGENTS.md`: a merge now follows the
  oracle's sign-off and the driver's own rerun, on the final tree, of the
  brief's verify commands. All verification is local on macOS. Linux coverage
  and the frozen experiments' automatic runs are gone.
- **Why stage 1 is private.** S19 forbids a public partial feature. The routes
  and client arrive together in stage 3.

## 5. This chunk

| Issue     | Commit    | PR and merge         | Built by        | Notes                                            |
| --------- | --------- | -------------------- | --------------- | ------------------------------------------------ |
| ROB-1119  | `fccd32e` | #14, merge `bdf5ba7` | Pi              | Output tails; CI was green on its second attempt |
| ROB-1120  | `0034860` | #15, merge `e4ba261` | Pi              | Documentation reconciliation; CI green           |
| (handoff) | `21fb0c2` | #16, merge `a6e9586` | driver          | The previous handoff                             |
| ROB-1229  | `9cdf817` | #17, merge `5481928` | Pi, then driver | Documentation only; no CI (disabled)             |
| ROB-1236  | `80b1c50` | #18, merge `763d61e` | driver          | Stage 1; no CI (disabled)                        |

What the oracle checked independently, and what is only driver-reported:

- **ROB-1229.** The oracle reran Prettier and `git diff --check` and confirmed
  the workflow state. The 249-link check is driver-reported.
- **ROB-1236.**
  - The oracle checked these itself:
    - all 19 invitation tests (rerun independently)
    - `cargo fmt`, Prettier and `git diff --check`
    - the mutation script and its logs
    - the `Cargo.lock` change
  - Driver-reported only, with logs the oracle inspected:
    - the workspace suite (248 passed)
    - clippy
    - the dev-identity tests
    - reference web verify
    - the probes (31 caught, 25 controls)
    - the mutation runs
- **ROB-1119 and ROB-1120.** Unchanged from the previous handoff, and recorded
  in their Linear comments.

## 6. Decisions and authorizations in force

- **Owner, October 3:**
  - Iris switches to the `crew` skill, with an Opus driver, a Sonnet builder and
    a Fable oracle.
  - Stage 2 is not built in this session.
  - The chunk ends with end-of-chunk steps 1–3 only (handoff, handoff review,
    commit). The existing oracle pane is left running, and the outgoing driver
    starts no new driver.
  - `AGENTS.md`'s `## Driver` section (the `driver` skill, `Worker: pi`) does
    not yet reflect the switch, and updating it was not part of this chunk.
- **Owner, October 3:** CI is disabled. Re-enabling it
  (`gh workflow enable Verify`) is a CI change that needs the owner's approval.
- **Owner, October 3:** "you are in control of linear." The driver creates,
  releases and closes issues. Starting new product work, such as stage 2, still
  waits for the owner's go-ahead.
- **Owner, October 2, for that backlog run:** each issue is delivered on a
  dedicated branch, through a PR, and merged after the oracle's sign-off and the
  current merge gate. `AGENTS.md` still requires authorization for issue-branch
  pushes, so a new session confirms delivery authorization with the owner rather
  than assuming it carries over.
- **Workflow, from earlier chunks and still in force:**
  - oracle review before every commit
  - PR text reviewed before posting
  - ask the owner one question per message

  This session's lapses are recorded in its earlier handoff
  (`763d61e:HANDOFF.md` section 6). After them, PR text was reviewed before
  posting for every PR.

- **Still excluded:** edits to the frozen Turso guide and to the PR #1
  description. Each needs separate owner approval.

## 7. Operational state

- **Agents.** The oracle (Codex) is still running in this tab's right pane, as
  the owner asked. No worker pane is open: Pi was closed after ROB-1229. No
  other process belongs to this session.
- **Ports.** 4001, 3003 and 5175 were free after the last suites.
- **Evidence.** In this session's scratch directory
  (`/private/tmp/claude-501/-Users-robertguss-Projects-startups-Iris/361b7984-181f-491b-b2e2-0162b0f2be55/scratchpad/`):
  `evidence-1119/`, `evidence-1236/`, `driver-1119/*.log`, the briefs and the
  reports. A private Cargo target, `target-1236/` (several GB), also remains
  there. All of it is temporary and nothing depends on it. The durable record is
  the Linear comments and `docs/decisions.md`. The disposable mutation copies
  were deleted.
- **Branches.**
  - Merged issue branches remain on `origin` and locally:
    `rob-1119-output-tails`, `rob-1120-doc-reconciliation`,
    `rob-1229-local-verification-gate`, `rob-1236-invitation-domain`, and the
    earlier handoff branch `driver-handoff-2026-10-03`. Delete them only with
    the owner's agreement.
  - This handoff's branch, `driver-handoff-2026-10-03b`, was pending delivery
    when this was written (section 1).
  - The older local branches `docs/s17-reference-app`, `lifecycle-design` and
    `s17-checkpoint-a` predate this chunk. Leave them alone.
- **Cleanup obligations:** none required. The scratch target can be deleted at
  any time.

## 8. Conventions and gotchas

- **Linear access.** Claude sessions here have no Linear MCP. The driver used
  Linear's GraphQL API with `LINEAR_API_KEY`, and mutations accept identifiers
  such as `ROB-1236`. The key is shared across projects, and its limit of 2,500
  requests an hour was exhausted once. Batch reads, and retry writes later.
- **Rust on this Mac.** Homebrew's `rustc` 1.99.0 shadows the
  `rust-toolchain.toml` pin 1.98.1. Record the actual version in evidence.
- **Local verification set** (it replaced CI; pick the commands each change
  touches):
  - `cargo fmt --all --check`
  - `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
  - `cargo test --workspace --locked`, with a private absolute
    `CARGO_TARGET_DIR`
  - `cargo test --locked -p iris-api-spike --features dev-identity`
  - `npm --prefix apps/reference/web run verify` (run
    `npm --prefix apps/reference/web ci` first)
  - `node apps/reference/scripts/probes.mjs`
  - `env -u NO_COLOR FORCE_COLOR=1 node --test apps/reference/scripts/test/dev.test.mjs`
    (about 145 s; needs ports 4001, 3003 and 5175 free)
  - `node apps/reference/scripts/browser.mjs --artifacts <dir>` (never overlap
    it with the command suite)
- **The probes' pinned count.** `probes.mjs:92` runs the `iris` and
  `iris-reference` tests and asserts the exact `iris-reference` library count,
  now 132 (`iris` separately has 34). Update it whenever reference tests change.
- **Storage tests.** Any test that opens `Storage` holds `storage::shared()`,
  and any test that spawns a child holds `exclusive()`. Run-time migrator
  fixtures must include every real migration; synthetic ones use high numbers
  such as 9999.
- **Test messages.** Tests must not print credentials, hashes or addresses on
  failure: use `assert!` with fixed messages, not `assert_eq!`, for those
  values.
- **A real COMMIT failure in SQLite tests** comes from a
  `DEFERRABLE INITIALLY DEFERRED` foreign key fired by a trigger
  (`http/memberships/tests.rs:706`, `domains/invitations/tests.rs`). A real
  BEGIN busy comes from another connection holding `BEGIN IMMEDIATE`.
- **Prettier 3.9.9**, `--print-width 80 --prose-wrap always`. It joins adjacent
  plain lines, so machine-read blocks must be fenced. Its re-wrapping breaks
  exact-string edits, so match whitespace-tolerantly.
- **Herdr.** `herdr agent read` cannot scroll a working agent's pane. Have
  agents write long reports to a file and reply with the path. Waits can outlast
  a 10-minute tool timeout, so repeat `herdr agent wait`.
- **Mutation evidence.** Use disposable copies with private targets and green
  controls before and after. A mutant must exercise the exact regression it
  names; the oracle rejected a weaker stand-in once.

## 9. Skills

- Required: `crew` for every role from now on. The `driver` and `oracle` skills
  governed this session's loop.
- Optional: `herdr` for pane operations.
