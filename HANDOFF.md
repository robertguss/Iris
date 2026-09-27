# Handoff

## 1. State

Observed September 27, 2026, by the outgoing driver.

- Repository: `/Users/robertguss/Projects/startups/Iris` (GitHub
  `robertguss/Iris`).
- Branch: `s17-checkpoint-a`, tracking `origin/s17-checkpoint-a`. `main` is
  unchanged at `9235c6e`.
- Reviewed through `12227aa` (step 3 of this chunk). This handoff is committed
  after it.
- Pushed through `12227aa`, which is `origin/s17-checkpoint-a` and the head of
  draft pull request #1 (https://github.com/robertguss/Iris/pull/1). The owner
  approved both pushes this session. This handoff's own commit is not pushed.
- The working tree was clean apart from this handoff before its commit.

Re-check HEAD, the working tree and the remote (`git status`, `git log -5`,
`git ls-remote origin`) before relying on any of this.

## 2. Read these first

- `docs/design-spec.md`: "How to interpret and maintain this spec", then S17:
  "Public response policy" (its last paragraph), "React client", "Acceptance
  checks", "Owner decisions", and "Current-state read evidence". S15's "Recovery
  discovery supplies capabilities, not instructions to mutate" and S14's
  counterexamples bound what a read may claim.
- `docs/decisions.md`: the last entry, "Reference application current-state
  read", then the two before it.
- `apps/reference/README.md`: commands, operations, verification matrix, probes
  and limits.
- `crates/iris/src/lib.rs`: `CurrentStateRead`, the bridge's rendering of
  `recovery.read`, and `check_current_state_read`, whose doc comment states the
  linkage rules.
- The client: `apps/reference/web/src/client.ts` (`parseRecovery`),
  `membership.ts` (`readback`), `present.ts` (`unconfirmed`) and `main.tsx` (the
  `Attempt` type and the readback button).
- `apps/reference/scripts/browser.mjs`: its header states the process-ownership
  guarantees, now with two API restarts. `withhold()` and `withheld()` document
  the withheld-response method.
- `.github/workflows/verify.yml`: what CI runs.
- `docs/design-review-brief.md`: how independent reviews are run.

## 3. Context

The owner answered the previous handoff's three questions one at a time: push
the local commits, declare `listProjectMembers` as the mutations' current-state
read on the terms the oracle recommended, and build that next. The oracle's
terms shaped every choice:

- `recovery.read` is `false` or `{operation_id, path_inputs}`, never `true`.
- Assembly checks the structure. Only an independent test can show that the
  binding names the right field: `user_id` has the same schema as `project_id`,
  and a probe shows assembly accepting that swap.
- A page resolves nothing (S14). The wording is page-scoped, and the readback
  never changes the attempt record. No read automatically follows a mutation.

Two data facts drove the browser design. The seed has two users and no
invitations, so every committed removal uses up a membership. "Alice removes
Bob" and "Bob removes himself" cannot happen in one dataset. So the runner now
restarts the API twice, and the third workflow (C) runs on fresh data. B's
self-removal became the withheld case, and A gained the acknowledged-outcome
retention check that B used to give.

A withheld response is not a dropped request. The request reaches the server; a
runner-owned `window.fetch` wrapper records the response and throws, and the
runner checks the recorded 200 acknowledgment itself. That proves a commit
happened while the client saw nothing. It does not test a real disconnect or
cancellation.

## 4. Agreed chunk and acceptance

- **Objective:**
  - the declaration, checked at assembly and by the client;
  - the console's unconfirmed-outcome wording, and one manual readback;
  - behavioral evidence that the readback resolves nothing.
- **Exclusions:**
  - receipts, inspection and replay;
  - invitations, the lifecycle pass, `experiments/` and retiring a frozen
    experiment;
  - automatic reads after a mutation;
  - recovery on reads;
  - MCP exposure, a recovery executor or a binding language;
  - merges, and pushes without the owner's go-ahead.
- **Stopping condition:** step 3 signed off and committed, then this handoff.
  The boundary moved once, in step 2's plan review, splitting the documentation
  into step 3. The oracle agreed; the user approved no other scope change.
- **Disposition:** `accepted`, as three commits:
  - `cfb4d18`: the contract (the descriptor, linkage checks, the reference
    declaration, both regenerations, the client parse, tests and probes);
  - `8b7e68a`: the console and its browser evidence;
  - `12227aa`: the documentation and evidence.

