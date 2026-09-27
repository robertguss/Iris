# Handoff

## 1. State

Observed September 27, 2026, by the outgoing driver.

- Repository: `/Users/robertguss/Projects/startups/Iris` (GitHub
  `robertguss/Iris`).
- Branch: `s17-checkpoint-a`, tracking `origin/s17-checkpoint-a`.
- Reviewed through `8594777`. This chunk changed no repository file; this
  handoff is committed after `8594777`.
- Pushed through `8594777`: `main`, `origin/main` and `origin/s17-checkpoint-a`
  are all at `8594777`. This handoff's commit is local only, so
  `s17-checkpoint-a` is one commit ahead of `main` and of its upstream. No push
  of it was authorized.
- PR #1 (https://github.com/robertguss/Iris/pull/1) is merged: `main` was
  fast-forwarded from `9235c6e` to `8594777` (29 commits, no merge commit), and
  GitHub marked the PR `MERGED` at 21:25:02 UTC with merge commit `8594777`.
- Verify passed on `8594777` both as the pull request (run 36351093637) and on
  `main` (run 36351648014), each on the first attempt.
- The working tree was clean apart from this handoff before its commit.

Re-check HEAD, the working tree and the remote (`git status`, `git log -3`,
`git ls-remote origin`, `gh pr view 1`) before relying on any of this.

## 2. Read these first

- `docs/decisions.md`: "Proposed architecture, not yet selected" (with its
  status notes), "Open decisions", then the last three entries: "Reference
  application current-state read", "Documentation hygiene" and "Architecture
  table status".
- `docs/design-spec.md`: "How to interpret and maintain this spec", S10, S11,
  then S17: "Operation sequence" (its follow-up row), "Ownership boundaries"
  (the "Deferred to a lifecycle pass" bullet) and "Owner decisions".
- `apps/reference/README.md`: commands, operations, verification matrix, probes
  and limits.
- `docs/design-review-brief.md`: how independent design reviews are run, if the
  next chunk is a design.
- `.github/workflows/verify.yml`: what CI runs.

## 3. Context

The owner answered three questions this session, one at a time, each from
options the driver offered:

- **Unpushed commits and PR #1:** "Push, then merge". The option read: push the
  three commits, watch CI, then merge PR #1 into `main`, fast-forward only as
  the PR recommends.
- **`HANDOFF.md` on `main`:** "Keep it on main". The option read: fast-forward
  `main` to `8594777` exactly as pushed, so `main` carries `HANDOFF.md`, which
  later handoffs keep overwriting.
- **Next chunk:** "Merge only". The option read: stop after the push and merge,
  and choose the follow-up in a later session. So no follow-up is chosen yet
  (section 9).

How the chunk was shaped:

- **Fast-forward, not a GitHub merge button:** "Rebase and merge" would create
  new SHAs, which the design record cites; "Squash" and "Create a merge commit"
  would change `main`'s linear history. The driver ran
  `git merge --ff-only 8594777` on local `main`, then pushed the explicit SHA to
  `refs/heads/main`. GitHub detected the PR's head on its base and marked the PR
  merged by itself; nothing was closed by hand.
- **PR body left stale, by agreement:** PR #1's description still has a "Merging
  (not yet authorized)" heading, says the branch is "26 commits ahead" of `main`
  (it was 29 at the merge), says the tracked `HANDOFF.md` is `fc3d17f`'s copy
  and that a newer handoff "is not pushed yet" (the merged copy is `8594777`'s).
  Its verification and head claims describe `0b50249`, not the merged head. The
  oracle agreed at plan review that updating it was not needed for the merge;
  any edit needs the owner and a reviewed text (section 6).
- **Handoff stays local:** no push was authorized beyond the three commits, so
  this handoff's commit exists only on the local `s17-checkpoint-a`.

## 4. Agreed chunk and acceptance

- **Objective:** push `90caff5`, `bdf8513` and `8594777` to PR #1, get CI green
  on that head, fast-forward `main` to `8594777`, confirm the PR is merged, and
  watch `main`'s push run.
- **Exclusions:** any repository file change in the step; any PR title, body,
  comment or approval; GitHub's merge buttons; branch deletion (the remote and
  local `s17-checkpoint-a` stay, as does local `docs/s17-reference-app`); CI
  reruns; any follow-up chunk work; pushing this handoff.
- **Stopping condition:** step 1 accepted after an evidence review, then this
  handoff. The boundary did not move.
