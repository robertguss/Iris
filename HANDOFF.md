# Handoff

Written October 3, 2026, by the outgoing driver, at the end of a chunk that
emptied the Ready queue: no Iris issue remains in Ready or in active work, and
ROB-1123 stays parked in Backlog. It replaces the September 30 snapshot. That
snapshot, with ROB-1120's CI corrections, is in git history at
`e4ba261:HANDOFF.md`.

## 1. State

Observed October 3, 2026, by the outgoing driver.

- Repository: `/Users/robertguss/Projects/startups/Iris` (GitHub
  `robertguss/Iris`). When this was written, the checkout was on branch
  `driver-handoff-2026-10-03`, created from `main` at `e4ba261`, with this
  `HANDOFF.md` uncommitted.
- Reviewed through `e4ba261`, the merge of PR #15 (ROB-1120). `origin/main` was
  at `e4ba261`.
- Pushed through `e4ba261`. Still pending at the time of writing: this handoff's
  commit, the push of its branch, its PR (with the PR text reviewed before
  posting), its CI, and its merge. Check `gh pr list --state all --limit 3` for
  the outcome rather than assuming it.
- CI: the post-merge `Verify` run for `bdf5ba7` (PR #14) succeeded (run
  37119292679). The run for `e4ba261` (37122553127) failed in the unrelated
  agent-interface MCP step (section 7, ROB-1227). The exact-candidate run for PR
  #15 (37121865287) succeeded.
- Apart from this handoff, the working tree was clean, and no pull request was
  open.

Re-check HEAD, the working tree and the remote (`git status`, `git log -5`,
`git ls-remote origin`, `gh run list --branch main --limit 3`) before relying on
any of this.

## 2. Queue

Linear team `ROB`, project `Iris`. Snapshot October 3, 2026:

- Nothing is in `Ready`, `Planning`, `Building`, `In Review` or `Needs Input`.
- No split issue has sub-issues left.
- `Backlog`: ROB-1123 (CSP-safe runtime validators). It is parked on purpose:
  the reference app has no restrictive CSP, and the owner picks a CSP policy
  before it can be released. The `[driver]` comment of October 2 records this.
- `Backlog`: ROB-1227 (intermittent agent-interface MCP focused-check failures
  in CI). The driver filed it on October 3 after a second failure (section 8).
  The owner decides whether to release it.

The next substantive work is **not in Linear yet**. Building invitations follows
the three stages in `docs/design-spec.md` S19, "Bounded later implementation
candidates":

1. Private persistence and domain rules.
2. The private delivery worker and its lifecycle.
3. Both public operations with the complete client, in one green candidate.

The owner has to write and release those issues. A fresh driver with an empty
queue reports that to the owner and stops.

## 3. Read these first

- `AGENTS.md`: the driver configuration (Linear, worker, delivery) and "Scope
  and safety". All the approval gates still apply.
- Linear completion comments on ROB-1119 and ROB-1120. Each records its brief,
  every oracle finding with its disposition, and the verify results.
- `docs/design-spec.md`: S18 "Development command evidence", including the
  ROB-1119 limits, and S19 for the invitation design and its later
  implementation candidates.
- `docs/decisions.md`: the latest entries, "Development-child output tails —
  October 2, 2026" (ROB-1119) and the ROB-1120 delivery note in the ROB-1112
  section.
- `apps/reference/README.md` "Run it", and the dated pointer above the
  verification matrix.

## 4. Context

- **Why the workflow changed:** on October 2 an Amp-based lead and its builders
  merged PRs #2 to #13 (ROB-1110 to ROB-1118, ROB-1121, ROB-1135 and ROB-1176).
  The same day, the owner switched Iris to the driver/oracle/worker loop: a
  Claude driver, a Codex oracle and a Pi worker. ROB-1120 rewrote `AGENTS.md` to
  match. The Amp lead's Linear comments, prefixed `[lead]`, remain the history
  of those issues.
- **ROB-1119's takeover:** the Amp lead had published an oracle-approved plan as
  the empty commit `7840188` on `rob-1119-output-tails`. It then handed the
  branch to an Amp Builder, which never pushed. The owner told the driver to
  take it over. The plan was restated in driver format and reviewed again by
  this oracle.
- **What the chunk changed:**
  - A development child's final unterminated output line is now printed once,
    before the final `dev: exit …` line.
  - Owned entries are kept until the group is gone and stdio has closed.
  - Every other ROB-1119 edge limit is retained and documented.
  - The documentation was reconciled with the merged baseline (ROB-1120).

## 5. This chunk

| Issue    | Commit    | PR and merge         | Exact-candidate CI                                                      |
| -------- | --------- | -------------------- | ----------------------------------------------------------------------- |
| ROB-1119 | `fccd32e` | #14, merge `bdf5ba7` | 37068803231: attempt 1 failed in an unrelated step; attempt 2 all green |
| ROB-1120 | `0034860` | #15, merge `e4ba261` | 37121865287: green on attempt 1                                         |

What the oracle checked independently, and what is only driver- or
worker-reported:

- **ROB-1119.** The oracle checked these itself:
  - lifecycle probes showing that `close` fires for ENOENT and for `ignore` and
    `inherit` stdio
  - reproductions of the test-cleanup failures it reported, and of their fixes
  - both direct-spawn controls
  - that the production files were unchanged across the review rounds

  The driver reran the full `dev.test.mjs` suite (31/31) and `browser.mjs`
  (PASS) on the final implementation. The red run and mutants (a) to (f) are
  worker-reported; the oracle inspected their logs.

