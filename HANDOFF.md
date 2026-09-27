# Handoff

## 1. State

Observed September 27, 2026, by the outgoing driver.

- Repository: `/Users/robertguss/Projects/startups/Iris` (GitHub
  `robertguss/Iris`).
- Branch: `s17-checkpoint-a`, which now tracks `origin/s17-checkpoint-a`. `main`
  is unchanged at `9235c6e`.
- Reviewed through `bf55a09` (step 3 of this chunk); this handoff is committed
  after it.
- Pushed through `a0c25ff`. That is `origin/s17-checkpoint-a`, the head of draft
  pull request #1 (https://github.com/robertguss/Iris/pull/1); `origin/main` is
  `9235c6e`. The three checkpoint B commits and this handoff are local only.
- Working tree clean apart from this handoff before its commit.

Re-check HEAD, the working tree and the remote (`git status`, `git log -5`,
`git ls-remote origin`) before relying on any of this.

## 2. Read these first

- `docs/design-spec.md`: "How to interpret and maintain this spec", then S17,
  especially "Read conventions", "React client", "Acceptance checks" (the React
  and B rows), "Owner decisions" and "Checkpoint B server-side evidence".
- `docs/decisions.md`: the last two entries, "Reference application checkpoint
  A, client" and "Reference application checkpoint B, server side".
- `apps/reference/README.md`: commands, operations and read rules, layout,
  verification matrix, cost of runtime validation, probes and limits.
- `crates/iris/src/lib.rs` and `apps/reference/src/read.rs`: what the crate
  owns, and the read machinery, whose doc comments state their guarantees.
- `apps/reference/web/src/client.ts`, `main.tsx` and `present.ts`: the client
  boundary, the checkpoint A console and the mutation-only presentation, all of
  which the next chunk extends.
- `apps/reference/scripts/browser.mjs`: the browser workflow; its header states
  the process-ownership guarantees to keep.
- `.github/workflows/verify.yml`: what CI runs.
- `docs/design-review-brief.md`: how independent reviews are run.

## 3. Context

