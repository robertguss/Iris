//! Invitation delivery claims (S19 stage 2, step 1): claiming an outbox row
//! under a lease, completing it under a fence, and sweeping rows that can no
//! longer be delivered. The private supervised worker uses these functions;
//! no route exposes them. Time is the caller's Unix-seconds `now`.
use super::{ActionError, Stage, StopReason, execution, finalize, unconfirmed, usable_contact};
use sqlx::{Connection, Row, SqliteConnection};
use std::convert::Infallible;
use std::fmt;

/// How long a claim holds its row.
pub const LEASE_SECONDS: i64 = 30;
/// Claims a row may consume, whether or not they were completed.
pub const CLAIM_BUDGET: i64 = 5;
/// Wait before the next claim after a retryable completion of claim 1 to 4.
pub const BACKOFF_SECONDS: [i64; 4] = [5, 10, 20, 40];

/// No rejection exists here: every failure is an execution failure.
pub type DeliveryError = ActionError<Infallible>;

/// One claimed delivery. The credential, address and Message-ID are exactly as
/// issued and never printed by `Debug`.
pub struct Claim {
    pub outbox_id: i64,
    pub invitation_id: i64,
    pub project_id: i64,
    pub expires_at: i64,
    pub attempt: i64,
    pub lease_until: i64,
    pub recipient_email: String,
    pub token: String,
    pub message_id: String,
}

impl fmt::Debug for Claim {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Claim")
            .field("outbox_id", &self.outbox_id)
            .field("invitation_id", &self.invitation_id)
            .field("project_id", &self.project_id)
            .field("expires_at", &self.expires_at)
            .field("attempt", &self.attempt)
            .field("lease_until", &self.lease_until)
            .field("recipient_email", &"<redacted>")
            .field("token", &"<redacted>")
            .field("message_id", &"<redacted>")
            .finish()
    }
}

/// What a send attempt came to, as the caller classifies it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Completion {
    Sent,
    Retryable,
    Permanent,
}

/// Claims the oldest eligible row in one write transaction, or `None`. A
/// malformed eligible row is finished as `malformed` and skipped. Callers must
/// dispose of the connection after failures.
pub async fn claim(conn: &mut SqliteConnection, now: i64) -> Result<Option<Claim>, DeliveryError> {
    let mut tx = conn
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|e| unconfirmed(Stage::Begin, e))?;
    let body: Result<Option<Claim>, StopReason<Infallible>> = async {
        let body = |e| execution(Stage::Body, e);
        // Terminates because the SELECT filters on `outcome IS NULL`: a row
        // marked malformed below is never selected again.
        loop {
            let row = sqlx::query(
                "SELECT o.id, o.invitation_id, i.project_id, i.expires_at, o.claims, o.token, o.recipient_email, o.message_id \
                 FROM invitation_outbox o JOIN invitations i ON i.id = o.invitation_id \
                 WHERE o.outcome IS NULL AND o.claims < ?2 AND o.next_claim_at <= ?1 \
                 AND (o.lease_until IS NULL OR o.lease_until <= ?1) \
                 AND i.accepted_at IS NULL AND ?1 < i.expires_at \
                 ORDER BY o.id LIMIT 1",
            )
            .bind(now)
            .bind(CLAIM_BUDGET)
            .fetch_optional(&mut *tx)
            .await
            .map_err(body)?;
            let Some(row) = row else { return Ok(None) };
            let outbox_id: i64 = row.try_get("id").map_err(body)?;
            let token: Option<String> = row.try_get("token").map_err(body)?;
            let email: Option<String> = row.try_get("recipient_email").map_err(body)?;
            let usable = token.as_deref().is_some_and(|t| !t.is_empty())
                && email.as_deref().is_some_and(usable_contact);
            let (Some(token), Some(recipient_email), true) = (token, email, usable) else {
                sqlx::query("UPDATE invitation_outbox SET outcome='malformed', token=NULL, recipient_email=NULL, lease_until=NULL WHERE id=?")
                    .bind(outbox_id)
                    .execute(&mut *tx)
                    .await
                    .map_err(body)?;
                continue;
            };
            let attempt = row.try_get::<i64, _>("claims").map_err(body)? + 1;
            let lease_until = now + LEASE_SECONDS;
            sqlx::query("UPDATE invitation_outbox SET claims=?, lease_until=? WHERE id=?")
                .bind(attempt)
                .bind(lease_until)
                .bind(outbox_id)
                .execute(&mut *tx)
                .await
                .map_err(body)?;
            return Ok(Some(Claim {
                outbox_id,
                invitation_id: row.try_get("invitation_id").map_err(body)?,
                project_id: row.try_get("project_id").map_err(body)?,
                expires_at: row.try_get("expires_at").map_err(body)?,
                attempt,
                lease_until,
                recipient_email,
                token,
                message_id: row.try_get("message_id").map_err(body)?,
            }));
        }
    }
    .await;
    match body {
        Ok(claim) => tx
            .commit()
            .await
            .map(|()| claim)
            .map_err(|e| unconfirmed(Stage::Commit, e)),
        Err(primary) => Err(finalize(primary, tx.rollback().await)),
    }
}