## 5. Verification and review

Environment: macOS, Rust 1.98.1, Node 24.20.0 (the client also under 26.8.1 via
`mise exec node@26.8.1`; CI pins 26.10.0), `agent-browser` 0.38.1, headless
Chrome 154. Driver evidence is in
`/private/tmp/claude-501/-Users-robertguss-Projects-startups-Iris/8700f9fe-cfa6-4749-b371-63bb1793592f/scratchpad/`
(temporary; below, `scratchpad/`).

| Claim                    | Commit and evidence                                                                                                                                                                                                                                                                                                                                                    | Checked by                                                                                                                 |
| ------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| Full local suite         | Every CI step except Mailpit and the credential-free flow, in CI's order: fmt, workspace (127), dev-identity, Clippy, web `verify` (Node 26 and 24), S16 `verify`, `verify:s16`, `probe:s16`, agent-interface (5/5), the OIDC fixture, the browser workflow. `cfb4d18`'s diff: `scratchpad/step1/final/` and `step1/fix/`. `8b7e68a`'s diff: `scratchpad/step2/final/` | Oracle reran `crates/iris` tests (34) and reference contract tests at step 1, and web `verify` under Node 26.8.1 at step 2 |
| Browser workflows        | `8b7e68a`: five consecutive passes, 15 s each; validator compile 17.3, 17.7, 29.4, 19.5, 19.0 ms (`scratchpad/step2/evidence.txt`, `browser-1`…`5`)                                                                                                                                                                                                                    | Oracle ran the full A/B/C workflow itself at step 2                                                                        |
| Second restart ownership | `8b7e68a`: `scratchpad/probe-restart2.py` (`step2/restart-probes.log`). A foreign listener, the killed api-c and SIGTERM in the window each fail cleanly                                                                                                                                                                                                               | Oracle reran all three                                                                                                     |
| Seeded mutations         | Step 1: 34 (`mutate-step1.py`, `step1/fix/mutations.log`). Step 2: 11 (`mutate-step2.py`, `step2/mutations.log`). All caught at the named check                                                                                                                                                                                                                        | Oracle inspected the logs; did not rerun                                                                                   |
| Omission probes          | Two new probes; the full run 72 s with a cold target (`step3/probes.log`)                                                                                                                                                                                                                                                                                              | Driver-reported                                                                                                            |
| Docs figures and links   | `12227aa`: 44 local anchored links resolve                                                                                                                                                                                                                                                                                                                             | Oracle checked every figure against the logs                                                                               |
| GitHub Actions           | `06ac967`: run 36329466284 failed on attempt 1 only in "Verify agent interface and real MCP scenarios" (`runner.test.mjs:79`, the `members` scenario reproduction), then passed every step on attempt 2. Assessed as a flake, cause not diagnosed. `12227aa`: run 36345360713, passed every step on attempt 1, including the three browser workflows                   | Oracle verified both `06ac967` attempts and run 36345360713                                                                |

Oracle verdicts and dispositions, all fixed unless stated:

1. Chunk proposal and step 1 plan: changes requested.
   - P2: prove a withheld response followed a commit. Fixed: the runner checks
     the recorded acknowledgment itself.
   - P3: isolate the target-read negative cases. Fixed.

   The re-review signed off.

2. Step 1 diff: changes requested.
   - P2: a required non-path parameter sharing a bound name passed. Fixed, with
     a query/header/cookie regression case.
   - P3: "no query parameters" wording. Fixed.

   The re-review signed off.

3. Step 2 plan, with the boundary split: changes requested.
   - P2: keep an acknowledged-outcome retention check. Added to A.
   - P2: make the cursor and hidden-button mutations detectable. Fixed.
   - P3: a real non-200 for the withholding probe. Fixed.

   The re-review signed off.

4. Step 2 diff: sign-off, no findings. It accepted one deviation: C2 removes
   while page 1 (which has a next cursor) is shown.
5. Step 3 plan: sign-off, with a P3 (scope "no automatic read" to mutations),
   applied.
6. Step 3 diff: sign-off, with two P3 wording fixes (the request, not the
   response, reaches the server; the probe shows assembly accepting the binding
   and the test rejecting it), applied.

