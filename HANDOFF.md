# Handoff

Written October 3, 2026, by the outgoing crew driver (Claude Opus) at the end of
its chunk. Jev had called for a fresh driver (context past its limit, and the
next step unrelated), and the owner then stopped for the night. A new session
starts tomorrow with `/crew`. This replaces the previous handoff, which git
history keeps at `8803201:HANDOFF.md`.

## 1. State

Observed October 3, 2026, by the outgoing driver.

- Repository: `/Users/robertguss/Projects/startups/Iris` (GitHub
  `robertguss/Iris`).
- When this was written, the checkout was on branch
  `driver-handoff-2026-10-03c`, created from `main` at `8803201`, with this
  `HANDOFF.md` uncommitted. `origin/main` was at `8803201`, the merge of PR #22
  (ROB-1246).
- Reviewed through `8803201`. Pushed through `8803201`.
- Still pending at the time of writing: this handoff's commit, its branch push,
  its PR and its merge. Check `gh pr list --state all --limit 3` rather than
  assuming.
- CI is disabled: `gh workflow list --all` shows `Verify disabled_manually`
  (ROB-1229). No push or PR runs a check.
- No PR was open, and apart from this handoff the working tree was clean.

Re-check HEAD, the working tree and the remote (`git status`, `git log -5`,
`git ls-remote origin`, `gh workflow list --all`) before relying on any of this.

## 2. Queue

Linear team `ROB`, project `Iris`. Snapshot October 3, 2026:

