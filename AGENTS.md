# Iris development guidance

## Amp reviewed development

```text
Linear: team ROB, project Iris (P-ROB-26)
Delivery: push-branch
Builder: grok47
Tester: inherit
Done: merged
```

[Iris in Linear](https://linear.app/robert-guss/project/iris-8b30c90a23d4) owns
the work queue, status, dependencies, scope approvals, and next-task selection.
Use the `amp-workflow` skill for selected development issues. Read the issue and
comments before planning; importing a backlog item does not authorize
implementation. Do not maintain a second backlog in this repository.

Use the existing Ready, Planning, Building, In Review, Needs Input, and Done
statuses. Done means confirmed merged, not merely verified or pushed. Obtain
authorization for the dedicated issue branch before pushing; the historical
main-only push instruction is not authorization for issue-branch pushes.

Keep technical contracts in `docs/design-spec.md`, rationale and dated evidence
in `docs/decisions.md`, and runnable instructions beside the application. Update
those when behavior changes; Linear replaces work tracking, not technical docs.
`HANDOFF.md` is a historical snapshot, not the current queue or environment.

## Scope and safety

- Oracle reviews plans and commit diffs. Preserve explicit owner approval gates
  for CI changes/reruns, frozen-experiment edits/retirement, PRs, merges,
  deployment, and releases. Migration into Linear grants none of these.
- Use disposable databases and isolated mutation source/build directories with
  passing controls. Never use the checkout's shared target for mutation runs.
- Do not overlap fixed-port suites or in-place browser mutations with other runs
  or edits. Never kill unrelated listeners without permission.
- Follow the storage-exclusive guard for child spawns in library tests.
- Contract edits regenerate OpenAPI and then the TypeScript client; generated
  TypeScript is not hand-edited or formatted.
- Add to `crates/iris` only what two concrete operations demonstrably share.
- Preserve historical evidence and provenance. Old scratch paths, processes,
  tools, and CI placeholders are not evidence of the current environment.
