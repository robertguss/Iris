# Handoff

## 1. State

Observed September 26, 2026, by the outgoing driver.

- Repository: `/Users/robertguss/Projects/startups/Iris` (GitHub
  `robertguss/Iris`).
- Branch: `s17-checkpoint-a`, created from `docs/s17-reference-app` at `5ad417d`
  for this chunk. `main` is unchanged at `9235c6e`.
- Reviewed through `35687c1` (step 6); this handoff is committed after it.
- Pushed through `9235c6e` (`origin/main`). Neither `docs/s17-reference-app` nor
  `s17-checkpoint-a` has been pushed.
- Working tree clean apart from this handoff before its commit.

Re-check HEAD, the working tree and the remote before relying on any of this.

## 2. Read these first

- `docs/design-spec.md`: "How to interpret and maintain this spec", then S17,
  especially "Checkpoint A server-side evidence", "Acceptance checks" (the React
  row) and "React client".
- `docs/decisions.md`: the last two entries, "S16 checks in CI" and "Reference
  application checkpoint A, server side".
- `apps/reference/README.md`: commands, layout, verification matrix, probes and
  limits.
- `crates/iris/src/lib.rs`: the crate doc states what it owns and why.
- `experiments/api-slice/s16.md`, `experiments/api-slice/web/s16/` (`client.ts`,
  `client.test.ts`, `narrowing.ts`) and
  `experiments/api-slice/web/scripts/s16.mjs`: the client harness still to be
  ported.
- `.github/workflows/verify.yml`: what CI runs.
- `docs/design-review-brief.md`: how independent reviews are run.

## 3. Context

The owner authorized this chunk as "the CI prerequisite, then S17 checkpoint A",
on a new branch, with no push or merge, and oracle review before every commit.
In the first plan review the driver and the oracle (Astra, GPT-6-Astra via
Codex) agreed to stop after checkpoint A's server side. Its React client and
browser workflow were left for the next session because the server port was
large and the browser check needs a tool that is not installed here. Checkpoint
A is therefore partial; the owner's authorization still covers its remainder.

The steps grew from four to six during plan review, without moving the stopping
point: step 2 split so session tests could land before any domain route, and the
extraction into `crates/iris` became its own step after `remove_member` proved
the need. Design choices made along the way are recorded in S17's evidence
subsection and the decision record; the most consequential are one checked
assembly shared by `app()` and `openapi()`, per-operation declarations that feed
rendering, export and checks alike, and what moved into `crates/iris`.

## 4. Agreed chunk and acceptance

- Objective: run `verify:s16` and `probe:s16` in CI, then implement checkpoint
  A's server side: the reference application, the ported `change_role`,
  `remove_member`, multi-operation assembly, the extraction decision and the A
  probes, with the spec and decision record updated.
- Exclusions: the React client and browser workflow (next chunk), checkpoint B,
  experiment edits, pushes and merges.
- Stopping condition: step 6 signed off and committed.
- Disposition: `accepted`, as commits `6af0d26`, `87b66bb`, `bdde3e7`,
  `e6412c3`, `a69e0dd` and `35687c1`.
- Scope changes approved by the user: none. The server-side boundary was agreed
  in plan review within the owner's authorization.

## 5. Verification and review

Environment: macOS, Rust 1.98.1, Node 24.20.0 in the driver's shell (step 1 also
under 26.8.1; CI pins 26.10.0), npm 11.19.0 (CI pins 10.9.9). The oracle's shell
resolved Node 24.19.0.

