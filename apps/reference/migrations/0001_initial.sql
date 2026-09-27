CREATE TABLE users (
    id INTEGER PRIMARY KEY,
    display_name TEXT NOT NULL CHECK (display_name <> '')
);
CREATE TABLE projects (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL CHECK (name <> '')
);
CREATE TABLE memberships (
    project_id INTEGER NOT NULL REFERENCES projects(id),
    user_id INTEGER NOT NULL REFERENCES users(id),
    role TEXT NOT NULL CHECK (role IN ('viewer', 'editor', 'owner')),
    PRIMARY KEY (project_id, user_id)
);
-- Delivery-only contact data: no read or response discloses it.
CREATE TABLE user_contacts (
    user_id INTEGER PRIMARY KEY REFERENCES users(id),
    email TEXT NOT NULL
);
CREATE TABLE iris_sessions (
    id TEXT PRIMARY KEY NOT NULL,
    data TEXT NOT NULL,
    expires_at INTEGER NOT NULL
);
CREATE TABLE iris_external_identities (
    issuer TEXT NOT NULL,
    subject TEXT NOT NULL,
    user_id INTEGER NOT NULL REFERENCES users(id),
    PRIMARY KEY(issuer,subject)
);
CREATE TABLE iris_login_attempts (
    state TEXT PRIMARY KEY NOT NULL,
    browser_id TEXT NOT NULL,
    nonce TEXT NOT NULL,
    verifier TEXT NOT NULL,
    expires_at INTEGER NOT NULL
);
