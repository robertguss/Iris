# Handoff

Observed October 6, 2026, during ROB-1247 verification. This replaces the
October 3 snapshot, retained in Git at `c2b6e5b:HANDOFF.md`. Linear owns live
status; this is a dated continuity record, not another backlog.

## 1. State

- Repository: `/Users/robertguss/Projects/startups/Iris` (`robertguss/Iris`).
- Candidate branch: `rob-1247-supervised-delivery`, based on main
  `c2b6e5ba3622fa9a940ef214aab10f2aa2ccf6ec` (retirement PR #24).
- The candidate implements ROB-1247. Commit, PR, merge and Done remain separate
  until verification and independent review finish; check Git, PR and Linear
  before assuming delivery.
- `Verify` is still `disabled_manually`; local verification and independent
  review remain the gate. Do not enable or rerun CI without owner approval.

## 2. Queue

ROB-1247 and parent ROB-1244 are Building during this verification snapshot.
ROB-1245 and ROB-1246 are Done. The parent was prematurely marked Done before
this task existed; the current-session audit corrected it to Building. Mark both
Done only after verified delivery. Re-read Linear for their current status.

ROB-1123 remains a conditional CSP-validator backlog item; no restrictive CSP
has been selected. Stage 3 (routes, OpenAPI/client and invitation console) is
still unimplemented and was not part of this candidate.

## 3. Read these first

- `AGENTS.md`: current delivery, independent review and scope requirements.
- `docs/design-spec.md` S19, including the dated stage-2 implementation note.
- `docs/decisions.md`: “Supervised invitation delivery — October 6, 2026”.
- `apps/reference/README.md`: current runnable commands and lifecycle limits.
- The live Linear issue and comments; older handoff queue rows are historical.

## 4. Context

Private issuance/acceptance and append-only migrations 0002/0003 are unchanged.
The delivery task opens a tracked connection per tick, sweeps, claims, sends
outside the transaction and completes under the fence. A stop observed before
SMTP leaves the spent claim/lease/payload. Admitted SMTP runs to its bound and
completion. Completion errors remain unconfirmed and can lead to duplicate
capture on eligible recovery. No public invitation routes or views exist.

DRAIN = 3 seconds, CLOSE = 1 second, SMTP whole-send timeout = 2 seconds and
outer supervisor escalation = 5 seconds. No task restart, claim refund or
unfenced repair. Failed opens keep the existing unacknowledged connection
ticket.

## 5. This chunk

Driver implemented the reviewed plan in this coding session, first retaining a
failing idle-task control. Six worker regressions cover tracked ownership, no
transaction during SMTP, shutdown admission, silent-send drain, database
failures, completion uncertainty/stale fences and forced-failure cleanup. The
SMTP fixture is reused, with JoinSet ownership so silent child sessions cannot
be detached.

The actual launcher test seeds a synthetic outbox, observes owned Mailpit
capture and durable sent/payload clearing, and proves inherited SMTP/Mailpit
configuration is overridden. The 32 launcher tests cover all five ports,
four-child readiness/cleanup, capture failure and persistent per-database
inboxes.

Driver logs: `/Users/robertguss/.local/state/codex/iris-1247/20261006/`.
Independent review/mutants:
`/Users/robertguss/.local/state/codex/iris-1247-oracle/20261006/`. Inspect
retained results; an intended run is not passing evidence.

## 6. Decisions and authorizations in force

The owner delegated implementable Linear work, technical choices, commit/push
and continuation in this session. Actual owner/account/production/scientific
gates remain separate. Crew roles, Jev routing, model requirements and Herdr
panes were retired on October 6. Project plan/diff review, final-tree driver
verification and dedicated issue-branch delivery still apply.

This candidate completes private stage 2 only. No real recipient mail,
authentication provisioning, inference spend, CI change, frozen-experiment
retirement, deployment or release occurred. Synthetic recipients end in `.test`;
the SMTP endpoint is loopback.

## 7. Operational state

`dev.mjs` owns 4001, 3003, 5175, 4025 and 4026. Never overlap it, launcher
tests, or browser workflows on their fixed ports; inspect ownership before
starting and never terminate an unrelated listener. The capture binary is pinned
Mailpit v1.31.2 under gitignored `apps/reference/.dev/bin/`.

Each chosen reference database owns the sibling
`<database filename>.mailpit/mailpit.db`. The default is
`apps/reference/.dev/reference.db.mailpit/mailpit.db`. An application reset
starts no capture/worker and does not clear the inbox. Different disposable
databases never share an inbox. `reference-dev` alone supervises delivery but
does not launch capture.

Re-check process/listener cleanup after any interrupted verification. The
existing outer supervisor owns children immediately, waits for process groups
and stdio closure, and keeps its existing escaped-pipe limitation.

## 8. Conventions and gotchas

Use Rust 1.98.1 with `~/.cargo/bin` first in PATH (the shell's other compiler
may shadow it) and an absolute private CARGO_TARGET_DIR. This chunk uses
`/Users/robertguss/.local/state/codex/iris-1247/20261006/target`. Mutation runs
require their own source/build directories and passing controls.

Required local verification from the repository root:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
cargo test --locked -p iris-api-spike --features dev-identity
bash apps/reference/scripts/mailpit.sh install
cargo test --locked -p iris-reference --lib mail -- --ignored
npm --prefix apps/reference/web run verify
node apps/reference/scripts/probes.mjs
env -u NO_COLOR FORCE_COLOR=1 node --test apps/reference/scripts/test/dev.test.mjs
node apps/reference/scripts/browser.mjs --artifacts <owned absolute directory>
```

The reference library has 160 default tests and five ignored real-capture tests;
the probe healthy-count pin matches. Real capture checks measure the 500-message
cap and 24-hour age pruning, which each take about a minute. Use only the pinned
wrapper and owned ephemeral capture ports for those tests.

The launcher regression uses Node's built-in SQLite (Node 22.23.2 here).
Assertion/timeout failure text is fixed when captured output may contain a
credential/address/hash/Message-ID/body/raw SMTP error. Never log that buffer or
equate a complete error to no transition.

## 9. Skills

Use current project guidance and applicable installed skills. The current
session applies Ponytail for minimal reuse of existing task, mailer, storage and
supervisor code. The Crew skill and scripts are deleted; historical references
grant no operational authority.
