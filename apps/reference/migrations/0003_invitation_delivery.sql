-- Invitation delivery (S19 stage 2, step 1): claim, lease, attempt and terminal
-- state for the outbox. Stage-1 rows migrate with no claims, no lease and no
-- outcome, so they are immediately eligible. A terminal outcome requires the
-- payload and lease to be gone, so clearing is a database invariant.
ALTER TABLE invitation_outbox ADD COLUMN claims INTEGER NOT NULL DEFAULT 0 CHECK (claims BETWEEN 0 AND 5);
ALTER TABLE invitation_outbox ADD COLUMN next_claim_at INTEGER NOT NULL DEFAULT 0;
ALTER TABLE invitation_outbox ADD COLUMN lease_until INTEGER;
ALTER TABLE invitation_outbox ADD COLUMN outcome TEXT CHECK (
    outcome IS NULL OR (
        outcome IN ('sent', 'exhausted', 'malformed', 'permanent', 'expired', 'accepted')
        AND token IS NULL
        AND recipient_email IS NULL
        AND lease_until IS NULL
    )
);