| Claim at `35687c1`                       | Evidence                                                                                                                | Checked by                                                                                                                      |
| ---------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| Workspace green                          | `cargo test --workspace --locked`: 81 passed; dev-identity 24 passed, 1 ignored                                         | Driver; the oracle ran the crate suites, not the workspace                                                                      |
| Crate suites                             | `-p iris` 9; `-p iris-reference` 24 (lib 15, contract 2, identity 7)                                                    | Driver and oracle                                                                                                               |
| Lints and format                         | Clippy `--all-targets --all-features -D warnings` (workspace and each crate alone), `cargo fmt --all --check`, Prettier | Driver; oracle for crate-alone Clippy and formatting                                                                            |
| Export unchanged by extraction           | `openapi.json` byte-identical after step 5; drift test                                                                  | Driver and oracle                                                                                                               |
| A probes                                 | `node apps/reference/scripts/probes.mjs`: 6 caught, 3 controls; 39 s locally, cold isolated target                      | Driver and oracle (38.5 s)                                                                                                      |
| Frozen experiments green                 | Web `verify`, `verify:s16` (10 Rust, 44 client), `probe:s16` (16 caught), rerun after each step                         | Driver; oracle ran all three once, in step 1                                                                                    |
| CI runs the S16 checks and the A probes  | Workflow parses and orders as asserted; commands pass locally                                                           | Driver and oracle; no GitHub Actions run observed                                                                               |
| Seeded mutations fail the intended tests | Driver logs listed in section 10; step 2 and 3 mutation output was not saved to files                                   | Driver; the oracle reproduced selected mutations in steps 2–5, and the complete A probe runner and anchor-drift check in step 6 |

Oracle verdicts and dispositions, all fixed unless stated:

1. Step 1 (CI): plan sign-off with two P3s (trigger wording; describe step 2 as
   the Rust port); diff sign-off.
2. Step 2 (identity): plan P2s (missing reqwest `json` feature; two planned
   mutations survived the retained tests), then sign-off; diff P2 (no
   wrong-token case), then sign-off.
3. Step 3 (`change_role`): plan P2s (parser referenced a removed variant; busy
   mutation missed the tested branch), then sign-off; diff sign-off.
4. Step 4 (`remove_member`, assembly): plan P2s (catalog lacked real metadata;
   application construction bypassed the checks), then sign-off with a P3
   applied in step 6; diff P2 (incomplete recovery assertions), then sign-off.
5. Step 5 (extraction): plan sign-off with a P3 on counts; diff sign-off.
6. Step 6 (probes, docs): plan P2s (shared probe target; probe 5 did not
   compile) and a P3, then sign-off with a timing P3; diff sign-off with three
   wording P3s, fixed without another round.

## 6. Remaining work

1. Checkpoint A client and browser acceptance, per S17's React row: a client in
   `apps/reference/web` with one whole-request boundary generalized from S16,
   typed narrowing and runtime decoder cases for both operations including
   oversize bodies, unchanged session bootstrap, and one browser workflow
   against the local issuer. Porting the S16 harness means restoring the
   whole-request fixture capture removed in step 3 (`IRIS_S16_FIXTURES`); the
   workflow also needs a development binary, which the reference application
   does not yet have.
