# Handoff

## 1. State

Observed September 27, 2026, by the outgoing driver.

- Repository: `/Users/robertguss/Projects/startups/Iris` (GitHub
  `robertguss/Iris`).
- Branch: `s17-checkpoint-a`, tracking `origin/s17-checkpoint-a`. `main` is
  unchanged at `9235c6e`.
- Reviewed through `420081e` (step 3 of this chunk). This handoff is committed
  after it.
- Pushed through `a0c25ff`, which is `origin/s17-checkpoint-a` and the head of
  draft pull request #1 (https://github.com/robertguss/Iris/pull/1). The branch
  was 7 commits ahead before this handoff: the previous chunk's three
  server-side commits and its handoff, then this chunk's `b55bfca`, `05cbfff`
  and `420081e`. None has been pushed.
- The working tree was clean apart from this handoff before its commit.

Re-check HEAD, the working tree and the remote (`git status`, `git log -8`,
`git ls-remote origin`) before relying on any of this.

## 2. Read these first

- `docs/design-spec.md`: "How to interpret and maintain this spec", then S17:
  "Public response policy" (its last paragraph), "Read conventions", "React
  client", "Acceptance checks", "Owner decisions", and the four evidence
  sections, especially "Checkpoint B client evidence".
- `docs/decisions.md`: the last three entries, the checkpoint A client and the
  checkpoint B server side and client.
- `apps/reference/README.md`: commands, operations and read rules, layout,
  verification matrix, validation cost, probes and limits.
- The client, whose doc comments state its guarantees:
  - `apps/reference/web/src/client.ts`: the boundary;
  - `membership.ts`: requests, with one `read` overload per read;
  - `present.ts`: wording;
  - `directory.ts`: paging, pairing and staleness transitions;
  - `main.tsx`: the console.
- `apps/reference/scripts/browser.mjs`: its header states the process-ownership
  guarantees, including the single API restart between the two workflows.
- `.github/workflows/verify.yml`: what CI runs. The browser command is unchanged
  but now runs both workflows.
- `docs/design-review-brief.md`: how independent reviews are run.

## 3. Context

The owner authorized all of checkpoint B on September 27. The previous chunk
built its server side, and this chunk built its client in three reviewed steps.
S17's evidence now marks checkpoints A and B complete. Every S17 checkpoint the
owner has authorized is done. What comes next is the owner's choice (section 9).

Two ideas shape the client, beyond what the decision record lists:

- **Present state only (S14).** A page describes the state when it was read.
  Every read sentence is scoped to its page, and no read ever resolves an
  earlier attempt. Nothing re-reads automatically after a mutation.
- **Attempt records.** An attempt's operation and target are captured when it is
  sent, and its outcome when it completes. The record then survives reloads,
  selections and same-user refreshes; only the next attempt or a session change
  replaces it. The "may predate your last attempt" note records request
  ordering, not observation.

These came from review findings, and the tests and mutations pin them.

The mutations still declare no current-state read. The owner was not asked this
session (section 9), because the answer to the first open question never came
and the owner asks for one question at a time.

The browser workflow needed fresh data for checkpoint B. Checkpoint A's workflow
ends with Bob removed from project 41, and without invitations nothing can
restore Bob's membership. So the runner restarts only the API between the
workflows.

## 4. Agreed chunk and acceptance

- **Objective:** S17's React row for both reads:
  - decoder, narrowing and presentation cases, including oversize bodies;
  - the member directory, with my projects and a project's members with
    next-page navigation, and role change and removal started from a member;
  - session bootstrap unchanged;
  - one checkpoint B browser workflow against the local issuer;
  - S17's evidence marking checkpoint B complete;
  - the frozen experiments kept green.
- **Exclusions:**
  - server, export or seed changes;
  - a current-state read declaration, which was conditional on the owner's
    answer, and that answer never came;
  - invitations, the lifecycle pass, and edits to `experiments/`;
  - retiring a frozen experiment;
  - merges, and pushes without the owner's go-ahead.
- **Stopping condition:** step 3 signed off and committed, then this handoff.
- **Disposition:** `accepted`, as three commits:
  - `b55bfca`: the reads in the client and their presentation;
  - `05cbfff`: the member directory, and checkpoint A's workflow driven from it;
  - `420081e`: checkpoint B's workflow, the API restart, and the evidence.
- **Scope changes approved by the user:** none.

## 5. Verification and review

