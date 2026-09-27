# Handoff

## 1. State

Observed September 27, 2026, by the outgoing driver.

- Repository: `/Users/robertguss/Projects/startups/Iris` (GitHub
  `robertguss/Iris`).
- Branch: `s17-checkpoint-a`, tracking `origin/s17-checkpoint-a`. `main` is
  unchanged at `9235c6e`, locally and on the remote.
- Reviewed through `0b50249` (step 1 of this chunk). Step 2 changed no
  repository file. This handoff is committed after `0b50249`.
- Pushed through `0b50249`, which is `origin/s17-checkpoint-a` and the head of
  pull request #1 (https://github.com/robertguss/Iris/pull/1). The owner
  approved that push this session. This handoff's own commit is not pushed.
- PR #1 is open and **ready for review** (no longer a draft), titled "S17
  reference application: checkpoints A and B and the current-state read",
  mergeable, not merged. Its description holds merge notes for the owner.
- The working tree was clean apart from this handoff before its commit.

Re-check HEAD, the working tree and the remote (`git status`, `git log -5`,
`git ls-remote origin`, `gh pr view 1`) before relying on any of this.

## 2. Read these first

- `docs/design-spec.md`: "How to interpret and maintain this spec", S10, S11,
  then S17: "Operation sequence" (its follow-up row), "Ownership boundaries"
  (the "Deferred to a lifecycle pass" bullet), "Owner decisions" and
  "Current-state read evidence".
- `docs/decisions.md`: "Open decisions" (annotated this session), then the last
  two entries, "Reference application current-state read" and "Documentation
  hygiene".
- PR #1's description: the "Merging (not yet authorized)" section.
- `apps/reference/README.md`: commands, operations, verification matrix, probes
  and limits.
- `docs/design-review-brief.md`: how independent design reviews are run, if the
  next chunk is a design.
- `.github/workflows/verify.yml`: what CI runs.

## 3. Context

The owner answered the previous handoff's two questions, one at a time: the next
chunk is documentation hygiene, and PR #1 should be prepared for merging ("Get
it ready (description, marked ready for review) but don't merge. Merging stays
unauthorized until you say so."). Both answers came from options the driver
offered; the hygiene option named the four items in the previous handoff's
remaining-work list.

- **The frozen experiment sentence:** that option named the API guide's "durable
  email delivery" item, so its choice was treated as authorization for that one
  documentation sentence in `experiments/api-slice/README.md`. The oracle agreed
  at plan review. It does not lift the general exclusion on editing
  `experiments/`.
- **Open decisions:** the record's rule is to keep superseded reasoning visible,
  so each original question stays word for word and gains a `_Status:_` clause
  linking the entry that settled or narrowed it. The clauses separate
  settlements for the experiments or the reference application from open
  framework choices. Two oracle P3s narrowed them: existence hiding is per
  concealed pair, and only the absence of an automatic transaction-retry policy
  is claimed, not that retries are never exercised.
- **CI records:** CI results live in S17's status paragraph, a dated decision
  entry and the reference guide's last limit. Run 36345360713 (at `12227aa`) was
  recorded in all three; dated evidence records that say "has not run" stay as
  written.
- **PR #1:** its description leads with the application's behavior and limits,
  groups the 26 commits, and gives a conditional recommendation: if merging is
  authorized, fast-forward only while `main` is an ancestor, which keeps `main`
  linear and keeps the SHAs the design record cites. GitHub's rebase merge would
  create new SHAs. `HANDOFF.md` is not on `main`, and any merge adds it. The PR
  head carries `fc3d17f`'s copy, not this one; a merge must target the revision
  the owner approves.

## 4. Agreed chunk and acceptance

- **Objective:**
  - the previous handoff's four hygiene items: the top-level README's next
    milestone, the API guide's email-delivery item, the settled open decisions,
    and the reference guide's CI status;
  - recording run 36345360713 where CI results are recorded;
  - preparing PR #1 for merging: push, CI on the exact head, a reviewed title
    and description, ready for review.
