# Iris independent design review brief

Use this brief to obtain multiple independent reviews and alternative ideas,
then select changes by evidence and fit—not by model consensus. This is a design
review, not a productivity study or an implementation assignment.

## Instructions for the person coordinating reviews

1. Give each reviewer this brief and the same revision of the
   [living design spec](design-spec.md). Include repository access or the
   relevant source files if available. A document-only review is valid; label
   that limit.
2. Record the full Git revision or document snapshot identifier. Do not compare
   different snapshots as though reviewers assessed the same proposal. Reviewers
   should report their actual inputs, not assume they have the latest revision.
3. Assign a primary track: **A — application authoring**, **B — correctness and
   security**, or **C — AI usability**. If enough reviewers are available,
   assign more than one independently to the same track. With one reviewer,
   cover all three. Track assignment is a focus, not a prohibition on important
   findings.
4. Start independent sessions. Withhold other reviewers' findings until their
   first reports are complete to reduce anchoring. Models may still share
   biases; multiple reports are not independent experimental proof.
5. Preserve each original report before discussion. Use the synthesis procedure
   below to reconcile findings and record decisions. Do not ask reviewers to
   edit a shared spec concurrently.

No reviews are launched by adding this document. No particular model, provider,
agent mode or number of reviewers is required. Do not disclose private
repository content to an external service without the owner's authorization for
that audience.

## Copyable assignment

```text
Review the attached Iris design spec using docs/design-review-brief.md.

Primary track: [A / B / C / all three]
Spec/repository revision: [full commit or snapshot identifier]
Available material: [spec only / repository / attached files]
Review identifier: [unique short label]

Work independently. Do not read other reviews before producing yours. Review
the design and propose alternatives; do not implement changes, edit the spec,
push code, deploy, access production, or launch productivity experiments.
Do not create additional agents or threads unless explicitly requested.

Follow the shared questions, scenarios, and report format in the brief. Cite
spec sections and supporting evidence. Distinguish implemented behavior from
proposals, and distinguish demonstrated contradictions from hypotheses.
Recommend simplifications as well as additions. There is no required finding
count and no requirement to endorse the proposed architecture.

Return a self-contained Markdown report in your response. State what you could
not inspect or verify. The coordinator will preserve and reconcile reports.
```

## Project context and scope

Iris is a personal learning project: design an API-first Rust framework by
assembling existing crates and owning the conventions. React is the first
client, not the only possible client. Rails inspires developer experience, not
Ruby's programming model. Ash, Loco, FastAPI and Dioxus are references, not
mandates to reproduce their architectures.

AI agents are expected application authors. The goal is a short path to an
independently verified change: predictable structure, useful diagnostics,
discoverable contracts and evidence that supports safe decisions. Human intent
and authorization remain necessary. Generated code and tests can share errors;
neither compilation nor a green report establishes intended business behavior.

The current design favors ordinary transaction-owning Rust actions, separate
transport adapters, shared static contract definitions, typed business
rejections versus infrastructure failures, and explicit uncertainty about
effects. Execution context and runtime evidence are proposals, not implemented
guarantees.

Keep these owner goals fixed for this review: Rust backend, API-first direction,
React as initial consumer, ecosystem reuse, and designing Iris rather than
simply adopting another framework. You may challenge proposed mechanisms,
including explicit context, descriptor mappings, derives and registration
boundaries. If an alternative requires changing a goal, label that dependency
instead of silently changing the assignment.

Do not enlarge the project into a general workflow engine, observability
backend, universal ORM, or policy language without showing why a smaller
solution fails. Equally, do not reject a useful abstraction solely because it is
an abstraction. Compare concrete authoring and failure behavior.

## Reading order and evidence

Read the whole spec once; then focus on the assigned track:

| Material                         | Purpose                                                                    |
| -------------------------------- | -------------------------------------------------------------------------- |
| Spec S01–S03                     | Goals, constraints and authoring alternatives                              |
| Spec S04–S05                     | Domain/adapter boundaries, identity, transactions and side effects         |
| Spec S06–S08                     | Result taxonomy, uncertainty and optional invocation receipts              |
| Spec S09–S10                     | Verification scope and implemented versus proposed status                  |
| Spec S11–S12                     | Open decisions and static contract authoring                               |
| Spec S13                         | Runtime context, evidence, two worked flows and detailed failure questions |
| [Decision history](decisions.md) | Rationale and superseded directions; not a competing current spec          |
| [Research map](research.md)      | Sources to investigate, not proof that a dependency solves a requirement   |

With repository access, inspect only sources needed to support your claims:

- [API adapter and error mappings](../experiments/api-slice/server/src/lib.rs)
  and [invitation adapter](../experiments/api-slice/server/src/invitations.rs).
- [Invitation action](../experiments/embedded-db/sqlite/src/issue.rs),
  [outbox operations](../experiments/embedded-db/sqlite/src/outbox.rs), and
  [delivery worker](../experiments/api-slice/server/src/delivery.rs).
