# First review synthesis: Opus 5.5 and oracle assessment

Date: September 25, 2026. Status: design synthesis and recommendations, not an
implementation plan approved in full. The owner requested this documentation
after the oracle assessment. Further independent reviews remain open; this is
not a multi-review consensus or a closed review round.

## Scope and provenance

- Preserve [opus55-all-01](opus55-all-01.md) unchanged as the original report.
  It reviewed
  [the review-brief snapshot](https://github.com/robertguss/Iris/commit/f674991a93071c87b6c647ef7c3d005c51d296e7)
  and did not run tests.
- The oracle assessed that report against the spec and source after the review
  file was pulled, before the independently developed membership workflow was
  integrated. It also consulted SQLx 0.9.0, strum 0.28.0 and openapi-fetch
  0.17.0 sources. Its assessment was read-only, not executable validation or a
  review of the later membership delivery.
- The coordinator subsequently pulled the
  [membership implementation](https://github.com/robertguss/Iris/commit/3121dd6264fcc6861d5c355c72472288abdee7a1).
  S10 records its separately reported verification. Do not retroactively apply
  the oracle's snapshot conclusions to this integration.
- [S14 of the living spec](../design-spec.md#s14--caller-loss-execution-ownership-and-recovery)
  contains the resulting caller-loss recommendation, failure table and future
  validation criteria. The spec owns current design; this file retains review
  reasoning. Oracle consultation is in the
  [source discussion](https://ampcode.com/threads/T-01a0d13e-b20f-73ee-bed5-747eb2d3346c).

## Preserve the direction; split claims from proposed mechanisms

The assessment did not establish a new critical defect. Retain ordinary actions,
explicit authority, separate transport contracts, and the distinction between
results, evidence and receipts. Adopt useful constraints without importing the
review's transaction-helper, detached-task and terminal-record design as a unit.
Costs below are qualitative, not measured compile-time or productivity results.

Disposition applies to the stated claim, not every sentence in a finding.
**Accept** means incorporate the constraint or design preference; implementation
remains deferred. **Reject** records a counterexample or mismatch with scope.
**Unresolved** requires a product choice or focused validation, not model votes.

| Finding                                   | Disposition and rationale                                                                                                                                                                                                                                                                                                           | Remaining decision or validation                                                                                                                                                                                                                                            |
| ----------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| O55-B-01: transaction effect facts        | **Accept with refinement:** classify stages at the transaction boundary and preserve cleanup errors separately. **Reject** the claim that this proves arbitrary action effects or a spec contradiction. The swallowed `Ok(Outcome)` composition concern targets the earlier model; S06 already proposes rejection through `Result`. | A helper is optional. Test driver cleanup/connection reuse and rejection after writes. A requested rollback is not an acknowledged rollback.                                                                                                                                |
| O55-B-02: server-owned execution          | **Unresolved mechanism; reject necessity and certainty claims.** Keeping a task alive after loss of its waiter is useful but does not guarantee terminal observation after panic, abort, shutdown or process death.                                                                                                                 | S14 recommends honest uncertainty by default. Select bounded supervision only for an operation that needs continuation; test actual request-future ownership.                                                                                                               |
| O55-C-01: recovery after unknown          | **Accept** client-held-before-loss recovery material and separation of first/later attempts. **Reject** universal safe resend, read-before-retry as a fence, and `changed` as proof of original causality. Client-held does not require client-generated.                                                                           | Receipt authorization, key binding, pending/absent/expired records and retention remain open. Test role restoration and invitation expiry between attempts.                                                                                                                 |
| O55-C-04: terminal records and dev sink   | **Accept** bounded vocabulary and explicit collection limits. **Unresolved** terminal aggregation/JSONL as a dev convenience. **Reject** guaranteed terminal retention in a finite fail-open collector or missing-after-restart as proof of crash timing.                                                                           | Distinguish retained observations from receipts; closed fields/IDs still need disclosure review. No collector selected.                                                                                                                                                     |
| O55-A-02: extractor response inference    | **Unresolved:** worthwhile focused comparison, not evidence that aide wins. Inference being disabled in the experiment does not decide quality.                                                                                                                                                                                     | Compare actual middleware/extractor coverage and same-status response merging before replacing exporter wiring.                                                                                                                                                             |
| O55-A-01: enum enumeration                | **Accept preference:** ecosystem enumeration for unit variants plus exhaustive matches before a custom Iris macro. `strum::VariantArray` is a candidate, not an installed dependency.                                                                                                                                               | Check dependency features, payload policy and schema export. Generated enumeration does not prove publication or business correctness. Custom trait diagnostics need an actual trait boundary; do not add one for its error message.                                        |
| O55-C-02: runtime client decoder          | **Accept:** generated TypeScript is not runtime validation. Cover unknown JSON, empty bodies, HTML errors, malformed successes and thrown request/parse failures.                                                                                                                                                                   | Choose schema/decoder integration and verify safe diagnostic projections. A decoder around only the resolved request value misses exceptions.                                                                                                                               |
| O55-C-03: identifiers and recovery        | **Accept** deriving Problem `type` from stable code if Problem Details is adopted. **Reject** collapsing rule/prerequisite/error identities where facts differ, or assigning `RequiresDifferentActor` to every forbidden result.                                                                                                    | Wire format stays open. Recovery metadata describes constraints, not a permission grant or established fix.                                                                                                                                                                 |
| O55-B-03: outbox completion fence         | **Accept** observing completion outcomes, acknowledgment failures and terminal reasons. **Unresolved** attempt-only versus live-lease fencing: existing live-lease behavior is a policy, not a demonstrated bug.                                                                                                                    | Attempt-only updates also need an open-attempt condition: after `Retry` clears the lease while retaining pending state/counter, another completion for that attempt must not apply. False is not always supersession. Test repeated/late acknowledgments before any change. |
| O55-B-04: last-owner invariant            | **Accept** engine-specific serialization and coverage across all mutation paths, already required by the design. **Reject** a naive counting trigger as an automatic concurrency solution.                                                                                                                                          | Validate on each supported engine. A correct-looking rejection plus rollback does not prove the business predicate was evaluated correctly.                                                                                                                                 |
| O55-A-04: DB-bearing invocation           | **Unresolved, deferred:** it could aid composition but couples execution context to persistence. Retain S13's narrow context as the reference.                                                                                                                                                                                      | Demonstrate a composition need first; worker SMTP executes after releasing its connection.                                                                                                                                                                                  |
| O55-B-05: delivery eligibility            | **Accept** explicit policy distinguishing committed notification from a newly authorized command. **Reject** a hard maximum in-flight lag derived from lease plus timeout.                                                                                                                                                          | Process suspension defeats the proposed bound. Pre-send checks cannot fence an already-started SMTP effect; Message-ID does not guarantee deduplication.                                                                                                                    |
| O55-C-05: discovery catalogs              | **Accept** deriving explicit CLI/MCP exposure subsets from shared definitions. **Reject** treating every difference as contradictory: CLI compile reproduction and MCP scenario-only exposure may intentionally differ.                                                                                                             | OpenAPI cannot be the sole catalog for non-HTTP actions or checks. The later membership agent separately reports fixing enum omissions; that is not proof the drift mechanism has been removed.                                                                             |
| O55-A-03: typed IDs and shared rejections | **Accept as design options:** typed IDs can prevent swaps; share rejection types only where actual permitted sets coincide.                                                                                                                                                                                                         | Avoid unrelated generic identity machinery. A bare success acknowledgment remains valid; returning stored state or `changed` is an API choice.                                                                                                                              |

## Decision to make before helpers or executors

Recommend **truthful uncertainty as the ordinary action baseline**. Treat
continuation after waiter loss and durable reconciliation as independent opt-in
capabilities. The former requires bounded ownership; the latter requires an
authorized, retained record correctly coupled to business effects. Neither
automatically supplies the other.

This choice favors a smaller framework and honest agent feedback, at the cost of
leaving some lost-response outcomes unresolved. Applications requiring a
recoverable answer need an explicit stronger operation contract. S14 documents
the concrete lifecycle cases rather than settling a generic executor API.

No runtime, dependency, telemetry, retry, database or productivity changes were
made for this synthesis. Future validation criteria are not passing tests. Other
reviewers should challenge the baseline and the counterexamples, identify which
operation needs stronger guarantees, and propose the smallest mechanism that
meets that requirement without changing unrelated action contracts.
