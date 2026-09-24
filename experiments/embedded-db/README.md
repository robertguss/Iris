# Embedded database experiment

Two concrete implementations of invitation acceptance, not a framework or a
database portability layer. Both compile the same scenario source against
driver-specific test fixtures. The schema and small input/outcome definitions
are also shared source; no runtime adapter trait is involved.

## Run

Install the pinned Rust toolchain, a C compiler, and pkg-config. On an Amp orb,
run `.agents/setup` from the repository root. Open a new login shell afterward
if Cargo was not previously on PATH.

```sh
cargo test --workspace --locked
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Run a candidate independently:

```sh
cargo test --locked -p iris-sqlite-spike --test scenarios
cargo test --locked -p iris-turso-spike --test scenarios
```

No server, cloud account, Docker, credentials, or pre-existing database is
needed. Every fixture creates a temporary directory, applies migrations twice to
exercise rerun behavior, and uses a file-backed database. Connections enable
foreign keys and set a 100 ms busy timeout. Temporary databases contain
synthetic data only.

## Experiment rules

- The caller supplies an already authenticated user ID and a timestamp for this
  attempt. Authentication and invitation-creation authorization are out of
  scope.
- Invitations target a specific user ID, not an email address. Fixtures create
  invitations directly; there is no public invitation-creation operation yet.
- Only token hashes are stored. Example tokens are test data, not a production
  token-generation scheme. Production bearer tokens would require high entropy.
- Unknown token and wrong recipient both produce `NotFound`.
- A previously accepted invitation returns `AlreadyAccepted` for its recipient,
  even after expiration; a different recipient still receives `NotFound`.
- An unaccepted invitation expires when `now >= expires_at`.
- Existing membership is retained without changing its role. A valid invitation
  is still consumed in that case.
- Database errors stay driver-specific. Busy is classified separately; there are
  no automatic retries. Tests retry a fresh attempt only after releasing a known
  write lock. Ambiguous commit outcomes are not retried.

These rules are choices for this experiment, not final Iris public API policy.
`now` is a fixed timestamp supplied for an attempt, not re-read at commit time.

## Transaction shape

1. Acquire write intent with `BEGIN IMMEDIATE` before reading.
2. Look up by token hash **and recipient**, then distinguish consumed/expired.
3. Conditionally claim the invitation with `UPDATE … RETURNING`, retaining the
   recipient, expiration, and unconsumed predicates.
4. Insert membership with a conflict target limited to `(project_id, user_id)`.
5. Commit both writes, or explicitly roll back on an operation error.

SQLx uses its migration runner with an embedded migration. Turso uses a small
explicit one-version migration guarded by `PRAGMA user_version`. The latter is
not a complete migration system: it lacks SQLx's checksum/history tooling.

The Turso implementation finishes and drops the `RETURNING` rows before the next
write. The test suite exercises the complete operation on the pinned release.

## What the tests establish

Ten scenarios per engine cover successful acceptance and reopen, recipient/token
authorization, expiration boundaries, preservation of an existing role,
concurrent acceptance on two independently acquired connections, rollback after
a database constraint failure, foreign-key enforcement on each connection,
bounded lock contention/recovery, database isolation, and invalid direct state
transitions.

The rollback test temporarily tightens the membership table's CHECK constraint.
This forces the real second write to fail after the invitation claim, then
checks state through another connection and verifies a subsequent attempt can
succeed.

The race uses two Tokio tasks synchronized by a barrier, not two futures inside
one shared transaction. It tests single-process connections; it is not a
multi-process or distributed concurrency test. Reopen is graceful close/reopen,
not a crash, power-loss, or fresh-OS-process durability test.

No HTTP, React, email, sync, MVCC mode, connection pool, property-based test
suite, mutation suite, or cancellation/commit-failure fault injection is
included yet. Those omissions are not implied to be verified by these scenarios.

## Reproduce the measurements

```sh
cargo fetch --locked
python3 experiments/embedded-db/measure.py --samples 3
```

The script creates disposable source copies with fresh target directories,
alternates candidate order, builds offline with four jobs, and records:

- Clean test-executable build, including dependencies and linking.
- Warm no-change invocation.
- Rebuild/relink after changing a function-body error string.
- Rebuild/relink after adding an outcome-enum variant in shared source.
- A targeted expiration test and all scenarios, including Cargo/process
  overhead.

Edits happen only in temporary copies. Each sample executes the tests after the
edits. Download caches and the OS file cache are warm; these are **not** fresh
machine installation times. Profiles are unmodified Cargo test defaults with
incremental compilation enabled, wrappers disabled, and Rust's default linker.
Do not run other compilation workloads concurrently with measurements.

Results default to ignored `target/embedded-db-results.json`; logs go beneath
`target/db-measure-*`. Supply `--output <path>` to retain a chosen run. The
script records commands, exit codes, toolchain/machine details, settings, and
source hashes. It aborts rather than treating a failed build or test as a timing
success.

See [the findings](../../docs/embedded-db-findings.md) and the recorded
[raw samples](results.json) for this orb's results and limitations.
