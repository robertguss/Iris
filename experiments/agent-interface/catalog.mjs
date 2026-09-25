export const conventions = {
  actions: {
    rule: 'Actions own authorization and transactions; handlers validate DTOs and map outcomes. No generic Action trait is selected.',
    sources: ['experiments/embedded-db/sqlite/src/issue.rs', 'experiments/api-slice/server/src/invitations.rs'],
  },
  authentication: {
    rule: 'Actor comes from session middleware, never caller identity fields. Logout and callback promotion compete for a live session. Test issuer does not establish human identity.',
    sources: ['experiments/api-slice/server/src/auth/mod.rs', 'experiments/api-slice/authentication.md'],
  },
  delivery: {
    rule: 'Commit invitation and outbox together. SMTP acceptance is not exactly-once delivery. Attempt-fenced acknowledgements reject stale workers. Demo databases are disposable.',
    sources: ['experiments/embedded-db/sqlite/src/outbox.rs', 'experiments/api-slice/delivery.md'],
  },
  verification: {
    rule: 'A passing subset is not full application verification. Expectations are reviewed intent, not inferred from implementation. Repository authors can change checks; this is not a protected evaluator.',
    sources: ['.github/workflows/verify.yml', 'experiments/agent-interface/README.md'],
  },
  membership: {
    rule: 'Owners may change existing member roles or remove memberships. Authorization, project-scoped owner count and mutation share BEGIN IMMEDIATE. The last owner cannot leave or be demoted. Removal does not revoke invitations.',
    sources: ['experiments/embedded-db/sqlite/src/members.rs', 'experiments/api-slice/server/src/members.rs'],
  },
};

export const checks = {
  compile: { command: ['cargo', 'check', '--locked', '-p', 'iris-api-spike', '--all-targets', '--message-format=json'], sources: ['experiments/api-slice/server/Cargo.toml'], events: [] },
  auth: {
    command: ['cargo', 'test', '--locked', '-p', 'iris-api-spike', '--test', 'auth', 'logout_wins_against_callback_waiting_on_provider', '--', '--exact'],
    sources: ['experiments/api-slice/server/tests/auth.rs', 'experiments/api-slice/server/src/auth/mod.rs'],
    events: [['provider_waiting', 1], ['logout_status', 204], ['callback_status', 401], ['remaining_sessions', 0]],
  },
  outbox: {
    command: ['cargo', 'test', '--locked', '-p', 'iris-sqlite-spike', '--test', 'delivery', 'snapshot_backoff_crash_recovery_and_stale_ack', '--', '--exact'],
    sources: ['experiments/embedded-db/sqlite/tests/delivery.rs', 'experiments/embedded-db/sqlite/src/outbox.rs'],
    events: [['first_attempt', 1], ['recovered_attempt', 2], ['stale_ack_accepted', 0], ['retry_attempt', 3], ['payload_cleared', 1]],
  },
  members: {
    command: ['cargo', 'test', '--locked', '-p', 'iris-sqlite-spike', '--test', 'members', 'concurrent_last_owner_and_authority', '--', '--exact'],
    sources: ['experiments/embedded-db/sqlite/tests/members.rs', 'experiments/embedded-db/sqlite/src/members.rs'],
    events: [['successful_departures', 3], ['last_owner_conflicts', 3], ['retained_owners', 3], ['revoked_actor_rejected', 1]],
  },
  contracts: { command: ['npm', '--prefix', 'experiments/api-slice/web', 'run', 'verify'], sources: ['experiments/api-slice/web/package.json'], events: [] },
};
export const profiles = { focused: ['auth', 'outbox', 'members'], compile: ['compile'], contracts: ['contracts'] };