- [Agent-interface guide](../experiments/agent-interface/README.md) and its
  catalog/runner as needed to distinguish test evidence from runtime telemetry.

At the time this brief was created, membership implementation was reported in a
separate, unpushed agent checkout. Do not assume its source is in your snapshot.
Use the spec's description as an assumption if the code is unavailable. A
missing proposal implementation is not itself a defect: assess whether its
design is coherent and feasible. Check the snapshot before making current-code
claims.

Prefer authoritative crate/framework documentation for external claims; include
URLs and versions or revisions when material. Never fabricate inspections,
benchmarks, tool results, source locations or test executions. Do not install or
run the application merely to turn a design review into an implementation task.
If a claim needs an experiment, describe the smallest experiment and leave the
claim conditional unless explicitly authorized to execute it.

## Track A — Application authoring and simplicity

Primary question: **Does Iris make ordinary application features
straightforward, or move boilerplate into additional layers that agents must
keep synchronized?**

Examine:

- Can an author locate inputs, outcomes, authorization, SQL, wire conversion,
  response mappings and registration without searching unrelated modules?
- Are domain versus wire types meaningful boundaries or unnecessary duplication?
  What happens when a command gains a field or a new rejection?
- Is one transport contract actually shared by rendering and export, or merely
  claimed to be? Where could runtime, OpenAPI, generated clients and MCP
  disagree?
- What does a handwritten enumeration fail to cover? Does a narrow derive solve
  enough to justify compile cost, diagnostics and maintenance?
- Are transaction composition and context propagation explicit without imposing
  ceremony on every helper? What breaks in a larger atomic workflow?
- Which existing crate APIs already supply the proposed capabilities? What new
  Iris code would remain, and who would maintain it?

Provide a short annotated `change_role` authoring sketch for your preferred
alternative. Label invented APIs. Compare the files/concepts an author must
understand with the current proposal; do not use line count alone as the metric.
Name one simplification worth considering, or explain why further simplification
would erase a useful guarantee. Preserve ordinary business logic in the
comparison.

## Track B — Correctness, authority and security

Primary question: **Where does the design promise more than its transaction,
runtime, evidence collector, or trust boundary can establish?**

Examine:

- Mutable authority and last-owner enforcement under concurrent requests.
- New deferred commands versus delivery of committed effects, issuer revocation,
  self-demotion, expiry, and work already in flight.
- Transaction ownership, cancellation, dropped futures, cleanup, commit errors,
  response loss, and whether any retry can duplicate a mutation or external
  effect.
- Claim counters, stale-worker fencing, SMTP ambiguity and the meaning of a
  successful or unsuccessful completion update.
- Untrusted trace context, baggage, correlation collisions, cross-environment
  IDs, inspector authorization, existence disclosure and token-bearing
  instrumentation.
- Missing/sampled/evicted records, incompatible producer versions and false
  completeness or causality claims.

For material findings, give an ordered failure sequence and explain the violated
invariant. Distinguish database-engine-specific guarantees from general claims.
Do not call a product-policy disagreement a security defect without identifying
the promised boundary it violates. A fence on database writes is not a fence on
SMTP, and an event called `committed` is not proof by its name alone.

## Track C — AI usability and feedback quality

Primary question: **Can an agent discover the applicable contract, distinguish
rejection from failure, obtain permitted evidence, and choose a safe next step
without inventing missing facts?**

Examine:

- Does discovery identify which contracts and evidence apply to this code/build?
  Can stale, partial or unavailable evidence be distinguished from a current
  pass?
- Do errors provide stable identities and useful source-local correction
  signals? Which omissions fail compilation, contract checks, or only behavioral
  tests?
- Can an agent distinguish a correctly enforced business rule from a bug, a
  recoverable infrastructure condition, and an unknown effect?
- Are recovery constraints prerequisites rather than instructions or permission?
  What should happen when a client encounters an unknown code or schema version?
- Can runtime strings or source-derived descriptions smuggle instructions into
  diagnostic responses? Is data clearly separated from trusted conventions?
- Are commands, contracts, catalogs and tool schemas duplicated? Would an agent
  need to update several lists correctly just to add one action?
- Is the evidence volume bounded and useful for diagnosis, or merely structured
  log noise? What important questions would the proposed tools still not answer?

Show one short hypothetical agent decision sequence: information available,
interpretation justified, next safe action, and information still missing. Label
it as analysis, not an executed agent experiment. Do not claim improved repair
speed or reliability without measurements; that study remains deferred.

## Shared scenario set

Every reviewer should consider these, emphasizing their track. Do not
manufacture findings merely to fill the table. State which cases you examined in
depth.

