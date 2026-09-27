# Handoff

## 1. State

Observed September 27, 2026, by the outgoing driver.

- Repository: `/Users/robertguss/Projects/startups/Iris` (GitHub
  `robertguss/Iris`).
- Branch: `s17-checkpoint-a`, created from `docs/s17-reference-app` at
  `5ad417d`. `main` is unchanged at `9235c6e`.
- Reviewed through `d602de5` (step 3 of this chunk); this handoff is committed
  after it.
- Pushed through `9235c6e` (`origin/main`). Neither `docs/s17-reference-app` nor
  `s17-checkpoint-a` has been pushed.
- Working tree clean apart from this handoff before its commit.

Re-check HEAD, the working tree and the remote before relying on any of this.

## 2. Read these first

- `docs/design-spec.md`: "How to interpret and maintain this spec", then S17,
  especially "Checkpoint A client evidence", "Operation sequence", "Read
  conventions", "Acceptance checks" (the B rows) and "Owner decisions".
- `docs/decisions.md`: the last three entries, from "S16 checks in CI" to
  "Reference application checkpoint A, client".
- `apps/reference/README.md`: commands, layout, verification matrix, cost of
  runtime validation, probes and limits.
- `crates/iris/src/lib.rs`: what the crate owns and why.
- `apps/reference/web/src/client.ts` and `apps/reference/scripts/browser.mjs`:
  the client boundary and the browser workflow, whose header comments state
  their guarantees.
- `.github/workflows/verify.yml`: what CI runs.
- `docs/design-review-brief.md`: how independent reviews are run.

## 3. Context

The owner authorized "the CI prerequisite, then S17 checkpoint A", on a branch,
with no push or merge and oracle review before every commit. The previous chunk
delivered the prerequisite and checkpoint A's server side. This chunk delivered
the client and browser acceptance, so checkpoint A is complete. The oracle
(Astra, GPT-6-Astra via Codex) and the driver agreed the three steps and the
stopping point in the first plan review. The owner then approved installing
`agent-browser` locally, which step 3 needed.

The choices made along the way, and the alternatives not taken, are in the
[decision record](docs/decisions.md#reference-application-checkpoint-a-client--september-27-2026).
The browser workflow's process handling took three review rounds. It now refuses
ports it does not own, trusts only its own children's readiness lines, and stops
on a signal or a server exit. It owns every child in its own process group, with
bounded SIGTERM-to-SIGKILL cleanup, and uses a browser session unique to each
run. Keep those properties when extending it.

## 4. Agreed chunk and acceptance

- Objective: checkpoint A's client and browser acceptance (S17's React row for
  both membership operations), with the frozen experiments kept green.
- Exclusions: checkpoint B, invitations, retiring a frozen experiment, edits to
  `experiments/`, the lifecycle pass, new wire routes, pushes and merges.
- Stopping condition: step 3 signed off and committed, then this handoff.
- Disposition: `accepted`, as commits `d3a4863` (client boundary and harness),
  `f096050` (React client and development server) and `d602de5` (browser
  workflow, CI and docs).
- Scope changes approved by the user: installing `agent-browser` 0.38.1 on this
  machine (done). No other change.

## 5. Verification and review

Environment: macOS, Rust 1.98.1, Node 24.20.0 (client checks also under 26.8.1;
CI pins 26.10.0), npm 11.19.0 (`npm@10.9.9 ci`, CI's pin, installed the final
lockfile: `scratchpad/npm10-final.log`), `agent-browser` 0.38.1 with headless
Chrome 154. The oracle's shell resolved Node 24.19.0. Driver evidence is in
`/private/tmp/claude-501/-Users-robertguss-Projects-startups-Iris/929e8516-0514-425e-bb47-6e2d3a65edb2/scratchpad/`
(temporary; below, `scratchpad/`).

