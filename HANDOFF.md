# Handoff

## 1. State

Observed September 27, 2026, by the outgoing driver.

- Repository: `/Users/robertguss/Projects/startups/Iris` (GitHub
  `robertguss/Iris`).
- Branch: `lifecycle-design`, local only, with no upstream. It was created at
  `8d2cfc7` (the previous handoff) at the owner's choice.
- Reviewed through `4bf412a`. This handoff is committed after it.
- Pushed through `8594777` only: `main`, `origin/main` and
  `origin/s17-checkpoint-a` are at `8594777`. Local `s17-checkpoint-a` is at
  `8d2cfc7`, one ahead of its upstream. `lifecycle-design` holds `8d2cfc7`,
  `f83688e`, `4bf412a` and this handoff, none pushed. No push was authorized.
- PR #1 is merged (`8594777`). No new pull request exists.
- The working tree was clean apart from this handoff before its commit.

Re-check HEAD, the working tree and the remote (`git status`, `git log -5`,
`git ls-remote origin`) before relying on any of this.

## 2. Read these first

- `docs/design-spec.md` S18 "Reference application lifecycle", in full: current
  behavior, recommendations, acceptance checks, the ten owner choices, and the
  review dispositions.
- `docs/reviews/astra-s18-all-01.md`: the design review behind S18's revision.
- `docs/decisions.md`: "Reference application lifecycle — September 27, 2026",
  and "Open decisions".
- `docs/design-spec.md`: "How to interpret and maintain this spec", S14 (caller
  loss), S17 "Ownership boundaries" and "Operation sequence".
- `apps/reference/README.md`: commands and limits.
- `docs/design-review-brief.md`: if another design is reviewed.

## 3. Context

The owner answered two questions this session, one at a time, each from offered
options:

- **Next follow-up:** "Lifecycle pass" (over the invitations design and the
  remaining documentation hygiene).
- **Branch:** "New branch at 8d2cfc7": a new local branch from the handoff
  commit, keeping the handoff without cherry-picking or pushing. A pull request
  needs a separate go-ahead.

How the chunk was shaped:

- **Design, not implementation:** S18 follows S17's pattern: status Proposed,
  current behavior cited to source at `8d2cfc7`, recommendations that are the
  driver's, acceptance checks for a later implementation, and owner choices in a
  table. Nothing in S18 is accepted direction until the owner decides it.
- **Same reviewer twice:** the oracle reviewed the plan and diffs and also did
  the brief-style design review (all three tracks). At plan review it judged
  that acceptable if disclosed; S18 and the report both disclose it. A fresh
  independent reviewer remains optional.
- **Evidence probes stayed outside the checkout:** the driver's journal-mode
  probe (Python SQLite 3.53.4, not the application's libsqlite3-sys 0.37.0
  build) and the oracle's probes (hard-link publication, reset with a live
  connection, WAL sidecars, tempfile after SIGTERM, Vite proxy selection). S18
  states their limits.

## 4. Agreed chunk and acceptance

- **Objective:** a documentation-only lifecycle design (S18) covering persistent
  storage, seed policy, SQLite journal mode, worker supervision and one
  development command; an independent-style design review preserved with
  dispositions; this handoff.
- **Exclusions:** any code, test, script, dependency, CI, migration or wire
  change; any edit under `experiments/`; implementing any recommendation; push,
  pull request, PR #1 edits, branch deletion, CI reruns; the invitations design
  (only its worker constraints are noted); retiring frozen experiments.
- **Stopping condition:** after step 2 and this handoff. The boundary did not
  move.
- **Disposition:** `accepted`.
  - Step 1, `f83688e`: S18 draft, pointers in S10, S11 and S17, "Last updated"
    corrected to September 27, change record, decision entry.
  - Step 2, `4bf412a`: review preserved as `docs/reviews/astra-s18-all-01.md`,
    S18 revised, "Review of the lifecycle proposal" subsection.

## 5. Verification and review

Environment: macOS; no application code changed, so no test suite was run and CI
did not run (nothing pushed). Evidence is in
`/private/tmp/claude-501/-Users-robertguss-Projects-startups-Iris/331764ef-8612-4d63-b584-c6072cd226d2/scratchpad/`
(temporary; below, `scratchpad/`).