Environment: macOS, Rust 1.98.1, Node 24.20.0 (client checks also under 26.8.1
via `mise exec node@26.8.1`; CI pins 26.10.0), `agent-browser` 0.38.1, headless
Chrome 154. Driver evidence is in
`/private/tmp/claude-501/-Users-robertguss-Projects-startups-Iris/42ba4b9b-aa28-4de6-bf60-99bbae6cdfe5/scratchpad/`
(temporary; below, `scratchpad/`). No client source changed in step 3, so the
step 3 `verify` runs describe `05cbfff`'s client too.

| Claim                     | Commit and evidence                                                                                                                                                                                                                                                                                               | Checked by                                                                                                                       |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| Client checks             | `420081e`: `npm --prefix apps/reference/web run verify`, with 231 runtime cases, 49 presentations, 9 request constructions, 11 directory transitions, `tsc` and the build (389.42 kB, 114.41 kB gzip). Also under Node 26.8.1 (`scratchpad/step3/verify-node24.log`, `verify-node26.log`)                         | Oracle ran Node 24 `verify` itself at steps 1 and 2, and Node 26 at the first step 1 review; the step 3 runs are driver-reported |
| Browser workflows         | `420081e`: `node apps/reference/scripts/browser.mjs`, five consecutive passes, 12.9–13.2 s each with builds cached, validator compile 18.0–20.0 ms (median 19.1 ms); a sixth after a wording-only change, 18.1 ms (`scratchpad/step3/browser-1` … `browser-6`, logs and screenshots)                              | Oracle ran the workflow itself at step 2 (19.1 ms after the fix) and step 3 (19.0 ms)                                            |
| Restart ownership         | `420081e`: `scratchpad/probe-restart.py`, on disposable copies of the runner (step3/probes.log). A foreign listener on 3003 in the restart window, the replacement API killed, and SIGTERM in the window each fail with no PASS and no leftovers; the listener survives; no replacement starts after a stop       | Oracle reran all three probes                                                                                                    |
| Seeded mutations          | Step 1: 17 (`mutate-step1.py`, step1/mutations.log). Step 2: 10 directory and 8 through the browser (`mutate-step2.py`, step2/mutations-directory.log, mutations-ui.log). Step 3: 5 through the browser (`mutate-step3.py`, step3/mutations.log). All caught at their named check; the decision record lists them | Oracle inspected the runners and logs, and read the new step 2 mutations' failures; it did not rerun the suites                  |
| Workspace, Clippy, frozen | Not rerun this chunk: no Rust, export or `experiments/` file changed. The last results are the previous chunk's at `5b16132` (106 workspace tests, Clippy, fmt, web `verify` and `verify:s16`)                                                                                                                    | Previous chunk                                                                                                                   |
| GitHub Actions            | None of the seven local commits has run in CI. The last run is 36317010742 at `a0c25ff`                                                                                                                                                                                                                           | —                                                                                                                                |

Oracle verdicts and dispositions, all fixed unless stated:

1. Chunk proposal and step 1 plan: changes requested.
   - P2: empty-page wording must be scoped to the page, since a continuation
     page can be empty. Fixed.
   - P2: extracted body cases needed a failing check if skipped. Fixed with a
     hand-written per-operation tally.
   - P3: update S10 in step 3. Done.

   The re-review signed off.

2. Step 1 diff: changes requested.
   - P2: a union-typed caller could omit the project ID. Fixed with per-read
     overloads and a negative compile-time case.
   - P3: own-project negative cases were missing. Added.
   - A formatting note on `client.ts:57`: Prettier 3.8.3 flags it, and flagged
     the base commit too, while 3.9.9 accepts it. Left as is.

   The re-review signed off.

3. Step 2 plan: changes requested.
   - P2: confirmation per target.
   - P2: freeze the attempt's operation.
   - P2: attempt-scoped stale wording, with defined ordering.

   All fixed. The re-review signed off, with a P3: show stale marking before the
   response arrives. Done by holding one response through a `window.fetch`
   wrapper.

4. Step 2 diff: changes requested.
   - P2: a same-user or failed session refresh erased the attempt. Fixed: only
     the session-transition effect clears it, and browser checks cover all three
     cases.

   The re-review signed off.

5. Step 3 plan: changes requested.
   - P2: the restart must be abort-aware. Fixed with a synchronous pre-spawn
     guard in `start()` and retirement limited to the original API.
   - P2: restore the aborted initial projects load check. Added.

   The re-review signed off.

