# Iris

A personal experiment in building an opinionated, API-first Rust application
framework from existing crates, with React as its first client.

This repository captures the design before implementation. **There is no working
framework or benchmark yet.** Iris is the home for the research, design record,
and upcoming experiments.

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

## Next milestone

Compare two small implementations of invitation acceptance: SQLite through a
concrete Rust driver, and native local Turso. Exercise identical behavioral
expectations without first introducing a universal database abstraction.

Use the results to sketch the framework-user experience, then build an
end-to-end slice: application action → HTTP endpoint → OpenAPI → generated
TypeScript client → React screen.

## Provenance and status

Recorded September 24, 2026, from a design discussion with Robert Guss:
[original Amp thread](https://ampcode.com/threads/T-01a0d13e-b20f-73ee-bed5-747eb2d3346c).
These documents are self-contained; thread access is not required to understand
the direction. Research included librarian source investigations and an oracle
architecture consultation. Findings are research notes, not executed
verification.

No engine, dependency versions, application API, authentication design, or
performance claims are finalized. No application code has been implemented.
