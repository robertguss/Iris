use std::{path::Path, time::Duration};

#[path = "../../shared/domain.rs"]
mod domain;
pub use domain::{AcceptInvitation, Outcome, token_hash};

pub async fn open(path: &Path) -> Result<turso::Database, turso::Error> {
    turso::Builder::new_local(&path.to_string_lossy())
        .build()
        .await
}

pub async fn connect(db: &turso::Database) -> Result<turso::Connection, turso::Error> {
    let conn = db.connect()?;
    conn.busy_timeout(Duration::from_millis(100))?;
    conn.execute("PRAGMA foreign_keys = ON", ()).await?;
    Ok(conn)
}

/// One explicit migration for the spike. Not a reusable migration runner.
pub async fn migrate(conn: &mut turso::Connection) -> Result<(), turso::Error> {
    let tx = conn
        .transaction_with_behavior(turso::transaction::TransactionBehavior::Immediate)
        .await?;
    let mut rows = tx.query("PRAGMA user_version", ()).await?;
    let version: i64 = rows.next().await?.unwrap().get(0)?;
    drop(rows);
    if version == 0 {
        tx.execute_batch(include_str!("../../shared/migrations/0001_invitations.sql"))
            .await?;
        tx.execute("PRAGMA user_version = 1", ()).await?;
    } else if version != 1 {
        return Err(turso::Error::ConversionFailure(
            "unsupported schema version".into(),
        ));
    }
    tx.commit().await
}

pub async fn accept(
    conn: &mut turso::Connection,
    input: AcceptInvitation<'_>,
) -> Result<Outcome, turso::Error> {
    let tx = conn
        .transaction_with_behavior(turso::transaction::TransactionBehavior::Immediate)
        .await?;
    let result = async {
        let mut rows = tx
            .query(
                "SELECT project_id, role, expires_at, accepted_at FROM invitations
             WHERE token_hash = ? AND recipient_id = ?",
                turso::params![token_hash(input.token), input.user_id],
            )
            .await?;
        let row = rows.next().await?;
        drop(rows);
        let Some(row) = row else {
            return Ok(Outcome::NotFound);
        };
        if row.get::<Option<i64>>(3)?.is_some() {
            return Ok(Outcome::AlreadyAccepted);
        }
        if row.get::<i64>(2)? <= input.now {
            return Ok(Outcome::Expired);
        }
        let project_id: i64 = row.get(0)?;
        let role: String = row.get(1)?;
        let mut rows = tx
            .query(
                "UPDATE invitations SET accepted_at = ?, accepted_by = ?
             WHERE token_hash = ? AND recipient_id = ?
               AND accepted_at IS NULL AND expires_at > ? RETURNING project_id",
                turso::params![
                    input.now,
                    input.user_id,
                    token_hash(input.token),
                    input.user_id,
                    input.now
                ],
            )
            .await?;
        let claimed = rows.next().await?;
        // Complete RETURNING before issuing another write on this connection.
        while rows.next().await?.is_some() {}
        drop(rows);
        if claimed.is_none() {
            return Err(turso::Error::ConversionFailure(
                "invitation claim lost under write lock".into(),
            ));
        }
        tx.execute(
            "INSERT INTO memberships (project_id, user_id, role) VALUES (?, ?, ?)
             ON CONFLICT (project_id, user_id) DO NOTHING",
            turso::params![project_id, input.user_id, role],
        )
        .await?;
        Ok(Outcome::Accepted { project_id })
    }
    .await;
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

pub fn is_busy(error: &turso::Error) -> bool {
    matches!(error, turso::Error::Busy(_))
}
