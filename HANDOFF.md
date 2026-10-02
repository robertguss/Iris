# Handoff

> Historical snapshot from September 30, 2026. Current work tracking, approvals,
> and next-task selection live in the
> [Iris Linear project](https://linear.app/robert-guss/project/iris-8b30c90a23d4).
> Do not update this file as a rolling queue. The dated environment and pending
> work below are historical; unsuperseded safety and authorization constraints
> still apply. See `AGENTS.md` for the current development workflow.

## 1. State

Observed September 30, 2026, by the outgoing driver.

- Repository: `/Users/robertguss/Projects/startups/Iris` (GitHub
  `robertguss/Iris`), branch `main` tracking `origin/main`.
- Reviewed through `89e1c7a` (S18 step 5, the development command). This handoff
  is committed after it.
- Pushed through `89e1c7a` to `main`, by fast-forward with an explicit SHA and
  refspec, under the owner's standing rule (section 8); this handoff is pushed
  the same way.
- CI: CI_LINE
- The previous handoff's commit `9e4fde5`: run 36762611726 succeeded.
- PR #1 is merged (`8594777`). No pull request is open.
- The working tree was clean apart from this handoff before its commit.

Re-check HEAD, the working tree and the remote (`git status`, `git log -5`,
`git ls-remote origin`) before relying on any of this.

## 2. Read these first

- `docs/design-spec.md` S18 "Reference application lifecycle": its status line,
  "Development command evidence" (the step just finished), and "Explicit
  exclusions". S18's authorized implementation is now complete.
- `docs/decisions.md`: the five "Reference application lifecycle" entries
  (September 27 and 30), the last being the development command.
- `apps/reference/scripts/dev.mjs`, `apps/reference/scripts/supervise.mjs` and
  `apps/reference/scripts/test/` (`dev.test.mjs` and the gated fixtures).
- `apps/reference/README.md` "Run it": the one-command start, its stop and exit
  rules, reset through the command, the tests, and the manual start.
- For choosing the next chunk: section 6 below, S14's focused validation items 2
  and 3, and S17's follow-up row on invitations.

## 3. Context

- **Why a stop exits 0 whatever the children's codes:** the API's exit 1 after
  an expired drain or unestablished closure is its documented forced exit (S18
  step 4), not a supervisor failure. The command prints each child's exit and
  reserves exit 1 for a child's unexpected exit, a readiness timeout, a failed
  build, a group that needed SIGKILL, and a reset a signal interrupted. The
  first cause of a stop is kept; later signals are only reported.
- **Why the handlers persist and are installed before the build:** the browser
  runner's `process.once` handlers let a second Ctrl-C kill the supervisor in
  the middle of cleanup, leaving detached children (the oracle reproduced it
  under Node 24.20.0 and 26.8.1). The browser runner keeps its own one-shot
  handlers on purpose, so its behavior did not change.
- **Why groups stay owned until empty:** the diff review showed a member that
  outlived its leader, or ignored SIGTERM after the leader exited, was left
  running while the stop reported clean. Ownership now ends when
  `process.kill(-pgid, 0)` says the group is gone.
- **Why readiness is tested with gated stand-ins:** a healthy run cannot show
  that each readiness is awaited. `gated-supervisor.mjs` runs the command's own
  supervision (`Supervisor`, `runSupervised`) over stand-ins that report
  readiness only when a gate file appears, so dev.mjs gains no test-only
  override.
- **Why `--database` exists on the command:** so the tests never touch a
  developer's data. It is an argument, never an environment variable, like the
  binary's. An empty value is a usage error: it once fell back to the default
  path, so `--database "" --reset` reset the default database.
- **Why ports stay fixed:** the seeded identities are bound to the issuer URL
  4001, 5175 is the console's origin and redirect URI, and S18 makes the command
  override inherited addresses. The owner asked whether a different port would
  do; the answer was that it needs a design change (a plan review and an S18
  amendment), and the owner chose to free the port instead.
- **What the owner said this session:** a takeover instruction to continue with
  the next chunk; a request for a status update; "cant you just run on a
  different port?"; and "kill it and run the remaining checks", after which the
  driver killed another session's Vite on 5175 (twice; see section 10).

## 4. Agreed chunk and acceptance

- **Objective:** S18 step 5, the development command (recommendation 6;
  acceptance row "Command"), as one reviewed commit; then this handoff.
- **Scope added in review:** plan review added persistent signal handlers
  installed before the build, the shared `startInOrder`/`runSupervised` with the
  gated fixtures, the build line, the cargo-shim test, and an ordered (d2) test
  asserting the final cause. Diff review added the empty-path refusal, group
  ownership until empty, the default directory for a reset, and absolute target
  resolution in the throwaway-checkout tests. The driver added, and the oracle
  accepted, an interrupted reset exiting 1 and the `dev: exit <code>: <cause>`
  final line.
- **Exclusions:** an npm script; port or address overrides; a CI change (so the
  command's tests are local only); watch-mode contract regeneration; a faster
  exit on a second signal; Windows; anything in Rust, the contract, the client,
  migrations or `experiments/`.
- **Stopping condition:** after step 5 and this handoff. The boundary did not
  move.
- **Disposition:** `accepted`. Step 5 is `89e1c7a`.

## 5. Verification and review

Environment: macOS (APFS), Rust 1.98.1, Node 24.20.0 and 26.8.1. Evidence in
`/private/tmp/claude-501/-Users-robertguss-Projects-startups-Iris/bd846a09-ef2b-4ffb-b99c-a06dcee93d0d/scratchpad/evidence/`
(temporary; below, `evidence/`). "Final tree" is `89e1c7a`; "pre-final" is the
tree just before its last two-line change (a relative `CARGO_TARGET_DIR`
resolved against the checkout in `dev.mjs` and the test fixture), which the
oracle judged needed no reruns.

| Claim                | Command and result                                                                                                                                                                                                                         | Tree                                          | Evidence                                                                                  | Checked by                                                                                    |
| -------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------- | ----------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------- |
| Failing first        | `node --test apps/reference/scripts/test/dev.test.mjs` against stubs: 18 of 18 failed (the four diff-review tests came later)                                                                                                              | stubs                                         | `evidence/00-failing-first.txt`                                                           | Driver-reported                                                                               |
| Command suite        | 22 passed under Node 24.20.0 and 26.8.1, and under `CARGO_TARGET_DIR=target`                                                                                                                                                               | final                                         | `evidence/01-dev-test.txt`, `01b-dev-test-node26.txt`, `01c-dev-test-relative-target.txt` | Driver; the oracle ran all 22 on both Node versions itself at the first re-review (pre-final) |
| Stability            | Ten consecutive runs, 22 passed each                                                                                                                                                                                                       | pre-final                                     | `evidence/03-repeat.txt`                                                                  | Driver; oracle inspected the log                                                              |
| Default path by hand | In this checkout: no `.dev` before; start created `apps/reference/.dev/reference.db`, ready, a session read through Vite 200, SIGINT exit 0; `--reset` exit 0; ignored by git. The directory was removed afterwards                        | pre-final                                     | `evidence/09-default-path.txt`                                                            | Driver; oracle inspected the log                                                              |
| Browser runner       | `mise exec node@24.20.0 -- node apps/reference/scripts/browser.mjs`: PASS after the move onto `supervise.mjs`                                                                                                                              | pre-final (runner and module unchanged since) | `evidence/08-browser.txt`, `browser-artifacts-final/`                                     | Driver; oracle inspected the logs                                                             |
| Mutations            | `mutate-dev.py --browser`: 38 of 39 caught, controls 22/22 before and after; survivor: `Supervisor.start`'s stop check before a spawn, unreachable from current callers; the browser workflow passed under all 23 shared-module mutants    | pre-final                                     | `evidence/04-mutations.txt`, `00-port-guard.txt`                                          | Driver-reported; oracle inspected the log                                                     |
| Rust, lint, client   | `cargo test --workspace --locked` 198 passed; clippy `-D warnings`; `cargo fmt --all --check`; `npm --prefix apps/reference/web run verify`; `node apps/reference/scripts/probes.mjs`: all pass. No Rust or client file changed afterwards | before the diff review                        | `evidence/02-cargo-test.txt`, `02b-clippy-fmt.txt`, `05-web-verify.txt`, `07-probes.txt`  | Driver-reported                                                                               |
| Markdown             | Prettier 3.9.9 clean on the edited files; `links.py`: 188 local links, 0 broken                                                                                                                                                            | final                                         | —                                                                                         | Driver-reported                                                                               |
| Linux                | CI_TABLE                                                                                                                                                                                                                                   | final                                         | `evidence/10-ci.json`, `10-ci-log.txt`                                                    | Driver                                                                                        |

Earlier, superseded runs are kept for the record in `evidence/pre-diff-review/`,
`evidence/disrupted-run/` (port 5175 taken mid-run by another session) and
`evidence/pre-rereview2/`, and the first mutation run in
`evidence/04a-mutations-first-run.txt`.

Oracle verdicts:

1. Plan: changes requested: two P2 (the extracted one-shot handlers contradict
   the repeated-signal policy; readiness and startup cancellation insufficiently
   tested) and two P3 (make "no build" observable; keep failing-first stubs
   uncommitted). All accepted.
2. Plan re-review: changes requested: two P2 (signal handling must begin before
   the build; (d2) raced signal observation against child-exit observation).
   Both accepted.
3. Plan second re-review: sign-off, with a nonblocking note that (d2) must
   assert the final cause, adopted.
4. Diff: changes requested: one P1 (an empty explicit path reset the default
   database) and two P2 (cleanup lost surviving process-group members; a default
   reset failed before the first start). All fixed with tests and mutations.
5. Diff re-review: changes requested: one P2 (a relative `CARGO_TARGET_DIR`
   misresolved in the throwaway checkout). Fixed.
6. Diff second re-review: sign-off, with one P3 (label the stability, browser
   and mutation evidence as preceding the last change), fixed before the commit.

None disputed, so none went to the owner.

## 6. Remaining work

1. **Choose the next chunk** (section 9). S18's authorized implementation is
   complete.
2. **The command's tests in CI:** they run only locally because S18 excludes a
   CI change. Running them in CI needs the owner; they need ports 4001, 3003 and
   5175 free and must not overlap the browser workflow step.
3. **Ubuntu timing gate for step 4's tests:** the binary's expiry test (2.9 s to
   4.5 s after the signal) and the library transaction child (600 ms plus 600
   ms) have now passed on Ubuntu in runs 36758026534 and 36762611726 (and
   36799909360, if it succeeded; section 1). If one flakes, diagnose from the
   signal's observation, the deadline and the child's exit timing before
   changing a bound; don't weaken the outer 5 s bound preemptively.
4. **Invitation issuance and acceptance design** (S17's follow-up row), which
   must also settle worker restart policy and send uncertainty per S18
   recommendation 8.
5. **Caller loss beyond the browser:** response loss on both sides of a commit
   and cancellation during a commit (S14's focused validation items 2 and 3) are
   still untested.
6. **Retiring frozen experiments:** S16 stays in CI until its remaining omission
   probes are carried by the reference application.
7. **The agent-interface CI flake:** if it recurs, diagnose
   `concurrent_last_owner_and_authority` in
   `experiments/embedded-db/sqlite/tests/members.rs`. Frozen; a fix needs the
   owner.
8. **Documentation hygiene, carried forward:**
   `experiments/embedded-db/README.md:64-66` describes Turso's migration as
   one-version (the migrator applies two since `2b1e820`); frozen, needs the
   owner. PR #1's merged description is stale; optional, needs the owner.
9. **Precompiled validators:** needed if a content security policy without
   `unsafe-eval` is adopted.
10. **Detecting a database created against another issuer:** documented in the
    README only.
11. **README verification matrix:** the development server, development
    database, lifecycle, development command and workspace rows date from
    September 30; the rest from September 26–27.
12. **Small known limits of the command** (S18's limits): a child's final output
    line without a trailing newline is not printed; the stop check before a
    spawn has no test that reaches it; a group refusing a probe (EPERM) is
    treated as gone.

## 7. Next chunk

`proposed`, pending the owner's choice (section 9). Candidates, in the driver's
order of preference:

- **S14 caller-loss validation** (section 6 item 5): response loss on either
  side of a commit and cancellation during a commit, as tests against the
  reference application. Acceptance: S14's focused validation items 2 and 3,
  tests first with the failing run kept, mutation checks.
- **Invitations design** (item 4): a design proposal, no implementation, under
  the design review brief.
- **The command's tests in CI** (item 2): a small CI change, if the owner
  authorizes it.

First action: ask the owner the question in section 9, then write that chunk's
plan and send it for plan review.

## 8. Decisions and authorizations in force

- **The seven S17 owner decisions** in S17's "Owner decisions".
- **The ten S18 decisions** in S18's "Lifecycle owner decisions" (choice 1 by
  the owner; 2–10 by the oracle at the owner's request).
- **Authorized and now done:** implementing S18 in reviewed steps (storage and
  initialization, reset and migrations, shutdown and session cleanup, the
  development command). Nothing further is authorized for implementation.
- **Pushes:** "Push to main, always": push each signed-off step and handoff
  straight to `main`, without asking, by explicit SHA and refspec. CI reruns
  still need the owner.
- **Killing another session's process:** the owner authorized killing the
  unrelated Vite on 5175 for this session's remaining checks only. It does not
  extend to later sessions; ask again.
- **Not authorized:** a new pull request; a CI change; deleting
  `s17-checkpoint-a`, `lifecycle-design` or `docs/s17-reference-app`;
  invitations; retiring frozen experiments; editing `experiments/`; editing PR
  #1.
- **Decided in earlier chunks:** the decision record's dated entries.
- **Workflow:** the owner asked for oracle review before every commit. PR text
  is reviewed before posting. Ask the owner one question per message.

## 9. Open questions for the user

- Which chunk next: S14 caller-loss validation, the invitations design, or the
  command's tests in CI (section 7)? Blocks the next chunk's plan.

## 10. Operational state

- **Running processes:** none of this project's. The CI poll ends when run
  36799909360 completes.
- **Port 5175:** another Claude session (the `wts-books-onix-rust-parser`
  project, a prototype Vite under its scratchpad `proto-wt/web`) repeatedly
  starts a Vite on 5175; the driver killed its process group twice with the
  owner's permission. It may be running again. The command, its tests and the
  browser workflow refuse to start while it holds the port; that is correct
  behavior, not a bug. Don't kill it without asking the owner again.
- **Remote:** `main` at this handoff's commit; `s17-checkpoint-a` at `8594777`;
  PR #1 merged. Don't edit or delete without the owner.
- **Local branches:** `main` tracking `origin/main`; `lifecycle-design` at
  `69c382b` (no upstream, behind `main`); `s17-checkpoint-a` at `8d2cfc7`, one
  ahead of its upstream; `docs/s17-reference-app` at `5ad417d` (older; leave
  it).
- **Local installs:** `experiments/agent-interface/node_modules` and
  `apps/reference/web/node_modules` (gitignored). No `apps/reference/.dev/`
  (removed after the hand run).
- **Retained evidence (temporary, possibly already deleted):**
  - this session's
    `/private/tmp/claude-501/-Users-robertguss-Projects-startups-Iris/bd846a09-ef2b-4ffb-b99c-a06dcee93d0d/scratchpad/`:
    `evidence/` (section 5), prompts `01-plan.txt` to `06-rereview2.txt`,
    `mutate-dev.py` (step 5's runner), `final-checks.sh` (the port-guarded run
    of every check), `default-path.mjs`, `regress.sh`, `impl/` (drafts, no
    longer needed), `browser-artifacts*/`, and `mutants-dev/` (a source copy and
    its Cargo target, safe to delete);
  - the oracle's `/private/tmp/iris-s18-step5-*` directories, if any (its
    reports and probes);
  - older sessions' scratchpads under
    `/private/tmp/claude-501/-Users-robertguss-Projects-startups-Iris/`
    (`00df25ad-…` holds `links.py` and step 4's `mutate.py`; `b04871ba-…` step
    3's evidence).
- **Known risk, carried forward:** `probe:s16` builds into the checkout's shared
  `target/` and can leave a mutated artifact that a later run treats as current.
  Frozen; recorded rather than fixed.

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
- **Link checking:** `links.py` (in the latest session's scratchpad, section 10;
  rewrite it if that is gone) takes the root and then Markdown paths relative to
  it, and applies GitHub-style slugs. A seeded control needs a full copy of the
  tree (`git archive HEAD | tar -x -C <dir>`, then the edited file copied over
  it); in a partial copy, links to files not copied are reported as broken.
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
- **History and pushes:** history on `main` is linear. Push only signed-off
  steps and handoffs, which the owner's standing rule (section 8) covers. In
  zsh, brace a variable before a colon in a refspec
  (`"${SHA}:refs/heads/main"`): `$SHA:r` is a history modifier that strips an
  extension and mangles the refspec. `s17-checkpoint-a` tracks
  `origin/s17-checkpoint-a`; `lifecycle-design` has no upstream. Push only by
  explicit SHA and refspec.
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
- **Oracle reviews that take long:** `herdr agent prompt --wait` can time out
  (it did once at 580 s on the design review) while the oracle keeps working;
  `herdr agent read` then refuses while the oracle is `working`. Run
  `herdr agent wait <oracle> --timeout 580000`, then read. For long reports, ask
  the oracle to write the report to a file and reply with its path.
- **Preserving a review verbatim:** add only a status note after the H1, as
  `docs/reviews/opus55-all-01.md` and `astra-s18-all-01.md` do, and check the
  body with `diff` against the original after Prettier.
- **Heading collisions:** S17 already has "Review of this proposal"; S18's is
  "Review of the lifecycle proposal" so the anchors stay unique.
- **Scripted section replacement:** S18 was drafted as unformatted text in a
  scratch file and spliced between `## S18` and
  `## References and design provenance`, then formatted. After the step 2 commit
  that scratch source no longer matters; edit the committed section directly.
- **Storage tests re-execute the test binary:** `kill_at` in
  `apps/reference/src/storage/tests.rs` runs the library's own test binary with
  `--exact storage::tests::interrupted_child`, blocks it at a `cfg(test)`
  barrier and kills it. libtest prints `test <name> ... ` without a newline, so
  the child's marker shares that line; match it with `ends_with`. The barrier
  names (`after-migrate`, `after-seed`, `after-link`, and `after-remove` inside
  a reset) and the injected failure exist only in the library's test build.
- **`reference-dev` prints errors by message:** `main` reports
  `reference-dev: <message>` and exits 1. Returning `Box<dyn Error>` from `main`
  prints the `Debug` form (e.g. `InUse(...)`), which tests matching the message
  miss. Its data line is printed before `listening on`, so readers that stop at
  the address line still see it.
- **Storage file names:** a database `<name>` owns `<name>.iris-lock` (never
  deleted, not even by a reset) and, while initializing, `<name>.iris-init/`
  holding `owner` and `reference.db`. Names ending in those suffixes, in any
  case, are refused as databases. `apps/reference/.dev/` is gitignored and is
  the development command's default data directory.
- **Mutation runner for storage:** `mutate-step3.py` in the latest session's
  scratchpad (section 10) copies `git ls-files -co --exclude-standard` to a
  scratch directory, uses its own `CARGO_TARGET_DIR`, applies one exact-string
  mutation at a time to `storage.rs` or `reference-dev.rs` and runs
  `cargo test -p iris-reference --lib storage` or `--test dev_binary`. A full
  run of 43 takes about 15 minutes. Exact strings break when the source changes;
  re-check each count.
- **Unset variables in shell calls:** shell state does not persist between tool
  calls, so a `$S` set in one call is empty in the next; `cat $S` then reads
  stdin and hangs until the tool times out. Set it in every call.
- **zsh globs:** an unmatched or huge glob such as `target/debug/deps/x-*` fails
  the whole command ("no matches found", "argument list too long"). Find test
  binaries with `cargo test --no-run --message-format=json` and the artifact's
  `executable` field.
- **Ownership locks and child processes:** the lock is on the open file, so a
  child spawned by any thread shares every held lock until it execs. A library
  test that spawns a process must take `crate::storage::exclusive()` around the
  spawn (storage tests that spawn take it for their whole length; the others
  take `shared()`). Integration tests under `tests/` are separate processes that
  hold no ownership locks, so spawn freely there. A new spawn in a library test
  without the guard shows up as intermittent `InUse` in unrelated tests.
- **Holding the guard across awaits** is intended: the guarded test future stays
  on its test thread, and the runtime flavor is chosen per test.
  `storage/tests.rs` and `lifecycle/tests.rs` allow `clippy::await_holding_lock`
  for that reason.
- **Mutation runner and stale artifacts:** `mutate.py` copies the checkout with
  its modification times, so Cargo can reuse a binary built from the previous
  run's last mutant when only test files changed. The runner touches the mutated
  sources before its control run; its control caught this once. Keep the control
  and the touch.
- **Run-time migrators in tests:** `sqlx::migrate::Migrator::new(<dir>)` over a
  temporary directory holding copies of `0001_initial.sql` gives the same
  checksum as the embedded `MIGRATOR`; `Storage::open_with` and `reset_with`
  take one. SQLx returns `VersionMismatch` before applying any later migration
  and writes nothing to the file.
- **A WAL-mode database loses its `-wal` and `-shm` when its last connection
  closes cleanly,** including a refused `Storage::open`. A test that needs
  crash-left WAL files must not open the database before using them.
- **Refusal messages are matched by tests:** `reserved`, `separate database`,
  `SQLite database`, `--reset`, `hard link`, `unrecognized`, `symbolic link`.
  The reset command in a message is shell-quoted; `dev_binary.rs` runs it as
  printed with the binary's directory on `PATH`.
- **`grep -c` with no match exits 1,** which makes a background command report
  failure even when the run it summarizes succeeded; read the output.
- **CI step metadata lags:** just after run 36721898213 completed,
  `gh run view --json jobs` still showed later steps as pending with the run and
  job already `success`; it caught up within minutes. `gh run watch` also exited
  non-zero early, before the job had started. Poll `status` until `completed`,
  then read the steps again, or read the log.
- **Lifecycle tests re-execute the test binary too:** `lifecycle::tests::child`
  is a no-op unless `IRIS_LIFECYCLE_CHILD` is set; the parent tests run it with
  `--exact lifecycle::tests::child --nocapture --test-threads=1`, parse
  `cookie=` and `listening=` from its stdout (it prints an empty line first to
  end libtest's own line), and send real signals with `kill`. With
  `IRIS_LIFECYCLE_BARRIER` set, a forced termination prints
  `barrier before-exit` and waits for a line on stdin, so the parent can check
  the lock while everything is still held. These tests hold
  `storage::exclusive()` for their whole length, and every child is killed and
  reaped if a test fails. The transaction parent uses a two-worker multi-thread
  runtime, because its request task must progress while the test thread blocks
  on the child; the two parents without concurrent request work use the default
  current-thread runtime.
- **Test-only mutation gate:** `domains::memberships::gate::before_commit(path)`
  holds every mutation of that database between its write and its commit. It
  matches the canonical path against `pragma_database_list`, so register the
  path as SQLite will report it (canonicalized).
- **Shutdown semantics a test or a supervisor must respect:**
  `lifecycle::finish` never returns; it calls `process::exit` on every path.
  Exit 0 needs a signal-requested stop that drained, had no failure and whose
  closure was acknowledged. Anything else is exit 1, and a forced exit holds the
  ownership lock until the process is gone. A failed tracked open anywhere in
  the process's life makes its stop a forced one. The browser runner and the
  development command both ignore the API's exit code when they stop it
  deliberately.
- **Domain connections go through `http::open`,** which returns a `Tracked`
  connection. Reads take `impl read::OwnedConnection` (a plain
  `SqliteConnection` or a `Tracked`, never a `&mut`). A production process
  constructs one `Connections` and shares clones of it through every `AppState`
  and into `Process` for `finish`; a second tracker would let `finish` see zero
  while handlers still hold tickets on the other.
  `connections: Default::default()` is right only where that one tracker is
  created, or for an isolated test fixture with its own.
- **Tokio timers round up:** an interval's first tick is not necessarily ready
  on the first poll. To reach a `select!` with a tick and the stop both ready,
  poll the task once by hand, let the tick come due unpolled, then publish the
  stop (`a_stop_is_seen_by_a_late_subscriber_and_wins_over_a_ready_tick`).
- **Mutants that hang:** a mutation that removes a deadline can hang a test with
  no bound of its own. `mutate.py` gives each suite 600 s, kills the mutant
  binaries on timeout and counts a hang as caught; keep mutant deadlines finite
  (for example 30 s rather than an hour) so bounded tests fail by themselves.
- **Heredocs and Python strings:** in an unquoted shell heredoc, `\\` becomes
  `\`, so a Python `\\n` meant as an escaped backslash-n reaches Python as `\n`
  and becomes a real newline; `$` and backticks are expanded too. Quote the
  heredoc delimiter (`<<'EOF'`), and dry-run a mutation list against the source
  before a long run.
- **The development command's tests own fixed ports:** they refuse to start
  while 4001, 3003 or 5175 is taken, and must never run alongside the browser
  workflow, the mutation runner or an oracle's own run of them. Before a long
  run, check `lsof -ti tcp:<port> -sTCP:LISTEN`; `final-checks.sh` (section 10)
  checks before every phase.
- **Supervisor output is asserted by tests:** every supervisor line starts
  `dev: `; children's lines are prefixed `[issuer]`, `[api]`, `[web]`,
  `[build]`, `[reset]`; the last line is
  `dev: exit <code>: <first cause>[; <names> needed SIGKILL]`;
  `dev: started <name> (pid N)` gives each child's process group, which the
  tests parse to kill everything after each test.
- **Node resolves a symbolic link before deciding the main module:** the issuer
  fixture serves only when run as the main module
  (`import.meta.url === pathToFileURL(argv[1])`), so a fixture reached through a
  linked directory exits 0 without listening. The throwaway checkout in
  `dev.test.mjs` copies it.
- **macOS `killpg` on a finished group can fail with EPERM** rather than ESRCH.
  Catch both (`mutate-dev.py`'s `kill_group`, `supervise.mjs`'s `alive`).
- **`agent-browser` leaves its session daemon outside the command's process
  group:** the group is empty once the command exits, so the browser runner's
  per-command groups drop out of ownership as before.
- **Vite's dev server prints `➜  Local:   http://127.0.0.1:5175/`** without
  colour when piped; readiness matches `Local:\s+http://127.0.0.1:5175/`.
- **Mutation runner for the command:** `mutate-dev.py` copies
  `git ls-files -co --exclude-standard` and links
  `apps/reference/web/ node_modules`, builds into its own `CARGO_TARGET_DIR`,
  stops a mutant at its first failing test (`✖` line) and then kills every
  process of the copy, the cargo shims (`iris-dev-test-`), the issuer and
  anything on the three ports. Check that each mutant is valid code: one early
  mutant left a trailing comma in `void ( …, )` and was "caught" as a syntax
  error. A full run of 39 with the browser workflow takes about 30 minutes.
- **`apps/reference/scripts/probes.mjs` is not Prettier-formatted;** leave it as
  is when formatting the other scripts (default Prettier, 80 columns).
- **Waiting in tool calls:** a foreground `sleep` is blocked; wait with
  `node -e 'setTimeout(()=>{},ms)'` or a polling `node -e` loop on a file, and
  keep each call under the 600 s tool limit.
- **`gh run watch` can die on a network timeout** while the run continues; poll
  `gh run view <id> --json status` instead.

## 12. Skills

- **Required:** `driver` for the driver, `oracle` for the oracle.
- **Optional:** `herdr` for pane and agent control.