| Claim                     | Evidence                                                                                                                                                                                                                                                             | Checked by                                                     |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| Formatting                | `prettier@3.9.9 --check --print-width 80 --prose-wrap always` on the changed Markdown, clean at each step                                                                                                                                                            | Oracle, independently, each review                             |
| Links                     | `links.py` on the changed files: 148, then 152, then 155 local links, 0 broken (`scratchpad/evidence/02-links.txt`, `04-links-rereview.txt`, `06-links-step2.txt`); a seeded control on a `git archive` copy reported all six seeded breaks (`03-links-control.txt`) | Oracle reran the link check; inspected, not reran, the control |
| Journal-mode probe        | `scratchpad/probe_journal.py`, output `evidence/01-journal-probe.txt`: rollback-journal writer fails after ~120 ms with a reader open; WAL commits                                                                                                                   | Driver-reported; the oracle ran its own WAL probes             |
| Source citations          | Every `file:line` in S18 re-read against `8d2cfc7`; SQLx 0.9.0 option defaults from the local registry                                                                                                                                                               | Oracle inspected sources and pinned crates                     |
| Review preserved verbatim | Body of `docs/reviews/astra-s18-all-01.md` byte-identical to the returned report (`evidence/05-astra-s18-all-01-original.md`) apart from the status note                                                                                                             | Oracle, independently                                          |

Oracle verdicts:

1. Chunk and step 1 plan: sign-off. One P3, accepted: read the worker's callers
   (`main.rs`, `auth-demo.rs`).
2. Step 1 diff: changes requested. Three P2s, all fixed: reset needs exclusive
   ownership; lease expiry does not guarantee delivery; temporary directories
   can survive a signal. Two P3s, fixed: narrow runtime-versus-test claims;
   qualify the WAL comparison.
3. Step 1 re-review: sign-off, no findings.
4. Design review `astra-s18-all-01` (not a sign-off round): AS18-B-01 and
   AS18-B-02 (significant), AS18-A-01 and AS18-C-01 (limited). All accepted;
   S18's table records each disposition. None disputed, so none went to the
   owner.
5. Step 2 diff: sign-off, no findings.

## 6. Remaining work

1. **The owner's decisions on S18's ten choices (section 9).**
2. **Authorization to implement S18,** once decided, probably in steps: storage
   and initialization, reset and migrations, shutdown and session cleanup, then
   the development command.
3. **A follow-up design for invitation issuance and acceptance** (S17's
   follow-up row), which must also settle worker restart policy and send
   uncertainty per S18 recommendation 8.
4. **Retiring frozen experiments:** S16 stays in CI until its remaining omission
   probes are carried by the reference application.
5. **The agent-interface CI flake:** not recurred (runs 36345360713,
   36348053700, 36351093637 and 36351648014 passed first time). If it does,
   diagnose `concurrent_last_owner_and_authority` in
   `experiments/embedded-db/sqlite/tests/members.rs`. Frozen; a fix needs the
   owner.