6. Step 3 diff: sign-off, with three P3 wording corrections, all applied:
   - pending projects reads can also complete as stale;
   - separate the missing recovery declaration from manual reads;
   - the outcome is recorded when the attempt completes, not when it is sent.

## 6. Remaining work

1. **The owner's open questions (section 9).**
2. **Running the seven local commits on GitHub Actions:** needs push approval.
3. **Declaring `listProjectMembers` as the mutations' current-state read, if the
   owner approves.** It changes `crates/iris::Recovery.read` (a boolean today)
   to name the read and its inputs, and needs:
   - both regenerations;
   - updates to the Rust and client recovery assertions;
   - wording that points to the member list without implying a page resolves the
     attempt.
4. **A follow-up design for invitation issuance and acceptance** (S17).
5. **A lifecycle design pass:** persistent storage, seed policy, journal mode,
   worker supervision, one development command.
6. **Retiring frozen experiments:** S16 stays in CI until its remaining omission
   probes are carried by the reference application.
7. **Documentation hygiene, carried forward:**
   - the top-level README's "Next milestone" omits S16 and S17;
   - `experiments/api-slice/README.md` still lists "durable email delivery" as
     absent;
   - the original "Open decisions" list in `docs/decisions.md` includes items
     settled later.
8. **Precompiled validators:** needed if a content security policy without
   `unsafe-eval` is adopted.

## 7. Next chunk

`proposed`. Nothing beyond checkpoint B is authorized.

- **First action:** ask the owner the open questions in section 9, one at a
  time, and wait for each answer before asking the next. Then propose the chunk
  the answers authorize, in the first plan review.
- **Acceptance:** set by that choice.
  - If the current-state read is approved: remaining-work item 3, with the
    frozen experiments and both browser workflows green.
  - If the invitations design is chosen: a design section in S17's style.
    Documentation only, reviewed through the design-review brief, authorizing no
    implementation until the owner says so.

## 8. Decisions and authorizations in force

- **The seven S17 owner decisions** in S17's "Owner decisions".
- **Authorized:** the CI prerequisite, checkpoint A and checkpoint B, all
  complete. Commits go on `s17-checkpoint-a`.
- **Approved by the owner in the previous chunk:** publishing `s17-checkpoint-a`
  and opening a pull request, without merging. Done as draft PR #1 at `a0c25ff`.
  Later pushes need a fresh go-ahead.
- **Not authorized:** merges; pushes; invitations; the lifecycle pass; retiring
  frozen experiments; declaring a current-state read on the mutations.
- **Decided in this chunk:** the choices in the decision record's "Reference
  application checkpoint B, client" entry.
- **Workflow:** the owner asked for oracle review before every commit.

## 9. Open questions for the user

Ask one at a time.

- May the driver push the local commits (seven, plus this handoff) to draft PR
  #1, so GitHub Actions runs them? This was asked at the start of this session
  and has not been answered. It blocks any CI claim for checkpoint B.
- Should `change_role` and `remove_member` declare `listProjectMembers` as their
  current-state read (`x-iris` `recovery.read`; S15 and S17 say "may")? It
  blocks remaining-work item 3. There is no firm recommendation. Declaring it
  matches S17's stated intent. But a page describes present state only (S14), so
  the wording must not imply that the page resolves an earlier attempt.
- Which follow-up should come next: the current-state read (if approved), the
  invitations design, or the lifecycle pass? This blocks the next chunk.

## 10. Operational state

- **Running processes:** none. No servers or browser sessions (checked: ports
  4001, 3003 and 5175 free; `agent-browser session list` empty). The browser
  workflow, the probes and every mutation runner stop what they start. The
  in-place mutation runners restored their files, confirmed by SHA-256.
- **Draft PR #1:** open at `a0c25ff`, with passing run 36317010742. Don't merge
  it.
- **`agent-browser`:** 0.38.1 is in mise's Node 24 global bin, with Chrome 154
  in `~/.agent-browser/browsers/`. Under another Node it may not be on `PATH`.
- **Gitignored build output:** `target/`, and `node_modules` and `dist` under
  both web directories. Nothing needs cleanup.
- **Retained evidence:** temporary, and possibly already deleted: the driver's
  scratchpad (section 5), and any review directories the oracle created under
  the system temp directories.