| Claim at `d602de5`         | Evidence                                                                                                                                                                                                                                                                                                                 | Checked by                                                                                                                  |
| -------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------- |
| Workspace and lints green  | `scratchpad/final-d602de5.log`: `cargo test --workspace --locked` 83 passed (`iris` 9, `iris-reference` 26, dev identity 24 plus 1 ignored), Clippy `-D warnings` for the workspace and each crate alone, `cargo fmt --all --check`, `git diff --check`                                                                  | Driver; the oracle ran `iris-reference` (26) and crate Clippy in step 2                                                     |
| Client harness             | `npm --prefix apps/reference/web run verify`: 2 Rust capture tests, `tsc`, 122 decoder cases, 24 presentation cases, `vite build`; also under Node 26.8.1                                                                                                                                                                | Driver; the oracle reran it in steps 1 and 2 (Node 24.19.0); step 3's results are driver-reported                           |
| Development server         | `tests/dev_binary.rs`, 2 passed: flag refusal, then session, domain boundary and plain 404                                                                                                                                                                                                                               | Driver and oracle                                                                                                           |
| Browser workflow           | `node apps/reference/scripts/browser.mjs`: five consecutive passes of the final script before commit, then once more at `d602de5`; screenshots in `scratchpad/browser-run*/` and `browser-final/`                                                                                                                        | Driver; the oracle ran it independently in each step 3 review                                                               |
| Workflow process ownership | `scratchpad/probe-browser/` (runner `probe-browser.py`): a taken port, foreign servers on every port, a spawn error, SIGTERM mid-run, a never-ready child, a child killed mid-run and a build ignoring SIGTERM each exit 1 without a PASS or leftover processes; a concurrent second run is refused and the first passes | Driver; the oracle reproduced the concurrent run, SIGTERM, missing executable, never-ready and stubborn build               |
| Seeded mutations           | Step 1 client and capture: `scratchpad/mut3/`, `mut4/`, `mut-m5*.log`. Step 2: `scratchpad/mut-step2/`. Step 3 browser: `scratchpad/mut-step3/`. Each failed its intended check; lists in the decision record                                                                                                            | Driver; the oracle inspected the logs, reproduced its own body and type findings in step 1, and did not rerun the mutations |
| Measurements               | Bundle 374.32 kB (111.87 kB gzip); a stub-validator build is 130.62 kB (38.24 kB gzip) smaller (`scratchpad/measure2/`). Browser validator compile 13.3–24.1 ms, median 13.9 ms, over nine production-build runs; Node 24.20.0 41.5 ms cold, 10.4 ms warm                                                                | Driver; the oracle confirmed the raw bundle sizes against the artifacts; the rest is driver-reported                        |
| Frozen experiments green   | Web `verify` and `verify:s16` (10 Rust, 44 client), rerun each step; `probe:s16` was not rerun this chunk (experiment unchanged)                                                                                                                                                                                         | Driver                                                                                                                      |
| Reference omission probes  | `node apps/reference/scripts/probes.mjs` at `d602de5`: 6 caught, 3 controls (`final-d602de5.log`)                                                                                                                                                                                                                        | Driver                                                                                                                      |
| CI runs the new checks     | The workflow parses; the "Verify reference client" and "Verify reference browser workflow" steps follow their dependencies; commands pass locally                                                                                                                                                                        | Driver and oracle; no GitHub Actions run observed                                                                           |

Oracle verdicts and dispositions, all fixed unless stated:

1. Step 1 plan: sign-off with four P3s. The oracle asked for fixture counts per
   tuple, a lazy stream with `highWaterMark: 0`, a conditional type helper, and
   the demo membership kept in the binary (applied in step 2). Step 1 diff: two
   P2s. Locked or consumed bodies escaped the boundary as exceptions, and a
   partly read, unlocked body validated its remaining bytes; separately, the
   `Document` type rejected the real snapshot. Both were fixed, and the
   re-review signed off.
2. Step 2 plan: sign-off with three P3s: the JSON import attribute, 503 treated
   as unconfirmed, and Vite's working directory. Step 2 diff: sign-off with two
   wording P3s ("Initially…" demo data, and "Another owner is required first."),
   fixed without another round.
3. Step 3 plan: one P2 (bind preview to 127.0.0.1) and one P3 (a discriminating
   wording mutation). The re-review signed off, noting that the mutation must
   change the shared text. Step 3 diff: one P1, three P2s and a P3:
   - P1: readiness accepted foreign servers;
   - P2: spawn errors bypassed cleanup;
   - P2: synchronous commands blocked signals;
   - P2: readiness was unbounded;
   - P3: stale bundle figures.

   The first re-review found two further P2s: a refused run closed another run's
   browser session, and build children were unowned. The second re-review signed
   off.

## 6. Remaining work

