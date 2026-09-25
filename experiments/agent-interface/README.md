# Agent verification interface experiment

A shared local runner with a JSON CLI and a thin stdio MCP interface. This tests
whether explicit conventions and inspectable evidence can shorten the path to an
independently verified change. It is not yet a framework CLI or live runtime
inspector. No improvement in agent repair speed has been measured.

## Run it

Prerequisites: Linux, Git, the repository's Rust toolchain, Node.js 26, and npm.
Cargo and Node must be on the MCP process's PATH. The auth scenario starts its
own loopback OIDC fixture; neither demo servers nor Mailpit are required.

From the repository root:

```sh
npm --prefix experiments/agent-interface ci
node experiments/agent-interface/cli.mjs describe
node experiments/agent-interface/cli.mjs run focused
node experiments/agent-interface/cli.mjs reproduce outbox
node experiments/agent-interface/cli.mjs inspect <run-id>
npm --prefix experiments/agent-interface test
```

Profiles are deliberately fixed:

| Profile     | Checks                                                     | Evidence                                                                                     |
| ----------- | ---------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| `focused`   | Exact logout/callback race and outbox crash-recovery tests | Ordered numeric checkpoints, expectations and observations                                   |
| `compile`   | API Cargo check, default features, all targets             | Exit status and Rust error codes, not rendered diagnostics                                   |
| `contracts` | Existing web `npm run verify`                              | Exit status; install web dependencies with `npm --prefix experiments/api-slice/web ci` first |

`describe` lists exact commands and source files. These subsets do not replace
the full [CI workflow](../../.github/workflows/verify.yml). CLI exits are 0 for
successful execution, 1 for a completed non-passing run, and 2 for an
unavailable operation. `inspect` exiting 0 only means the report was read:
callers must check `stale`, the report status, and the plan before relying on
it.

## Connect an MCP client

Use this stdio server entry in the client's MCP configuration, replacing the
example path with the absolute path of a trusted checkout:

```json
{
  "mcpServers": {
    "iris-local": {
      "command": "node",
      "args": ["/absolute/path/to/Iris/experiments/agent-interface/server.mjs"]
    }
  }
}
```

Launch Node directly, not `npm run mcp`, because npm's status output would
pollute the protocol stream. Allow at least 400 seconds for the focused profile
on a cold checkout. The tools are `describe_application`, `get_convention`,
`run_checks`, `reproduce_scenario`, `inspect_run`, and `inspect_operation`; the
fixed resource is `iris://conventions`. Outputs have schemas and structured
content. Restart the server after changing interface code, its catalog, or its
dependency lockfile; ordinary application edits do not require a restart.

The official `@modelcontextprotocol/sdk` 1.30.1 implements protocol/transport;
Zod 4.6.5 describes inputs and outputs. Librarian research selected the
published SDK instead of implementing JSON-RPC. The Node adapter is provisional,
not a decision to move Iris application logic out of Rust.

## Evidence contract and limits

- A focused scenario passes only when its process exits zero and all expected
  checkpoints occur in order with matching values. A zero-test Cargo result
  cannot masquerade as successful scenario verification. Nonzero exit means
  `check_failed`, not a claim that a particular assertion or root cause is
  known.
- Observed mismatches report expected and actual integers. Missing or malformed
  evidence with a zero exit is `blocked`. Timeouts, output overflow, and missing
  executables are also blocked. Compiler/build/setup failures inside an invoked
  command may only be identifiable as `check_failed` in this pilot.
- Reports under ignored `target/iris-verification/` include a UUID, plan, times,
  source digest, Cargo lock digest, Node/Rust/Cargo versions, and reproduction
  commands. The digest includes tracked and nonignored untracked file contents
  and deletions, excluding `.amp/in/`. Ignored files, external dependencies,
  service state, and arbitrary environment variables are not captured.
- Fingerprints bracket a run, not an atomic filesystem snapshot. Avoid editing
  while a check runs. A detected source change blocks the report permanently;
  `stale: true` means it cannot verify the current tree, even if files are
  restored. Edit-and-revert entirely between snapshots is not detectable.
  Reports are schema-validated, not cryptographically authenticated.
- Events are test-observed checkpoints, **not production traces or deterministic
  replay**. Source links identify where to investigate, not proven causes. Auth
  pauses a fixture at a known boundary; the outbox test advances explicit times.
- Only fixed event names, integers, metadata and compile-profile error codes are
  exported. Raw stdout/stderr, panic text, compiler excerpts, email contents,
  cookies, tokens and SQL parameters are not exported or persisted by the
  runner. For errors not covered by checkpoints, deliberately run the listed
  command in a trusted local terminal; its ordinary output is outside this
  filtering policy.
- Each check has a three-minute timeout and two-megabyte output cap. CLI/MCP
  runs serialize through `target/iris-verification/active`. A crashed runner can
  leave a stale lock; remove it only after confirming no check is still running.
  Tool cancellation is not a rollback or a guarantee that subprocesses stopped.

## Trust boundary

Only use a trusted local checkout. Commands are allowlisted and arguments do not
come from callers, but Cargo build scripts, test code, and npm scripts execute
with the user's permissions. A reduced child environment is not a sandbox:
processes still have filesystem/network access. MCP annotations are hints, not
authorization. There is no production connection or arbitrary command/SQL tool.
External OS/container permissions must enforce any stronger boundary.

An agent can edit the catalog, source, assertions, or reports. These checks are
not a protected evaluator and do not establish business intent. Reviewers must
own acceptance criteria; production permissions stay outside this interface.

## Verification and next experiment

`npm test` uses the official MCP client to discover tools, read conventions, run
both real scenarios, reproduce outbox recovery and inspect their observations.
It also exercises bad inputs, missing/reordered/mismatched evidence, missing
executables, timeouts/output caps, stale reports, lock contention, interface
restart requirements, malformed reports, and canary filtering including Git
stderr. Synthetic bad evidence tests validate the reporter, not its ability to
diagnose every real application regression.

**Deferred as of September 25, 2026:** the owner chose framework design before
further productivity experiments. See the
[living design spec](../../docs/design-spec.md) for the current direction. The
following study remains a proposal, not the next authorized task.

When resumed, compare fresh agent sessions using ordinary Cargo/repository
tooling versus the same tasks with this interface. Seed logout-race and
outbox-fencing defects in disposable checkouts, include valid-change controls,
and retain independently reviewed acceptance checks outside each agent's
editable workspace. Measure accepted repairs within a fixed budget, elapsed
time/tool cost, unnecessary edits, weakened checks, human intervention and
disclosure incidents. This study has not run; expand the interface only when an
observed repair obstacle warrants it.
