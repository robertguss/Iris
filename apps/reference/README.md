# Iris reference application

The S17 reference application: membership management over session identity,
built to test whether the S16 contract generalizes across operations before any
of it becomes framework API. It is application code; `crates/iris` holds only
what two operations demonstrably share and is private and provisional. See
[S17](../../docs/design-spec.md#s17--reference-application-and-first-reads) for
the design, its checkpoint A
[server-side](../../docs/design-spec.md#checkpoint-a-server-side-evidence) and
[client](../../docs/design-spec.md#checkpoint-a-client-evidence) evidence, and
its checkpoint B
[server-side](../../docs/design-spec.md#checkpoint-b-server-side-evidence) and
[client](../../docs/design-spec.md#checkpoint-b-client-evidence) evidence, and
the mutations'
[current-state read](../../docs/design-spec.md#current-state-read-evidence).

**Status:** checkpoints A and B are implemented. Checkpoint A: both membership
operations, their React client and one browser workflow. Checkpoint B: both
reads, with GET and HEAD; the React member directory; and one browser workflow.
The mutations then declared `listProjectMembers` as their current-state read,
which the console offers after an unconfirmed attempt; a third browser workflow
covers it.

## Tracked COMMIT-failure disposal checks

From the repository root, with an absolute private target directory:

```sh
CARGO_TARGET_DIR=/tmp/iris-tracked-check cargo test --locked -p iris-reference --lib tracked_commit
```

Two tests compose a real deferred-FK COMMIT error with bounded acknowledged
`Tracked` disposal, immediate WAL absence before reopening, independent state
readback and fresh writer progress. The same-shaped valid-FK control requires
the owner role and exactly its trigger row to persist. These tests start no
provider, pool, HTTP server or `Storage`; WAL belongs only to this disposable
fixture. The error stays `Unconfirmed`, even after closure. They do not prove
arbitrary commit ambiguity, rollback I/O failure, failed-handle reuse or
process/storage-lock handoff. See the
[contract and limits](../../docs/design-spec.md#tracked-deferred-commit-disposal-evidence--october-2-2026)
and
[measured checks](../../docs/decisions.md#tracked-commit-disposal--october-2-2026).

## Future invitations: not runnable yet

ROB-1113 records an **accepted future design, not an implemented or verified
feature**. The commands below still run only the existing reference application:
they do not start an invitation worker or mail capture, and there are no
reference invitation routes or views yet. The frozen experiment's delivery
commands and token-preview response are not reference-app instructions.

[S19](../../docs/design-spec.md#s19--reference-invitations-and-delivery) owns
the future contract, decision/failure-window tables and bounded later stages;
the
[dated decision](../../docs/decisions.md#reference-invitation-design--october-2-2026)
records rationale and provenance. Public issue and accept operations must land
with their OpenAPI export, generated TypeScript, client tests, presentation,
views and browser workflow in the same later green candidate. No runtime test or
mail-retention result is claimed by this documentation change.

Future prerequisites, not steps to run now:

- Private persistence/domain and delivery/lifecycle work must first supply
  append-only migrations, atomic invitation/outbox storage, tracked worker
  connections and supervised shutdown under the existing 3-second drain,
  1-second close and 5-second outer kill deadlines.
- Future fresh initialization or explicit reset adds `.test` contacts. Existing
  databases are never reseeded or backfilled and remain usable; issuance with a
  missing usable contact returns 409 `invitations.recipient_unavailable` after
  the preceding authority/conflict checks. Reset is destructive and is not
  required merely to keep using an old database.
- A dedicated loopback-only capture service, without relay, needs a pinned and
  verified configuration. The 24-hour retention precedent and proposed
  500-message cap are not measured reference behavior. Resetting the application
  database does not clear that separate inbox or remove its old links.
- The intended happy demo is Bob (`29`) inviting Alice (`11`) to project `43`.
  Issuance acknowledges enqueue, not mail delivery, and returns no token. The
  recipient signs in first, then opens or reopens the capture link and
  explicitly accepts. Full login discards the memory-only fragment credential;
  neither browser storage nor an invitation lookup can recover it. Unknown
  outcomes do not permit automatic retry or become confirmed by later membership
  reads.

## Caller-loss boundary checks — October 2, 2026

Run `cargo test --locked -p iris-reference --lib caller_loss` from the
repository root. Four tests cover authenticated role change: owned-future abort
before mutation, full raw-socket loss before commit, full raw-socket loss while
a successful inner response is held before exposure, and a loss-free control
using the same framing and holds. The socket cases require an explicit
non-panicking `Dropped` event before releasing the hold, not mere disappearance
of a connection. Every case checks acknowledged tracked closure, independently
reads state, and commits and reads back a third role through independent SQL to
prove writer progress.

The four tests passed on Linux with Axum 0.8.9, Hyper 1.11.1 and SQLx 0.9.0;
four isolated mutants were caught with green controls. Workspace tests passed
202 cases, including the reference library's 99. Strict Clippy, rustfmt, web
verification and all three browser workflows passed. Commands, tool versions,
mutant failures and isolation details are in the
[dated decision](../../docs/decisions.md#observable-membership-caller-loss--october-2-2026).

Only test hooks and evidence changed. These holds do not interrupt a commit or
distinguish FIN from RST, cover removal, promise continued execution, or supply
a receipt. A caller without a validated terminal response still has an unknown
outcome. The older browser-withheld-response checks below remain distinct from
these real socket-loss tests.

## Actual-COMMIT cancellation checks — October 2, 2026

ROB-1111 adds three authenticated owned-future controls, separate from the four
historical socket tests above. Run from the repository root:

```sh
cargo test --locked -p iris-reference --lib commit_latch
cargo test --locked -p iris-reference --lib commit_hook_
cargo test --locked -p iris-reference --lib caller_loss
```

The first command checks persistent early release, controller-drop cleanup and
unoverwriteable deadline provenance. The three request cases cancel before
COMMIT, cancel while the actual transaction's SQLite commit hook is held, and
retain the request through a schema-valid successful acknowledgment. Each checks
tracked closure, exact independent role readback, and a fresh one-row writer's
commit/readback. While COMMIT is held, tracked closure remains unacknowledged
and the same application's unmatched real HTTP GET returns 404 without database
or session access. No fresh database read is attempted during the hook hold.

The test pins SQLite 3.51.3, its full source ID and DELETE journal mode before
installing the test-only hook; a mismatch requires mechanism re-review. Internal
waits are bounded, a synchronous 30-second latch deadline unblocks the worker,
and abnormal release provenance fails the scenario. See the
[dated evidence](../../docs/decisions.md#rob-1111-actual-commit-cancellation--october-2-2026)
for the fingerprint, eight compiled counterfactuals, controls and verification.

This demonstrates continuation of an already-entered COMMIT on the pinned stack
after owned-request cancellation. It is not actual socket-loss evidence inside
COMMIT, a detached-operation policy, durability at hook entry, a receipt, safe
retry, or guaranteed commit/liveness under faults. S14 item 3 remains partial.

## Run it

From the repository root, with Rust 1.98.1 and Node available (the tests start
the local OIDC issuer fixture from `experiments/api-slice/checks`):

```sh
cargo test --locked -p iris -p iris-reference
npm --prefix apps/reference/web ci
node apps/reference/scripts/probes.mjs
npm --prefix apps/reference/web run verify
node apps/reference/scripts/browser.mjs --artifacts /tmp/reference-artifacts
cargo run --locked -p iris-reference --bin export-openapi -- apps/reference/openapi.json
npm --prefix apps/reference/web run generate
```

`openapi.json` is the committed export. A Rust test fails on drift and names the
export command; `verify` fails when `web/src/generated.ts` no longer matches it
and names `generate`. `verify` also captures real responses from the Rust
whole-request tests, type-checks, runs the decoder, presentation,
request-construction and directory cases, and builds the client. The browser
workflows need `agent-browser` 0.38.1 and its browser (`agent-browser install`).
The script builds and starts everything it uses and stops it on exit. It
restarts the API before checkpoint B's workflow and again before the
current-state read workflow, so each starts from the seed data.

To use the console, run the development command from the repository root, then
open `http://127.0.0.1:5175` and sign in as Alice or Bob:

```sh
node apps/reference/scripts/dev.mjs
```

It needs `npm --prefix apps/reference/web ci` once. It refuses to start while
any of ports 4001, 3003 or 5175 is taken, builds the development server, then
starts the local issuer on 4001, the server on 3003 and Vite on 5175, each only
once the one before has reported its address, and prints `dev: ready: …` when
all three have. Each child's output is shown under its name (`[issuer]`,
`[api]`, `[web]`). A final line without a trailing newline is printed once when
that stream closes, before the command's final `dev: exit …` line. Complete
lines, blank lines, whitespace-only tails, stdout/stderr separation, chunk local
decoding and readiness scanning otherwise keep the existing behavior. The data
persists in `apps/reference/.dev/reference.db`, which is gitignored;
`--database PATH` uses another file, resolved against the current directory, in
a directory that must exist. The command sets every address it owns itself: an
inherited `IRIS_API_TARGET`, `IRIS_LISTEN`, `IRIS_PUBLIC_ORIGIN` or
`IRIS_OIDC_ISSUER` does not reach its children, so the console cannot reach
another server or database.

To stop it, press Ctrl-C or send it SIGTERM. It sends SIGTERM to each child's
process group, SIGKILL to any group still running five seconds later, and waits
for all three process groups and their stdio closure. It prints how each child
exited, then exits 0 once all three have, whatever their own exit codes, or 1 if
one needed SIGKILL. A descendant that escaped the owned group can keep an
inherited pipe open and delay the final report; there is no stream destroy,
forced process exit or output-drain timeout. The server's exit code is its own
drain and closure outcome, described below: its exit 1 after an expired drain is
printed, not treated as a failure of the command. A later Ctrl-C does not
shorten the stop. If a child exits on its own, or is not ready within 60
seconds, the command stops the others and exits 1, naming it.

To start over, reset through the command:

```sh
node apps/reference/scripts/dev.mjs --reset
```

It runs the server's own reset, described below, on the same database with the
command's issuer, and starts nothing else. It is refused while a server runs on
that database. A reset interrupted by a signal exits 1; run it again.

The command's tests start it and its children on the same fixed ports, so they
refuse to run while any is taken and must not run beside the browser workflow.
The port check is not a reservation. Readiness is still recognized from child
logs, not active probes. The defensive stop-before-spawn guard remains even
though current callers do not reach it. The macOS EPERM/catch-all process-group
probe and escaped-session limit are unchanged pending macOS-specific evidence; a
Linux result would not settle that behavior. CI runs them in the foreground
immediately after reference client verification, before the browser workflows in
the same serial job. To run them locally:

```sh
node --test apps/reference/scripts/test/dev.test.mjs
```

To start the three by hand instead:

```sh
node experiments/api-slice/checks/oidc-provider.mjs --port 4001 --issuer http://127.0.0.1:4001 --redirect-uri http://127.0.0.1:5175/api/auth/callback
IRIS_PUBLIC_ORIGIN=http://127.0.0.1:5175 IRIS_OIDC_ISSUER=http://127.0.0.1:4001 cargo run --locked -p iris-reference --bin reference-dev -- --local-oidc-demo
npm --prefix apps/reference/web run dev
```

The development server listens on `127.0.0.1:3003` (`IRIS_LISTEN` overrides it)
and seeds Bob as an editor of Alice's project 41 so a role change or removal can
succeed without invitations. By default it uses a disposable SQLite database, so
restarting it resets the data; the development command always passes a database
path.

To keep data across restarts, pass a path after the flag, for example
`-- --local-oidc-demo --database /tmp/iris-reference.db` (the development
command's default is `apps/reference/.dev/reference.db`). The path is only ever
an argument, never an environment variable, so the tests and the browser
workflow stay disposable. The parent directory must exist. On the first start
the server builds, migrates and seeds the database in a staging directory next
to it (`<name>.iris-init`) and publishes it only when complete; later starts
apply new migrations and never seed again. The seeded identities are bound to
the issuer in use when the database was created. One server owns a path at a
time: a lock on `<name>.iris-lock`, released when the process exits even after a
crash, refuses a second start. The server also refuses a database that is a
symbolic link, has another hard link, is not in SQLite's rollback journal mode,
is missing while its `-journal`, `-wal` or `-shm` files remain, or whose name
ends in one of those three suffixes.

After opening and migrating the database, `reference-dev` inspects its external
identity mappings before contacting the configured provider or binding its
listener. A populated table must contain at least one issuer exactly equal to
`IRIS_OIDC_ISSUER`; other issuers may coexist, and subjects need not be the
seeded names. The comparison does not normalize URLs, so a trailing slash is a
different issuer. If no mapping matches, the read-only issuer check refuses
startup and first recommends restoring the matching issuer; its printed reset
command is the explicitly destructive alternative. `Storage::open` may already
have applied pending migrations before that check. An empty mapping table warns
and starts normally without seeding it. An inspection error is fatal.

To reset a database by hand, run the server's reset rather than deleting its
files:

```sh
IRIS_OIDC_ISSUER=http://127.0.0.1:4001 cargo run --locked -p iris-reference --bin reference-dev -- --local-oidc-demo --database /tmp/iris-reference.db --reset
```

The reset takes the same lock, so it is refused while a server runs on the path.
It deletes the database and its `-journal`, `-wal` and `-shm` files, builds and
seeds a new database against `IRIS_OIDC_ISSUER`, prints
`reset database at <path>` and exits without listening or contacting the issuer.
It keeps `<name>.iris-lock`. Sessions and login attempts live in the same
database, so every browser session ends. Before deleting anything it checks the
path as a start does, and it deletes the database file only if that file is
empty or starts with SQLite's header: a plausibility check, not proof that this
application made the file. It never opens the old database, so it also works on
one left in WAL mode or refused for a modified migration; a database whose
header bytes are damaged is refused and must be removed by hand. If a reset is
interrupted, run it again.

Once a persistent database exists, `migrations/` is append-only: change the
schema by adding a migration, not by editing an applied one. SQLx records a
checksum for each applied migration, and a start against an edited one stops
with a message naming the database and the migration version. The reset command
in a refusal has the path quoted for a POSIX shell, so it can be pasted as
printed. Startup never resets. To keep the data, restore the applied migration's
source and put the intended change in a new migration; to discard it, run the
reset the message names.

To stop the server, send it SIGINT (Ctrl-C) or SIGTERM. It prints
`received SIGTERM; draining for up to 3s`, stops accepting connections, and lets
requests in flight and the session cleanup finish. Once they have, it closes the
session pool and waits up to one second for every request's database connection
to report that it closed, releases the lock, prints `stopped after draining` and
exits 0. A later signal neither shortens nor extends this.

If a request is still running after three seconds, the server prints that the
drain deadline expired and exits 1 without sending it a final response. The
caller has no validated final response (an interim `100 Continue` may already
have been sent), so the outcome of that request is unconfirmed: the exit is
neither an acknowledgment nor proof of a rollback. The server also exits 1,
after the one second, if a connection's closure was never acknowledged. That
includes a request that failed to open its connection at any time while the
server ran (for example against a busy database): nothing is left to confirm
that such a connection closed, so the stop is reported as unestablished closure,
not as a known open connection. In both cases the process ends while still
holding the lock, so no other server or reset can start while a connection might
be live, and a disposable database's temporary directory is left behind. The two
deadlines are fixed at three seconds and one second, measured to end inside five
seconds; they are not configurable.

While it runs, the server deletes expired sessions and login attempts at start
and every 60 seconds. A database error in that task is printed and the next tick
tries again. If the task itself panics or ends, the server prints
`task session-cleanup panicked` (or `stopped unexpectedly`), drains as above and
exits 1.

### A database at a sidecar name

SQLite keeps a database's recovery files beside it as `<name>-journal`,
`<name>-wal` and `<name>-shm`, and a reset deletes them. Those three names
therefore belong to the database `<name>`, and a start or reset of `<name>` is
refused, leaving the database and those files as they are (it may create an
empty `<name>.iris-lock`), when one of them shows recognizable evidence of being
something else: it is not a regular file with one link, it starts with SQLite's
database header, or it has an `.iris-lock` or `.iris-init` sibling of its own
(whether or not the file itself exists yet). These checks detect that evidence;
a file that passes them is not proven to be a sidecar, and a reset deletes it.

A binary from before this rule could create a database at such a name, for
example `dev.db-wal`. To keep one, move it as a complete file set:

1. Stop every process using it.
2. If `<name>.iris-init` exists, decide whether it is the initializer's. Check
   with `ls -la` on the name itself and on its contents, without a trailing
   slash. It is the initializer's only if all of these hold:
   - it is a real directory, not a symbolic link;
   - it holds nothing but `owner`, `reference.db` and that file's `-journal`,
     `-wal` and `-shm`;
   - `owner` is a regular file, not a symbolic link, containing exactly the
     database's canonical path followed by a newline;
   - `reference.db` is absent or a regular file, not a symbolic link.

   If any of these fails, including a missing, empty or partly written `owner`,
   leave everything as it is and inspect it by hand; this guide offers no
   procedure for it.

3. If it is the initializer's, its `reference.db` is the published database only
   if `<name>` exists and `stat` reports the same device and inode for both,
   with a link count of 2 (`stat -f '%d %i %l' A B` on macOS,
   `stat -c '%d %i %h' A B` on Linux). In every other case it is an unpublished
   database that may be partly migrated or unseeded, and is not worth keeping.
   Remove the directory by deleting `reference.db` and its `-journal`, `-wal`
   and `-shm` first, then `owner`, then the directory itself with `rmdir`. Never
   remove it recursively.
4. Move `<name>` to its new name, and each of `<name>-journal`, `<name>-wal` and
   `<name>-shm` that exists to the matching new name (`<new>-journal`, and so
   on). A `-journal` or `-wal` left behind can hold changes the database needs,
   so never move the database without them.
5. Remove the old `<name>.iris-lock` by hand once nothing uses it. While it or
   an `.iris-init` sibling exists, the database that `<name>` would be a sidecar
   of stays refused.

## Layout

| Path                                       | Owns                                                                                                                                                                          |
| ------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/app.rs`                               | State, connections, the migration, seed fixtures, and the one checked assembly behind `app()` and `openapi()`                                                                 |
| `src/identity.rs`                          | Session/OIDC identity copied as application code; session endpoints keep their `{code,message}` bodies                                                                        |
| `src/domains/mod.rs`                       | The stage, failure and cleanup vocabulary that mutations and reads share                                                                                                      |
| `src/domains/memberships.rs`               | Commands, rejection types, one private SQLite transaction for both mutations, and member listing                                                                              |
| `src/domains/projects.rs`                  | The caller's own projects, filtered by actor                                                                                                                                  |
| `src/read.rs`                              | One owned `query_only` connection and deferred transaction per read, finalized and classified                                                                                 |
| `src/http/`                                | DTOs, declarations, endpoints, mounts, the session-marker classifier, wire IDs, page parameters                                                                               |
| `src/storage.rs`                           | The development database: one owner per path, atomic initialization with the development fixtures, the rollback journal check, reset, and the refusal of a modified migration |
| `src/bin/reference-dev.rs`                 | The `--local-oidc-demo` development server, disposable unless given `--database PATH`; with `--reset`, replaces that database and exits                                       |
| `web/src/client.ts`                        | The whole-request boundary: linkage, the recovery parse, single bounded reads and validation against the bundled export                                                       |
| `web/src/membership.ts`                    | One attempt per domain request: mutations as POST with the CSRF token, reads as a bodiless GET with one overload per read; the declared readback's inputs                     |
| `web/src/session.ts`, `main.tsx`           | Session bootstrap over the existing session contracts, and the member directory console with its readback                                                                     |
| `web/src/directory.ts`                     | The directory's paging, result pairing and staleness rules as pure transitions                                                                                                |
| `web/src/present.ts`                       | Outcome wording, exhaustive over all four operations' codes; read wording scoped to the page as read; the readback pointer                                                    |
| `scripts/dev.mjs`, `scripts/supervise.mjs` | The development command, and the process ownership it shares with the browser workflow                                                                                        |
| `migrations/`, `tests/`, `scripts/`        | Schema; contract, session and development-server tests; omission probes, the browser workflow and the development command's tests                                             |
| `../../crates/iris`                        | Envelope rendering, the response bridge, the shared profile, the request-ID boundary (HEAD served as GET), assembly and current-state read linkage checks                     |

## Operations

| Operation                   | Route                                    | Success                                   | Rejections                                                                                    |
| --------------------------- | ---------------------------------------- | ----------------------------------------- | --------------------------------------------------------------------------------------------- |
| `memberships.change_role`   | `POST /api/memberships/role`             | 200 `{completion:"acknowledged"}`         | 403 `memberships.forbidden`; 404 `memberships.member_not_found`; 409 `memberships.last_owner` |
| `memberships.remove_member` | `POST /api/memberships/remove`           | 200 `{completion:"acknowledged"}`         | Same as `change_role`                                                                         |
| `memberships.list`          | `GET /api/projects/{project_id}/members` | 200 page of `{user_id,display_name,role}` | 403 `memberships.forbidden` for an unknown project or a non-member                            |
| `projects.list_mine`        | `GET /api/projects`                      | 200 page of `{project_id,name,role}`      | None; rows are filtered by actor                                                              |

Every operation also declares 400 `http.invalid_request`, 401
`http.unauthenticated`, 500 `iris.internal` and 503 `iris.unavailable`; the
mutations add 403 `http.csrf_refused`. Reads declare no recovery capabilities.
Both mutations declare `listProjectMembers` as their current-state read, fed
from the request's `project_id`: a fresh first page, sent under the caller's
current authorization. A returned page, a member's absence or a 403 resolves
nothing about the attempt, and the read authorizes no new submission.

Reads return `{items, next_cursor}` pages in ascending ID order, with IDs as
decimal strings. `limit` is a decimal from 1 to 100 written without leading
zeros, so `05` is refused, and defaults to 50. `cursor` echoes an earlier page's
`next_cursor` (`c1.` and a position, at most 32 bytes); `next_cursor` is null
when no further rows existed when the page was read. Unknown or duplicate
parameters and invalid values return 400, after authentication. Visibility is
checked on every page, so a forged cursor can only reposition within the
caller's rows. HEAD returns GET's status and headers without a body.

## Verification matrix

Measured September 26–27, 2026, on macOS with Node 24.20.0 (CI pins 26.10.0);
the client checks also passed under Node 26.8.1. The development server,
development database, lifecycle and workspace rows were measured again, and the
development command row first, on September 30.

| Check                  | Result                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `iris` unit tests      | 34 passed: component and path conflicts, duplicate IDs and names, surviving handler IDs, unbridged operations, five kinds of shared-code metadata drift, HEAD served as GET only for GET operations, and reads without recovery metadata. Current-state reads: rendering, a path parameter bound twice, and a linked document passing (also with an inline body, and with no read declared); each unsupported declaration fails with its own message: `true`, extra fields, an unresolved or non-Iris target, a POST target, a target with recovery, missing or extra bindings, required non-path parameters (including query, header and cookie ones sharing the bound name), a missing or non-local request body, and a body field that is undeclared, optional or of a different schema                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| Ported S16 Rust suites | 10 passed. Whole-request fixtures run against the assembled application; focused tests exercise domain code or hand-built routers. Expectations are unchanged except that the marked-response case uses `LoginFailed` (401), because the session enum no longer has `Forbidden`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| `remove_member`        | Independent contract, whole request (every status, schema-validated), and two concurrent owner self-removals leaving one owner                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| Cross-operation        | Identical declared mappings; full recovery metadata, including the declared current-state read, written by hand for both                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| Read transactions      | Finalize classification (a rejection whose rollback fails is never a rejection), pages, a writer committing after success and rejection, `query_only` refusing a write, a cancelled read releasing a writer it blocked, and a page-query failure for each read                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| Reads                  | For each read: an independent contract including its parameters; whole requests at every status, schema-validated; authorization (uniform 403, any member role, removal between pages, exact own projects, email canaries, GET without CSRF); pagination (default, 1, 100, traversal, foreign and forged cursors); input refusals after authentication; and HEAD                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| Contract               | Snapshot drift, and a hand-written inventory of eight paths, methods and operation IDs                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| Session                | 7 passed, including CSRF matrices, a wrong-token case and revocation after user deletion; CSRF and permission checks cover both domain routes                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| Development server     | 12 passed on September 30: it refuses to start without `--local-oidc-demo` or with an unknown argument; serves the session, the domain boundary (an anonymous 401 envelope) and a plain 404; keeps a `--database` path across a kill and admits one process; refuses `--reset` while a server runs, keeps a signed-in session across a restart and ends it after a reset; resets with only the issuer's name configured and without listening; treats `--reset` without `--database` or out of place as a usage error; and runs the reset command a refusal prints, as printed, for paths with spaces, quotes and shell metacharacters; stops with code 0 on SIGTERM and on SIGINT when idle, freeing the path at once; completes and acknowledges a role change that was in flight at the signal; abandons one never finished at the 3 s deadline with code 1 and no final response; and deletes expired sessions and login attempts at start                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| Client decoder         | 231 cases: 19, 17, 12 and 9 real responses from the four operations, counted per status, kind and code; independent envelopes, mismatches and undeclared statuses for each operation, including pages with additive fields for the reads; 13 body-handling cases (oversize, unusable, non-JSON, thrown) for every operation, tallied per operation; one attempt each and no raw diagnostics. Linkage refuses a read declared as POST and a mutation as GET                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| Client recovery        | Both mutations' declared read pinned by hand; a `read: false` document accepted; 16 unsupported documents refused, one per clause (extra fields, inspection, replay, the new-submission constraint, `true`, an unknown read, a mutation as the read, missing, extra or malformed bindings, unknown or optional body fields, a body outside component schemas, recovery on a read); `api.recovery` refuses a read at compile time                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| Client types           | Status and code narrowing for all four operations, distinct operation literals, no contact field on either summary, read parameters (a project ID only for members, and a read must be narrowed first), and the bundled snapshot accepted as a document without a cast                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| Presentation           | 51 cases: every declared outcome of all four operations and `client_unknown`; unconfirmed outcomes have exact wording, point to the declared read in page-scoped words, and claim no effect; the pointer follows the declaration; listings show only declared fields and every sentence is scoped to the page, including an empty continuation page                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| Request construction   | 13 cases: each read is one GET with no body or CSRF header, the project ID encoded, `limit` and `cursor` only when given and the cursor exactly as received; a mutation is one POST with its body and the CSRF token; the readback's inputs follow the declared binding, are absent without a declaration, and send nothing                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| Directory              | 11 transition cases: first page on sign-in, forward paging with the shown cursor, resets, results accepted only under their token (never reused), and a listing marked when an attempt was sent after its request                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| Browser workflows      | Five consecutive passes of the final script, about 15 s each with builds cached. Checkpoint A, from the directory: OIDC sign-in and bootstrap recovery; role change with the listing marked while the response is held, and its acknowledged outcome kept across a reload with no readback offered; one unconfirmed attempt kept across a reload, a new selection and refreshes; non-owner refusal; removal with per-member confirmation, absence from the stale row and last-owner protection. The API is then restarted. Checkpoint B: a lost initial projects read and a lost members read, each sent once; traversal of members and own projects one row per page to the end; a member who left the project, with the response withheld after the commit, is refused their next page, and the readback, used from another project, is refused too, with the outcome and the offer kept. The API is restarted again. The current-state read, on fresh data: a withheld role change read back showing the new role, and a withheld removal, sent while page 1 had a next cursor, read back from page 1 with the member absent; each attempt sends one mutation and each readback one first-page GET, and the outcome stays unconfirmed. For each withheld response, the request reaches the server, a wrapper records the response and throws, and the runner then checks the recorded 200 acknowledgment independently. Then HttpOnly session, narrow layout and no browser storage. Earlier probes: a taken port, foreign servers, a spawn error, SIGTERM, a child that never reports ready, a child killed mid-run and a build that ignores SIGTERM each stop it without a PASS or leftover processes, and a second invocation is refused without disturbing the first. Each restart: a foreign listener on its port, the replacement killed, and SIGTERM inside the window each stop it the same way, with no replacement started after a stop; the second restart's window was probed separately |
| Development database   | 38 passed on September 30; [S18](../../docs/design-spec.md#reset-and-migration-evidence) maps them to its acceptance rows                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| Lifecycle              | 22 passed on September 30; [S18](../../docs/design-spec.md#shutdown-and-session-cleanup-evidence) maps them to its acceptance row                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| Development command    | 22 passed on September 30 under Node 24.20.0 and 26.8.1, and ten consecutive times under 24.20.0 just before a final two-line change; [S18](../../docs/design-spec.md#development-command-evidence) maps them to its acceptance row                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| Workspace              | 198 passed on September 30; the frozen experiments stay green                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |

## Cost of runtime validation

| Measurement                                  | Result                                                                                                                                                                                                                                                                                                                                                                                            |
| -------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Production bundle (`vite build`)             | With the readback: 391.75 kB JavaScript, 115.10 kB gzip. With the directory: 389.42 kB, 114.41 kB gzip. Four operations before it: 382.39 kB, 112.55 kB gzip. Checkpoint A's two: 374.32 kB, 111.87 kB gzip                                                                                                                                                                                       |
| Ajv (difference from a stub-validator build) | 130.62 kB, 38.24 kB gzip, measured with two operations                                                                                                                                                                                                                                                                                                                                            |
| Validator compile, production build, browser | Four operations with the readback: 17.3–29.4 ms, median 19.0 ms, across five runs of all three workflows; the 29.4 ms run was not investigated. With the directory: 18.0–20.0 ms, median 19.1 ms, across five runs of both workflows. Before it: 17.5–21.0 ms, median 19.5 ms, across nine runs at `5b16132`. Two operations: 13.3–24.1 ms, median 13.9 ms, across nine runs. Headless Chrome 154 |
| Validator compile, Node                      | Two operations. 24.20.0: 41.5 ms cold, 10.4 ms warm median; 26.8.1: 30.6 ms cold, 9.0 ms warm median                                                                                                                                                                                                                                                                                              |

The browser figure is the `iris:client-compile` performance measure, taken once
per page load.

## Omission probes

`scripts/probes.mjs` edits a disposable source copy, builds it in its own
temporary target, and removes both on exit. It never edits the checkout. Install
dependencies as setup before running it; the runner never installs packages:

```sh
cargo fetch --locked
npm --prefix apps/reference/web ci
node apps/reference/scripts/probes.mjs
```

The runner copies installed web dependencies (not a symlink, because Vite writes
inside `node_modules`) and Cargo registry/git caches. Cargo runs offline with a
private home; build, cache and fixture writes stay disposable. Direct and nested
commands override inherited `CARGO_TARGET_DIR`, and inherited
`IRIS_REFERENCE_FIXTURES` is removed. Allow disk space for a cold target and the
dependency copies. Both before the first mutation and after byte-for-byte
restoration, it runs `cargo test --quiet --locked -p iris -p iris-reference` and
the real web verifier. Behavioral negatives require the named failing test and
its intended diagnostic; compilation failures or zero tests do not count. `PASS`
is printed only after cleanup succeeds, not before `finally`.

The
[complete S16-to-reference mapping](../../docs/design-spec.md#reference-omission-parity)
adds shared-rejection omissions, status/export/generated-client drift, required
success projection, raw DTO bypass, runtime mounting and both CSRF directions to
the existing checks below. A valid mounted request first passes with 200; its
omitted-mount variant asserts raw 404 versus 200 before attempting to decode an
envelope. The declared inventory and export remain valid in that variant. The
[dated evidence](../../docs/decisions.md#reference-omission-parity--october-2-2026)
records commands, counts, timings and limits. Historical timings before these
additions were 72 s after current-state reads, 84 s with checkpoint B, and 39 s
for checkpoint A. Frozen experiment source remains unchanged; the bounded
ROB-1115 CI retirement below supersedes the earlier all-entrypoints-in-CI state.

After integrating the separately reviewed ROB-1121 observer tests, the complete
runner passed 31 caught probes and 25 controls in 222.75 s on Linux. Its healthy
reference library control now includes 103 tests. The earlier SQLite Busy
failures remain recorded; no historical scheduling interleaving is claimed
reproduced.

That plain-output run did not prove CI's forced-color behavior. After a CI
failure, diagnostic matching now uses Node's ANSI-stripped snapshot while thrown
errors retain raw output. A real run with `NO_COLOR` unset,
`CARGO_TERM_COLOR=always` and `FORCE_COLOR=1` passed the same 31 probes and 25
controls in 230.37 s, including healthy controls and cleanup. The decision
record retains the CI failure and the real-ESC replay checks separately.

Later CI exposed a separate startup-test Busy observer. After integrating the
reviewed ROB-1176 dependency, the byte-identical runner passed the complete
forced-color 31/25 sequence in 219.82 s. Separate healthy before/after suites
passed 163 Rust tests, including 103 library and 17 dev-binary tests, plus the
web verifier. The CI failure and unknown historical interleaving remain
recorded; no probe or control was relaxed.

| Temporary change                                           | Executed signal                                                              |
| ---------------------------------------------------------- | ---------------------------------------------------------------------------- |
| Second operation collected without the bridge              | Assembly panics: surviving inferred handler ID `remove_member_endpoint`      |
| Second operation reuses `changeMemberRole`                 | Assembly panics: duplicate OpenAPI operation ID                              |
| Second operation changes `memberships.forbidden`'s message | Assembly panics: the public code differs between the two operations          |
| Wrong route path                                           | Compiler passes; the independent inventory fails                             |
| Same-named success component with a different definition   | Assembly panics: component conflict `schemas/ChangeRoleSuccess`              |
| Same-named success component with an identical definition  | Control: assembles, exported once and referenced by both operations          |
| Second operation left out of collection                    | The independent inventory fails; this does not probe a missing runtime mount |
| `limit` accepted up to 101 at runtime                      | Compiler passes; both reads' input-refusal tests fail                        |
| Exported `limit` pattern admits 101                        | Compiler passes; the hand-written parameter assertion and export drift fail  |
| Member listing without its membership check                | Compiler passes; the member-listing authorization test fails                 |
| Own-project listing without its actor filter               | Compiler passes; the own-project authorization test fails                    |
| The mutations' current-state read names no operation       | Assembly panics: the recovery read is not in the document                    |
| The current-state read binds `user_id` to the project path | Control: assembly accepts it; the hand-written recovery test rejects it      |

These are selected seeded mistakes, not exhaustive mutation testing. The client
and browser checks were also mutation-tested while they were built; the decision
record lists those mutations for
[checkpoint A](../../docs/decisions.md#reference-application-checkpoint-a-client--september-27-2026),
[checkpoint B](../../docs/decisions.md#reference-application-checkpoint-b-client--september-27-2026)
and the
[current-state read](../../docs/decisions.md#reference-application-current-state-read--september-27-2026).

## Partial S16 CI retirement

ROB-1115 removes automatic direct `verify:s16` and frozen omission mutations,
not the frozen Rust tests or runner safety regression. The
[assertion-level matrix](../../docs/design-spec.md#partial-s16-ci-retirement)
maps all 10 Rust tests, 19 captured plus 25 hand-written client cases, uncounted
recovery and type narrowing, both drift edges, 16 omissions and 7 controls to
maintained reference checks. It explicitly records the marked-response and
current-state-read semantic adaptations; aggregate test counts are not parity.

CI still runs normal frozen-web `verify`, workspace/default and dev-identity
Rust, reference Rust/client/probes, and every other existing check. In the old
compound `probe:s16` position, after web dependencies, CI now runs only:

```sh
node --test experiments/api-slice/web/scripts/test/s16-probes.test.mjs
```

This is one mechanics regression for nested private-target inheritance and
exception cleanup, not the real 16-mutant suite. Direct frozen export/snapshot/
generated-TypeScript synchronization and frozen narrowing/client executions no
longer run automatically through S16 entrypoints; reference equivalents remain
automatic.

Historical manual entrypoints are unchanged:

```sh
npm --prefix experiments/api-slice/web run verify:s16
npm --prefix experiments/api-slice/web run probe:s16
```

Install web dependencies before either command. For an isolated standalone
`verify:s16`, run it in an externally prepared disposable source copy with an
explicit private `CARGO_TARGET_DIR`: the verifier itself isolates fixtures but
does **not** isolate Cargo. The compound `probe:s16` first runs the mechanics
regression, then its runner owns a disposable source copy and private target,
including nested verifier builds. Do not overlap fixed-port suites or in-place
mutations; preserve checkout hashes and target sentinels and check cleanup. No
frozen source, artifact, package script or dependency changed, and retaining
these commands does not promise they stay green indefinitely without CI. The
[dated baseline](../../docs/decisions.md#partial-s16-ci-retirement--october-2-2026)
records what was actually executed; candidate reference verification is a
separate gate.

## Limits

- A failed connection open is tested to return 500; its busy-to-503
  classification is source-inspected only.
- The client bundles the export it was built with, so client and server must
  ship from the same commit.
- The development command's tests use fixed ports locally and in CI; they must
  own 4001, 3003 and 5175 exclusively and must not overlap browser workflows.
- Ajv compiles validators with `new Function`; a strict content security policy
  would need precompiled validators. None is set here.
- The browser workflows check selected paths in one browser, not every code.
  Each API restart's failure window was probed with an injected delay in a copy
  of the runner.
- For a withheld response, the runner records the response before the page is
  denied it, so those cases show client uncertainty kept after a commit, not a
  real disconnect, cancellation or server work continuing after the caller is
  lost.
- At send time, an attempt marks only a shown members listing of its project as
  possibly predating it; either list whose pending request preceded an attempt
  is marked when that result arrives. The readback and manual reloads provide
  new state observations and never alter an attempt's outcome; no read
  automatically follows a mutation.
- The reads' 503 captures use the operation's mount with an injected actor and
  no cookie. Cancellation is shown for the read future, not an HTTP disconnect.
- The explicit 32-byte cursor bound cannot change behavior while keys are
  canonical i64 values; it is documented rather than observed.
- Locally, only macOS was used. GitHub Actions (Ubuntu) passed at `a0c25ff`,
  before checkpoint B, and at `06ac967`, checkpoint B's handoff, on a rerun: the
  first attempt failed only in the frozen agent-interface check, assessed as a
  flake. The current-state read commits passed every step on the first attempt
  at `12227aa` (run 36345360713), including all three browser workflows.
