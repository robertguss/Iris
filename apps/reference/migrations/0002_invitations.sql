-- Invitations (S19 stage 1). The invitation keeps only the credential's hash;
-- the outbox keeps the plaintext credential and the contact snapshot until
-- delivery clears them, so both payload columns are nullable. Delivery's
-- claim and lease columns arrive with the worker, in a later migration.
CREATE TABLE invitations (
    id INTEGER PRIMARY KEY,
    project_id INTEGER NOT NULL REFERENCES projects(id),
    recipient_id INTEGER NOT NULL REFERENCES users(id),
    issuer_id INTEGER NOT NULL REFERENCES users(id),
    role TEXT NOT NULL CHECK (role = 'editor'),
    token_hash TEXT NOT NULL UNIQUE,
    created_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL,
    accepted_at INTEGER
);
CREATE INDEX invitations_project_recipient ON invitations (project_id, recipient_id);
CREATE TABLE invitation_outbox (
    id INTEGER PRIMARY KEY,
    invitation_id INTEGER NOT NULL UNIQUE REFERENCES invitations(id),
    recipient_email TEXT,
    token TEXT,
    message_id TEXT NOT NULL UNIQUE,
    created_at INTEGER NOT NULL
);