## 6. Remaining work

1. **The owner's choice of the next chunk (section 9).**
2. **A follow-up design for invitation issuance and acceptance** (S17's
   follow-up row).
3. **A lifecycle design pass:** persistent storage, seed policy, journal mode,
   worker supervision, one development command.
4. **Retiring frozen experiments:** S16 stays in CI until its remaining omission
   probes are carried by the reference application.
5. **The agent-interface CI flake:** if it recurs, diagnose the `members`
   scenario (`concurrent_last_owner_and_authority` in
   `experiments/embedded-db/sqlite/tests/members.rs`, run through
   `reproduce_scenario`). The experiment is frozen, so any fix needs the owner.
6. **Documentation hygiene, carried forward:**
   - the top-level README's "Next milestone" omits S16 and S17;
   - `experiments/api-slice/README.md` still lists "durable email delivery" as
     absent;
   - the original "Open decisions" list in `docs/decisions.md` includes items
     settled later;
   - `apps/reference/README.md`'s last limit still says the current-state read
     commits have not run in CI. They passed in run 36345360713 at `12227aa`.
     Update that current-status sentence; the dated evidence records stay as
     written.
7. **Precompiled validators:** needed if a content security policy without
   `unsafe-eval` is adopted.
8. **Caller loss beyond the browser:** a real disconnect or cancellation during
   a mutation (S14's focused validation items 2 and 3) is still untested.

## 7. Next chunk

`proposed`. Nothing beyond the current-state read is authorized.

- **First action:** ask the owner the open questions in section 9, one at a
  time, and wait for each answer before asking the next. Then propose the chunk
  the answers authorize, in the first plan review.
- **Acceptance:** set by that choice. If the invitations design is chosen: a
  design section in S17's style, documentation only, reviewed through the
  design-review brief, authorizing no implementation until the owner says so.

## 8. Decisions and authorizations in force

- **The seven S17 owner decisions** in S17's "Owner decisions".
- **Authorized and complete:** the CI prerequisite, checkpoints A and B, and the
  mutations' current-state read. Commits go on `s17-checkpoint-a`.
- **Pushes:** the owner approved two pushes to draft PR #1 this session, through
  `12227aa`. Every later push needs a fresh go-ahead.
- **Not authorized:** merges; invitations; the lifecycle pass; retiring frozen
  experiments; editing `experiments/`.
- **Decided in this chunk:** the choices in the decision record's "Reference
  application current-state read" entry.
- **Workflow:** the owner asked for oracle review before every commit.

## 9. Open questions for the user

Ask one at a time.

- Which follow-up should come next: the invitations design, the lifecycle pass,
  or something else? This blocks the next chunk.
- Should draft PR #1 stay a draft on `s17-checkpoint-a`, or be prepared for
  merging into `main`? Nothing technical waits on this, but merges are not
  authorized, and `main` still predates all S17 work.

## 10. Operational state

- **Running processes:** none. No servers or browser sessions (checked: ports
  4001, 3003 and 5175 free; `agent-browser session list` empty). The browser
  workflow, the probes and every mutation runner stop what they start. The
  in-place mutation runners restored their files, confirmed by SHA-256.
- **Draft PR #1:** open at `12227aa`. Don't merge it.
- **Local installs:** `experiments/agent-interface/node_modules` was installed
  with `npm ci` to run its test; it is gitignored.
- **Retained evidence:** temporary, and possibly already deleted: the driver's
  scratchpad (section 5; about 2 GB, mostly `step1/mutation-target`, a Cargo
  target safe to delete), and any review directories the oracle created under
  the system temp directories.
- **Known risk, carried forward:** `probe:s16` builds into the checkout's shared
  `target/` and can leave a mutated artifact that a later run treats as current.
  The experiment is frozen, so the risk is recorded rather than fixed.

## 11. Conventions and gotchas

- **Shell aliases:** in interactive shells `tr` is a trash command, `npm` a
  package guard and `ls` another tool. Use `command tr`, `command npm` and
  `/bin/ls`. A pipeline with a bare `tr '\n' ';'` tried to trash files named
  `\n` and `;` this session; it failed harmlessly only because none existed.
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

## 12. Skills

- **Required:** `driver` for the driver, `oracle` for the oracle.
- **Optional:** `herdr` for pane and agent control.
