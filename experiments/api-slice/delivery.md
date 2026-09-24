# Local invitation delivery and recovery

This experiment adds email capture, not a production mail service. Existing
users only: Alice (`11`, `alice@example.test`) and Bob (`29`,
`bob@example.test`). Addresses are delivery metadata, never authentication or
account-linking keys.

## Integration choices

Lettre **0.11.23** composes messages and sends SMTP with only its builder,
smtp-transport, and tokio1 features. Mailpit **1.31.2** captures them at
loopback port 1025 and offers an inbox/API at 8025. The installer pins and
checks the Linux amd64 archive SHA-256. No relay or real recipient configuration
is added. The sender connects only to loopback and rejects non-`.test`
addresses.

`issue_with_delivery` owns the same `BEGIN IMMEDIATE` transaction as the owner,
recipient, membership and pending checks. Invitation insertion, contact lookup,
and outbox insertion all commit or roll back together. Missing contact is a
configuration error, not permission to silently skip delivery. Existing `issue`
remains token-only for the earlier database experiment; both HTTP demos use the
delivery variant. No generic queue/transport framework was introduced.

SQLite-local migration 0003 creates contacts and the outbox. Shared and local
migrations are composed into one SQLx migrator: running two unrelated migrators
against one ledger would reject each other's recorded versions. Tests upgrade
from the actual shared migration history and run migration again. Turso does not
acquire delivery support by implication.

## Delivery contract

- HTTP 201 means invitation and delivery job are committed, not SMTP success.
  The old response token remains a clearly labelled demo preview for
  compatibility.
- An outbox row snapshots recipient and raw token. The invitation table still
  stores only its hash. Raw outbox fields are cleared when sent or dead.
- A worker claims one eligible row in a short transaction, increments attempts,
  and releases the connection before SMTP. Completion requires the same attempt
  and a still-live lease; old workers cannot acknowledge newer claims.
- Lease: 30 seconds. Whole-send timeout: 10 seconds. Up to five attempts, with
  delays of 5, 10, 20, and 40 seconds after the first four failures. Timeout and
  transient errors retry; malformed payloads and permanent SMTP errors stop.
- Expired or accepted invitations are suppressed before claiming.
  Already-running sends may finish after acceptance or expiry. Acceptance itself
  still enforces expiry, recipient and single-use rules.
- A final crashed attempt becomes dead when its lease expires and cleanup runs.
  Dead jobs are not manually requeued in this milestone. After invitation
  expiry, a new issuance creates a new token; retries never renew invitation
  lifetime.
- SMTP acceptance followed by a crash before DB acknowledgement may duplicate
  mail. Retries have the same Message-ID, derived from the token digest, but
  receivers need not deduplicate it. Tests explicitly capture duplicate mail.
- Error records/logs contain bounded categories rather than SMTP bodies/tokens.
  Clearing columns does not erase WAL/pages/backups or already captured mail.

The queue survives closing/reopening connections to its database, but both demo
binaries intentionally use temporary databases. **Restarting either demo loses
its invitations and jobs.** Mailpit's orb inbox persists at
`/tmp/iris-mailpit.db` and retains up to 500 messages, with a 24-hour age limit.
It may contain old, now-invalid links after a demo reset. This is not production
durability.

## Try it in an orb

Run `.agents/setup`, then `amp orb services ensure`. The output includes the
existing legacy application portal and **Iris local inbox**. No credentials or
real email account are required.

1. In the app portal, issue as Alice for project 41 to Bob.
2. Open Bob's newest project-41 message in the Mailpit inbox.
3. Follow its link, select Bob's development identity, and accept explicitly.
4. Submit again to see the single-use conflict.

The legacy worker uses the portal URL from `.amp/portals/iris-web.json`; no
thread-specific domain is committed. Its service starts after the web portal is
registered. The auth worker instead uses its configured loopback frontend
origin. Those auth links are for the orb's browser, not the user's browser.
Exposing the inbox does not make loopback OIDC externally reachable.

Email links use `/#invitation=<token>`. React reads the fragment into memory and
replaces the address-bar URL; fragment changes on an already-loaded page work
too. No acceptance occurs on GET. No token goes in local/session storage or OIDC
parameters. A full login navigation discards it: **sign in first, then reopen
the email link**. The banner explains this. Browser extensions/history sync and
email systems are outside this privacy guarantee.

For a non-orb local run, use the API guide's explicit frontend origin and start
Mailpit separately with loopback SMTP/UI bindings. The pinned installer
currently supports Linux x86_64; other platforms need the matching upstream
binary.

## Verification

```sh
cargo test --workspace --locked
cargo test --locked -p iris-api-spike --features dev-identity
bash experiments/api-slice/checks/install-mailpit.sh
cargo test --locked -p iris-api-spike --features dev-identity --test mail_delivery -- --ignored
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
npm --prefix experiments/api-slice/web run verify
# Fresh auth demo fixtures and running Mailpit required:
python3 experiments/api-slice/checks/auth-browser.py http://127.0.0.1:5174 \
  --artifacts .amp/in/artifacts/delivery --mailpit http://127.0.0.1:8025
```

Database tests inject enqueue failure and explicit times; verify rollback,
contact snapshots, duplicate issuance/claim exclusion, lease boundaries,
backoff, stale acknowledgements, accepted/expired suppression and terminal
cleanup. The separately invoked real-Mailpit test proves HTTP 201 during SMTP
failure, retry classification, resumed sending and ambiguous-success
duplication. It does not require the shared interactive Mailpit service.

The browser suite uses the email's token—not the API preview—to test wrong-user
rejection, sign-in/reopening the link, recipient acceptance, replay rejection,
fragment scrubbing and narrow layout. It exercises real SMTP, OIDC and SQLite.
CI runs both real-mail and browser checks explicitly. Local passing checks do
not imply the new workflow has run on GitHub.

Executed locally: 36 default workspace tests, 13 feature-enabled API tests, the
separately enabled real-Mailpit test, Clippy with warnings denied, generated
contract/TypeScript/build checks, and both legacy and mail-backed auth browser
suites passed. Screenshots of queued delivery, signed-out email entry, accepted
and replayed states, narrow layout and the captured email were inspected. The
mail test also asserts UTF-8 decoding and stable Message-ID across duplicates.
Setup ran twice successfully (approximately 3.2 s then 2.3 s).

The [delivery build-loop measurements](delivery-results.json) retain the earlier
result files. Three-sample medians: no-change targeted test 0.422 s, body edit
plus test 3.188 s, contract-shape edit through React typechecking 5.176 s. One
fresh-target test build took 54.026 s. The auth milestone's corresponding
medians were 0.372 s, 2.588 s and 4.774 s. These sequential orb observations are
not controlled benchmarks or proof of a causal dependency cost. Exact
commands/source hashes are recorded; subsequent documentation and UTF-8-header
corrections are not part of that measurement snapshot.

No delivery-status dashboard/API, manual retry/resend, bounce handling, real
SMTP credentials, verified emails, account signup, or production persistence is
implemented. These need product decisions before further framework extraction.