/// Records a send's result, fenced by the outbox id, the claim's attempt, a
/// live lease and no terminal outcome. `Ok(false)` means the fence matched
/// nothing and no row changed. Callers must dispose of the connection after
/// failures.
pub async fn complete(
    conn: &mut SqliteConnection,
    outbox_id: i64,
    attempt: i64,
    result: Completion,
    now: i64,
) -> Result<bool, DeliveryError> {
    let mut tx = conn
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|e| unconfirmed(Stage::Begin, e))?;
    let body: Result<bool, StopReason<Infallible>> = async {
        let terminal = match result {
            Completion::Sent => Some("sent"),
            Completion::Permanent => Some("permanent"),
            Completion::Retryable if attempt >= CLAIM_BUDGET => Some("exhausted"),
            Completion::Retryable => None,
        };
        let changed = match terminal {
            Some(outcome) => sqlx::query("UPDATE invitation_outbox SET outcome=?, token=NULL, recipient_email=NULL, lease_until=NULL WHERE id=? AND claims=? AND lease_until > ? AND outcome IS NULL")
                .bind(outcome)
                .bind(outbox_id)
                .bind(attempt)
                .bind(now)
                .execute(&mut *tx)
                .await,
            None => {
                let backoff = usize::try_from(attempt - 1)
                    .ok()
                    .and_then(|i| BACKOFF_SECONDS.get(i))
                    .copied()
                    .unwrap_or(BACKOFF_SECONDS[0]);
                sqlx::query("UPDATE invitation_outbox SET lease_until=NULL, next_claim_at=? WHERE id=? AND claims=? AND lease_until > ? AND outcome IS NULL")
                    .bind(now + backoff)
                    .bind(outbox_id)
                    .bind(attempt)
                    .bind(now)
                    .execute(&mut *tx)
                    .await
            }
        }
        .map_err(|e| execution(Stage::Body, e))?
        .rows_affected();
        Ok(changed == 1)
    }
    .await;
    match body {
        Ok(changed) => tx
            .commit()
            .await
            .map(|()| changed)
            .map_err(|e| unconfirmed(Stage::Commit, e)),
        Err(primary) => Err(finalize(primary, tx.rollback().await)),
    }
}

/// Finishes rows that can no longer be delivered and have no live lease: the
/// invitation was accepted, has expired, or the claim budget is spent (checked
/// in that order). Returns how many rows it finished. Callers must dispose of
/// the connection after failures.
pub async fn sweep(conn: &mut SqliteConnection, now: i64) -> Result<u64, DeliveryError> {
    let mut tx = conn
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|e| unconfirmed(Stage::Begin, e))?;
    let body: Result<u64, StopReason<Infallible>> = async {
        let finished = sqlx::query(
            "UPDATE invitation_outbox SET \
             outcome = CASE \
               WHEN EXISTS (SELECT 1 FROM invitations i WHERE i.id = invitation_id AND i.accepted_at IS NOT NULL) THEN 'accepted' \
               WHEN EXISTS (SELECT 1 FROM invitations i WHERE i.id = invitation_id AND ?1 >= i.expires_at) THEN 'expired' \
               ELSE 'exhausted' END, \
             token=NULL, recipient_email=NULL, lease_until=NULL \
             WHERE outcome IS NULL AND (lease_until IS NULL OR lease_until <= ?1) \
             AND (EXISTS (SELECT 1 FROM invitations i WHERE i.id = invitation_id AND (i.accepted_at IS NOT NULL OR ?1 >= i.expires_at)) \
                  OR claims >= ?2)",
        )
        .bind(now)
        .bind(CLAIM_BUDGET)
        .execute(&mut *tx)
        .await
        .map_err(|e| execution(Stage::Body, e))?
        .rows_affected();
        Ok(finished)
    }
    .await;
    match body {
        Ok(finished) => tx
            .commit()
            .await
            .map(|()| finished)
            .map_err(|e| unconfirmed(Stage::Commit, e)),
        Err(primary) => Err(finalize(primary, tx.rollback().await)),
    }
}

#[cfg(test)]
mod tests;
