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

**Start with the [living design specification](docs/design-spec.md).** It
captures the current direction, rationale, authoring conventions, AI-oriented
error and evidence contracts, alternatives, and open questions. Status labels
distinguish agreed principles from proposed APIs and implemented experiments.
Update it as the design evolves; the records below retain history and supporting
evidence.

For independent feedback, give reviewers the spec and the
[design review brief](docs/design-review-brief.md). It contains copyable
instructions, three review tracks, a shared report format, and a process for
comparing recommendations without treating model agreement as proof.

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
6. [Agent interface](experiments/agent-interface/README.md) — shared
   verification runner, JSON CLI, MCP conventions and scenario evidence, trust
   boundaries, and the proposed agent-repair evaluation.

## Run the database experiment

With the pinned Rust toolchain, a C compiler, and Node.js installed (the API
authentication tests start a local OIDC fixture):

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

The API experiment now supports OIDC-backed browser sessions, owner-authorized
issuance, and recipient acceptance. The credential-free
[authentication experiment](experiments/api-slice/authentication.md) validates
the protocol against a local test issuer; it is not production authentication.
The [local delivery experiment](experiments/api-slice/delivery.md) adds atomic
email enqueueing, bounded retries, and Mailpit capture without sending real
email. Real-provider authentication remains deferred until a stable callback
environment is available. Production delivery, signup, and deployment are not
implemented. SQLite and utoipa remain provisional.

The [agent interface pilot](experiments/agent-interface/README.md) now exposes
conventions, bounded checks and test-observed evidence through a CLI and MCP.
Further productivity experiments are deferred while we design the framework's
features and conventions in the [living spec](docs/design-spec.md). The pilot
does not yet inspect production requests or profile performance; successful tool
calls alone do not demonstrate better repairs.

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