- **ROB-1244** "Invitations stage 2: private delivery and lifecycle" is
  `Building`. It is split into three sub-issues, each blocking the next:
  - ROB-1245 "1/3 Outbox claims, fencing and clearing": `Done` (PR #21).
  - ROB-1246 "2/3 Local mail capture and SMTP send": `Done` (PR #22).
  - **ROB-1247 "3/3 Supervised delivery task and shutdown": `Ready`, unblocked,
    not started.** This is the next step. No brief exists yet. When it is done,
    move ROB-1244 to `Done`.
- Nothing is in `Planning`, `In Review` or `Needs Input`.
- `Backlog`: **ROB-1123**, CSP-safe runtime validators, parked until the owner
  chooses a restrictive CSP.
- `Canceled` this chunk: **ROB-1227**, the CI focused-check flake. CI is off, so
  it blocks nothing. A `[driver]` comment says to reopen it if CI returns.
- Stage 3 (public routes, OpenAPI, generated TypeScript and the client, in one
  green candidate; S19 "Bounded later implementation candidates" item 3) is not
  in Linear yet. Propose it to the owner after ROB-1244 is done.

## 3. Read these first

- `AGENTS.md` (the crew workflow, delivery and "Scope and safety").
- `docs/design-spec.md` S19, all of it, including the step 1 and step 2 status
  paragraphs.
- `docs/decisions.md`, its last two entries: "Invitation delivery claims and
  fencing" (ROB-1245) and "Invitation mail capture and SMTP send" (ROB-1246).
- The completion comments on ROB-1245 and ROB-1246. Each records every review
  finding with its disposition, and the verification. ROB-1246's ends with the
  conditions ROB-1247 must keep.
- ROB-1247's description in Linear, and ROB-1244's.
- Code: `apps/reference/src/domains/invitations/delivery.rs` (claim, complete,
  sweep), `apps/reference/src/mail.rs` (`Mailer`, `SEND_TIMEOUT`, `Sent`,
  `Category`), `apps/reference/scripts/mailpit.sh`,
  `apps/reference/src/lifecycle.rs` (Task, periodic, serve, Connections), and
  `apps/reference/src/bin/reference-dev.rs`.

## 4. Context

**ROB-1247 must keep these conditions.** They come from the reviews and are
recorded on ROB-1246 and in the decision record.

- Build the task on `lifecycle::Task::new`, which hands the task its `Shutdown`.
  `lifecycle::periodic`'s unit never sees shutdown, and a started unit runs to
  its end (lifecycle.rs:180-197), so it cannot stop a send between claim and
  SMTP. The alternative is an explicit change to `periodic`.
- Each tick: sweep, then claim, then send, then complete. After an observed
  stop, start no sweep or claim. Check shutdown immediately before the send; a
  claim that sees shutdown there does not send and is not refunded.
- `mail::SEND_TIMEOUT` (2 s) is the only bound on a send once connected:
  lettre's async `.timeout` covers only the TCP connect. The worst case inside
  the 3 s drain is one admitted send plus one `complete` on `app::connect`'s 100
  ms busy timeout. Do not lengthen either.
- Use `lifecycle::Connections` (tracked), and dispose of a connection after any
  `Err` from claim, complete or sweep (each function's doc comment says so).
- A database error from `complete` leaves acknowledgment unknown: never treat it
  as `false`. Report bounded diagnostics only (ids, attempt, stage, the
  `Category` label). Never log a token, hash, address, Message-ID, body or raw
  error.
- A panic or early return of the task stops the process through existing
  supervision, with no restart. S18's `DRAIN` 3 s, `CLOSE` 1 s and the outer 5 s
  kill stay unchanged.
- Development wiring: `reference-dev` needs the mailer's origin
  (`IRIS_PUBLIC_ORIGIN`) and the SMTP address. `dev.mjs` would start the capture
  with
  `bash apps/reference/scripts/mailpit.sh run 4025 4026 apps/reference/.dev/mailpit/mailpit.db`.
  Adding 4025 and 4026 to `dev.mjs` and its fixed-port suite is a planning
  decision for ROB-1247's brief.
- A composition refusal completes as `Permanent`, so the stored outcome is
  `permanent`, not `malformed`. Step 1's `malformed` comes only from claim.

**Why the split.** The previous handoff suggested schema and claims first, then
SMTP with capture, then shutdown integration. The oracle approved that split in
ROB-1245's plan review and amended step 3 to build on `Task::new`.

## 5. This chunk

| Issue          | Commit    | PR and merge         | Built by | Reviewed by                           |
| -------------- | --------- | -------------------- | -------- | ------------------------------------- |
| ROB-1245       | `ac09ced` | #21, merge `7a7f763` | builder  | oracle (plan 2 rounds, diff 2 rounds) |
| ROB-1246       | `66ea94e` | #22, merge `8803201` | builder  | oracle (plan 2 rounds, diff 2 rounds) |
| (this handoff) | pending   | pending              | driver   | oracle (handoff review)               |

Jev sent every plan and diff to the oracle (concurrency and data flags), and
ROB-1244 carries the `oracle` label. The oracle also reviewed both PR texts
before posting.

What was checked independently, and what is only reported:

- **ROB-1245.**
  - The driver reran the whole verify set on the final tree, on Homebrew rustc
    1.99.0; it was not rerun on 1.98.1.
  - The oracle ran 23 mutants itself in its own copy: 22 caught, and one (claim
    without `outcome IS NULL`) hangs.
  - Builder-reported only: mutants m3, m4 and m5.
- **ROB-1246.**
  - The driver reran the whole verify set on rustc 1.98.1, including the 5
    real-capture tests, plainly and with hostile `MP_*` exports. An earlier
    driver run on Homebrew 1.99.0 was also green.
  - The oracle reproduced the Mailpit behaviors (environment, retention, relay
    and chaos) with the pinned binary, and confirmed both checksums and the
    one-line `Cargo.lock` change.
  - Builder-reported only: mutants m1–m8, run before the fix round and not rerun
    after it.
- In both steps the builder wrote tests alongside the code, so they were seen
  failing only through mutants. The decision entries say so.

## 6. Decisions and authorizations in force

- **Owner, October 3, this chunk:** asked three questions (release stage 2;
  confirm issue-branch push, PR and merge for this session; cancel or keep
  ROB-1227), the owner answered "you decide for me". Under that delegation the
  driver decided:
  - Build stage 2 (ROB-1244, filed and released by the driver).
  - Deliver each issue on its own branch (`rob-<number>-<slug>`): push, PR, then
    merge after oracle sign-off and the driver's own rerun of the verify set on
    the final tree. PR text is reviewed by the oracle before posting.
  - Cancel ROB-1227.

  The owner had not commented on these when this was written. The delegation
  covers all of ROB-1244, including ROB-1247. The delivery confirmation was
  asked for this session, though, and `AGENTS.md` requires authorization before
  an issue-branch push. So the new driver confirms delivery once with the owner
  before ROB-1247's first push. Nothing here authorizes stage 3, re-enabling CI
  or a release.

- **Workflow, from earlier chunks and still in force:** oracle (or, on Jev's
  call, driver) review before every commit; PR text reviewed before posting; ask
  the owner one question per message.
- **Owner, October 3 (earlier):** Iris uses the `crew` skill. "You are in
  control of linear": the driver creates, releases and closes issues. New
  product work beyond what the owner released or delegated still waits for the
  owner.
- **Owner, October 3:** CI is disabled. Re-enabling it is a CI change that needs
  the owner's approval.
- **Still excluded:** edits to the frozen Turso guide and to the PR #1
  description, each needing separate owner approval. Merged issue branches on
  `origin` (`rob-1119-*`, `rob-1120-*`, `rob-1229-*`, `rob-1236-*`,
  `rob-1245-delivery-claims`, `rob-1246-mail-capture` and the handoff branches)
  are deleted only with the owner's agreement. The older local branches
  `docs/s17-reference-app`, `lifecycle-design` and `s17-checkpoint-a` predate
  these chunks: leave them alone.

## 7. Operational state

- **Agents.** The owner stopped for the night, so no replacement driver was
  started. The oracle and builder panes were restarted fresh and left idle. The
  next session runs `/crew`: setup reuses or recreates the panes, then reads
  this file.
- **Processes.** No Mailpit or test process belongs to this chunk. The last
  check (`lsof -nP -iTCP -sTCP:LISTEN | grep mailpit`) found none. Ports 4001,
  3003, 5175, 4025 and 4026 were free.
- **Installed binary.** `apps/reference/.dev/bin/mailpit` (v1.31.2, gitignored)
  is installed in this checkout. `mailpit.sh install` is idempotent.
- **Evidence.** Everything is temporary; the durable record is the Linear
  comments and `docs/decisions.md`. It all lives in this session's scratch
  directory,
  `/private/tmp/claude-501/-Users-robertguss-Projects-startups-Iris/cb16d28d-17c1-46c4-91f6-bea6af8decf3/scratchpad/`:
  - the briefs, builder reports and oracle replies
  - the `verify-1245*/` and `verify-1246*/` logs
  - the downloaded `mailpit/` tarballs
  - private Cargo targets (`target-driver*`, several GB)

  The builder's and oracle's own scratch copies may also remain in their session
  directories. Delete any of it freely.

- **Cleanup obligations:** none.

## 8. Conventions and gotchas

- **Linear access.** Claude sessions here have no Linear MCP. Use Linear's
  GraphQL API with `LINEAR_API_KEY` (identifiers such as `ROB-1247` work as
  ids). The key is shared and rate-limited, so batch reads. State ids for team
  ROB: Ready `d4e9d1ee-b9c6-42f8-85bc-6a85178e4f97`, Planning
  `d9b6cf52-534d-4bdc-82ae-ad03910b2e7f`, Building
  `22d49b3e-620d-4f38-8df2-028fcaa5f3c8`, In Review
  `cdb24e79-a6e3-4979-8ffd-4f669efab6a2`, Needs Input
  `ed024e35-4332-4f16-80b5-fe149add5e10`, Done
  `8646d9fb-67ea-405e-9490-6812c90eac3a`. The `oracle` label exists (created
  this chunk).
- **Crew scripts.**
  - Run `jev.py` directly (it is a `uv run --script`), never as
    `python3 jev.py`, which fails on the missing `typesafe_sdk` module.
  - `jev.py fresh` crashes with `StopIteration` for a pane that has never been
    prompted (no transcript). Treat such a pane as fresh.
  - `crewlog.py usage --issue` needs the issue that `crewlog.py step` was logged
    under.
- **Herdr.** `herdr agent read` returns only the visible screen. Ask the oracle
  and builder to write replies and reports to a file and reply with the path,
  then poll for the file and a non-`working` status. An agent can end its turn
  with background work still running (the builder's mutation runs did), so wait
  for the file, not the status alone.
- **Scratch briefs.** A PostToolUse formatter reflows Markdown written to the
  scratch directory and joins adjacent plain lines. Put brief headers and lists
  in separate paragraphs or list items.
- **Rust toolchains.** Homebrew `rustc` 1.99.0 shadows the 1.98.1 pin. Put
  `~/.cargo/bin` first on `PATH` to run on 1.98.1, and record the version used.
- **Local verification set** (it replaced CI; pick the commands a change
  touches):
  - `cargo fmt --all --check`
  - `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
  - `cargo test --workspace --locked` (private absolute `CARGO_TARGET_DIR`)
  - `cargo test --locked -p iris-api-spike --features dev-identity`
  - **new:** `bash apps/reference/scripts/mailpit.sh install`, then
    `cargo test --locked -p iris-reference --lib mail -- --ignored` (5
    real-capture tests, about 123 s, holding `storage::exclusive()`), then check
    `lsof -nP -iTCP -sTCP:LISTEN | grep mailpit` shows nothing
  - `npm --prefix apps/reference/web run verify` (after `npm ci`)
  - `node apps/reference/scripts/probes.mjs`
  - `env -u NO_COLOR FORCE_COLOR=1 node --test apps/reference/scripts/test/dev.test.mjs`
    (about 145 s; ports 4001, 3003, 5175)
  - `node apps/reference/scripts/browser.mjs --artifacts <dir>`
- **The probes' pinned count.** `probes.mjs:92` asserts the `iris-reference`
  library count, now **154** (5 more are ignored). Update it whenever reference
  tests change.
- **Ports.** The fixed ones are 4001, 3003 and 5175 (dev suite), plus 4025 and
  4026 (the development mail capture, documented but not yet started by any
  script). Capture tests reserve their own ports by binding `127.0.0.1:0`. Never
  use 1025 or 8025, and never touch a shared Mailpit inbox.
- **Mailpit 1.31.2 facts** (measured this chunk):
  - Inherited `MP_*` variables override missing flags, so the launcher's
    `env -i` is load-bearing.
  - Pruning runs about once a minute: the 500 cap and the 24 h age both take
    effect within about 60 s.
  - `/api/v1/webui` reports `ChaosEnabled` and `MessageRelay.Enabled`.
  - `mailpit version` checks GitHub for updates unless given
    `--no-release-check`.
- **lettre 0.11.23 (async).** `.timeout()` bounds only the connect; reads and
  writes have no timeout. Bodies over 76-character lines get quoted-printable
  unless pre-encoded, which is why `mail.rs` sends a checked 7-bit body.
- **Storage tests.** Tests that open `Storage` hold `storage::shared()`; those
  that spawn a child hold `exclusive()`. Run-time migrator fixtures must include
  every real migration (now 0001–0003); synthetic ones use 9999.
- **Test messages.** Never `assert_eq!` on a credential, hash, address,
  Message-ID or a row holding them. Use `assert!` with a fixed message.
- **A real COMMIT failure in SQLite tests** comes from a
  `DEFERRABLE INITIALLY DEFERRED` foreign key fired by a trigger
  (`http/memberships/tests.rs:706`, `domains/invitations/tests.rs`,
  `domains/invitations/delivery/tests.rs`'s
  `database_errors_are_errors_not_false_none_or_zero`). A real BEGIN busy comes
  from another connection holding `BEGIN IMMEDIATE`. ROB-1247 needs both to test
  a `complete` database error as distinct from `false`.
- **Mutation runs.** A mutant must exercise the exact regression it names; the
  oracle rejected a weaker stand-in once. Use disposable copies with private
  targets and green controls before and after. Don't overlap two agents'
  mutation runs on the same test filter: the oracle once killed hung binaries by
  name pattern.
- **Prettier 3.9.9**, `--print-width 80 --prose-wrap always`.

## 9. Skills

- Required: `crew` for every role (driver, oracle, builder).
- Optional: `herdr` for pane operations.
