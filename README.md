# Iris

A personal experiment in building an opinionated, API-first Rust application
framework from existing crates, with React as its first client.

Iris contains the design record, a runnable SQLite/Turso comparison, and an
end-to-end Axum → OpenAPI → TypeScript → React experiment. **There is no
application framework yet.**

## Why build this?

The goal is to learn by designing a framework: choose conventions, integrate the
Rust ecosystem, and explore a productive application-development experience.
Adopting an existing batteries-included framework would miss part of that goal.

Rails is an inspiration for developer experience, not a programming model to
translate into Rust. Ash, Laravel, API-focused frameworks, and Dioxus contribute
other ideas. We want to synthesize useful existing standards and tools, then
experiment with new integrations where they improve the experience.

AI-assisted development is a primary use case. Rust's compiler can provide a
strong feedback loop, but compilation and generated tests do not establish that
an application implements the intended business behavior. Verification is part
of the framework design, not a later addition.

## Working direction

- Assemble existing crates; own the conventions and integration.
- Build API-first, with React as the initial consumer and room for other
  clients.
- Use standards such as OpenAPI rather than inventing every contract/tool.
- Explore application actions independent of HTTP, inspired by Ash.
- Offer an embedded-first development experience; compare SQLite and local
  Turso.
- Verify database behavior against the engine an application actually deploys.
- Measure build and test feedback loops before optimizing them.
- Begin with readable Rust and explicit wiring, not a large DSL or plugin
  system.

## Design record

1. [Decisions and open questions](docs/decisions.md) — agreed direction versus
   provisional proposals, including choices that changed during discussion.
2. [Research map](docs/research.md) — references, ideas to borrow, tradeoffs,
   and claims requiring validation.
3. [Verification and first experiment](docs/first-experiment.md) — intended
   behavior, database comparison, build measurements, and subsequent API slice.
4. [Embedded database findings](docs/embedded-db-findings.md) — executed checks,
   measured results, limitations, and a provisional SQLite recommendation.
5. [API slice](experiments/api-slice/README.md) — runnable React demo,
   utoipa/aide comparison, contract verification, and application-authoring
   conventions.

## Run the database experiment

With the pinned Rust toolchain and a C compiler installed:

```sh
cargo test --workspace --locked
```

See the [experiment guide](experiments/embedded-db/README.md) for setup, test
semantics, and reproducible measurements. In an orb, `.agents/setup` installs
the toolchain and fetches locked dependencies.

## Run the API experiment

Follow the [API slice guide](experiments/api-slice/README.md) for setup and
checks. In an orb, `amp orb services ensure` starts the API and React and prints
a portal. The demo uses disposable data and explicitly gated synthetic
identities, not production authentication. Generated contracts are checked
against real HTTP responses and the actual React consumer.

## Next milestone

The API experiment now supports owner-authorized issuance and recipient
acceptance. Both actions follow the same explicit boundaries without a generic
Action trait. Design real authentication and token delivery/recovery before
making this a deployable application. SQLite and utoipa remain provisional; the
concrete Turso comparison is still available.

## Provenance and status

Recorded September 24, 2026, from a design discussion with Robert Guss:
[original Amp thread](https://ampcode.com/threads/T-01a0d13e-b20f-73ee-bed5-747eb2d3346c).
These documents are self-contained; thread access is not required to understand
the direction. Research included librarian source investigations and an oracle
architecture consultation. The initial research notes are distinguished from the
subsequently executed database experiment and its findings.

Experiment dependencies are pinned; the framework API, default engine, and
authentication design remain provisional or open. Measurements describe this
specific orb and workload, not general performance guarantees.
