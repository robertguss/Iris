# Research map

This is the initial source-research snapshot. For subsequent executed evidence,
see [embedded database findings](embedded-db-findings.md); the unverified claims
listed below describe the state at the time of research, not the current test
status.

Research snapshot: September 24, 2026. Links to `main` and unversioned
documentation move over time. Confirm the exact released API before implementing
or pinning a dependency. No benchmark or dependency compatibility matrix has
been executed.

## Framework references

### Ash: application semantics

Borrow domains that group capabilities, named actions with explicit inputs and
outputs, actor/tenant context, and deliberate transport exposure. Actions need
not be HTTP handlers or universal CRUD operations.

Ash distinguishes queries, changesets, and generic action inputs. Its policy
system can constrain reads rather than only approve/reject requests. This is
powerful for lists, pagination, and relationships, but reproducing the policy
expression/query machinery would be a substantial project.

Start with normal Rust functions, types, and explicit authorization. Study Ash's
semantics before translating its DSL or extension machinery into Rust macros.

- [Domains](https://hexdocs.pm/ash/domains.html)
- [Actions](https://hexdocs.pm/ash/actions.html)
- [Policies](https://hexdocs.pm/ash/policies.html)
- [Explicit AshJsonApi route bindings](https://github.com/ash-project/ash_json_api/blob/main/documentation/topics/routing.md)

### Rails and Laravel: application workflow

Study conventions, generators, commands, testing, deployment paths, and escape
hatches. Investigate how queue placement and transaction boundaries affect job
delivery rather than assuming enqueueing is atomic with application writes.

Rails was examined more directly than Laravel during initial research. Laravel
remains a comparison target, not a source of settled implementation conclusions.

- [Rails testing](https://guides.rubyonrails.org/testing.html)
- [Rails Active Job](https://guides.rubyonrails.org/active_job_basics.html)
- [Laravel documentation](https://laravel.com/docs)

### API-focused frameworks: contracts and dependency wiring

FastAPI connects input/output models to validation and OpenAPI; its response
models can also validate/filter returned data. Rust schema generation alone does
not imply equivalent runtime validation.

Study ASP.NET Core typed results, Problem Details, dependency injection, and
endpoint metadata. Explicit Axum state and ordinary parameters may be sufficient
for our initial dependency wiring; a container is not an initial requirement.

- [FastAPI response models](https://fastapi.tiangolo.com/tutorial/response-model/)
- [ASP.NET Core](https://learn.microsoft.com/en-us/aspnet/core/)
- [OpenAPI specification](https://spec.openapis.org/oas/latest.html)
- [RFC 9457 Problem Details](https://www.rfc-editor.org/rfc/rfc9457)

OpenAPI security declarations describe requirements; they do not install runtime
authorization. Generated clients cannot establish business-rule correctness.

### Dioxus: development orchestration

Study `dx` coordination of builds, watching, assets, client/server processes,
and diagnostics. Keep React/Vite responsible for frontend refresh.

Dioxus's compiler-free RSX reload depends on its UI representation and does not
reload arbitrary backend Rust. Subsecond recompiles code and loads patches; it
does not eliminate compilation. Experimental limitations include layout changes,
globals, live futures, and state retained across patches. Development-branch
capabilities must not be mistaken for stable-release behavior.

Do not replay side-effecting requests to apply patches. Keep clean restart and
fresh-process verification authoritative.

- [Dioxus repository](https://github.com/DioxusLabs/dioxus)
- [Dioxus 0.7 release](https://dioxuslabs.com/blog/release-070/)
- [Subsecond documentation](https://docs.rs/subsecond/)

### Existing Rust frameworks: references, not our foundation

Loco already integrates many Rails-like capabilities and a React API workflow.
Its existence validates the integration opportunity but does not replace the
goal of designing our own framework. Cot offers Django-inspired conventions and
admin/persistence ideas. Axum supplies a smaller HTTP foundation. Rocket, Poem,
Salvo, and Pavex are additional references rather than shortlisted commitments.

- [Loco](https://github.com/loco-rs/loco)
- [Loco React workflow](https://github.com/loco-rs/loco/blob/master/website/src/content/docs/docs/how-to/build-a-spa.md)
- [Cot](https://github.com/cot-rs/cot)
- [Axum](https://github.com/tokio-rs/axum)

## Persistence candidates

### SQLite through SQLx

Useful integrated capabilities include async-facing connections/pools,
migrations, isolated database tests, and checked SQL macros. SQLite work uses
worker threads; an async Rust API does not mean the underlying engine uses
native async I/O.

Checked query macros require an appropriate build-time database or prepared
offline metadata. Not all SQL execution APIs have those checks. Bundled SQLite
introduces a C build step; whether that is costly requires measurement.

SQLite ordinarily permits one writer. `BEGIN IMMEDIATE` can establish write
intent before reads, but can itself encounter contention. WAL does not create
multiple simultaneous writers. Verify selected connection settings, including
foreign keys.

- [SQLx](https://docs.rs/sqlx/)
- [SQLx SQLite](https://docs.rs/sqlx/latest/sqlx/sqlite/)
- [SQLx query macros](https://docs.rs/sqlx/latest/sqlx/macro.query.html)
- [SQLx test support](https://docs.rs/sqlx/latest/sqlx/attr.test.html)
- [SQLite transactions](https://sqlite.org/lang_transaction.html)
- [SQLite foreign keys](https://sqlite.org/foreignkeys.html)
- [SQLite in-memory databases](https://sqlite.org/inmemorydb.html)

### Native local Turso

Distinguish the newer Rust Turso engine and `turso` crate from the
SQLite-derived `libsql` lineage and the hosted Turso Cloud product. Local
experimentation should not require a cloud account or sync.

Research found native async APIs and less integrated migration/pool/test tooling
than SQLx. Validate exact released behavior rather than relying on the moving
compatibility table. Production adoption does not imply complete SQLite parity.

Do not assume SQLx's SQLite driver accesses the native Turso engine, or open one
database concurrently through different engines. File compatibility is not a
driver or concurrency compatibility guarantee. Experimental MVCC capabilities
are not prerequisites for our slice.

- [Turso Rust quickstart](https://docs.turso.tech/sdk/rust/quickstart)
- [Turso Rust reference](https://docs.turso.tech/sdk/rust/reference)
- [Turso repository](https://github.com/tursodatabase/turso)
- [Moving SQLite compatibility table](https://github.com/tursodatabase/turso/blob/main/COMPAT.md)
- [Native crate documentation](https://docs.rs/turso/)

### Other persistence options

SeaORM may offer useful CRUD/relationship productivity; Diesel offers a strongly
typed query DSL; rusqlite is a synchronous SQLite option. None is rejected, but
the first comparison does not need an ORM decision or a universal backend API.

## Contract and verification candidates

- [utoipa](https://docs.rs/utoipa/) and
  [utoipa-axum](https://docs.rs/utoipa-axum/): explicit OpenAPI metadata with
  route registration integration.
- [aide](https://docs.rs/aide/): an alternative Axum/schema integration to
  evaluate against the same endpoint and generated client, not just source
  aesthetics.
- [proptest](https://proptest-rs.github.io/proptest/intro.html): generated cases
  and operation sequences for meaningful invariants.
- [cargo-mutants](https://mutants.rs/): test whether injected faults are
  detected.

The earlier `ts-rs` discussion remains useful: shared DTO types alone do not
describe endpoint methods, paths, statuses, security, or runtime response
validity.

## Claims deliberately not made

- No crate versions or feature combinations have been validated in a lockfile.
- No build speed, query latency, concurrency fairness, or binary-size results
  exist.
- No universal SQLite/Turso/PostgreSQL portability is promised.
- No compiler, OpenAPI document, generated test suite, or hotpatch session
  proves the application implements its intended behavior.
