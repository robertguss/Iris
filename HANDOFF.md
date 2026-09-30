# Handoff

## 1. State

Observed September 30, 2026, by the outgoing driver.

- Repository: `/Users/robertguss/Projects/startups/Iris` (GitHub
  `robertguss/Iris`).
- Branch: `main`, tracking `origin/main`. The work was committed directly on
  `main`; `lifecycle-design` stays at `69c382b` and is no longer where work
  happens.
- Reviewed through `66d0608` (S18 step 3). This handoff is committed after it.
- Pushed through `66d0608` to `main`, by fast-forward with an explicit SHA and
  refspec, at the owner's instruction "go straight to main". This handoff is
  pushed the same way as part of the same chunk, in two commits: `9b838a5` went
  out without the handoff review's three P3 corrections, because the script
  applying them failed and the commit ran anyway; the commit after it carries
  them.
- CI: GitHub Actions run 36721898213 (Verify, push event, Ubuntu) on `66d0608`
  concluded `success` on its first attempt. All 25 steps report success. The log
  (`evidence/09-ci-log.txt`) shows the 72 library tests, including the new
  storage tests, the 8 development-binary tests, and both browser workflows
  passing. The API's per-step metadata was incomplete for some minutes after the
  run completed (steps 11 onward still pending) and caught up later. The log's
  two `##[error]` lines are the contract and S16 probes' intended seeded
  failures.
- PR #1 is merged (`8594777`). No pull request is open.
- The working tree was clean apart from this handoff before each of its commits.

Re-check HEAD, the working tree and the remote (`git status`, `git log -5`,
`git ls-remote origin`) before relying on any of this.

## 2. Read these first

- `docs/design-spec.md` S18 "Reference application lifecycle": the
  recommendations (8 for the next step), "Acceptance checks for a later
  implementation" (row "Tasks, shutdown"), "Storage and initialization evidence"
  and "Reset and migration evidence".
- `docs/decisions.md`: the three "Reference application lifecycle" entries
  (September 27 and 30).
- `apps/reference/src/storage.rs` and `apps/reference/src/storage/tests.rs`:
  `claim`, `open`, `reset`, the sidecar checks and the test-process guard at the
  end of `storage.rs`.
- `apps/reference/src/bin/reference-dev.rs` and
  `apps/reference/tests/dev_binary.rs`: where shutdown and session cleanup land.
- `apps/reference/README.md` "Run it" and "A database at a sidecar name".
- `docs/design-spec.md` S14 (caller loss) and S17 "Ownership boundaries", for
  steps 4 and 5.

## 3. Context

- **Why reset grew sidecar-name rules:** the plan review reproduced that a
  database created at another database's sidecar name (`dev.db-wal` beside
  `dev.db`, each with its own lock) would be deleted by a reset of `dev.db`, and
  that an initializer which had not published yet left only its `.iris-lock` and
  `.iris-init` siblings as evidence. Hence the refused names, the sibling checks
  that apply even when the sidecar is absent, and the occupant checks, all in
  the front half that `open` and `reset` share (`claim`).
- **Why the guide's manual procedure is so exact:** three plan re-reviews
  corrected it. Moving a database without its `-wal` loses committed changes
  (reproduced); `sqlite3 <db> .quit` does not clear a WAL (reproduced); a
  staging directory may be removed only after the checks `inspect_staging`
  makes, and a symbolic link to a directory passes naive checks (reproduced).
  Don't simplify that paragraph without re-deriving these.
- **Why library tests take a process guard:** the ownership lock belongs to the
  open file, so a child spawned by any thread shares every held lock until it
  execs. The oracle reproduced intermittent `InUse` failures twice: from the
  storage tests' own child processes, and from the membership fixture's Node
  spawn. Storage tests that spawn run exclusively for their whole length, the
  others shared, and the fixture takes the exclusive guard around its spawn.
- **Test-first was not followed in this step:** tests and implementation were
  written together and passed on the first run. S18 says so; the 43 mutations
  are the evidence that the tests detect the behavior. The oracle noted this as
  disclosed, not as compliant.
- **The owner said little this session:** "do it" and "whats next?" while work
  was in progress, and "go straight to main" in answer to whether this step
  should be pushed without a pull request.

## 4. Agreed chunk and acceptance

- **Objective:** S18 step 3, reset and migrations (recommendations 4 and 5;
  acceptance rows Reset and Migrations), as one reviewed commit; then this
  handoff.
- **Scope added in plan review:** sidecar-name ownership for both `open` and
  `reset`, and the guide's procedure for a database at a sidecar name. Added in
  diff review: shell quoting of the printed reset command, and the test-process
  guard.