- **Disposition:** `accepted`. Step 1 made no commit: its outcome is the remote
  state in section 1.

## 5. Verification and review

Environment: macOS, `git` and `gh` against GitHub. Raw evidence is in
`/private/tmp/claude-501/-Users-robertguss-Projects-startups-Iris/5f627447-348f-4cce-bab4-e70a69ceab52/scratchpad/`
(temporary; below, `scratchpad/`), in `evidence/` numbered 01 to 06, with the
review prompts beside it.

| Claim                 | Evidence                                                                                                                                                                                                                                                             | Checked by                                                                                  |
| --------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| Preconditions held    | `01-preconditions.txt`: HEAD `8594777` clean; remote topic `0b50249`; `main` and `origin/main` `9235c6e`, ancestors of HEAD; 29 commits, 0 merges. No branch protection or rulesets on `main`: the driver's and oracle's API checks at plan review, not in this file | Oracle: at plan review, and via reflog and range counts after                               |
| Push                  | `02-push.txt`: `0b50249..8594777` to `s17-checkpoint-a` by explicit SHA; `gh` `headRefOid` `8594777`                                                                                                                                                                 | Oracle, live                                                                                |
| CI green before merge | `03-pr-run.json`: run 36351093637, `pull_request`, head `8594777`, attempt 1, success; Socket checks pass (`03-pr-checks.txt`)                                                                                                                                       | Oracle: live records match; the synthetic merge checkout's tree equals `8594777`'s          |
| Fast-forward merge    | `04-merge.txt`: `origin/main` re-checked at `9235c6e`; `--ff-only` to `8594777`; push `9235c6e..8594777` to `main`                                                                                                                                                   | Oracle, via reflog and `ls-remote`                                                          |
| PR merged             | `05-confirm.txt`: all four remote refs at `8594777`; PR `MERGED`, merge commit `8594777`                                                                                                                                                                             | Oracle, live                                                                                |
| CI green on `main`    | `06-main-run.json`: run 36351648014, `push`, `main`, head `8594777`, attempt 1, success                                                                                                                                                                              | Oracle: all 25 steps succeeded in both runs; compiler-error annotations are expected probes |

No local test suite was run: no code changed, and CI ran the whole `verify`
workflow on the merged revision.

Oracle verdicts:

1. Chunk proposal and step 1 plan: sign-off, no findings. Advice, followed: keep
   the local `--ff-only` merge but push `main` by explicit SHA; record run ID,
   URL, event and workflow; a post-merge `main` failure would be recorded, not
   repaired; leave the PR body untouched and record its staleness; keep raw
   evidence.
2. Step 1 evidence: sign-off, no findings.

## 6. Remaining work

1. **The owner's choice of the next chunk (section 9).**
2. **A follow-up design for invitation issuance and acceptance** (S17's
   follow-up row).
3. **A lifecycle design pass:** persistent storage, seed policy, journal mode,
   worker supervision, one development command.
4. **Retiring frozen experiments:** S16 stays in CI until its remaining omission
   probes are carried by the reference application.
5. **The agent-interface CI flake:** it has not recurred (runs 36345360713,
   36348053700, 36351093637 and 36351648014 passed on the first attempt). If it
   does, diagnose the `members` scenario (`concurrent_last_owner_and_authority`
   in `experiments/embedded-db/sqlite/tests/members.rs`, run through
   `reproduce_scenario`). The experiment is frozen, so any fix needs the owner.
6. **Documentation hygiene, carried forward:**
   - `docs/design-spec.md`'s "Last updated: September 26, 2026" line is older
     than its change record's September 27 entries.
   - `experiments/embedded-db/README.md:64-66` describes Turso's migration as
     one-version; the migrator applies two since `2b1e820`. The experiment is
     frozen, so an edit needs the owner.
   - PR #1's merged description is stale (section 3). Optional; editing it needs
     the owner.
7. **Precompiled validators:** needed if a content security policy without
   `unsafe-eval` is adopted.
