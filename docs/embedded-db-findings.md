# Embedded database spike: findings

Measured September 24, 2026. Scope: invitation acceptance, not a framework,
database throughput benchmark, or production qualification.

## Recommendation

Use **SQLx + SQLite as the provisional baseline for the next API slice**. Both
implementations passed the selected behavior checks. SQLx provided migration
tooling directly and had lower clean/incremental test-build cost in this orb.
Retain native Turso as a runnable comparison rather than declaring it
unsuitable.

This is a recommendation supported by this experiment, not a permanent engine
selection or a claim that every SQLx application builds faster. No universal
database adapter is justified by two small implementations.

## Executed evidence

```sh
cargo test --workspace --locked
# SQLite: 10 passed; 0 failed. Turso: 10 passed; 0 failed.
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
# Both checks passed.
python3 experiments/embedded-db/measure.py --samples 3 \
  --output experiments/embedded-db/results.json
# All 36 measured commands succeeded; all six full scenario runs passed.
```

The concurrent-acceptance test also passed 50 additional fresh invocations per
engine using the built test executables with
`scenarios::concurrent_acceptance --exact`. Each invocation creates a fresh
database and two barrier-synchronized tasks. This increases confidence in that
race, but does not exhaust possible schedules.

Both implementations exercised foreign keys, `BEGIN IMMEDIATE`, conditional
`UPDATE … RETURNING`, CHECK/unique constraints, targeted conflict handling, and
explicit rollback. The membership-failure test injects a real database
constraint failure and verifies the invitation claim rolls back through another
connection.

Tests use distinct recipients/projects and check both sides of expiration,
including equality. The database rejects half-populated acceptance state as well
as an accepted user different from the intended recipient. SQL CHECK expressions
must explicitly reject NULL here: an expression evaluating to NULL is not false.

See the [experiment guide](../experiments/embedded-db/README.md) for the exact
behavior choices, commands, and test limitations.

## Build/test feedback measurements

Median seconds across three samples; ranges in parentheses:

| Operation                                 | SQLx + SQLite          | Native Turso           |
| ----------------------------------------- | ---------------------- | ---------------------- |
| Clean test-executable build               | 18.926 (18.570–19.335) | 58.819 (58.713–59.446) |
| Warm no-change build                      | 0.166 (0.165–0.166)    | 0.165 (0.165–1.019)    |
| Function-body edit + rebuild/relink       | 0.973 (0.971–0.976)    | 1.986 (1.947–3.436)    |
| Shared outcome-type edit + rebuild/relink | 1.201 (1.170–1.238)    | 2.200 (2.189–2.310)    |
| Targeted expiration test, warm            | 0.216 (0.215–0.218)    | 0.268 (0.266–0.272)    |
| All ten scenarios, warm                   | 0.277 (0.266–0.320)    | 0.316 (0.316–0.318)    |

Raw [results and source hashes](../experiments/embedded-db/results.json) are
retained. The measurement script checks command exits, uses fresh disposable
target/source directories per sample, alternates engine order, and runs offline
after fetching. No compilation workloads were intentionally run alongside these
measurements.

Environment: Linux x86-64 E2B orb, four available CPUs, Intel Xeon 2.60 GHz,
approximately 8 GB reported memory, Rust/Cargo 1.98.1, default test debug
profile, incremental compilation enabled, four Cargo jobs, compiler wrappers
disabled, no custom Rust flags/linker. System C compiler: Debian GCC 12.2;
system ld: 2.40. Rust's default linker selection was not overridden or
independently profiled.

Dependencies: SQLx 0.9.0, libsqlite3-sys 0.37.0 bundling SQLite 3.51.3, Turso
0.7.2, Tokio 1.53.1. Exact transitive dependencies are in the root Cargo.lock.
SQLx defaults are disabled with runtime-tokio/sqlite/migrate/macros enabled;
Turso defaults are disabled (no opt-in FTS, mimalloc, or cloud sync).

These are **test-executable** builds, not release-server builds. The clean build
includes dependencies and linking but excludes downloads/toolchain installation.
The targeted/full test numbers include Cargo startup, process startup, fixture
creation, and migrations. They are not SQL operation latency. OS caches remain
warm. The third Turso sample has slower no-change/body-edit timings whose cause
was not isolated; ranges are retained rather than discarding outliers. Three
samples in one orb do not establish cross-machine performance.

## Integration lessons

### SQLx supplies more of the initial workflow

The SQLite implementation embeds one SQL migration through SQLx's migration
runner. The native Turso implementation needs its own small version check and
transactional schema application. That is manageable here, but schema history,
checksums, upgrades, and migration diagnostics would require more design.

This spike uses concrete connections, not a pool, and runtime SQL rather than
`query!` checking. SQLx's other capabilities were not verified by implication.
SQLx 0.9's SQL-string safety API required static SQL in our test helpers; bound
parameters are used for input values in both implementations.

### Native Rust does not mean a C-free dependency graph

`cargo tree -p iris-turso-spike -i cc` shows C build dependencies through
`aegis` and `simsimd`, even with Turso's default features disabled. The graph
also includes SDK/sync-related crates without opting into cloud sync. The
earlier research suggestion that native Turso necessarily removes C toolchain
requirements was too broad. Evaluate actual selected features and dependencies.

### Core transaction semantics were sufficient on both engines

We did not need experimental MVCC, cloud infrastructure, or PostgreSQL-specific
locking. Acquiring write intent first, checking the recipient, conditionally
claiming the invitation, and enforcing membership uniqueness sufficed for the
tested single-process races. Busy errors remain explicit and recoverable after
the lock is released; no automatic retry policy is hidden in the implementation.

## Limits and next decision

Not tested: power loss, process crashes, cancellation mid-transaction, commit
failure/uncertainty, multi-process writes, sustained load, migration upgrades,
Turso sync/MVCC, in-memory connection sharing, other platforms, PostgreSQL, or
alternative feature/profile/linker configurations. No property-based or mutation
suite has been run. Graceful close/reopen is not crash durability verification.

Authentication, invitation creation/permission checks, HTTP, OpenAPI, generated
clients, React, and email are still outside this spike. The test user ID is a
trusted input, not an authentication implementation.

Next: sketch the action/HTTP/error contract around the SQLite baseline, then
compare OpenAPI integrations using the generated client. Keep engine-specific
code visible until multiple real actions show which abstractions earn their
cost.
