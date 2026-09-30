# Handoff

## 1. State

Observed September 30, 2026, by the outgoing driver.

- Repository: `/Users/robertguss/Projects/startups/Iris` (GitHub
  `robertguss/Iris`), branch `main` tracking `origin/main`.
- Reviewed through `a1d51da` (S18 step 4). This handoff is committed after it.
- Pushed through `a1d51da` to `main`, by fast-forward with an explicit SHA and
  refspec. The owner made pushing to `main` after sign-off a standing rule
  (section 8); this handoff is pushed the same way.
- CI: GitHub Actions run 36758026534 (Verify, push event, Ubuntu) on `a1d51da`
  concluded `success` on its first attempt, all 25 steps successful. Its log
  (`evidence/09-ci-log.txt`) shows every lifecycle test and the four new
  development-binary tests passing on Linux; its two `##[error]` lines are the
  contract and S16 probes' intended seeded failures. The previous chunk's
  handoff commits: run 36724475298 on `9b838a5` was cancelled (superseded by the
  next push), and run 36724537212 on `e8ea62f` succeeded on its first attempt.
- PR #1 is merged (`8594777`). No pull request is open.
- The working tree was clean apart from this handoff before its commit.

Re-check HEAD, the working tree and the remote (`git status`, `git log -5`,
`git ls-remote origin`) before relying on any of this.

## 2. Read these first

- `docs/design-spec.md` S18 "Reference application lifecycle": recommendation 6
  (the development command, next), the acceptance row "Command", and the
  sections "Shutdown and session cleanup evidence", "Reset and migration
  evidence" and "Storage and initialization evidence".
- `docs/decisions.md`: the four "Reference application lifecycle" entries
  (September 27 and 30).
- `apps/reference/src/lifecycle.rs` and `apps/reference/src/lifecycle/tests.rs`:
  the shutdown timeline, `Connections`/`Tracked`, `serve`, `finish`, and the
  child-process tests.
- `apps/reference/src/bin/reference-dev.rs`,
  `apps/reference/tests/dev_binary.rs` and `apps/reference/scripts/browser.mjs`:
  the binary and the one supervisor that exists; step 5 adopts the runner's
  process rules (S18 recommendation 6).
- `apps/reference/README.md` "Run it": the manual three-command start, the
  shutdown paragraphs, reset and the verification matrix.

## 3. Context

- **Why closure is acknowledged rather than assumed:** the first plan released
  the ownership lock after shutting the Tokio runtime down. The oracle's probe
  showed that establishes nothing about SQLx, whose SQLite connections run on
  their own threads: a second owner took the lock while a worker of the first
  was still inside an update. Hence the ticket per domain connection, taken
  before the open is awaited and released only by a successful awaited close,
  and the forced exit that keeps the lock whenever closure is not established.
  Don't replace it with `Runtime::shutdown_timeout` or a drop-based count.
- **Why a failed open poisons the stop:** SQLx can create a connection before
  `connect` returns an error, so a failed open leaves nothing to acknowledge.
  The owner-facing consequence (every later stop of that process exits 1 after
  the 1 s close deadline) was accepted in plan review and is documented in the
  README and S18. It applies to `http::open` only, not to a busy statement or
  the session pool.
- **Why `finish` owns the exit:** a mutation that discarded its returned code in
  the binary survived, since no binary test reaches a failed-but-closed stop.
  With `finish -> !`, the library child tests exercise the binary's own exit
  path.
- **Why tests could be written first this time:** the handoff before this one
  asked for it; the failing run against stub bodies is kept
  (`evidence/00-failing-first.txt`). Keep doing it.
- **The owner said little this session:** a takeover instruction to continue
  with the next chunk, and "Push to main, always" in answer to whether to push.

## 4. Agreed chunk and acceptance

- **Objective:** S18 step 4, shutdown and session cleanup (recommendation 8's
  process part; acceptance row "Tasks, shutdown"), as one reviewed commit; then
  this handoff.
- **Scope added in review:** plan review added tracked domain connections and
  the forced-exit policy, the outcome record that keeps cause, drain, failures
  and closure apart, and child-process tests with a test-only mutation gate and
  exit barrier. Diff review moved each task's factory inside its watched task.
  The driver narrowed reads to `impl read::OwnedConnection`, which the oracle
  accepted.
- **Exclusions:** the development command, `apps/reference/.dev/` and
  `.gitignore` (step 5); workers and invitations; request timeouts; busy-timeout
  changes; any change to the contract, the client, CI, migrations or
  `experiments/`; Windows; a faster exit on a second signal.
- **Stopping condition:** after step 4 and this handoff. The boundary did not
  move.
- **Disposition:** `accepted`. Step 4 is `a1d51da`.

## 5. Verification and review

Environment: macOS (APFS), Rust 1.98.1, Node 24.20.0 for the browser workflow.
Evidence in
`/private/tmp/claude-501/-Users-robertguss-Projects-startups-Iris/00df25ad-0f15-436c-99d4-44598fbde2a4/scratchpad/evidence/`
(temporary; below, `evidence/`). The driver's verification runs below are for
the tree committed as `a1d51da`, run after its last source edit; the
failing-first run and the oracle's own runs are earlier snapshots, labeled as
such in the table.

