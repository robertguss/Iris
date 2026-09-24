-- Initial, idempotent schema for this disposable authentication experiment.
CREATE TABLE IF NOT EXISTS iris_sessions (
    id TEXT PRIMARY KEY NOT NULL,
    data TEXT NOT NULL,
    expires_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS iris_external_identities (
    issuer TEXT NOT NULL,
    subject TEXT NOT NULL,
    user_id INTEGER NOT NULL REFERENCES users(id),
    PRIMARY KEY(issuer,subject)
);
CREATE TABLE IF NOT EXISTS iris_login_attempts (
    state TEXT PRIMARY KEY NOT NULL,
    browser_id TEXT NOT NULL,
    nonce TEXT NOT NULL,
    verifier TEXT NOT NULL,
    expires_at INTEGER NOT NULL
);