- **Known risk, carried forward:** `probe:s16` builds into the checkout's shared
  `target/` and can leave a mutated artifact that a later run treats as current.
  The experiment is frozen, so the risk is recorded rather than fixed.

## 11. Conventions and gotchas

- **Shell aliases:** in interactive shells `tr` is a trash command, `npm` a
  package guard and `ls` another tool. Use `command tr`, `command npm` and
  `/bin/ls`.
- **zsh:** no word-splitting of `$VAR` into a command; no `PIPESTATUS`; a bare
  `=====` word triggers `=` expansion (use quotes).
- **Formatting:**
  - A user-level hook formats Markdown written through the driver's Write and
    Edit tools, including scratch prompt files. Exact-string patches applied to
    such a file afterwards can silently miss. Append to it or rewrite it.
  - Scripted edits bypass the hook, so run
    `bunx prettier@3.9.9 --write --print-width 80 --prose-wrap always <files>`.
    Pin the version: bare `bunx prettier` resolves the latest release, and 3.8.3
    formats `client.ts`'s `Result` union differently.
  - TypeScript under `apps/reference/web/src` is Prettier-formatted, except
    `generated.ts`, which is never formatted. `style.css` keeps its compact,
    unformatted style.
- **Contract changes need two regenerations:** `export-openapi`, then
  `npm --prefix apps/reference/web run generate`.
- **Every new or changed operation touches the client in the same step:**
  - `client.ts`: `DOMAIN_OPERATIONS`, `METHODS`, and `MUTATIONS` or `READS` (a
    test checks they partition the operations);
  - `client.test.ts`: `NAMES`, `PATHS`, `CAPTURED` and `BODY_CASES`;
  - a presentation arm, since the switches are exhaustive.
- **Typed operation unions lose statuses:** `Known<Op>` or `Result<Op>` over a
  union of operations keeps only their shared statuses. Use explicit unions
  (`Result<"a"> | Result<"b">`) and per-operation overloads.
- **Two build paths:** the browser workflow's `vite build` does not type-check,
  so browser mutations need only bundle. `verify` runs `tsc`.
- **Nothing runs concurrently with in-place browser mutations:** they edit the
  checkout's client files and share the `dist` build and the workflow's ports.
  Don't run them beside `verify` or the browser workflow.
- **`agent-browser`:**
  - `network route` has only `--abort` and `--body`. Hold a response by wrapping
    `window.fetch` through `eval`, as the workflow does.
  - The request log includes aborted requests. `--filter /api/projects` also
    matches member paths, so count before opening a project.
  - `eval` prints JSON.
- **Driver tooling:** tool calls time out at 600 s, and a foreground `sleep` is
  blocked, so run long suites in the background with output in files.
- **Browser workflow ports:** 4001, 3003 and 5175 must be free. The API is
  restarted once, logging to `api.log`, then `api-b.log`.
- **Test fixtures and seeds:**
  - The development server seeds Bob as an editor of project 41.
  - The Rust tests and the workflow start the frozen issuer fixture
    `experiments/api-slice/checks/oidc-provider.mjs`.
  - Refer to test identities without gendered pronouns.
- **Rust, carried forward:**
  - A `///` comment on a `#[utoipa::path]` handler becomes the exported
    `summary`, so use `//`.
  - Test `validate` helpers accept anything for an undeclared status, so check
    `responses[status]` directly.
  - The shared test fixtures live in `http::memberships::{tests, list_tests}`.
    The previous handoff's section 11 (`git show 091129a:HANDOFF.md`) still
    holds the other server-side recipes: SQLite fault injection for 503, 500 and
    a busy `COMMIT`; rerunning `cargo fmt` before anchoring seeded mutations;
    libtest's quiet failure blocks; `cargo test --no-fail-fast`;
    `npm install --before=<date>`; JSON import attributes; and
    `vite preview --host 127.0.0.1`.
  - `verify` captures responses only from library tests whose names contain
    `whole_request`.
- **`crates/iris`:** add to it only what two operations demonstrably share,
  naming both.
- **History and pushes:** history on `main` is linear. Never push without the
  owner's go-ahead. The branch has an upstream, so a bare `git push` would
  update PR #1.
- **Panes:** the driver works in the left pane and the oracle in the right. The
  driver sends oracle prompts through a file.

## 12. Skills

- **Required:** `driver` for the driver, `oracle` for the oracle.
- **Optional:** `herdr` for pane and agent control.
