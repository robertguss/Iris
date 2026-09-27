# Handoff

## 1. State

Observed September 26, 2026, by the outgoing driver.

- Repository: `/Users/robertguss/Projects/startups/Iris` (GitHub
  `robertguss/Iris`).
- Branch: `docs/s17-reference-app`, created from `main` for this chunk. `main`
  is unchanged at `9235c6e`.
- Reviewed through `e26fc18` (S17 docs commit); this handoff is committed after
  it.
- Pushed through `9235c6e` (`origin/main`). The branch has never been pushed.
- Working tree clean apart from this handoff before its commit.

Re-check HEAD, the working tree and the remote before relying on any of this.

## 2. Read these first

- `docs/design-spec.md`: "How to interpret and maintain this spec" (the binding
  maintenance rules), then S17 "Reference application and first reads",
  including its "Owner decisions" and "Review of this proposal" subsections. S16
  is the predecessor that S17 ports.
- `docs/decisions.md`: the latest entry, "Reference application and first reads
  — September 26, 2026", with the local rerun commands.
- `experiments/api-slice/s16.md`: the S16 experiment's commands, verification
  matrix and limits.
- `.github/workflows/verify.yml`: what CI currently runs.
- `docs/design-review-brief.md`: how independent reviews are run and preserved.

## 3. Context

Iris is the owner's personal project to design an API-first Rust framework. The
owner works in rhythm: a design pass in the living spec, then an explicitly
authorized, bounded experiment. Nothing becomes framework API without evidence.

This chunk began as an independent assessment of the repository by the driver
(Claude) and the oracle (Astra, GPT-6-Astra via Codex). Both concluded that the
S15/S16 result contract has one instance, that no domain read endpoint exists,
and that the next risk is breadth rather than deeper failure semantics. The
driver drafted S17; the oracle reviewed it. The owner then answered S17's seven
open choices one at a time, choosing the recommended option each time; the S17
"Owner decisions" table records them. Settling those choices did not authorize
implementation. The owner asked for the oracle's review before any commit, which
is why the driver workflow is in use.

## 4. Agreed chunk and acceptance

- Objective: add S17 to the living spec, record the owner's seven decisions, and
  add the matching decision-record entry.
- Exclusions: no implementation, CI, dependency or wire change; no push.
- Stopping condition: oracle sign-off, then a commit on a new branch.
- Disposition: `accepted`, as commit `e26fc18`.
- Scope changes approved by the user: none.

## 5. Verification and review

| Claim                                        | Evidence                                                                                                                                    | Checked by                                                                                                                                                                                                                                                                      |
| -------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Docs formatted                               | `bunx prettier --check --print-width 80 --prose-wrap always docs/design-spec.md docs/decisions.md` passed before commit                     | Driver only                                                                                                                                                                                                                                                                     |
| No whitespace errors                         | `git diff --check` passed                                                                                                                   | Driver and oracle                                                                                                                                                                                                                                                               |
| Baseline green at `9235c6e`                  | Commands and results in the `docs/decisions.md` entry; macOS, Rust 1.98.1, Node 24.20.0 as the driver's shell resolved it (CI pins 26.10.0) | Driver only; the oracle did not rerun                                                                                                                                                                                                                                           |
| Query parameters under `deny_unknown_fields` | Scratch probe of the serde_urlencoded 0.7.1 path behind axum 0.8.9's `Query`: unknown and duplicate parameters rejected, `limit=0` accepted | Driver ran it and reported the results; the oracle read its retained source, `/private/tmp/claude-501/-Users-robertguss-Projects-startups-Iris/78effb44-a3f6-4ede-8fd4-2452205d4410/scratchpad/qprobe/src/main.rs`, which is temporary session scratch space and may be deleted |

Oracle verdicts, in order:

1. First S17 draft: no blockers, four major and three minor findings. All seven
   were fixed; S17 "Review of this proposal" lists them.
