CREATE TABLE user_contacts (
    user_id INTEGER PRIMARY KEY REFERENCES users(id),
    email TEXT NOT NULL CHECK (email LIKE '%@%.test' AND instr(email, char(10)) = 0 AND instr(email, char(13)) = 0)
);
CREATE TABLE invitation_outbox (
    invitation_id INTEGER PRIMARY KEY REFERENCES invitations(id),
    recipient TEXT,
    token TEXT,
    state TEXT NOT NULL DEFAULT 'pending' CHECK (state IN ('pending','sent','dead')),
    attempts INTEGER NOT NULL DEFAULT 0,
    next_attempt_at INTEGER NOT NULL,
    lease_until INTEGER,
    reason TEXT,
    CHECK ((state='pending' AND recipient IS NOT NULL AND token IS NOT NULL)
        OR (state!='pending' AND recipient IS NULL AND token IS NULL))
);