- **Exclusions:**
  - invitations, the lifecycle pass and retiring frozen experiments;
  - any code change, and any edit in `experiments/` beyond the one sentence;
  - the CI flake;
  - `docs/research.md`, `docs/first-experiment.md`,
    `docs/embedded-db-findings.md`, and the "Proposed architecture, not yet
    selected" table in `docs/decisions.md`;
  - dated evidence records;
  - merging, approving the PR, changing its base, labels or reviewers, rerunning
    CI without the owner, and pushes without the owner's go-ahead.
- **Stopping condition:** step 2 done (PR #1 ready for review, CI green on its
  head, not merged), then this handoff. The boundary did not move.
- **Disposition:** `accepted`:
  - step 1, `0b50249`: five documentation files (`README.md`,
    `experiments/api-slice/README.md`, `docs/decisions.md`,
    `apps/reference/README.md`, `docs/design-spec.md`);
  - step 2, no commit: the push, CI run 36348053700, and PR #1's new title,
    description and ready-for-review state.

## 5. Verification and review

Environment: macOS, `gh` against GitHub, Prettier 3.9.9 through `bunx`. Driver
evidence is in
`/private/tmp/claude-501/-Users-robertguss-Projects-startups-Iris/c354486e-9630-4d4b-9720-c4deb5e23ee7/scratchpad/`
(temporary; below, `scratchpad/`). No application suite was run locally this
session: no code, contract or generated file changed, as agreed at plan review.

| Claim                          | Commit and evidence                                                                                                                                                                                                                                             | Checked by                                                                                                 |
| ------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| Formatting                     | `0b50249`: `bunx prettier@3.9.9 --check --print-width 80 --prose-wrap always` on the five files passes; `git diff --check` clean                                                                                                                                | Oracle reran both                                                                                          |
| Links and anchors              | `0b50249`: `scratchpad/links.py` (GitHub-style slugs, fenced code skipped): 125 local links, 0 broken, including all 17 distinct new links. A copy with two seeded bad anchors reported both                                                                    | Oracle's own check: 125 links, 62 anchors, 0 broken                                                        |
| Status clauses and new entries | `0b50249`: each claim traced to a cited source (listed in `scratchpad/diff-step1.txt`)                                                                                                                                                                          | Oracle inspected the sources, including transaction boundaries and delivery retries                        |
| CI at `12227aa`                | Run 36345360713: attempt 1, every step successful, including all three reference browser workflows                                                                                                                                                              | Oracle inspected the run and its logs                                                                      |
| Push                           | `git ls-remote origin` and PR #1's `headRefOid` both `0b50249`                                                                                                                                                                                                  | Oracle verified                                                                                            |
| CI at `0b50249`                | Run 36348053700 (event `pull_request`): attempt 1, all 25 reported steps successful, including the agent-interface check and the reference browser workflow (`scratchpad/ci-36348053700.json`)                                                                  | Oracle verified the run and logs; it tested synthetic merge `d3003b3`, whose tree matches `0b50249`        |
| PR #1 text and state           | Title and body posted from the reviewed files (`scratchpad/pr-title.txt`, `pr-body.md`) after rechecking the head; the readback (`pr-body.posted.md`) is identical apart from the trailing newline; `isDraft` false, `mergeStateStatus` `CLEAN`, head `0b50249` | Oracle reviewed the text before posting, and verified the live title, body and state during handoff review |

Oracle verdicts and dispositions, all applied:

1. Chunk proposal and step 1 plan: sign-off.
   - P3: scope the existence-hiding clause per concealed pair. Applied.
   - It judged the frozen-experiment sentence authorized by the owner's choice,
     and the README intro, list entry and CI records in scope.
2. Step 1 diff: sign-off.
   - P3: claim only that the invitation and membership actions have no automatic
     transaction-retry policy (a shared test does retry acceptance after
     contention, and the delivery worker repeats after database failures).
     Applied.
   - P3: keep "disposable demo databases" beside the delivery experiment in the
     new decision entry. Applied.
3. Step 2 plan: sign-off, with advice, all followed: a conditional
   fast-forward-only recommendation; lead with behavior and limits; describe
   earlier reviews as recorded history; recheck the head before posting and read
   the body back; write inclusive commit ranges as "A through B"; use
   `gh run watch <id> --exit-status`.
