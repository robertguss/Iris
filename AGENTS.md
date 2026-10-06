# Iris development guidance

## Delivery and tracking

Work happens in the current coding session. Crew roles, model assignments, Jev
routing and Herdr panes are no longer required (Robert, 2026-10-06).

Delivery is one dedicated branch per issue (`rob-<number>-<slug>`), a PR, and a
merge after independent review sign-off and the driver's own rerun, on the final tree,
of the brief's verify commands. CI (`Verify`) is disabled by the owner as of
2026-10-03, and re-enabling it is a CI change that needs the owner's approval.
Done means merged. The approval gates under "Scope and safety" still apply.

[Iris in Linear](https://linear.app/robert-guss/project/iris-8b30c90a23d4) owns
the work queue, status, dependencies, scope approvals, and next-task selection.
Read the issue and comments before planning; importing a backlog item does not
authorize implementation. Do not maintain a second backlog in this repository.

Use the existing Ready, Planning, Building, In Review, Needs Input, and Done
statuses. Obtain authorization for the dedicated issue branch before pushing;
the historical main-only push instruction is not authorization for issue-branch
pushes.

Keep technical contracts in `docs/design-spec.md`, rationale and dated evidence
in `docs/decisions.md`, and runnable instructions beside the application. Update
those when behavior changes; Linear replaces work tracking, not technical docs.
The driver replaces `HANDOFF.md` at chunk boundaries. Until the first
replacement, its September 30 historical-snapshot banner remains authoritative.
Linear holds the queue.

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
