# Credential-free authentication experiment

Status: implemented and locally verified; not a deployable authentication
service. This builds on the invitation actions rather than creating a generic
authentication framework.

## Selected integration

- `openidconnect 4.0.1`: discovery, authorization code + S256 PKCE, state,
  nonce, signature/issuer/audience/expiry validation, and optional access-token
  hash verification. Provider tokens never enter the browser application.
- `tower-sessions 0.15.0`: opaque HttpOnly browser cookies and session
  lifecycle.
- A private SQLx 0.9 SQLite store: insert-only creation with collision retry,
  update-only saves, expired-row rejection, and periodic cleanup. The published
  `tower-sessions-sqlx-store 0.15.0` uses SQLx 0.8 and sessions-core 0.14;
  `axum-login 0.18.0` uses sessions 0.14. Repository main is not the published
  dependency contract. Neither adapter is included.
- Existing local users are explicitly mapped by `(issuer, subject)`. No email
  matching, implicit registration, password storage, or access-token API exists.

The librarian checked package compatibility and library source. Oracle review
identified the callback/logout race; a regression now pauses token exchange,
logs out, releases the exchange, and asserts no authenticated session is
created.

## Session and request rules

HTTPS origins use `__Host-iris-session`, Secure, HttpOnly, SameSite=Lax, Path=/,
and no Domain. Explicit HTTP loopback development uses `iris-session-dev`
without Secure. Origin is configured, never derived from forwarded headers.
Unsafe requests require both an exact Origin match and the session's CSRF
header. GET `/api/auth/session` bootstraps the CSRF token and local user ID.
POST login returns the authorization URL; GET callback completes login; POST
logout revokes this session. Callback errors are structured JSON, not a custom
error page; returning to the app permits a fresh attempt.

Anonymous sessions and login attempts last ten minutes. Successful login rotates
both session ID and CSRF token and establishes an absolute eight-hour deadline.
Reads do not slide expiry. The fixed deadline is checked in both stored row and
payload. Deleted local users cannot authenticate new domain requests; removing
an external identity mapping alone does not revoke existing sessions.

Attempts are browser-bound and atomically consumed before exchange. After token
validation, promotion atomically consumes the still-live browser row. Thus
logout that wins that race prevents promotion; two concurrent promotions cannot
both win. Generic stale saves cannot recreate a missing row. Promotion that wins
first may finish, and already-authorized business requests may finish after
logout. Logout does not revoke other browser sessions or provider SSO.

`Session::flush()` alone does **not** rotate its retained record ID in 0.15.0.
The implementation explicitly uses `cycle_id()`. Session state is server-side;
React keeps only its CSRF token in memory. Invitation tokens are displayed once
and must be copied before switching users; no browser storage is used.

## Run and verify

The orb service file preserves the legacy portal on port 5173 with the explicit
development identity feature and frontend flag. It also starts the local issuer
on 4000, auth API on 3002, and auth frontend on 5174:

```sh
amp orb services ensure
```

The auth services deliberately use loopback issuer/callback URLs. They are for
the orb's browser or a developer running locally, **not the existing public
portal**. An externally reachable demonstration needs matching HTTPS issuer,
origin, and registered callback URLs; merely exposing port 5174 is insufficient.
`iris-auth-demo` requires `--local-oidc-demo`, `IRIS_PUBLIC_ORIGIN`, and
`IRIS_OIDC_ISSUER`; it creates disposable data and two allowlisted subjects.
Start the issuer before this server, since discovery happens once at startup.

Run with the pinned Rust and Node toolchains and installed web dependencies:

```sh
cargo fmt --all --check
cargo test --workspace --locked
cargo test --locked -p iris-api-spike --features dev-identity
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
node --test experiments/api-slice/checks/oidc-provider.test.mjs
npm --prefix experiments/api-slice/web run verify
# Start fresh disposable auth services before this browser test.
python3 experiments/api-slice/checks/auth-browser.py http://127.0.0.1:5174 \
  --artifacts .amp/in/artifacts/auth
```

The Rust tests launch their own random-port local issuer. They cover invalid
signature/issuer/audience/nonce/expiry/missing ID token; browser binding and
concurrent callback replay; unmapped identities; expired attempts; CSRF/Origin;
cookie rotation; independent browser sessions; owner/recipient permissions;
logout during exchange; stale saves; expiration and storage failure. Response
bodies are checked against generated OpenAPI. The fixture separately tests real
RSA signatures, one-use codes and failed PKCE. It exposes fault/hold controls
only under the explicit test option, not in the interactive service.

The browser test exercises real redirects and cookies, Alice issuance, Bob
acceptance, replay conflict, logout/reload, disabled controls, no browser
storage, HttpOnly visibility, and narrow Chromium layout. Inspected screenshots
are evidence of appearance; HTTP/DOM assertions establish tested behavior. The
existing legacy browser suite remains separate.

GitHub Actions now specifies formatting, default and dev-feature tests, Clippy,
contract drift, TypeScript/build/breaking-change checks, fixture tests and this
credential-free browser flow. Local success is not a GitHub Actions run.

Executed locally: 32 default-workspace tests, 13 feature-enabled API tests, five
Node fixture tests, formatting, warnings-denied Clippy, web verification, and
both legacy/auth browser suites passed. The auth browser also injected a
session-bootstrap transport failure and verified disabled controls and recovery
through Refresh session. HTTPS cookie attributes were checked through the HTTP
router, not a real HTTPS browser/provider deployment. Workflow YAML and shell
syntax were checked, and pinned action commits were resolved through GitHub; the
workflow itself has not run on GitHub.

## Limitations and next experiment

The local issuer lets anyone choose Alice or Bob: valid signatures demonstrate
protocol integration, **not human identity assurance**. No real-provider smoke
test has run. Discovery/JWKS are loaded once; provider key rotation/retry
policy, operational logging, rate limiting, durable migrations/account
lifecycle, provider logout, token refresh, native-client bearer authentication
and deployment are outside this milestone. Auth schema setup is idempotent for
disposable databases, not a versioned production migration system.

The Utoipa contract includes auth endpoints. Aide remains an action-only
comparison, not a second authentication integration. The OpenAPI cookie name
describes HTTPS deployment; loopback intentionally differs. Generated TypeScript
does not perform runtime validation. Test coverage is not proof of every
security property or business requirement.

The [new measurements](auth-results.json) preserve historical `results.json`.
Three warm samples gave medians of 0.372 s for the no-change targeted invitation
test, 2.588 s for an error-message edit plus test, and 4.774 s for a contract
shape edit through React typechecking. One fresh-target test build took 49.643
s. Historical medians were 0.267 s, 1.494 s, and 3.523 s, with one 37.965 s
fresh-target build. These sequential orb runs show integration cost, not a
controlled causal benchmark. Exact source hashes and commands are in the result
files; later documentation and test additions do not change that measurement
snapshot. No compile-time optimization is claimed.

Repeat `measure.py` with an explicit output path to preserve both records. The
workload measures invitation HTTP tests and contract edits, not OIDC network
latency or production performance. Review a real provider next, then invitation
delivery/recovery; no new abstraction is justified yet.