The owner authorized checkpoint B at the start of this session. They also
approved publishing `s17-checkpoint-a` and opening a pull request, so the
updated workflow would run on GitHub Actions; the pull request is not to be
merged. The driver then published the branch and opened draft pull request #1.
Its run passed every step at `a0c25ff`
([record](docs/decisions.md#reference-application-checkpoint-b-server-side--september-27-2026)).

The oracle (Astra, GPT-6-Astra via Codex) agreed to split checkpoint B the way
checkpoint A was split:

- This chunk built the server side in three steps: the member read, the
  own-project read, then the probes and the evidence.
- The next chunk does the client and the browser workflow.

Checkpoint B therefore stays partial.

Every export change forces client linkage in the same step, even without UI. The
client refuses any `x-iris` operation it doesn't list with its hand-written
method, and verification decodes each operation's captured responses against
hand-written counts. That is why the client already accepts both reads, while
`main.tsx` is still checkpoint A's by-ID console and `present.ts` covers only
the two mutations (the `Mutation` type).

Choices made along the way, and the alternatives not taken, are in the decision
record's checkpoint B entry.

## 4. Agreed chunk and acceptance

- **Objective:** checkpoint B's server side:
  - `memberships.list` / `listProjectMembers` and `projects.list_mine` /
    `listMyProjects`, with GET and HEAD;
  - S17's read conventions;
  - the B authorization, pagination and input, and cleanup rows;
  - the read probes;
  - the client linkage the export forces;
  - the frozen experiments kept green.
- **Exclusions:**
  - read UI and presentation, and browser workflow changes;
  - declaring a current-state read on the mutations;
  - invitations, the lifecycle pass, and edits to `experiments/`;
  - retiring a frozen experiment;
  - merges, and pushes beyond the approved initial publish.
- **Stopping condition:** step 3 signed off and committed, then this handoff.
- **Disposition:** `accepted`, as commits `9b548a3` (member read, iris GET/HEAD
  and optional recovery, `read::run`), `5b16132` (own-project read) and
  `bf55a09` (read probes and evidence).
- **Scope changes approved by the user:** publishing the branch and opening a
  pull request, without merging (done: draft #1). No other change.

## 5. Verification and review

Environment: macOS, Rust 1.98.1, Node 24.20.0 (the client checks also ran under
26.8.1; CI pins 26.10.0), `agent-browser` 0.38.1 with headless Chrome 154.
Driver evidence is in
`/private/tmp/claude-501/-Users-robertguss-Projects-startups-Iris/d46ebdae-6aee-430c-911e-a7625e5cb965/scratchpad/`
(temporary; below, `scratchpad/`). Step 3 changed only `probes.mjs` and
Markdown, so the step 2 suite results still describe `bf55a09`'s code.

| Claim                     | Commit and evidence                                                                                                                                                                                                                                                                                                                                                    | Checked by                                                                                                                         |
| ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| Workspace, lints, format  | `5b16132`: `cargo test --workspace --locked --no-fail-fast`, 106 passed (`iris` 13; `iris-reference` 45 = lib 34, contract 2, dev binary 2, identity 7); dev-identity suite 24 passed, 1 ignored; Clippy `-D warnings` for the workspace (all targets and features) and each crate alone; `cargo fmt --all --check`; `git diff --check` (`scratchpad/step2/final.log`) | Driver. The oracle ran `iris` plus `iris-reference` (51 tests at step 1, 58 at step 2), Clippy for both crates, fmt and diff check |
| Client linkage            | `5b16132`: `npm --prefix apps/reference/web run verify`, 143 runtime cases across 4 operations (captured 19, 17, 12 and 9), 24 presentations, build; also under Node 26.8.1 (`scratchpad/step3/verify-node26.log`)                                                                                                                                                     | Driver and oracle (Node 24); Node 26.8.1 is driver-reported                                                                        |
| B rows (server)           | Hand-written tests in `http/memberships/list_tests.rs`, `http/projects/tests.rs`, `read.rs` and both domains; S17 "Checkpoint B server-side evidence" maps them to rows                                                                                                                                                                                                | Driver and oracle (tests read and run)                                                                                             |
| Probes                    | `bf55a09` content: `node apps/reference/scripts/probes.mjs`, 10 caught mutations and 7 controls, 1 min 24 s cold (`scratchpad/step3/probes.log`); a no-op probe makes the harness fail (`probes-negative.log`)                                                                                                                                                         | Driver; the oracle reran the harness (10 and 7, 12 CAUGHT lines, 109.65 s) and the negative control                                |
| Seeded mutations          | Step 1: `iris` and `read.rs` in-tree, then 12 on a disposable copy (`scratchpad/mutate-step1.py`, `mut-s1/`, `mut-s1-rerun/`); step 2: 8 (`mutate-step2.py`, `mut-s2/`). All caught except the equivalent cursor-length mutant; the decision record lists them                                                                                                         | Driver; the oracle inspected the logs, did not rerun them                                                                          |
| Frozen experiments green  | `5b16132`: web `verify` and `verify:s16` (`step2/final.log`); `probe:s16` not rerun (experiment unchanged)                                                                                                                                                                                                                                                             | Driver                                                                                                                             |
| Checkpoint A browser flow | `5b16132`: `node apps/reference/scripts/browser.mjs` passed after each step and nine times in a row; validator compile for 4 operations 17.5–21.0 ms, median 19.5 ms (`scratchpad/measure-b/runs.log`); bundle 382.39 kB (112.55 kB gzip)                                                                                                                              | Driver; the oracle checked the log figures                                                                                         |
| GitHub Actions            | Run 36317010742 on PR #1 at `a0c25ff`: every step passed, 7 min 37 s; reference client 23 s, browser workflow 15 s (`scratchpad/ci-run-36317010742.log`). The checkpoint B commits have not run in CI                                                                                                                                                                  | Driver; the oracle confirmed the run's success                                                                                     |

Oracle verdicts and dispositions, all fixed unless stated:

1. Chunk proposal: sign-off; record B as partial until the client chunk.
2. Step 1 plan: changes requested.
   - P2: the forced client linkage has more consumers. Fixed with the `Mutation`
     type and by scoping the tests.
   - P2: writer progress cannot prove explicit finalization. Fixed:
     classification is unit-tested with injected rollback results, awaited
     finalization rests on source review, and cancellation uses the real
     `read::run` future after showing the writer is blocked.
   - P3: positive page bounds and parameter assertions. Added.

   The re-review signed off, adding that the cancellation writer should use
   explicit SQL so a busy `COMMIT` can be retried; this was applied.

3. Step 1 diff: sign-off.
   - P3: the `BrowserSession` description claimed CSRF for every method. Fixed.
   - A suggestion to call `mount_list_members` in the 503 test. Applied.
   - The equivalent cursor-length mutant is kept as a documented bound.
4. Step 2 plan: sign-off, with two details: move `open` to `http/mod.rs`, and
   make the pagination fixture discriminating. Both done.
5. Step 2 diff: sign-off, no findings.
6. Step 3 plan: sign-off.
   - P3: the S10 status row. Fixed.
   - Probe details: named failures, separate cargo runs, valid SQL bindings.
     Kept.
7. Step 3 diff: sign-off.
   - P3: a mutation label. Renamed to "mismatched declared handler identity".
   - P3: the probes' named-test guarantee was stated too broadly. Narrowed to
     the checkpoint B probes.

## 6. Remaining work

1. **S17 checkpoint B, client:**
   - read decoder, narrowing and presentation cases;
   - the React member directory;
   - one checkpoint B browser workflow;
   - completing B's evidence.

   See section 7.

2. **Running the checkpoint B commits on GitHub Actions:** they're local only.
   Pushing them to draft PR #1 needs the owner's go-ahead (section 9).
3. **A follow-up design for invitation issuance and acceptance.**
4. **A lifecycle design pass:** persistent storage, seed policy, journal mode,
   worker supervision, and one development command. Running the console by hand
   takes three processes.
5. **Retiring frozen experiments:** S16 stays in CI until its remaining omission
   probes are carried by the reference application.
6. **Documentation hygiene, carried forward:**
   - the top-level README's "Next milestone" omits S16 and S17;
   - `experiments/api-slice/README.md` still lists "durable email delivery" as
     absent;
   - the original "Open decisions" list in `docs/decisions.md` includes items
     settled later.
7. **Precompiled validators:** needed if a content security policy without
   `unsafe-eval` is adopted, to replace Ajv's runtime compilation.

## 7. Next chunk

`approved` (the owner authorized all of checkpoint B): S17 checkpoint B's
client, on `s17-checkpoint-a`. It still goes through plan review.

- **Acceptance:** S17's React row for both reads:
  - typed narrowing, and runtime decoder cases including oversize bodies;
  - presentation of read outcomes;
  - the member directory: my projects, then a project's members with next-page
    navigation, with role change and removal started from it;
  - session bootstrap unchanged;
  - one checkpoint B browser workflow against the local issuer, extending
    `browser.mjs` while keeping its process-ownership properties.

  Then S17's evidence marks checkpoint B complete. The frozen experiments stay
  green, and each step is signed off by the oracle.

- **First action:** ask the owner the open questions in section 9, one at a
  time, then propose the client chunk's steps and stopping point in the first
  plan review. The planning and building are already authorized, so neither
  waits on the answers. Push approval only gates publishing and CI. The
  current-state-read answer only gates the presentation work that depends on it.

## 8. Decisions and authorizations in force

- **The seven S17 owner decisions** in S17's "Owner decisions".
- **Authorized:**
  - the CI prerequisite and checkpoint A, both complete;
  - checkpoint B: the server side is done and the client is next.

  Commits go on `s17-checkpoint-a`.

- **Approved by the owner this session:** publishing `s17-checkpoint-a` and
  opening a pull request, without merging. Done as draft PR #1 at `a0c25ff`. The
  driver has treated later pushes as needing a fresh go-ahead.
- **Not authorized:** merges; invitations; the lifecycle pass; retiring frozen
  experiments.
- **Decided in this chunk:** the choices in the decision record's "Reference
  application checkpoint B, server side" entry, and the agreed server/client
  split.
- **Workflow:** the owner asked for oracle review before every commit.

## 9. Open questions for the user

- May the driver push the checkpoint B commits (and later ones) to draft PR #1,
  so GitHub Actions runs them? This blocks item 2 of the remaining work, and any
  CI claim for checkpoint B.
- Should `change_role` and `remove_member` declare `listProjectMembers` as their
  current-state read (`x-iris` `recovery.read`; S15 and S17 say "may")? This
  changes the mutations' exported recovery metadata, and lets the unconfirmed
  wording point to the member list. It shapes the client chunk's presentation
  step. The outgoing driver has no firm recommendation: declaring it matches
  S17's stated intent, but a page describes present state only (S14), so the
  wording must not imply the page resolves an earlier attempt.

## 10. Operational state

- **Running processes:** none, and no servers or browser sessions. The browser
  workflow and the probes stop everything they start.
- **Draft PR #1:** open at `a0c25ff`, with passing run 36317010742. Don't merge
  it.
- **`agent-browser`:** 0.38.1 is installed in mise's Node 24 global bin, with
  Chrome 154 in `~/.agent-browser/browsers/`. Under another Node it may not be
  on `PATH`.
- **Gitignored build output:** `target/`, `apps/reference/web/node_modules` and
  `dist`, and `experiments/api-slice/web/node_modules` and `dist`. Nothing needs
  cleanup.
- **Retained evidence:** temporary, and possibly already deleted: the driver's
  scratchpad (section 5), and any review directories the oracle created under
  the system temp directories.
- **Known risk, carried forward:** `probe:s16` builds into the checkout's shared
  `target/` and can leave a mutated artifact that a later run treats as current.
  The experiment is frozen, so the risk is recorded rather than fixed. The
  reference probes and the driver's mutation runners use their own temporary
  targets.

## 11. Conventions and gotchas

- **Shell aliases:** in this machine's interactive shells, `tr` is aliased to a
  trash command, `npm` to a package guard and `ls` to another tool. Use
  `command tr`, `command npm` and `/bin/ls`. A bare `tr` slipped through again
  this chunk and tried to trash files named "\n" and " "; neither existed.
- **zsh:** it doesn't word-split `$VAR` into a command and its arguments; use a
  shell function. `PIPESTATUS` is bash-only.
- **Markdown formatting:** a user-level hook formats Markdown written through
  the driver's Write and Edit tools. Scripted edits bypass it, so run
  `bunx prettier --write --print-width 80 --prose-wrap always <files>`
  afterwards. TypeScript under `apps/reference/web/src` is Prettier-formatted
  too; pass explicit file lists, and never format
  `apps/reference/web/src/generated.ts`.
- **Contract changes:** changing an operation's contract takes two
  regenerations:
  `cargo run --locked -p iris-reference --bin export-openapi -- apps/reference/openapi.json`,
  then `npm --prefix apps/reference/web run generate`.
- **Every new or changed operation touches the client in the same step:**
  - `client.ts`: `DOMAIN_OPERATIONS` and `METHODS`;
  - `client.test.ts`: `NAMES`, `PATHS` and the hand-written `CAPTURED` counts.

  `verify` captures real responses only from library tests whose names contain
  `whole_request`, so name a new operation's capture test accordingly.

- **Doc comments leak into the export:** a `///` comment on a `#[utoipa::path]`
  handler becomes the exported `summary`, so use `//`. The reads' independent
  contracts assert that there is no summary.
- **Test `validate` helpers:** they build a schema from the export, and an
  undeclared status yields an empty schema that accepts anything. Check
  `responses[status]` directly when asserting absence.
- **SQLite fault injection that works here:**
  - A read's 503: hold `BEGIN EXCLUSIVE` on another connection, and send the
    request through the operation's own mount with an injected `Actor` and no
    cookie, so the session layer needs no database.
  - A read's 500: store a BLOB in a TEXT column the page decodes; SQLx decodes
    by the value's runtime type.
  - Retrying a busy `COMMIT`: drive the writer with explicit SQL.
    `Transaction::commit` consumes the transaction and queues a rollback on
    failure.
- **Shared test fixtures:** they live in `pub(crate)` `#[cfg(test)]` modules,
  `http::memberships::{tests, list_tests}`.
- **Seeded mutations and probes:** they anchor on exact source text, so rerun
  `cargo fmt` before writing anchors; a rustfmt rewrap caused one anchor drift.
  Quiet libtest prints failures as `---- <path> stdout ----` blocks, not
  `test … FAILED` lines.
- **New packages in `apps/reference/web`:** install with
  `command npm install --before=<date>` if the lockfile should keep matching the
  experiment's resolved set.
- **Node JSON imports:** under Node, a TypeScript file that imports JSON needs
  `with { type: "json" }`.
- **`vite preview`:** it must get `--host 127.0.0.1`, because its default may
  bind `::1`.
- **`agent-browser` output:** `eval` prints its result JSON-encoded.
  `network requests --json` returns `{ data: { requests: [...] } }`, including
  aborted requests.
- **Browser workflow ports:** it needs ports 4001, 3003 and 5175 free, and
  refuses to run otherwise.
- **Timeouts:** the driver's tool calls time out at 600 s, so run long suites in
  the background with output written to files. `cargo test` stops at the first
  failing binary unless given `--no-fail-fast`.
- **Frozen fixture in use:** the reference tests and the browser workflow start
  the local issuer from `experiments/api-slice/checks/oidc-provider.mjs`.
- **`crates/iris`:** add to it only what two operations demonstrably share,
  naming both.
- **History and pushes:** history on `main` is linear and was previously pushed
  by the owner. Never push without the owner's go-ahead. The branch has an
  upstream now, so a bare `git push` would update PR #1.
- **Panes:** the driver works in the left pane and the oracle in the right. Send
  oracle prompts through a file, as the driver skill describes.

## 12. Skills

- **Required:** `driver` for the driver, `oracle` for the oracle.
- **Optional:** `herdr` for pane and agent control.