8. **Caller loss beyond the browser:** a real disconnect or cancellation during
   a mutation (S14's focused validation items 2 and 3) is still untested.

## 7. Next chunk

`proposed`. Nothing beyond this chunk is authorized.

- **First action:** ask the owner the open questions in section 9, one at a
  time, in that order, and wait for each answer before asking the next. Then
  propose the chunk the answers authorize, in the first plan review.
- **Acceptance:** set by those answers. If the invitations design is chosen: a
  design section in S17's style, documentation only, reviewed through the
  design-review brief, authorizing no implementation until the owner says so.
  Any push or pull request needs its own go-ahead.

## 8. Decisions and authorizations in force

- **The seven S17 owner decisions** in S17's "Owner decisions".
- **Authorized and complete:** the CI prerequisite, checkpoints A and B, the
  mutations' current-state read, the documentation hygiene pass, the
  architecture table pass, and pushing and fast-forward merging PR #1 with
  `HANDOFF.md` kept on `main`.
- **Pushes:** none authorized. This session's push to `s17-checkpoint-a` and
  `main`, through `8594777`, is spent; every push needs a fresh go-ahead.
- **CI reruns:** rerunning a CI job or workflow needs the owner. Watching and
  recording a run that a push triggers needs nothing further.
- **Not authorized:** any further push or merge into `main`; a new pull request;
  deleting `s17-checkpoint-a` locally or on the remote; invitations; the
  lifecycle pass; retiring frozen experiments; editing `experiments/`; editing
  PR #1.
- **Decided in earlier chunks:** the choices in the decision record's dated
  entries.
- **Workflow:** the owner asked for oracle review before every commit. PR text
  is reviewed before posting.

## 9. Open questions for the user

Ask one at a time, in this order.

- Which follow-up should come next: the invitations design, the lifecycle pass,
  the remaining documentation hygiene (section 6, item 6), or something else?
  This blocks the next chunk.
- Where should the next chunk's commits go: on `s17-checkpoint-a` (which already
  carries this handoff's local commit, one ahead of `main`), or on a new branch
  from `main` (which this handoff commit would then have to join, be
  cherry-picked onto, or be left behind)? And is a new pull request wanted? A
  third option: a new branch at the local handoff commit, which is already one
  commit above `main`; that keeps the handoff without cherry-picking or pushing.
  This blocks the next chunk's plan review, as section 7 says.

## 10. Operational state

- **Running processes:** none. No servers or browser sessions were started this
  session; both CI watches finished.
- **PR #1:** merged at `8594777`. Don't edit it without the owner.
- **Remote branches:** `main` and `s17-checkpoint-a`, both at `8594777`. Don't
  delete `s17-checkpoint-a` without the owner.
- **Local installs:** `experiments/agent-interface/node_modules` (gitignored) is
  still present from an earlier session.
- **Local branches:** `main` at `8594777`, tracking `origin/main`;
  `docs/s17-reference-app` at `5ad417d` is an older branch; leave it.
- **Retained evidence:** temporary, and possibly already deleted, under
  `/private/tmp/claude-501/-Users-robertguss-Projects-startups-Iris/`:
  - this session's `5f627447-348f-4cce-bab4-e70a69ceab52/scratchpad/` (section
    5: evidence, review prompts);
  - the previous session's `730b5501-295c-4a77-86e8-91f6df19f8c8/scratchpad/`
    (the architecture table review, `links.py`, `seed/`);
  - `c354486e-9630-4d4b-9720-c4deb5e23ee7/scratchpad/` (the CI record for run
    36348053700, the PR text);
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
- **Link checking:** `links.py` (in the `730b5501…` scratchpad, section 10)
  takes Markdown paths relative to the repository root and applies GitHub-style
  slugs. A seeded control needs a full copy of the tree
  (`git archive HEAD | tar -x -C <dir>`, then the edited file copied over it);
  in a partial copy, links to files not copied are reported as broken.
- **Frozen guides can be stale:** check the source before citing an experiment
  guide's description of its own implementation (see the Turso migration in
  section 6).
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
  owner's go-ahead. `s17-checkpoint-a` tracks `origin/s17-checkpoint-a`, so a
  bare `git push` would push this handoff's local commit to the merged PR's
  branch; push only by explicit SHA and refspec.
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
- **Merging by fast-forward:** pushing the PR's head SHA to `refs/heads/main`
  made GitHub mark PR #1 merged within seconds, with the head as its merge
  commit. While a pull-request run was in progress, `gh pr view` reported
  `mergeStateStatus` `UNSTABLE`. Here it reflected pending checks, but GitHub
  uses it for any mergeable head whose commit status is not passing, failed
  checks included; read the checks themselves. `git rev-parse --short` takes one
  revision; with several it fails with "Needed a single revision".

## 12. Skills

- **Required:** `driver` for the driver, `oracle` for the oracle.
- **Optional:** `herdr` for pane and agent control.