| Claim              | Command and result                                                                                                                                                                                                 | Evidence                                                             | Checked by                                                                                                                                                                             |
| ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Failing first      | Tests against stub bodies: 19 of 19 library and 4 of 4 new binary tests failed; three library tests were added later                                                                                               | `evidence/00-failing-first.txt`                                      | Oracle inspected the log                                                                                                                                                               |
| Rust suite         | `cargo test --workspace --locked`: 198 passed, 0 failed (22 lifecycle, 12 development-binary)                                                                                                                      | `evidence/01-cargo-test.txt`                                         | Driver; the oracle ran the library (93, before the diff fixes) and binary (12) suites at diff review, and the lifecycle suite (22 tests plus the no-op child entry point) at re-review |
| Stability          | 25 consecutive runs each of `cargo test -p iris-reference --lib` (95) and `--test dev_binary` (12), all green                                                                                                      | `evidence/03-repeat.txt`                                             | Driver; oracle inspected the log                                                                                                                                                       |
| Lint and format    | clippy `-D warnings` on the workspace; `cargo fmt --all --check`: clean                                                                                                                                            | `evidence/02-clippy.txt`, `03-fmt.txt`                               | Driver; the oracle ran clippy on the reference crate at diff review, before the final fixes, and the format check again at re-review                                                   |
| Client and browser | `npm --prefix apps/reference/web run verify`: PASS; the browser workflow under Node 24.20.0: PASS, and all three API logs show `received SIGTERM; draining for up to 3s` and `stopped after draining`              | `evidence/05-web-verify.txt`, `08-browser.txt`, `browser-artifacts/` | Driver-reported; oracle inspected the logs                                                                                                                                             |
| Mutations          | `mutate.py`: 39 of 39 caught, controls green before and after; `mutate-step3.py`: the previous step's 43 of 43                                                                                                     | `evidence/04-mutations.txt`, `04b-mutations-step3.txt`               | Driver-reported; oracle inspected the logs and ran its own factory-panic and native-worker probes                                                                                      |
| Markdown           | Prettier 3.9.9 check clean on the edited files; `links.py`: 226 local links, 0 broken                                                                                                                              | `evidence/06-links.txt`                                              | Driver-reported                                                                                                                                                                        |
| Linux              | GitHub Actions run 36758026534 on `a1d51da`: success, first attempt, all 25 steps; every lifecycle and new binary test passed once on Ubuntu                                                                       | `evidence/09-ci.json`, `09-ci-log.txt`                               | Driver                                                                                                                                                                                 |
| Limits             | See S18 "Shutdown and session cleanup evidence", Limits: configured budgets with a measured margin; one gated transaction per forced exit; `100 Continue` shows admission only; registration order source-reviewed | S18                                                                  | Oracle agreed                                                                                                                                                                          |

Oracle verdicts:

1. Plan: changes requested: one P1 (runtime shutdown does not establish SQLite
   closure before the lock is released) and two P2 (the outcome conflated cause
   with drain and closure; tests did not reach a started transaction or the
   close deadline). All accepted.
2. Plan re-review: changes requested: two P2 (the tracked connection could not
   reach the domain read functions; the close-timeout children were killed
   before their exit code was observed) and one P3 (the close-timeout message
   after a skipped close). All accepted.
3. Plan second re-review: sign-off, confirming the failed-open consequence.
4. Diff: changes requested: one P2 (a task's factory ran outside its supervised
   handle) and one P3 (evidence predating the final tree). Both fixed; the
   oracle's two nonblocking suggestions (a native-worker close regression, the
   README's "no response" wording) were also taken.
5. Diff re-review: sign-off, with one nonblocking wording note, fixed before the
   commit.

None disputed, so none went to the owner.

## 6. Remaining work

1. **S18 step 5: the development command** (recommendation 6; acceptance row
   "Command"), including `apps/reference/.dev/` in `.gitignore`. Its reset
   invokes `reference-dev --reset`; it stops the API with SIGTERM and must not
   treat the API's exit 1 during a deliberate stop as a failure.
2. **Ubuntu timing gate for step 4's tests:** the binary's expiry test asserts
   2.9 s to 4.5 s after the signal, and the library transaction child asserts
   its barrier within 600 ms plus 600 ms. Both passed once on Ubuntu in run
   36758026534; one run is not a stability result. If one flakes, diagnose from
   the signal's observation, the deadline and the child's exit timing before
   changing a bound; don't weaken the outer 5 s acceptance bound preemptively.