- **Exclusions:** graceful shutdown, signal handling, session cleanup (step 4);
  the development command, `apps/reference/.dev/` and `.gitignore` (step 5);
  detecting a database created against another issuer; WAL adoption; backups; a
  confirmation prompt; any change to `migrations/0001_initial.sql`, the
  contract, the client, CI, dependencies or `experiments/`.
- **Stopping condition:** after step 3 and this handoff. The boundary did not
  move.
- **Disposition:** `accepted`. Step 3 is `66d0608`: `storage.rs` and its tests,
  `reference-dev --reset`, four new `dev_binary.rs` tests, the membership
  fixture's guard, README, S18 evidence, S10, S11 and S17 pointers, the decision
  entry, the change record, and the correction of the first step's stale CI
  statements (the previous handoff's item 11).

## 5. Verification and review

Environment: macOS (APFS), Rust 1.98.1, Node 24.20.0 for the browser workflow.
Evidence in
`/private/tmp/claude-501/-Users-robertguss-Projects-startups-Iris/b04871ba-05ce-4ae0-87cf-13f508c093a3/scratchpad/evidence/`
(temporary; below, `evidence/`). All results are for the tree committed as
`66d0608`.

| Claim              | Command and result                                                                                                                                                                                          | Evidence                                        | Checked by                                                                                             |
| ------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| Rust suite         | `cargo test --workspace --locked`: 171 passed, 0 failed (38 storage, 8 development-binary)                                                                                                                  | `evidence/01-cargo-test.txt`                    | Driver; the oracle ran the library tests (72) 20 times and, earlier, the 38 storage and 8 binary tests |
| No spawn race      | `cargo test --locked -p iris-reference --lib`, 40 consecutive default-parallel runs passed                                                                                                                  | not retained as a file                          | Driver; the oracle's 20 runs and a targeted probe with zero transient refusals                         |
| Lint and format    | `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`; `cargo fmt --all --check`: clean                                                                                           | `evidence/02-clippy.txt`, `03-fmt.txt`          | Driver; the oracle ran clippy on the reference crate and the format check                              |
| Client and browser | `npm --prefix apps/reference/web run verify`: PASS; `mise exec node@24.20.0 -- node apps/reference/scripts/browser.mjs --artifacts <dir>`: PASS                                                             | `evidence/05-web-verify.txt`, `08-browser.txt`  | Driver-reported                                                                                        |
| Mutations          | `mutate.py`: 43 of 43 caught, both controls green (12 repeating the first step's, 9 reset, 12 sidecar names, 5 migrations and message, 5 binary)                                                            | `evidence/04-mutations.txt`, `mutate.py`        | Driver-reported; the oracle ran two deletion-order variants itself                                     |
| Markdown           | Prettier 3.9.9 check clean; `links.py`: 174 local links, 0 broken; a seeded control reported its three breaks                                                                                               | `evidence/06-links.txt`, `07-links-control.txt` | Oracle reran Prettier and the link check                                                               |
| Linux              | GitHub Actions run 36721898213 on `66d0608`: success, first attempt, all 25 steps                                                                                                                           | `evidence/09-ci-log.txt`                        | Driver; the oracle compared the saved log with the live one                                            |
| Limits             | See S18 "Reset and migration evidence", Limits: plausibility checks only; one signed-in session through the issuer fixture, not the browser; the guide's Linux `stat` command and manual steps not executed | S18                                             | Oracle agreed at plan level                                                                            |

Oracle verdicts:

1. Plan: changes requested: one P1 (reset deletes a separately owned database
   through a sidecar name). Accepted.
2. Plan re-review: changes requested: two P1 (ownership siblings of an absent
   sidecar; rename guidance strands WAL data). Accepted.
3. Plan second re-review: changes requested: two P1 (staging directory's
   existence does not authorize deletion; the `sqlite3` shortcut). Accepted; the
   shortcut was removed.
4. Plan third re-review: changes requested: one P1 (a symbolic link passes the
   manual staging checks). Accepted.
5. Plan fourth re-review: sign-off.
6. Diff: changes requested: two P2 (reset command's path unquoted; the WAL test
   lost its files before the reset) and one P3 (wording on lock creation). All
   fixed.
7. Diff re-review: changes requested: one P2 (the spawn race made the storage
   suite flaky; recording it was not enough). Fixed with the process guard.
8. Diff second re-review: changes requested: one P2 (the membership fixture's
   Node spawn was outside the guard). Fixed.
9. Diff third re-review: sign-off, no findings.

None disputed, so none went to the owner.

## 6. Remaining work

1. **S18 step 4: shutdown and session cleanup** (recommendation 8; acceptance
   row "Tasks, shutdown"): SIGINT and SIGTERM handling, graceful shutdown with
   an inner deadline inside the supervisor's 5 s kill, a supervised periodic
   session cleanup whose panic stops the process, pools closed before the lock
   is released.
2. **S18 step 5: the development command** (recommendation 6; row Command),
   including `apps/reference/.dev/` in `.gitignore`; its reset invokes
   `reference-dev --reset`.
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
8. **Caller loss beyond the browser:** a real disconnect or cancellation during
   a mutation (S14's focused validation items 2 and 3) is still untested.
9. **Detecting a database created against another issuer:** documented in the
   README only.
10. **README verification matrix:** only its development server, development
    database and workspace rows were remeasured on September 30; the rest date
    from September 26–27.

## 7. Next chunk

`proposed`. Implementation of steps 4 and 5 is authorized; the chunk's shape is
not yet agreed.

- **Proposal:** S18 step 4, shutdown and session cleanup, then a handoff.
- **Acceptance:** S18's "Tasks, shutdown" acceptance row, with tests, mutation
  checks and the browser workflow unchanged.
- **First action:** write the step 4 plan and send it for plan review. Write the
  tests first this time and record the failing run.

## 8. Decisions and authorizations in force

- **The seven S17 owner decisions** in S17's "Owner decisions".
- **The ten S18 decisions** in S18's "Lifecycle owner decisions" (choice 1 by
  the owner; 2–10 by the oracle at the owner's request).
- **Authorized:** implementing S18 in reviewed steps: storage and initialization
  (done), reset and migrations (done), shutdown and session cleanup, the
  development command.
- **Pushes:** the owner said "go straight to main" for this chunk, in answer to
  whether this step should be pushed without a pull request; that was done. It
  was not stated as a standing rule, so every later push needs a fresh go-ahead.
  CI reruns need the owner.
- **Not authorized:** any further push or merge; a new pull request; deleting
  `s17-checkpoint-a`, `lifecycle-design` or `docs/s17-reference-app`;
  invitations; retiring frozen experiments; editing `experiments/`; editing PR
  #1.
- **Decided in earlier chunks:** the decision record's dated entries.
- **Workflow:** the owner asked for oracle review before every commit. PR text
  is reviewed before posting. Ask the owner one question per message.

## 9. Open questions for the user

- Is "go straight to main" the rule for later chunks too, or only for this one?
  Blocks the next push.
- Is the next chunk (step 4 alone, then a handoff) the right size, or should
  steps 4 and 5 run in one chunk? Blocks only the chunk boundary; the plan
  review can settle it if the owner has no preference.

## 10. Operational state

- **Running processes:** none. The browser workflow, the mutation runs and every
  test child finished.
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
    `/private/tmp/claude-501/-Users-robertguss-Projects-startups-Iris/b04871ba-05ce-4ae0-87cf-13f508c093a3/scratchpad/`:
    `evidence/01`–`09` (`09-ci-log.txt` is the CI log), plan and review prompts
    `01`–`10`, `links.py`, `mutate.py`, `browser-artifacts/`, `control/` (a
    `git archive` copy, safe to delete), `mutants/` and `mutants-target/` (a
    source copy and its Cargo target, safe to delete);
  - the oracle's `/private/tmp/iris-s18-step3-*`,
    `/private/tmp/iris-s18-diff-rereview2-3l198apt/` and
    `/private/tmp/iris-s18-diff-rereview3-ougu01jz/` directories (probe scripts
    and logs for each finding).
  - Earlier sessions' scratchpads, including the previous `links.py` and
    `mutate.py`, are gone; both were rewritten this session.
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
- **Link checking:** `links.py` (in this session's scratchpad, section 10;
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
- **Mutation runner for storage:** `mutate.py` in this session's scratchpad
  (section 10) copies `git ls-files -co --exclude-standard` to a scratch
  directory, uses its own `CARGO_TARGET_DIR`, applies one exact-string mutation
  at a time to `storage.rs` or `reference-dev.rs` and runs
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
- **Holding the guard across awaits** is intended: each `#[tokio::test]` owns a
  single-threaded runtime. `storage/tests.rs` allows
  `clippy::await_holding_lock` for that reason.
- **Mutation runner and stale artifacts:** `mutate.py` copies the checkout with
  its modification times, so Cargo can reuse a binary built from the previous
  run's last mutant when only test files changed. The runner touches the two
  mutated sources before its control run; its control caught this once. Keep the
  control and the touch.
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

## 12. Skills

- **Required:** `driver` for the driver, `oracle` for the oracle.
- **Optional:** `herdr` for pane and agent control.