4. Step 2 review of the PR text and CI record: sign-off.
   - P3: say that the four domain operations use the envelope and the session
     endpoints keep their contracts. Applied.
   - P3: distinguish plan and diff reviews of work steps from handoff reviews.
     Applied.

## 6. Remaining work

1. **The owner's choice of the next chunk (section 9).**
2. **The owner's merge decision for PR #1 (section 9),** including whether
   `main` should carry `HANDOFF.md`.
3. **A follow-up design for invitation issuance and acceptance** (S17's
   follow-up row).
4. **A lifecycle design pass:** persistent storage, seed policy, journal mode,
   worker supervision, one development command.
5. **Retiring frozen experiments:** S16 stays in CI until its remaining omission
   probes are carried by the reference application.
6. **The agent-interface CI flake:** it has not recurred (runs 36345360713 and
   36348053700 passed on the first attempt). If it does, diagnose the `members`
   scenario (`concurrent_last_owner_and_authority` in
   `experiments/embedded-db/sqlite/tests/members.rs`, run through
   `reproduce_scenario`). The experiment is frozen, so any fix needs the owner.
7. **Documentation hygiene, carried forward:** the "Proposed architecture, not
   yet selected" table in `docs/decisions.md` has the same staleness as the open
   decisions had (Axum, SQLx/SQLite and utoipa have since been exercised). It
   was excluded because neither the owner nor the handoff named it.
8. **Precompiled validators:** needed if a content security policy without
   `unsafe-eval` is adopted.