2. Sign-off pass on the revised draft: APPROVE WITH NITS. The nit (the client
   boundary owns thrown request and body-read errors) was applied.
3. Diff review of the recorded owner decisions: sign-off, with one P3 on
   review-chronology wording, fixed without another round.

## 6. Remaining work

1. CI prerequisite: run `npm --prefix experiments/api-slice/web run verify:s16`
   in the web verification step and `probe:s16` as its own step. `probe:s16`
   took about 39 seconds locally.
2. S17 checkpoint A: scaffold `apps/reference` in the S04 layout, port S16's
   `change_role` unchanged, add `remove_member` with the shared rejection type,
   and meet checkpoint A's acceptance checks.
3. S17 checkpoint B: the two reads, pagination, HEAD handling, read cleanup, and
   the React member directory.
4. A follow-up design for invitation issuance and acceptance.
5. A lifecycle design pass: persistent storage, seed policy, journal mode,
   worker supervision and one development command.
6. Documentation hygiene: the README "Next milestone" section omits S16 and S17;
   `experiments/api-slice/README.md` still lists "durable email delivery" as
   absent despite the outbox; the original "Open decisions" list in
   `docs/decisions.md` includes items settled later.

## 7. Next chunk

`proposed`, not approved: the CI prerequisite, then S17 checkpoint A.

- Acceptance: CI runs `verify:s16` and `probe:s16`; S17's checkpoint A
  acceptance checks pass; each step signed off by the oracle.
- First action: ask the owner to authorize this chunk. Then send the plan for
  review.
- Branch: unless the owner directs otherwise, commit on a new branch created
  from `docs/s17-reference-app`. Do not merge into `main` or push without the
  owner's go-ahead.

## 8. Decisions and authorizations in force

- The seven S17 owner decisions in `docs/design-spec.md` S17 "Owner decisions".
  Scope: the S17 reference application design.
- The earlier accepted direction and deferrals recorded in the spec (S01–S16)
  and `docs/decisions.md`.
- Authorized: none for implementation, CI changes, dependencies, pushes or
  merges.
- Workflow: the owner asked for oracle review before committing.

## 9. Open questions for the user

- Authorize the next chunk (CI prerequisite, then checkpoint A)? Blocks all
  implementation.

No other question blocks work. Merging or pushing `docs/s17-reference-app` stays
the owner's call; until then, those are constraints, not open questions.

## 10. Operational state

- No running jobs or servers.
- Gitignored build output exists from the rerun: `target/` (about 6.5 GB),
  `experiments/api-slice/web/node_modules` and `experiments/api-slice/web/dist`.
  Nothing needs cleanup.
- Not installed locally: `experiments/agent-interface` dependencies, Mailpit and
  `agent-browser`; the Mailpit and browser CI steps have not been rerun here.

## 11. Conventions and gotchas

- A user-level hook formats every Markdown file edited through the agent's
  write/edit tools with `prettier --print-width 80 --prose-wrap always`. The
  docs already conform, so run the same check after editing any other way.
- The repository's history is linear on `main`, previously pushed by the owner.
  This chunk branched because agents branch before committing on the default
  branch; never push without the owner's go-ahead.
- In this machine's interactive shells `npm` is aliased to a package guard
  (`pmg npm`) and `ls` to another tool. Use `command npm` and `/bin/ls` in
  scripts.
- Node is managed per shell: the driver's shell resolved 24.20.0 and the
  oracle's 24.19.0, while CI pins 26.10.0. Check `node --version` in each
  session. `verify:s16` runs `client.test.ts` directly and relies on Node's
  TypeScript type stripping.
- A cold `cargo test --workspace --locked` took about 52 seconds here.
- The driver works in the left pane and the oracle in the right. Send the oracle
  prompts through a file, as the driver skill describes.

## 12. Skills

- Required: `driver` for the driver, `oracle` for the oracle.
- Optional: `herdr` for pane and agent control.
