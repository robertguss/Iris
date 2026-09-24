CREATE TABLE memberships_with_owners (
    project_id INTEGER NOT NULL REFERENCES projects(id),
    user_id INTEGER NOT NULL REFERENCES users(id),
    role TEXT NOT NULL CHECK (role IN ('viewer', 'editor', 'owner')),
    PRIMARY KEY (project_id, user_id)
);
INSERT INTO memberships_with_owners SELECT project_id, user_id, role FROM memberships;
DROP TABLE memberships;
ALTER TABLE memberships_with_owners RENAME TO memberships;
CREATE INDEX invitations_pending_lookup ON invitations(project_id, recipient_id, expires_at)
    WHERE accepted_at IS NULL;