9. **Caller loss beyond the browser:** a real disconnect or cancellation during
   a mutation (S14's focused validation items 2 and 3) is still untested.

## 7. Next chunk

`proposed`. Nothing beyond this chunk is authorized.

- **First action:** ask the owner the open questions in section 9, one at a
  time, and wait for each answer before asking the next. Then propose the chunk
  the answers authorize, in the first plan review.
- **Acceptance:** set by that choice.
  - If the invitations design is chosen: a design section in S17's style,
    documentation only, reviewed through the design-review brief, authorizing no
    implementation until the owner says so.
  - If a merge is authorized: merge exactly the approved revision, as the owner
    directs (the PR recommends fast-forward only), then confirm `main` and the
    PR's state.

## 8. Decisions and authorizations in force

- **The seven S17 owner decisions** in S17's "Owner decisions".
- **Authorized and complete:** the CI prerequisite, checkpoints A and B, the
  mutations' current-state read, this documentation hygiene pass, and preparing
  PR #1 (ready for review). Commits go on `s17-checkpoint-a`.
- **Pushes:** the owner approved one push this session, through `0b50249`. Every
  later push needs a fresh go-ahead.
- **Not authorized:** merging PR #1 or anything into `main`; invitations; the
  lifecycle pass; retiring frozen experiments; editing `experiments/` (this
  session's one sentence was covered by the owner's choice, and nothing more).
- **Decided in earlier chunks:** the choices in the decision record's dated
  entries.
- **Workflow:** the owner asked for oracle review before every commit. This
  session also had the PR text reviewed before posting.

## 9. Open questions for the user

Ask one at a time.

- Which follow-up should come next: the invitations design, the lifecycle pass,
  or something else? This blocks the next chunk.
- Should PR #1 be merged into `main`, and if so, how, and with or without
  `HANDOFF.md`? This blocks the merge only. The PR is ready for review, its
  description recommends fast-forward only, and its head does not yet include
  this handoff's commit.

## 10. Operational state

- **Running processes:** none. The CI watcher exited (0). No servers or browser
  sessions (checked: ports 4001, 3003 and 5175 free;
  `agent-browser session list` empty).
- **PR #1:** open, ready for review, at `0b50249`. Don't merge it without the
  owner's go-ahead.
- **Local installs:** `experiments/agent-interface/node_modules` (gitignored) is
  still present from an earlier session.
- **Retained evidence:** temporary, and possibly already deleted: this session's
  `scratchpad/` (section 5; review prompts, replies, the CI record, the PR text
  and `links.py`), and the previous driver's scratchpad
  `/private/tmp/claude-501/-Users-robertguss-Projects-startups-Iris/8700f9fe-cfa6-4749-b371-63bb1793592f/scratchpad/`
  (about 2 GB, mostly `step1/mutation-target`, a Cargo target safe to delete).
  Any review directories the oracle created are under the system temp
  directories.
- **Known risk, carried forward:** `probe:s16` builds into the checkout's shared
  `target/` and can leave a mutated artifact that a later run treats as current.
  The experiment is frozen, so the risk is recorded rather than fixed.

## 11. Conventions and gotchas

- **Shell aliases:** in interactive shells `tr` is a trash command, `npm` a
  package guard and `ls` another tool. Use `command tr`, `command npm` and
  `/bin/ls`. A pipeline with a bare `tr '\n' ';'` tried to trash files named
  `\n` and `;` in an earlier session; it failed harmlessly only because none
  existed.
- **zsh:** no word-splitting of `$VAR` into a command; no `PIPESTATUS`; a bare
  `=====` word triggers `=` expansion (use quotes).
- **Formatting:**
  - A user-level hook formats Markdown written through the driver's Write and
    Edit tools, including scratch prompt files. Exact-string patches applied to
    such a file afterwards can silently miss. Append to it or rewrite it.
  - Scripted edits bypass the hook, so run
    `bunx prettier@3.9.9 --write --print-width 80 --prose-wrap always <files>`.
    Pin the version. `bunx` prints "Saved lockfile"; it writes nothing into the
    repository.
  - TypeScript under `apps/reference/web/src` is Prettier-formatted, except
    `generated.ts`, which is never formatted. `style.css` keeps its compact,
    unformatted style.
- **Contract changes need two regenerations:** `export-openapi`, then
  `npm --prefix apps/reference/web run generate`. `generate` runs the whole
  `verify` after writing, so a stale client test fails it; that does not mean
  generation failed. openapi-typescript drops `x-iris`, so recovery changes
  leave `generated.ts` unchanged.
- **Every new or changed operation touches the client in the same step:**
  - `client.ts`: `DOMAIN_OPERATIONS`, `METHODS`, and `MUTATIONS` or `READS` (a
    test checks they partition the operations);
  - `client.test.ts`: `NAMES`, `PATHS`, `CAPTURED`, `BODY_CASES`, and the
    recovery expectations;
  - a presentation arm, since the switches are exhaustive.
- **Recovery:**
  - `api.recovery` takes only a mutation, and construction refuses recovery on a
    read.
  - A new declared read needs `check_catalog` to pass, hand-written Rust and
    client expectations, and wording in `present.ts`'s `READ_AGAIN`, or the page
    will not point to it.
  - `check_catalog`'s linkage messages each name the source operation.
    `should_panic(expected = …)` tests match a substring of the message; the
    query/header/cookie regression case asserts the exact message.
- **Typed operation unions lose statuses:** `Known<Op>` or `Result<Op>` over a
  union of operations keeps only their shared statuses. Use explicit unions and
  per-operation overloads.
- **The directory stores only a page's `next` cursor**, not the cursor it was
  requested with. A browser case about cursors needs a shown page whose `next`
  is non-null.
- **Two build paths:** the browser workflow's `vite build` does not type-check,
  so browser mutations need only bundle. `verify` runs `tsc`.
- **Nothing runs concurrently with in-place browser mutations:** they edit the
  checkout's client files, share the `dist` build and the workflow's ports, and
  restore from the text they read at the start. An edit made meanwhile is lost.
- **Mutation runners:**
  - Rust mutations run on a disposable copy of `git ls-files` output, with a
    runner-owned `CARGO_TARGET_DIR`. Never use the checkout's `target/`.
  - Client runtime cases need fixtures. From the repository root, run
    `cargo test -p iris-reference --lib whole_request` with
    `IRIS_REFERENCE_FIXTURES=<absolute dir>`, then
    `node apps/reference/web/src/client.test.ts apps/reference/openapi.json <absolute dir>`.
  - `node:assert` `assert.throws(fn, regex, label)` reports the label, so label
    every case.
- **`agent-browser`:**
  - `network route` has only `--abort` and `--body`. Hold or withhold a response
    by wrapping `window.fetch` through `eval`, as the workflow does.
  - `network requests --json` gives `data.requests[].url`, absolute. The log
    includes aborted requests, and `--filter /api/projects` also matches member
    paths, so count from a baseline taken just before the action.
  - `eval` prints JSON.
  - 0.38.1 is in mise's Node 24 global bin, with Chrome 154 in
    `~/.agent-browser/browsers/`. It is not on `PATH` under Node 26.8.1, so run
    the workflow under Node 24, e.g.
    `mise exec node@24.20.0 -- node apps/reference/scripts/browser.mjs`.
- **Driver tooling:** tool calls time out at 600 s, and a foreground `sleep` is
  blocked, so run long suites in the background with output in files.
- **Browser workflow:** ports 4001, 3003 and 5175 must be free. The API logs to
  `api.log`, `api-b.log` and `api-c.log`. `restartApi(entry, name)` returns the
  replacement.
- **Test fixtures and seeds:**
  - The development server seeds Bob as an editor of project 41. Alice belongs
    only to 41; Bob also owns 43 ("Field notes").
  - The Rust tests and the workflow start the frozen issuer fixture
    `experiments/api-slice/checks/oidc-provider.mjs`.
  - Refer to test identities without gendered pronouns.
- **The agent-interface test** needs
  `npm --prefix experiments/agent-interface ci` before its first local run.
- **Rust, carried forward:**
  - A `///` comment on a `#[utoipa::path]` handler becomes the exported
    `summary`, so use `//`.
  - Test `validate` helpers accept anything for an undeclared status, so check
    `responses[status]` directly.
  - The shared test fixtures live in `http::memberships::{tests, list_tests}`.
    `git show 091129a:HANDOFF.md` section 11 holds the other server-side
    recipes.
  - `verify` captures responses only from library tests whose names contain
    `whole_request`.
- **`crates/iris`:** add to it only what two operations demonstrably share,
  naming both.
- **History and pushes:** history on `main` is linear. Never push without the
  owner's go-ahead. The branch has an upstream, so a bare `git push` would
  update PR #1.
- **Panes:** the driver works in the left pane and the oracle in the right. The
  driver sends oracle prompts through a file.
- **Scripted edits to wrapped Markdown:** Prettier's prose wrap moves line
  breaks, so an exact-string replacement can miss. Match with a
  whitespace-tolerant pattern (the words joined by `\s+`), assert exactly one
  match, then rerun Prettier. Python's `re.sub` expands escapes such as `\n` in
  the replacement string, so pass literal text as a function (`lambda _: text`).
- **Heading anchors:** GitHub's slugs drop the em dash and keep both spaces as
  hyphens: `## Experiment follow-up — September 24, 2026` is
  `#experiment-follow-up--september-24-2026`.
- **Prompt files:** writing the driver's prompt files as `.txt` avoids the
  Markdown formatting hook.
- **PR text:** write the title and body to files, post with
  `gh pr edit 1 --title "$(cat <file>)" --body-file <file>`, then read the body
  back with `gh pr view 1 --json body --jq .body` and diff it. Pin repository
  links in a PR body to a commit SHA so they do not drift.
- **After a push:** in this session, `refs/pull/1/head` in `git ls-remote` still
  showed the old head just after the push, while
  `gh pr view 1 --json headRefOid` already showed the new one; check the PR's
  head through `gh`. Verify also runs on pushes to `main`. The pull-request runs
  observed here were `pull_request` events on GitHub's synthetic merge commit,
  whose tree equals the head's while `main` is an ancestor. Watch a run in the
  background with `gh run watch <id> --exit-status`, then record each step with
  `gh run view <id> --json headSha,attempt,conclusion,jobs`.

## 12. Skills

- **Required:** `driver` for the driver, `oracle` for the oracle.
- **Optional:** `herdr` for pane and agent control.
