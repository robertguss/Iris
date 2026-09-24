use sqlx::{Connection, Row, SqliteConnection};

pub const MAX_ATTEMPTS: i64 = 5;
pub const LEASE_SECONDS: i64 = 30;

// Deliberately no Debug: this value contains a credential.
pub struct Delivery {
    pub invitation_id: i64,
    pub recipient: String,
    pub token: String,
    pub attempt: i64,
    pub project_id: i64,
}

pub enum Completion {
    Sent,
    Retry,
    PermanentFailure,
}

/// Short write transaction; never hold it across SMTP.
pub async fn claim(conn: &mut SqliteConnection, now: i64) -> Result<Option<Delivery>, sqlx::Error> {
    let mut tx = conn.begin_with("BEGIN IMMEDIATE").await?;
    sqlx::query("UPDATE invitation_outbox SET state='dead',recipient=NULL,token=NULL,lease_until=NULL,reason='expired_accepted_or_exhausted'
        WHERE state='pending' AND (lease_until IS NULL OR lease_until<=?) AND
        (attempts>=? OR invitation_id IN (SELECT id FROM invitations WHERE expires_at<=? OR accepted_at IS NOT NULL))")
        .bind(now).bind(MAX_ATTEMPTS).bind(now).execute(&mut *tx).await?;
    let row = sqlx::query("UPDATE invitation_outbox SET attempts=attempts+1,lease_until=? WHERE invitation_id=(
        SELECT o.invitation_id FROM invitation_outbox o JOIN invitations i ON i.id=o.invitation_id
        WHERE o.state='pending' AND o.next_attempt_at<=? AND (o.lease_until IS NULL OR o.lease_until<=?)
        AND o.attempts<? AND i.expires_at>? AND i.accepted_at IS NULL ORDER BY o.next_attempt_at,o.invitation_id LIMIT 1)
        RETURNING invitation_id,recipient,token,attempts")
        .bind(now+LEASE_SECONDS).bind(now).bind(now).bind(MAX_ATTEMPTS).bind(now).fetch_optional(&mut *tx).await?;
    let result = if let Some(row) = row {
        let invitation_id = row.get("invitation_id");
        let project_id = sqlx::query_scalar("SELECT project_id FROM invitations WHERE id=?")
            .bind(invitation_id)
            .fetch_one(&mut *tx)
            .await?;
        Some(Delivery {
            invitation_id,
            recipient: row.get("recipient"),
            token: row.get("token"),
            attempt: row.get("attempts"),
            project_id,
        })
    } else {
        None
    };
    tx.commit().await?;
    Ok(result)
}

/// A previous lease holder cannot acknowledge a replacement worker's attempt.
pub async fn complete(
    conn: &mut SqliteConnection,
    delivery: &Delivery,
    outcome: Completion,
    now: i64,
) -> Result<bool, sqlx::Error> {
    let (state, reason) = match outcome {
        Completion::Sent => ("sent", "smtp_accepted"),
        Completion::PermanentFailure => ("dead", "permanent_failure"),
        Completion::Retry if delivery.attempt >= MAX_ATTEMPTS => ("dead", "attempts_exhausted"),
        Completion::Retry => ("pending", "transient_failure"),
    };
    let delay = (5 * (1_i64 << (delivery.attempt - 1))).min(60);
    let result = sqlx::query("UPDATE invitation_outbox SET state=?,reason=?,next_attempt_at=?,lease_until=NULL,
        recipient=CASE WHEN ?='pending' THEN recipient ELSE NULL END,token=CASE WHEN ?='pending' THEN token ELSE NULL END
        WHERE invitation_id=? AND state='pending' AND attempts=? AND lease_until>?")
        .bind(state).bind(reason).bind(now+delay).bind(state).bind(state)
        .bind(delivery.invitation_id).bind(delivery.attempt).bind(now).execute(conn).await?;
    Ok(result.rows_affected() == 1)
}