6. **Documentation hygiene, carried forward:**
   - `experiments/embedded-db/README.md:64-66` describes Turso's migration as
     one-version; the migrator applies two since `2b1e820`. Frozen; needs the
     owner.
   - PR #1's merged description is stale. Optional; needs the owner.
   - (Done this chunk: design-spec's "Last updated" date.)
7. **Precompiled validators:** needed if a content security policy without
   `unsafe-eval` is adopted.
8. **Caller loss beyond the browser:** a real disconnect or cancellation during
   a mutation (S14's focused validation items 2 and 3) is still untested.
9. **Publishing `lifecycle-design`:** push and any pull request need the owner's
   go-ahead.

## 7. Next chunk

`proposed`. Nothing beyond this chunk is authorized.

- **First action:** ask the owner S18's choices (section 9), one at a time, in
  table order, each with the recommended option first and the alternative
  second. Record the answers in S18 ("Owner decisions" in S17's style) and the
  decision record in a reviewed step. Then ask whether implementation is
  authorized.
- **Acceptance:** set by the answers. A decisions-only chunk is documentation;
  an implementation chunk must meet S18's acceptance checks for the parts
  authorized.

## 8. Decisions and authorizations in force

- **The seven S17 owner decisions** in S17's "Owner decisions".
- **This session:** the lifecycle pass as a design; the `lifecycle-design`
  branch at `8d2cfc7`.
- **Authorized and complete:** everything in the previous handoff's list, plus
  this chunk's S18 design and review.
- **Pushes:** none authorized; every push needs a fresh go-ahead.
- **CI reruns:** need the owner.
- **Not authorized:** implementing any of S18; any push or merge; a new pull
  request; deleting `s17-checkpoint-a`, `lifecycle-design` or
  `docs/s17-reference-app`; invitations; retiring frozen experiments; editing
  `experiments/`; editing PR #1.
- **Decided in earlier chunks:** the decision record's dated entries.
- **Workflow:** the owner asked for oracle review before every commit. PR text
  is reviewed before posting. Ask the owner one question per message.

## 9. Open questions for the user

Ask one at a time, in this order (S18 "Lifecycle choices for the owner"). Each
blocks the corresponding part of any S18 implementation, and together they block
the next chunk:

1. Default for the development binary: disposable unless given a path, or
   persistent by default?
2. How the path is given: command-line argument, or an environment variable
   stripped from the browser runner?
3. Where the development command keeps data: gitignored `apps/reference/.dev/`,
   or the per-user data directory?
4. When seeds run: only during atomic initialization, or an explicit idempotent
   seed command?
5. Migration policy after persistence: append-only, or keep editing
   `0001_initial.sql` and reset?
6. Journal mode: rollback everywhere, checked at startup, or WAL for the
   persistent database?
7. Development command: a Node supervisor script (optionally an npm script), or
   a process-runner dependency, Rust binary or Makefile?
8. Session cleanup: a supervised periodic task once storage persists, or none
   for now?
9. Owners of a persistent database: one API at a time, or convergent concurrent
   starts?
10. Shutdown budget: an inner drain deadline inside the outer kill bound, or a
    coarse stop that promises no drain?

Then: is implementation authorized, and in which steps? Should
`lifecycle-design` be pushed, and is a pull request wanted?

## 10. Operational state

- **Running processes:** none started by the driver. The oracle ran scratch
  probes only.
- **Remote:** `main` and `s17-checkpoint-a` at `8594777`; PR #1 merged. Don't
  edit or delete without the owner.
- **Local branches:** `lifecycle-design` (this work, unpushed);
  `s17-checkpoint-a` at `8d2cfc7`; `main` at `8594777`; `docs/s17-reference-app`
  at `5ad417d` (older; leave it).
- **Local installs:** `experiments/agent-interface/node_modules` and
  `apps/reference/web/node_modules` (gitignored).
- **Retained evidence (temporary, possibly already deleted):**
  - this session's
    `/private/tmp/claude-501/-Users-robertguss-Projects-startups-Iris/331764ef-8612-4d63-b584-c6072cd226d2/scratchpad/`
    (evidence 01–06, prompts, `probe_journal.py`, `links.py`, `s18.txt`,
    `control/` — a full `git archive` copy for the link control, safe to
    delete);
  - the oracle's `/private/tmp/astra-s18-all-01-ew4q3nzy/` and
    `/var/folders/f8/ft7ygqg92pj8qh0rwplbw2x80000gn/T/iris-s18-oracle-whe2dgpk/`;
  - earlier sessions' scratchpads under
    `/private/tmp/claude-501/-Users-robertguss-Projects-startups-Iris/`
    (`5f627447…`, `730b5501…`, `c354486e…`, and `8700f9fe…`, about 2 GB, mostly
    a Cargo target safe to delete).
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
  owner's go-ahead. `s17-checkpoint-a` tracks `origin/s17-checkpoint-a`;
  `lifecycle-design` has no upstream. Push only by explicit SHA and refspec.
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

## 12. Skills

- **Required:** `driver` for the driver, `oracle` for the oracle.
- **Optional:** `herdr` for pane and agent control.