| Scenario                                  | Question to answer                                                                  |
| ----------------------------------------- | ----------------------------------------------------------------------------------- |
| Add a rejection variant                   | What fails if metadata, enumeration, HTTP mapping or schema export is omitted?      |
| Two owners concurrently depart            | Which serialized operation prevents the final owner disappearing?                   |
| Authority changes after enqueue           | Is this a new command or delivery of committed intent, and whose authority applies? |
| Initial lock acquisition times out        | What supports `not_started`, and when does that inference stop being valid?         |
| Commit succeeds; response disappears      | What can the caller know without a durable receipt?                                 |
| Caller cancels near commit                | Who owns execution/cleanup, and what remains uncertain?                             |
| SMTP may accept; lease is replaced        | Can the old worker still send? What does a rejected acknowledgment prove?           |
| Telemetry disabled, dropped or evicted    | Which identifiers survive, and what can inspection legitimately conclude?           |
| Forged propagation or unrelated inspector | Can it change authority, force collection, or expose forbidden existence?           |
| Old client or stale diagnostic producer   | How are unknown codes and version mismatches handled without unsafe inference?      |

## Required report format

Return Markdown. Be concise but retain the evidence needed to assess each claim.
There is no quota of defects, no numeric score requirement, and no need to
repeat every question if it reveals no issue.

### 1. Review identity and scope

- Review identifier and primary track.
- Model/provider/version if known; say unknown rather than inventing it.
- Exact spec/repository revision, inputs inspected, tool/research access and
  material limitations. Separate source inspection from executed verification.
- A short statement of the design as you understand it, to expose disagreements
  about assumptions before debating recommendations.

### 2. Assessment

Give the most important conclusion first. Name what should be preserved as well
as what should change. Separate blockers before implementation from refinements
that can wait. Do not substitute praise or general best practices for analysis.

### 3. Findings

Use stable reviewer-local IDs, e.g. `review-A-01`. For each finding include:

```text
ID and title:
Kind: contradiction / risk / open question / alternative / simplification
Impact: critical / significant / limited (explain; not confidence)
Confidence: supported / conditional / speculative (state why)
Spec section and short excerpt:
Assumptions and source evidence:
Failure sequence or concrete authoring/agent example:
Why it matters for Iris's goals:
Smallest recommended correction:
Alternative, including retaining the current design:
Tradeoffs, new obligations and dependencies:
Validation that would distinguish the alternatives:
```

Critical means a plausible violation of a core safety/correctness boundary if
implemented as specified. Significant means recurring ambiguity, drift or cost
that materially affects the design. Limited means a localized improvement. For a
question or opportunity, use the applicable impact or say not applicable.
Severity labels do not turn unverified premises into facts.

### 4. Preferred design and unresolved choices

Give a short coherent recommendation, not just a list of unrelated improvements.
Include the track-specific sketch or example. State which owner decisions
remain, what you would explicitly defer, and what evidence would reverse your
recommendation.

## Synthesis instructions after independent reports are collected

The coordinator should preserve the original reports and create a synthesis only
after collection. Do not create empty result files or imply reviews already
exist.

1. Normalize findings by claim and spec section while retaining reviewer IDs.
   Several phrasings of the same claim are one issue, not multiple votes.
2. Separate observed contradictions, conditional risks, policy questions and
   optional ideas. Check cited source and failure sequences against the same
   snapshot; use direct inspection rather than trusting a model's citation.
3. Compare alternatives by fit with owner goals, strength of guarantees,
   authoring burden, feedback quality, dependencies, compile/runtime cost and
   reversibility. Say unmeasured when costs are estimates. Prefer the smallest
   change satisfying the requirement, not automatically the most elaborate or
   popular proposal.
4. For disagreements, identify the conflicting assumption or product choice.
   Keep uncertainty if evidence does not decide it; specify a focused validation
   or ask the owner. Do not run a broad productivity study to resolve a local
   design question, and do not launch any experiment without the relevant scope.
5. Check interactions before combining recommendations. For example, independent
   choices about lossy telemetry, durable receipts, and completion after
   disconnect can become contradictory when assembled. Selecting the best ideas
   individually is not sufficient; the resulting design must be coherent.
6. Propose dispositions for owner approval. Agreement among reviewers is neither
   proof nor authorization to change accepted direction. A single concrete
   counterexample can outweigh several endorsements.

Suggested synthesis table (template only):

| Claim / source finding IDs | Evidence and assumptions | Options and tradeoffs | Proposed disposition | Owner decision / validation needed |
| -------------------------- | ------------------------ | --------------------- | -------------------- | ---------------------------------- |

Use **accept**, **reject with rationale**, or **unresolved**. An accepted idea
can still have deferred implementation; record that separately. After approval,
update the living spec and its change record, retain rejected alternatives with
their rationale, and link the evidence. Do not silently rewrite history or claim
that a documented recommendation has been implemented.

## Definition of completion

A review is complete when it supplies the report above, identifies its limits,
and gives actionable findings or explains why no change is justified. It need
not produce code, execute tests, or agree with the proposal.

The review round is complete when findings have traceable dispositions,
important disagreements are resolved or explicitly open, and approved design
changes are recorded with rationale. Implementation and verification are
separate future work.
