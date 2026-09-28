# Handoff

## 1. State

Observed September 27, 2026, by the outgoing driver.

- Repository: `/Users/robertguss/Projects/startups/Iris` (GitHub
  `robertguss/Iris`).
- Branch: `lifecycle-design`, local only, with no upstream (created at `8d2cfc7`
  at the owner's choice in the previous chunk).
- Reviewed through `d0ae9d8`. This handoff is committed after it.
- This chunk's commits: `858bcf8` (S18 decisions recorded) and `d0ae9d8`
  (storage and initialization implemented), on top of the previous handoff
  `1c3aa65`.
- Pushed through `c810f91` (the previous handoff commit) to `main`, by
  fast-forward, at the owner's instruction "skip the pr and go straight to
  main". `origin/main` and local `main` include every commit through `c810f91`;
  the commit carrying this correction is pushed to `main` as well.
  `origin/s17-checkpoint-a` stays at `8594777`; local `s17-checkpoint-a` is at
  `8d2cfc7`, one ahead of its upstream. `lifecycle-design` still has no
  upstream; its commits reach GitHub only through `main`.
- PR #1 is merged (`8594777`). No new pull request exists; the owner chose to
  skip one.
- CI: GitHub Actions run 36364999959 (Verify, push event) on `c810f91` passed on
  its first attempt, every step green, including the new storage tests on Linux.
- The working tree was clean apart from this handoff before its commit. This
  correction of sections 1, 5, 6, 8, 9 and 10 after the push was made without an
  oracle review, at the owner's instruction ("fix handoff and don't worry about
  the review just do it now").

Re-check HEAD, the working tree and the remote (`git status`, `git log -5`,
`git ls-remote origin`) before relying on any of this.

## 2. Read these first

- `docs/design-spec.md` S18 "Reference application lifecycle": the
  recommendations, "Acceptance checks for a later implementation", "Lifecycle
  owner decisions" and "Storage and initialization evidence".
- `docs/decisions.md`: "Reference application lifecycle — September 27, 2026"
  (with its appended decisions paragraph) and "Reference application lifecycle
  storage — September 27, 2026".
- `apps/reference/src/storage.rs` and `apps/reference/src/storage/tests.rs`: the
  protocol and its tests; the next steps build on them.
- `apps/reference/src/bin/reference-dev.rs` and `apps/reference/README.md` ("Run
  it").
- `docs/design-spec.md` S14 (caller loss) and S17 "Ownership boundaries", for
  steps 3–5.

## 3. Context

- **How the ten S18 choices were settled:** the owner answered choice 1
  ("Disposable unless given an explicit path") from offered options, then, asked
  choice 2, replied "have the oracle answer your questions please." The oracle
  chose the recommended option for choices 2–10 and flagged none for the owner.
  S18 and the decision record disclose that the same model reviewed S18, so
  those nine are not an independent approval.
- **Authorization:** asked "With all ten S18 choices settled, is implementing
  S18 authorized?", the owner chose "Record, then implement (Recommended)": a
  reviewed documentation step, then implementation in reviewed steps (storage
  and initialization; reset and migrations; shutdown and session cleanup; the
  development command). Push and pull request were explicitly left out.
- **Why the storage protocol is elaborate:** step 2's plan went through two
  rounds of changes requested. The oracle reproduced with scratch probes that
  (a) unconditional cleanup of a `.init` sibling can delete another database,
  (b) publishing a new database beside a stale hot journal can replay old
  content into it, (c) symlink and hard-link aliases get separate sibling locks,
  (d) three-link recovery was admitted, and (e) APFS treats `.IRIS-INIT` and
  `.iris-init` as one name. The owned staging directory with an `owner` record,
  exact two-link recovery, sidecar refusal, canonical parent and case-folded
  reserved names answer those.
- **Pause:** the owner paused the session twice ("pause when you finish what you
  are working on"; later "I need to pause this project for the night"). The
  chunk was brought to its agreed stopping point before stopping. The oracle
  restart and the fresh driver (end-of-chunk steps 4 and 5) were not done; see
  section 10.

## 4. Agreed chunk and acceptance

- **Objective:** record S18's decisions and authorization (step 1); implement
  S18's storage and initialization slice: recommendations 1, 2 and 3 and the
  startup journal check of 7 (step 2); this handoff.
- **Exclusions:** reset command, migration refusal messaging, graceful shutdown,
  session cleanup, the development command and its `.gitignore` entry (steps
  3–5); detecting a database created against another issuer; any contract,
  client, CI, dependency or frozen-experiment change; push or pull request.
- **Stopping condition:** after step 2 and this handoff. The boundary did not
  move.
- **Disposition:** `accepted`.
  - Step 1, `858bcf8`: S18 status, "Lifecycle owner decisions" table (renamed
    from "Lifecycle choices for the owner"), pointers in the introduction, S10,
    S11, S17 and the decision record's open-decisions row; decision paragraph;
    change record.
  - Step 2, `d0ae9d8`: `storage.rs` (+ tests), `reference-dev --database PATH`,
    two new `dev_binary.rs` tests, README, S18 evidence, S10 row, decision
    entry, change record.

## 5. Verification and review

Environment: macOS (APFS), Rust 1.98.1, Node 26.8.1 for `verify`, Node 24.20.0
for the browser workflow. Evidence in
`/private/tmp/claude-501/-Users-robertguss-Projects-startups-Iris/a1086d40-571b-4bc2-91e9-0105bb9d9d75/scratchpad/`
(temporary; below, `scratchpad/`).

| Claim (step 2 unless noted) | Command and result                                                                                                                                                                                                            | Evidence                                        | Checked by                                                 |
| --------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------- | ---------------------------------------------------------- |
| Rust suite                  | `cargo test --workspace --locked`: 150 passed, 0 failed                                                                                                                                                                       | `evidence/03-cargo-test.txt`                    | Driver; the oracle reran the 21 storage and 4 binary tests |
| Lint and format             | `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`; `cargo fmt --all --check`: clean                                                                                                             | `evidence/04-clippy.txt`, `05-fmt.txt`          | Driver-reported                                            |
| Client and browser          | `npm --prefix apps/reference/web run verify`: PASS; `mise exec node@24.20.0 -- node apps/reference/scripts/browser.mjs --artifacts <dir>`: PASS (disposable path through `Storage`)                                           | `evidence/06-web-verify.txt`, `07-browser.txt`  | Driver-reported                                            |
| Mutations                   | `mutate.py`: 12 of 12 caught (journal check, lock, seed existing, hard link to rename, reclaim ×2, migrate existing, sidecar, symlink, link count, owner record, case-sensitive names)                                        | `evidence/08-mutations.txt`, `mutate.py`        | Oracle inspected, did not rerun                            |
| Markdown (both steps)       | Prettier 3.9.9 check clean; `links.py`: 156 (step 1), then 169 local links, 0 broken; step 1 seeded control reported all three breaks                                                                                         | `evidence/01-links.txt`, `02-links-control.txt` | Oracle reran Prettier and the link check                   |
| Limits                      | GitHub Actions run 36364999959 on Linux passed after the push to `main`; publication-after-close is source-reviewed only; a failure while opening a pool or connection is outside the closure guarantee; torn `owner` refused | S18 "Storage and initialization evidence"       | Oracle agreed with the limits                              |

Oracle verdicts:

1. Owner-delegated choices 2–10: all recommended; none flagged.
2. Step 1 plan and chunk: sign-off; one P3 (update the remaining S18 pointers),
   fixed.
3. Step 1 diff: sign-off, no findings.
4. Step 2 plan: changes requested: two P1 (staging cleanup ownership; stale
   sidecars) and four P2 (aliases; close before unlock; real interruption tests;
   existing-file migration test). All accepted.
5. Step 2 plan re-review: changes requested: one P1 (reserved names are not
   ownership; case-insensitive collision) and two P2 (three-link recovery;
   manual reset lacks exclusion). All accepted.
6. Step 2 plan second re-review: sign-off, with ordering rules (validate before
   deleting; database files before `owner`, then rmdir; never recursive), all
   followed.
7. Step 2 diff: sign-off; one P3 (narrow cleanup and closure wording in
   `storage.rs` and S18), fixed before commit.

None disputed, so none went to the owner.

## 6. Remaining work

1. **S18 step 3: reset and migrations.** A reset command in the Rust application
   (exclusive ownership through the same lock; refuses while an API holds it;
   removes the database and every sidecar, keeps `.iris-lock`), and the
   migration checksum refusal naming the path and version and offering
   restoration before the reset (recommendations 4 and 5; acceptance rows Reset,
   Migrations). Then update the README's "use a fresh path" guidance.
2. **S18 step 4: shutdown and session cleanup** (recommendation 8; row Tasks,
   shutdown).
3. **S18 step 5: the development command** (recommendation 6; row Command),
   including `apps/reference/.dev/` in `.gitignore`.
4. **Invitation issuance and acceptance design** (S17's follow-up row), which
   must also settle worker restart policy and send uncertainty per S18
   recommendation 8.
5. **Retiring frozen experiments:** S16 stays in CI until its remaining omission
   probes are carried by the reference application.
6. **The agent-interface CI flake:** not recurred. If it does, diagnose
   `concurrent_last_owner_and_authority` in
   `experiments/embedded-db/sqlite/tests/members.rs`. Frozen; a fix needs the
   owner.
7. **Documentation hygiene, carried forward:**
   `experiments/embedded-db/README.md:64-66` describes Turso's migration as
   one-version (the migrator applies two since `2b1e820`); frozen, needs the
   owner. PR #1's merged description is stale; optional, needs the owner.
8. **Precompiled validators:** needed if a content security policy without
   `unsafe-eval` is adopted.
9. **Caller loss beyond the browser:** a real disconnect or cancellation during
   a mutation (S14's focused validation items 2 and 3) is still untested.
10. **Detecting a database created against another issuer:** documented in the
    README only.
11. **Stale CI statements in the design record:** S18's "Storage and
    initialization evidence" and the decision record's storage entry say Linux
    and GitHub Actions were not run; run 36364999959 on `c810f91` has since
    passed. Correct them in the next reviewed step.

## 7. Next chunk

`proposed`. Implementation of steps 3–5 is authorized; the chunk's shape is not
yet agreed.

- **Proposal:** S18 step 3, reset and migrations, then a handoff.
- **Acceptance:** S18's Reset and Migrations acceptance rows, with tests,
  mutation checks and the browser workflow unchanged.
- **First action:** write the step 3 plan (building on `storage.rs`'s lock and
  names) and send it for plan review.

## 8. Decisions and authorizations in force

- **The seven S17 owner decisions** in S17's "Owner decisions".
- **The ten S18 decisions** in S18's "Lifecycle owner decisions" (choice 1 by
  the owner; 2–10 by the oracle at the owner's request).
- **Authorized:** implementing S18 in reviewed steps: storage and initialization
  (done), reset and migrations, shutdown and session cleanup, the development
  command.
- **Pushes:** the owner authorized pushing this chunk's work straight to `main`
  without a pull request; that was done. Every later push needs a fresh
  go-ahead. CI reruns need the owner.
- **Not authorized:** any further push or merge; a new pull request; deleting
  `s17-checkpoint-a`, `lifecycle-design` or `docs/s17-reference-app`;
  invitations; retiring frozen experiments; editing `experiments/`; editing PR
  #1.
- **Decided in earlier chunks:** the decision record's dated entries.
- **Workflow:** the owner asked for oracle review before every commit. PR text
  is reviewed before posting. Ask the owner one question per message.

## 9. Open questions for the user

- Should later chunks also go straight to `main` without a pull request? Blocks
  the next push.
- Is the next chunk (step 3 alone, then a handoff) the right size, or should
  steps 3–5 run in one chunk? Blocks only the chunk boundary; the plan review
  can settle it if the owner has no preference.

## 10. Operational state

- **Running processes:** none. The browser workflow and mutation runs finished;
  no stray `interrupted_child` or `reference-dev` processes remained.
- **Not yet done from end-of-chunk:** the oracle was not restarted and no fresh
  driver was started, because the owner paused for the night. Next session:
  restart the oracle and start a fresh driver from this file, or have the
  current pair continue.
- **Remote:** `main` at this handoff's commit (through `c810f91` and this
  correction); `s17-checkpoint-a` at `8594777`; PR #1 merged. Don't edit or
  delete without the owner.
- **Local branches:** `lifecycle-design` (this work, no upstream; pushed only
  through `main`); `s17-checkpoint-a` at `8d2cfc7`; `main` tracking
  `origin/main`; `docs/s17-reference-app` at `5ad417d` (older; leave it).
- **Local installs:** `experiments/agent-interface/node_modules` and
  `apps/reference/web/node_modules` (gitignored).
- **Retained evidence (temporary, possibly already deleted):**
  - this session's
    `/private/tmp/claude-501/-Users-robertguss-Projects-startups-Iris/a1086d40-571b-4bc2-91e9-0105bb9d9d75/scratchpad/`:
    `evidence/01`–`08`, prompts `01`–`07`, `links.py`, `mutate.py`,
    `browser-artifacts/`, `control/` (a full `git archive` copy, safe to
    delete), `mutants/` and `mutants-target/` (a source copy and its Cargo
    target, safe to delete);
  - the oracle's `/private/tmp/iris-s18-storage-plan-axD2dM/` and
    `/private/tmp/iris-s18-plan-rereview-9bzhwwge/` (probe results);
  - earlier sessions' scratchpads under
    `/private/tmp/claude-501/-Users-robertguss-Projects-startups-Iris/`
    (`331764ef…`, `5f627447…`, `730b5501…`, `c354486e…`, `8700f9fe…`; several
    GB, mostly Cargo targets, safe to delete).
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
- **Link checking:** `links.py` (in this session's scratchpad and the
  `730b5501…` one, section 10) takes Markdown paths relative to the repository
  root and applies GitHub-style slugs. A seeded control needs a full copy of the
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
  names (`after-migrate`, `after-seed`, `after-link`) and the injected failure
  exist only in the library's test build.
- **`reference-dev` prints errors by message:** `main` reports
  `reference-dev: <message>` and exits 1. Returning `Box<dyn Error>` from `main`
  prints the `Debug` form (e.g. `InUse(...)`), which tests matching the message
  miss. Its data line is printed before `listening on`, so readers that stop at
  the address line still see it.
- **Storage file names:** a database `<name>` owns `<name>.iris-lock` (never
  deleted, also by the future reset) and, while initializing,
  `<name>.iris-init/` holding `owner` and `reference.db`. Names ending in those
  suffixes, in any case, are refused as databases. Don't use
  `apps/reference/.dev/` by hand before step 5 adds it to `.gitignore`.
- **Mutation runner for storage:** `mutate.py` in this session's scratchpad
  (section 10) copies `git ls-files -co --exclude-standard` to a scratch
  directory, uses its own `CARGO_TARGET_DIR`, applies one exact-string mutation
  at a time to `storage.rs` and runs
  `cargo test -p iris-reference --lib storage`. Exact strings break when the
  source changes; re-check each count.
- **Unset variables in shell calls:** shell state does not persist between tool
  calls, so a `$S` set in one call is empty in the next; `cat $S` then reads
  stdin and hangs until the tool times out. Set it in every call.
- **zsh globs:** an unmatched or huge glob such as `target/debug/deps/x-*` fails
  the whole command ("no matches found", "argument list too long"). Find test
  binaries with `cargo test --no-run --message-format=json` and the artifact's
  `executable` field.

## 12. Skills

- **Required:** `driver` for the driver, `oracle` for the oracle.
- **Optional:** `herdr` for pane and agent control.
