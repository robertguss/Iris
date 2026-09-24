use sqlx::{Connection, SqliteConnection};

pub struct IssueInvitation {
    pub project_id: i64,
    pub recipient_id: i64,
    pub actor_id: i64,
    pub now: i64,
}

#[derive(Debug, PartialEq)]
pub enum IssueOutcome {
    Issued { token: String, expires_at: i64 },
    Forbidden,
    RecipientNotFound,
    AlreadyMember,
    InvitationPending,
}

/// Owner check and duplicate check share the write transaction with insertion.
/// Invitations grant editor only; neither token nor role is caller-controlled.
pub async fn issue(
    conn: &mut SqliteConnection,
    input: IssueInvitation,
) -> Result<IssueOutcome, sqlx::Error> {
    let mut tx = conn.begin_with("BEGIN IMMEDIATE").await?;
    let result = async {
        let owner: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM memberships WHERE project_id=? AND user_id=? AND role='owner')",
        ).bind(input.project_id).bind(input.actor_id).fetch_one(&mut *tx).await?;
        if !owner { return Ok(IssueOutcome::Forbidden); }
        let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE id=?)")
            .bind(input.recipient_id).fetch_one(&mut *tx).await?;
        if !exists { return Ok(IssueOutcome::RecipientNotFound); }
        let member: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM memberships WHERE project_id=? AND user_id=?)")
            .bind(input.project_id).bind(input.recipient_id).fetch_one(&mut *tx).await?;
        if member { return Ok(IssueOutcome::AlreadyMember); }
        let pending: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM invitations WHERE project_id=? AND recipient_id=? AND accepted_at IS NULL AND expires_at>?)")
            .bind(input.project_id).bind(input.recipient_id).bind(input.now).fetch_one(&mut *tx).await?;
        if pending { return Ok(IssueOutcome::InvitationPending); }
        let expires_at = input.now.checked_add(3600)
            .ok_or_else(|| sqlx::Error::Protocol("invitation expiry overflow".into()))?;
        let mut bytes = [0_u8; 32];
        getrandom::fill(&mut bytes).map_err(|e| sqlx::Error::Io(std::io::Error::other(e.to_string())))?;
        let token: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
        sqlx::query("INSERT INTO invitations (token_hash, project_id, recipient_id, role, expires_at) VALUES (?, ?, ?, 'editor', ?)")
            .bind(crate::token_hash(&token)).bind(input.project_id).bind(input.recipient_id).bind(expires_at)
            .execute(&mut *tx).await?;
        Ok(IssueOutcome::Issued { token, expires_at })
    }.await;
    match result {
        Ok(outcome) => {
            tx.commit().await?;
            Ok(outcome)
        }
        Err(error) => {
            tx.rollback().await?;
            Err(error)
        }
    }
}