1. Observe a GitHub Actions run of the updated workflow. It now includes the
   reference client verification and the browser workflow, whose CI duration is
   unmeasured. This needs the feature branch published and a pull request
   opened, or a push to `main` (owner's call).
2. S17 checkpoint B: reads, pagination, GET/HEAD and a member directory. Not yet
   authorized.
3. A follow-up design for invitation issuance and acceptance.
4. A lifecycle design pass: persistent storage, seed policy, journal mode,
   worker supervision, and one development command. Running the console by hand
   currently takes three processes.
5. Retiring frozen experiments: S16 stays in CI until its remaining omission
   probes are carried by the reference application.
6. Documentation hygiene, carried forward:
   - the top-level README's "Next milestone" omits S16 and S17;
   - `experiments/api-slice/README.md` still lists "durable email delivery" as
     absent;
   - the original "Open decisions" list in `docs/decisions.md` includes items
     settled later.
7. If a content security policy that forbids runtime compilation (no
   `unsafe-eval`) is adopted, replace Ajv's runtime compilation with precompiled
   validators.

## 7. Next chunk

`proposed`: S17 checkpoint B (item 2), subject to the owner's authorization.

- Acceptance: S17's B rows (authorization, pagination and input, cleanup), the
  React member directory with one browser workflow, and the read probes; the
  frozen experiments stay green; each step signed off by the oracle.
- First action: ask the owner the open questions below, one at a time. Plan
  review starts only once checkpoint B is authorized. If it is not, propose the
  documentation-hygiene item instead.
- Branch: continue on `s17-checkpoint-a` unless the owner directs otherwise.

## 8. Decisions and authorizations in force

- The seven S17 owner decisions in S17 "Owner decisions".
- Authorized and complete: the CI prerequisite and S17 checkpoint A. Commits go
  on a branch; no push or merge.
- Approved by the owner this chunk: installing `agent-browser` 0.38.1 locally
  (done).
- Not authorized: checkpoint B, invitations, pushes and merges.
- Decided in this chunk: the choices in the decision record's "Reference
  application checkpoint A, client" entry, the browser workflow's ownership
  rules in section 3, and no frozen experiment retired.
- Workflow: the owner asked for oracle review before every commit.

## 9. Open questions for the user

- May the next chunk implement S17 checkpoint B? This blocks item 2.
- May the driver publish `s17-checkpoint-a` and open a pull request, so the
  updated workflow runs on GitHub Actions? This blocks item 1.

## 10. Operational state

- No running jobs, servers or browser sessions. The browser workflow and the
  probes stop everything they start.
- Installed this chunk: `agent-browser` 0.38.1 in the global bin of mise's Node
  24 (`~/.local/share/mise/installs/node/24/bin`), with Chrome 154 in
  `~/.agent-browser/browsers/`. npm's allow-scripts policy skipped its
  postinstall script; the CLI works. Under another Node it may not be on `PATH`.
- Gitignored build output: `target/`, `apps/reference/web/node_modules` and
  `dist`, and `experiments/api-slice/web/node_modules` and `dist`. Nothing needs
  cleanup.
- Retained evidence, temporary and possibly already deleted: the driver's
  scratchpad (section 5) and the oracle's review directories under
  `/var/folders/f8/ft7ygqg92pj8qh0rwplbw2x80000gn/T/iris-oracle-*` and
  `/tmp/iris-oracle-step3-*`.
- Known risk, carried forward: `probe:s16` builds into the checkout's shared
  `target/` and can leave a mutated artifact that a later run treats as current.
  The experiment is frozen, so the risk is recorded rather than fixed.

## 11. Conventions and gotchas

- In this machine's interactive shells, `tr` is aliased to a trash command,
  `npm` to a package guard and `ls` to another tool. Use `command tr`,
  `command npm` and `/bin/ls`. A bare `tr` was invoked again this chunk and
  tried to trash a file named " "; nothing was moved.
- zsh does not word-split `$VAR` into a command and its arguments; use a shell
  function.
- A user-level hook formats Markdown written through the driver's Write and Edit
  tools. Scripted edits bypass it, so run
  `bunx prettier --write --print-width 80 --prose-wrap always <files>`
  afterwards.
- Never format `apps/reference/web/src/generated.ts`: `verify` compares it byte
  for byte with the generator's output. Pass explicit file lists to Prettier,
  not `src/*.ts`.
- Changing an operation's contract takes two regenerations:
  `cargo run --locked -p iris-reference --bin export-openapi -- apps/reference/openapi.json`,
  then `npm --prefix apps/reference/web run generate`.
- New packages in `apps/reference/web`: install with
  `command npm install --before=<date>` if the lockfile should keep matching the
  experiment's resolved set. A plain install pulled a newer transitive
  `rolldown`.
- Under Node, a TypeScript file that imports JSON needs `with { type: "json" }`.
- `vite preview` must get `--host 127.0.0.1`; its default may bind `::1`. Vite
  8.3 carries `server.proxy` over to preview.
- `agent-browser eval` prints its result JSON-encoded. `network requests --json`
  returns `{ data: { requests: [...] } }`, and aborted requests are included.
- The browser workflow needs ports 4001, 3003 and 5175 free, and refuses to run
  otherwise.
- The driver's tool calls time out at 600 s. Run long loops in the background
  with output written to files. `cargo test` stops at the first failing binary
  unless given `--no-fail-fast`.
- The reference tests and the browser workflow start the local issuer from
  `experiments/api-slice/checks/oidc-provider.mjs`; that frozen file is in use.
- Add to `crates/iris` only what two operations demonstrably share, naming both.
- History on `main` is linear and was previously pushed by the owner. Never push
  without the owner's go-ahead.
- The driver works in the left pane and the oracle in the right. Send oracle
  prompts through a file, as the driver skill describes.

## 12. Skills

- Required: `driver` for the driver, `oracle` for the oracle.
- Optional: `herdr` for pane and agent control.
