# Handoff

## 1. State

Observed September 27, 2026, by the outgoing driver.

- Repository: `/Users/robertguss/Projects/startups/Iris` (GitHub
  `robertguss/Iris`).
- Branch: `s17-checkpoint-a`, tracking `origin/s17-checkpoint-a`. `main` is
  unchanged at `9235c6e`, locally and on the remote.
- Reviewed through `bdf8513`, this chunk's one step. This handoff is committed
  after `bdf8513`.
- Pushed through `0b50249`, which is `origin/s17-checkpoint-a` and the head of
  pull request #1 (https://github.com/robertguss/Iris/pull/1). Not pushed:
  `90caff5` (the previous handoff), `bdf8513`, and this handoff's commit. No
  push was authorized this session.
- PR #1 is open, ready for review, mergeable (`mergeStateStatus` `CLEAN`), and
  not merged. CI run 36348053700 passed on its head. GitHub Actions has not run
  `90caff5` or `bdf8513`.
- The working tree was clean apart from this handoff before its commit.

Re-check HEAD, the working tree and the remote (`git status`, `git log -5`,
`git ls-remote origin`, `gh pr view 1`) before relying on any of this.

## 2. Read these first

- `docs/decisions.md`: "Proposed architecture, not yet selected" (its status
  notes are this chunk's work), "Open decisions", then the last three entries:
  "Reference application current-state read", "Documentation hygiene" and
  "Architecture table status".
- `docs/design-spec.md`: "How to interpret and maintain this spec", S10, S11,
  then S17: "Operation sequence" (its follow-up row), "Ownership boundaries"
  (the "Deferred to a lifecycle pass" bullet) and "Owner decisions".
- PR #1's description: the "Merging (not yet authorized)" section.
- `apps/reference/README.md`: commands, operations, verification matrix, probes
  and limits.
- `docs/design-review-brief.md`: how independent design reviews are run, if the
  next chunk is a design.
- `.github/workflows/verify.yml`: what CI runs.

## 3. Context

The owner answered the previous handoff's two questions, one at a time, each
from options the driver offered:

- **Next chunk:** "Architecture table hygiene". The option read: update the
  stale "Proposed architecture, not yet selected" table in `docs/decisions.md`
  (Axum, SQLx/SQLite and utoipa have since been used); documentation only, like
  the last chunk.
- **PR #1:** "Not yet". The option read: keep PR #1 open and unmerged; the table
  fix goes on this branch, and the owner decides the merge once, over the final
  revision, after this chunk. That decision is now due (section 9). Putting the
  new commits on PR #1 needs a push, which needs its own go-ahead.

How the chunk was shaped:

- **Table notes:** the record keeps superseded reasoning visible, so the heading
  and all eight rows are unchanged. Below the table, each row gains a
  `_Status:_` bullet, in the style the open decisions received in `0b50249`. The
  statuses separate settlements for the experiments or the reference application
  from framework choices.
- **Turso migration history:** `experiments/embedded-db/README.md:64-66` still
  says Turso uses a "one-version migration". Since `2b1e820` the migrator
  (`experiments/embedded-db/turso/src/lib.rs:20-42`) applies both shared
  migrations. The oracle caught this at plan review. The table note states the
  current fact; the frozen README was not edited.
- **Spec untouched:** no design decision changed, so `docs/design-spec.md` has
  no change-record entry for this pass. The oracle judged this consistent with
  both documents' maintenance rules. The spec's "Last updated: September 26,
  2026" line predates its September 27 change-record entries; it was out of
  scope (section 6).

## 4. Agreed chunk and acceptance

- **Objective:** dated status notes for the eight rows of "Proposed
  architecture, not yet selected" in `docs/decisions.md`, and a dated entry
  recording the pass.
- **Exclusions:**
  - any code, contract, CI or generated change;
  - `docs/design-spec.md`, and any edit in `experiments/`;
  - `docs/research.md`, `docs/first-experiment.md`,
    `docs/embedded-db-findings.md`;
  - dated entries and the open decisions in `docs/decisions.md`;
  - merging, approving or editing PR #1, and any push;
  - the CI flake, invitations and the lifecycle pass.
- **Stopping condition:** step 1 committed after sign-off, then this handoff.
  The boundary did not move.
- **Disposition:** `accepted`: step 1, `bdf8513`, `docs/decisions.md` only (74
  lines added, none removed).

## 5. Verification and review

Environment: macOS, Prettier 3.9.9 through `bunx`, `gh` against GitHub. Driver
evidence is in
`/private/tmp/claude-501/-Users-robertguss-Projects-startups-Iris/730b5501-295c-4a77-86e8-91f6df19f8c8/scratchpad/`
(temporary; below, `scratchpad/`). No application suite was run: no code,
contract or generated file changed, as agreed at plan review.

| Claim             | Commit and evidence                                                                                                                                                                                                                                                           | Checked by                                                                        |
| ----------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------- |
| Rows unchanged    | `bdf8513`: `git diff --numstat` 74 added, 0 deleted; `git diff --word-diff=porcelain` has no deleted words (`scratchpad/diff-step1.patch`)                                                                                                                                    | Oracle: removing the two additions restores the base file byte for byte           |
| Formatting        | `bunx prettier@3.9.9 --check --print-width 80 --prose-wrap always docs/decisions.md` passes; `git diff --check` clean                                                                                                                                                         | Oracle reran both                                                                 |
| Links and anchors | `scratchpad/links.py docs/decisions.md`: 58 local links (38 before), 0 broken. On a `git archive` copy of the tree with the edited file, two seeded bad anchors were both reported                                                                                            | Oracle's own check: 58 links, 0 broken, both seeded anchors detected              |
| Status claims     | Each traced to its cited record and to source: `Cargo.lock` pins (not written into the notes), no `sqlx::query!`-family macro in the repository, session-store pools exercised by demo and test setup, the Turso migrator, domain `MemberSummary` against its HTTP conversion | Oracle checked the final wording against the cited records and the implementation |

Oracle verdicts and dispositions, all applied:

1. Chunk proposal and step 1 plan: changes requested.
   - P2: the Turso note carried forward stale migration history ("one-version
     migration", "later work did not use it"). Applied: the note says the custom
     migrator was since extended to the shared second migration and lacks SQLx's
     checksum and history tooling, and that the API slice, S16 and the reference
     application use SQLite.
   - P3: name the demonstrated boundary as domain/wire separation and explicit
     projections, not a persistence-model convention, and say "Project-member
     reads". Applied.
   - P3: say "the API slice's original invitation actions", since the frozen API
     now has four operations. Applied.
   - Advice, followed: "Session-store pools"; keep the new dated entry; leave
     the spec untouched. It judged the one-step boundary coherent.
2. Plan re-review: sign-off; all three findings resolved.
3. Step 1 diff: sign-off, no findings.

## 6. Remaining work

1. **The owner's decision on PR #1 (section 9):** whether to push `90caff5`,
   `bdf8513` and this handoff to PR #1, whether to merge, how, and whether
   `main` should carry `HANDOFF.md`.
2. **The owner's choice of the next chunk (section 9).**
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
7. **Documentation hygiene, carried forward:**
   - `docs/design-spec.md`'s "Last updated: September 26, 2026" line is older
     than its change record's September 27 entries.
   - `experiments/embedded-db/README.md:64-66` describes Turso's migration as
     one-version; the migrator applies two since `2b1e820`. The experiment is
     frozen, so an edit needs the owner.
8. **Precompiled validators:** needed if a content security policy without
   `unsafe-eval` is adopted.
9. **Caller loss beyond the browser:** a real disconnect or cancellation during
   a mutation (S14's focused validation items 2 and 3) is still untested.

## 7. Next chunk

`proposed`. Nothing beyond this chunk is authorized.

- **First action:** ask the owner the open questions in section 9, one at a
  time, in that order, and wait for each answer before asking the next. Then
  propose the chunk the answers authorize, in the first plan review.
- **Acceptance:** set by those answers.
  - If a push is authorized: push exactly the approved commits, confirm PR #1's
    head through `gh`, and watch CI on it to completion. A push-only answer
    stays push-only: it authorizes no merge.
  - If a merge is authorized: merge exactly the approved revision, named by its
    SHA, with the owner's chosen treatment of `HANDOFF.md` and as the owner
    directs (the PR recommends fast-forward only while `main` is an ancestor),
    then confirm `main` and the PR's state.
  - If the invitations design is chosen: a design section in S17's style,
    documentation only, reviewed through the design-review brief, authorizing no
    implementation until the owner says so.

## 8. Decisions and authorizations in force

- **The seven S17 owner decisions** in S17's "Owner decisions".
- **Authorized and complete:** the CI prerequisite, checkpoints A and B, the
  mutations' current-state read, the documentation hygiene pass, preparing PR #1
  (ready for review), and this architecture table pass. Commits go on
  `s17-checkpoint-a`.
- **PR #1:** stays open and unmerged. The owner decides the merge once, over the
  final revision, now that this chunk is done.
- **Pushes:** none authorized. The previous session's one push, through
  `0b50249`, is spent; every push needs a fresh go-ahead.
- **CI reruns:** rerunning a CI job or workflow needs the owner, even after an
  authorized push whose run fails. Watching and recording a run that a push
  triggers needs nothing further.
- **Not authorized:** merging PR #1 or anything into `main`; invitations; the
  lifecycle pass; retiring frozen experiments; editing `experiments/`.
- **Decided in earlier chunks:** the choices in the decision record's dated
  entries.
- **Workflow:** the owner asked for oracle review before every commit. PR text
  is reviewed before posting.

## 9. Open questions for the user

Ask one at a time, in this order: the answer to the first decides whether the
next chunk's commits go on this branch or on a new one from `main`.

- Should `90caff5`, `bdf8513` and this handoff be pushed to PR #1, and should PR
  #1 then be merged into `main`, and if so, how, and with or without
  `HANDOFF.md`? This blocks any push and the merge. The PR recommends
  fast-forward only; its head is `0b50249`. Offer choices that keep apart
  keeping the commits local, pushing only, and pushing then merging.
- Which follow-up should come next: the invitations design, the lifecycle pass,
  the remaining documentation hygiene (section 6, item 7), or something else?
  This blocks the next chunk.

## 10. Operational state

- **Running processes:** none. No servers or browser sessions (checked: ports
  4001, 3003 and 5175 free; `agent-browser session list` empty).
- **PR #1:** open, ready for review, at `0b50249`. Don't merge it, or push to
  its branch, without the owner's go-ahead.
- **Local installs:** `experiments/agent-interface/node_modules` (gitignored) is
  still present from an earlier session.
- **Local branches:** `docs/s17-reference-app` at `5ad417d` is an older branch;
  leave it.
- **Retained evidence:** temporary, and possibly already deleted, under
  `/private/tmp/claude-501/-Users-robertguss-Projects-startups-Iris/`:
  - this session's `730b5501-295c-4a77-86e8-91f6df19f8c8/scratchpad/` (section
    5: review prompts and replies, the diff, `links.py`, and `seed/`, a
    `git archive` copy of the tree used for the link control);
  - the previous session's `c354486e-9630-4d4b-9720-c4deb5e23ee7/scratchpad/`
    (the CI record for run 36348053700, the PR text);
  - `8700f9fe-cfa6-4749-b371-63bb1793592f/scratchpad/` (about 2 GB, mostly
    `step1/mutation-target`, a Cargo target safe to delete).

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
- **Link checking:** `links.py` (in this session's scratchpad) takes Markdown
  paths relative to the repository root and applies GitHub-style slugs. A seeded
  control needs a full copy of the tree (`git archive HEAD | tar -x -C <dir>`,
  then the edited file copied over it); in a partial copy, links to files not
  copied are reported as broken.
- **Frozen guides can be stale:** check the source before citing an experiment
  guide's description of its own implementation (see the Turso migration in
  section 3).
- **Reading oracle replies:** `herdr agent read` output can splice the echoed
  prompt's first line into the middle of the reply. Take the text after the last
  `Verdict:` line, and re-read if a finding looks cut.
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
- **After a push:** in an earlier session, `refs/pull/1/head` in `git ls-remote`
  still showed the old head just after the push, while
  `gh pr view 1 --json headRefOid` already showed the new one; check the PR's
  head through `gh`. Verify also runs on pushes to `main`. The pull-request runs
  observed here were `pull_request` events on GitHub's synthetic merge commit,
  whose tree equals the head's while `main` is an ancestor. Watch a run in the
  background with `gh run watch <id> --exit-status`, then record each step with
  `gh run view <id> --json headSha,attempt,conclusion,jobs`.

## 12. Skills

- **Required:** `driver` for the driver, `oracle` for the oracle.
- **Optional:** `herdr` for pane and agent control.