3. **Invitation issuance and acceptance design** (S17's follow-up row), which
   must also settle worker restart policy and send uncertainty per S18
   recommendation 8.
4. **Retiring frozen experiments:** S16 stays in CI until its remaining omission
   probes are carried by the reference application.
5. **The agent-interface CI flake:** if it recurs, diagnose
   `concurrent_last_owner_and_authority` in
   `experiments/embedded-db/sqlite/tests/members.rs`. Frozen; a fix needs the
   owner.
6. **Documentation hygiene, carried forward:**
   `experiments/embedded-db/README.md:64-66` describes Turso's migration as
   one-version (the migrator applies two since `2b1e820`); frozen, needs the
   owner. PR #1's merged description is stale; optional, needs the owner.
7. **Precompiled validators:** needed if a content security policy without
   `unsafe-eval` is adopted.
8. **Caller loss beyond the browser:** response loss on both sides of a commit
   and cancellation during a commit (S14's focused validation items 2 and 3) are
   still untested; step 4 covered one gated transaction completed and one
   abandoned at shutdown.
9. **Detecting a database created against another issuer:** documented in the
   README only.
10. **Stale CI statements from step 4:** S18's "Shutdown and session cleanup
    evidence" (its last limit) and the step 4 decision entry say Linux and
    GitHub Actions have not run and that the step is not pushed. Correct them in
    step 5 with run 36758026534, as step 4 corrected step 3's.
11. **README verification matrix:** the development server, development
    database, lifecycle and workspace rows were remeasured on September 30; the
    rest date from September 26–27.

## 7. Next chunk

`proposed`. Implementation of step 5 is authorized; the chunk's shape is not yet
agreed.

- **Proposal:** S18 step 5, the development command, then a handoff. That
  completes S18's authorized implementation.
- **Acceptance:** S18's "Command" acceptance row, with tests written first and
  their failing run kept, mutation checks, and the browser workflow and frozen
  experiments behaving as before.
- **First action:** write the step 5 plan and send it for plan review.

## 8. Decisions and authorizations in force

- **The seven S17 owner decisions** in S17's "Owner decisions".
- **The ten S18 decisions** in S18's "Lifecycle owner decisions" (choice 1 by
  the owner; 2–10 by the oracle at the owner's request).
- **Authorized:** implementing S18 in reviewed steps: storage and initialization
  (done), reset and migrations (done), shutdown and session cleanup (done), the
  development command.
- **Pushes:** asked whether to push this chunk, the owner answered "Push to
  main, always": push each signed-off step and handoff straight to `main`,
  without asking, by explicit SHA and refspec. CI reruns still need the owner.
- **Not authorized:** a new pull request; deleting `s17-checkpoint-a`,
  `lifecycle-design` or `docs/s17-reference-app`; invitations; retiring frozen
  experiments; editing `experiments/`; editing PR #1.
- **Decided in earlier chunks:** the decision record's dated entries.
- **Workflow:** the owner asked for oracle review before every commit. PR text
  is reviewed before posting. Ask the owner one question per message.

## 9. Open questions for the user

- Is the next chunk (step 5 alone, then a handoff) the right size? Blocks only
  the chunk boundary; the plan review can settle it if the owner has no
  preference.

## 10. Operational state

- **Running processes:** none. Every test child, mutation run and browser
  workflow finished; the CI watch ended when the run completed.
- **Remote:** `main` at this handoff's commit; `s17-checkpoint-a` at `8594777`;
  PR #1 merged. Don't edit or delete without the owner.
- **Local branches:** `main` tracking `origin/main`; `lifecycle-design` at
  `69c382b` (no upstream, behind `main`); `s17-checkpoint-a` at `8d2cfc7`, one
  ahead of its upstream; `docs/s17-reference-app` at `5ad417d` (older; leave
  it).
- **Local installs:** `experiments/agent-interface/node_modules` and
  `apps/reference/web/node_modules` (gitignored).
- **Retained evidence (temporary, possibly already deleted):**
  - this session's
    `/private/tmp/claude-501/-Users-robertguss-Projects-startups-Iris/00df25ad-0f15-436c-99d4-44598fbde2a4/scratchpad/`:
    `evidence/00`–`09`, plan and review prompts `01`–`05`, `mutate.py` (step 4),
    `mutate-step3.py` (step 3, one string adjusted), `links.py`, `docs.py` and
    `evidence-section.md` (the scripts that wrote this step's documents; no
    longer needed), `browser-artifacts/`, `mutants/` and `mutants-target/` (a
    source copy and its Cargo target, safe to delete);
  - the oracle's `/private/tmp/iris-s18-step4-plan-review.*`,
    `/private/tmp/iris-s18-step4-plan-rereview.*`,
    `/private/tmp/iris-s18-step4-diff-review.*` and
    `/private/tmp/iris-s18-step4-diff-rereview.*` directories (reports, probes
    and logs);
  - the previous session's scratchpad, with the step 3 evidence, still exists at
    `/private/tmp/claude-501/-Users-robertguss-Projects-startups-Iris/b04871ba-05ce-4ae0-87cf-13f508c093a3/scratchpad/`.
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
- **History and pushes:** history on `main` is linear. Never push without the
  owner's go-ahead. In zsh, brace a variable before a colon in a refspec
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
  case, are refused as databases. Don't use `apps/reference/.dev/` by hand
  before step 5 adds it to `.gitignore`.
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
  the process's life makes its stop a forced one. The browser runner already
  ignores exit codes when it stops the API deliberately; step 5's supervisor
  must do the same.
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

## 12. Skills

- **Required:** `driver` for the driver, `oracle` for the oracle.
- **Optional:** `herdr` for pane and agent control.