- **ROB-1120.** The oracle independently ran:
  - Prettier
  - the placeholder and stale-phrase checks
  - the link check (231 links, 0 broken)
  - the `gh` lookups of every run and PR fact

## 6. Decisions and authorizations in force

- **Workflow, from earlier chunks and still in force:** the owner asked for
  oracle review before every commit. PR text is reviewed before posting. Ask the
  owner one question per message.
  - This chunk kept the first rule but lapsed on the other two:
    - PR #14's and PR #15's bodies were posted without an oracle review of the
      text. Both PRs are merged, and their bodies stand as posted.
    - The owner was asked four questions in one prompt at the start.
- **Owner, October 2, 2026, for this backlog run:**
  - Use the driver loop.
  - Take over ROB-1119 from the stalled Amp Builder.
  - Deliver each issue on a dedicated branch, through a PR, and merge after the
    oracle's sign-off and green exact-candidate CI. Done means merged.
  - Release ROB-1120, including the `AGENTS.md` driver section.
  - Leave ROB-1123 in Backlog.

  `AGENTS.md` still requires authorization before pushing an issue branch and
  keeps the PR and merge gates. A new session confirms delivery authorization
  with the owner instead of assuming that this run's still holds.

- **Owner, October 3, 2026:** one rerun of the failed job in run 37068803231.
  This was a one-off and is not standing approval for CI reruns.
- **Still excluded:** edits to the frozen Turso guide and to the PR #1
  description. Each needs its own owner approval.

## 7. Operational state

- **Worker:** `pi` with no arguments, as recorded in `AGENTS.md`. No worker is
  running. The oracle is Codex in the right pane.
- **Jobs and processes:** no local process was left running, and ports 4001,
  3003 and 5175 were free after the last suites. The post-merge `Verify` run
  37122553127 for `e4ba261` **failed** in the agent-interface MCP step
  (ROB-1227), not in anything ROB-1120 changed. The outgoing driver reported it
  to the owner and did not rerun it, since a rerun needs the owner's approval.
  At this update, `main`'s latest push run is red. A later successful run would
  not establish that ROB-1227 is resolved. This handoff's first PR run
  (37123140729, at `0311dac`) passed.
- **Evidence:** this session's scratch directory
  (`/private/tmp/claude-501/-Users-robertguss-Projects-startups-Iris/361b7984-181f-491b-b2e2-0162b0f2be55/scratchpad/`)
  keeps only logs and reports: `evidence-1119/`, `driver-1119/*.log`,
  `report-1119.md` and `report-1120.md`. The driver's private Cargo target and
  browser artifacts were deleted. The scratch directory is temporary and nothing
  depends on it; the durable record is the Linear comments and
  `docs/decisions.md`.
- **Branches:**
  - Merged issue branches stay on `origin` (`rob-1110-…` to `rob-1121-…`,
    `rob-1176-…`, `rob-1135-…`, `rob-1119-output-tails`,
    `rob-1120-doc-reconciliation`).
  - Local branches `rob-1119-output-tails` and `rob-1120-doc-reconciliation` are
    merged. Delete them only with the owner's agreement.
  - This handoff's branch, `driver-handoff-2026-10-03`, was pending delivery
    when this was written (section 1).
  - The older local branches `docs/s17-reference-app`, `lifecycle-design` and
    `s17-checkpoint-a` predate this chunk. Leave them alone.
- **Cleanup obligations:** none.

## 8. Conventions and gotchas

- **Linear access:** this machine's Claude sessions have no Linear MCP. The
  driver uses Linear's GraphQL API with `LINEAR_API_KEY` from the environment.
  Mutations accept issue identifiers such as `ROB-1120` as IDs. The key is
  shared with other projects' sessions, and its limit of 2,500 requests an hour
  was once exhausted mid-chunk. Batch reads, and retry writes later instead of
  in a loop.
- **The agent-interface MCP test**
  (`experiments/agent-interface/runner.test.mjs:70`) failed twice in CI, after
  11–14 s: run 37068803231, attempt 1, on a JS-only diff, which passed on rerun;
  and run 37122553127 on a documentation-only merge. It passed locally (5/5,
  about 50 s). The cause is not diagnosed, and ROB-1227 tracks it. Reruns need
  the owner's approval.
- **Rust on this Mac:** Homebrew's `rustc` 1.99.0 shadows the
  `rust-toolchain.toml` pin 1.98.1. CI uses the pins. Record the actual version
  in evidence.
- **The command suite:**
  `env -u NO_COLOR FORCE_COLOR=1 node --test apps/reference/scripts/test/dev.test.mjs`
  takes about 145 s, needs ports 4001, 3003 and 5175 free, and must not overlap
  `browser.mjs`. Run `npm --prefix apps/reference/web ci` first, and use a
  private absolute `CARGO_TARGET_DIR`.
- **Prettier 3.9.9** with `--print-width 80 --prose-wrap always` joins adjacent
  plain lines. Machine-read blocks, such as the driver configuration in
  `AGENTS.md`, must be fenced.
- **Reading agents in Herdr:** `herdr agent read` cannot scroll a pane while its
  agent is working, and long worker reports scroll out of reach. Ask the worker
  to write its report to a file and reply with the path. Oracle reviews and
  builds can outlast a 10-minute tool timeout, so wait in repeated
  `herdr agent wait` calls.
- **Mutation evidence:** run mutants in disposable copies with green controls
  before and after. A brief that asks for them should also ask for a failing run
  on the base, with the new tests copied in.

## 9. Skills

- Required: `driver` for the driver, and `oracle` for the oracle.
- Optional: `worker` (loaded by each worker from its prompt), and `herdr` for
  pane operations.
