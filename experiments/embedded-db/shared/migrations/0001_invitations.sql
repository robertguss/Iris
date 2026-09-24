CREATE TABLE users (id INTEGER PRIMARY KEY);
CREATE TABLE projects (id INTEGER PRIMARY KEY);
CREATE TABLE invitations (
    id INTEGER PRIMARY KEY,
    token_hash BLOB NOT NULL UNIQUE,
    project_id INTEGER NOT NULL REFERENCES projects(id),
    recipient_id INTEGER NOT NULL REFERENCES users(id),
    role TEXT NOT NULL CHECK (role IN ('viewer', 'editor')),
    expires_at INTEGER NOT NULL,
    accepted_at INTEGER,
    accepted_by INTEGER REFERENCES users(id),
    CHECK ((accepted_at IS NULL AND accepted_by IS NULL)
        OR (accepted_at IS NOT NULL AND accepted_by IS NOT NULL AND accepted_by = recipient_id))
);
CREATE TABLE memberships (
    project_id INTEGER NOT NULL REFERENCES projects(id),
    user_id INTEGER NOT NULL REFERENCES users(id),
    role TEXT NOT NULL CHECK (role IN ('viewer', 'editor')),
    PRIMARY KEY (project_id, user_id)
);