2. Observe a GitHub Actions run of the updated workflow, which needs a pull
   request or a push to `main` (owner's call).
3. S17 checkpoint B (reads, pagination, HEAD), not yet authorized.
4. A follow-up design for invitation issuance and acceptance.
5. A lifecycle design pass: persistent storage, seed policy, journal mode,
   worker supervision and one development command.
6. Documentation hygiene, carried forward: the README "Next milestone" omits S16
   and S17; `experiments/api-slice/README.md` still lists "durable email
   delivery" as absent; the original "Open decisions" list in
   `docs/decisions.md` includes items settled later.

## 7. Next chunk

`proposed`: checkpoint A's client and browser acceptance (item 1 above).

- Acceptance: S17's React row for both membership operations; the frozen
  experiments stay green; each step signed off by the oracle.
- First action: prepare the client chunk's plan and send it for oracle review.
  Ask the owner the open question below separately; client planning and
  implementation need not wait for the answer.
- Branch: continue on `s17-checkpoint-a` unless the owner directs otherwise. Do
  not merge or push without the owner's go-ahead.

## 8. Decisions and authorizations in force

- The seven S17 owner decisions in S17 "Owner decisions".
- Authorized: the CI prerequisite (done) and S17 checkpoint A, including its
  remaining client and browser work. Commit on a branch; no push or merge.
- Not authorized: checkpoint B, invitations, pushes and merges. The original
  proposal reserved installing `agent-browser` locally for the owner (section
  9).
- Decided in this chunk, and recorded in S17's evidence subsection and the
  decision record: one checked assembly; `Operation` declarations; what
  `crates/iris` owns; frozen experiments kept in CI (none retired).
- Workflow: the owner asked for oracle review before every commit.

## 9. Open questions for the user

- May the driver install `agent-browser` 0.38.1 (the version CI pins) on this
  machine for the checkpoint A browser workflow? This blocks local browser
  verification, not the client code.

## 10. Operational state

- No running jobs or servers, and no leftover issuer processes.
- Gitignored build output: `target/`, `experiments/api-slice/web/node_modules`
  and `dist`. Nothing needs cleanup.
- Not installed locally: `experiments/agent-interface` dependencies, Mailpit and
  `agent-browser`.
- Retained evidence, temporary and possibly already deleted. Driver logs are in
  `/private/tmp/claude-501/-Users-robertguss-Projects-startups-Iris/47301b81-b660-45d0-ac24-2ba218f6c3df/scratchpad/`:
  `node26-run.log` (step 1 under Node 26.8.1), `mut4-*.log` (step 4 mutations),
  `mut5-*.log` (step 5 mutations), `probes-a.log` and `probes-a2.log` (step 6
  probe runs), `drift-check.log` (anchor-drift check) and `probe4–6.log`
  (`probe:s16` reruns). The oracle's scratch evidence is in
  `/var/folders/f8/ft7ygqg92pj8qh0rwplbw2x80000gn/T/iris-oracle-step*`. Nothing
  there needs cleanup.
- Known risk: `probe:s16` builds into the checkout's shared `target/`, and the
  oracle reproduced in scratch that this can leave a mutated artifact a later
  run treats as current. The experiment is frozen, so it is recorded, not fixed.
  The reference probe runner builds in its own temporary target.

## 11. Conventions and gotchas

- In this machine's interactive shells `tr` is aliased to a trash command, `npm`
  to a package guard and `ls` to another tool. Use `command tr`, `command npm`
  and `/bin/ls`, or run scripts with `bash`, which does not expand the aliases.
  A `tr / _` once invoked the trash command on `/` and `_`; it reported errors
  for both, and git status was unchanged afterwards.
- In the driver's Claude Code environment, a user-level hook formats Markdown
  written through its Write and Edit tools; edits made by script bypass it. Run
  `bunx prettier --write --print-width 80 --prose-wrap always <edited-files>`
  afterwards and check the diff stays within your changes.
- In the driver's environment, tool calls time out at 600 s. Run long mutation
  loops in the background with output written to files, not captured through
  `$(...)`: a leaked child that holds the pipe hangs the capture. Test fixtures
  now own the issuer process from spawn, which fixed one such leak.
- `cargo test` stops at the first failing test binary; use `--no-fail-fast` for
  mutation evidence.
- Changing an operation's contract requires regenerating the snapshot:
  `cargo run --locked -p iris-reference --bin export-openapi -- apps/reference/openapi.json`.
- The reference tests start the local issuer from
  `experiments/api-slice/checks/oidc-provider.mjs`; that frozen file is in use.
- Add to `crates/iris` only what two operations demonstrably share, naming both.
- History on `main` is linear and was previously pushed by the owner. Never push
  without the owner's go-ahead.
- The driver works in the left pane and the oracle in the right. Send oracle
  prompts through a file, as the driver skill describes.

## 12. Skills

- Required: `driver` for the driver, `oracle` for the oracle.
- Optional: `herdr` for pane and agent control.
