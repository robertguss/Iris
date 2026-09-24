use std::{path::Path, time::Duration};

use sqlx::{Connection, Row, SqliteConnection, sqlite::SqliteConnectOptions};

#[path = "../../shared/domain.rs"]
mod domain;
pub use domain::{AcceptInvitation, Outcome, token_hash};

mod issue;
pub use issue::{IssueInvitation, IssueOutcome, issue};

pub async fn connect(path: &Path) -> Result<SqliteConnection, sqlx::Error> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .foreign_keys(true)
        .busy_timeout(Duration::from_millis(100));
    SqliteConnection::connect_with(&options).await
}

pub async fn migrate(conn: &mut SqliteConnection) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("../shared/migrations").run(conn).await
}

pub async fn accept(
    conn: &mut SqliteConnection,
    input: AcceptInvitation<'_>,
) -> Result<Outcome, sqlx::Error> {
    let mut tx = conn.begin_with("BEGIN IMMEDIATE").await?;
    let result = async {
        let row = sqlx::query(
            "SELECT project_id, role, expires_at, accepted_at FROM invitations
             WHERE token_hash = ? AND recipient_id = ?",
        )
        .bind(token_hash(input.token))
        .bind(input.user_id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(row) = row else {
            return Ok(Outcome::NotFound);
        };
        if row.try_get::<Option<i64>, _>("accepted_at")?.is_some() {
            return Ok(Outcome::AlreadyAccepted);
        }
        if row.try_get::<i64, _>("expires_at")? <= input.now {
            return Ok(Outcome::Expired);
        }
        let project_id: i64 = row.try_get("project_id")?;
        let role: String = row.try_get("role")?;
        // Write intent was acquired before the read. Retain the predicates on
        // the write so the state transition itself expresses the invariant.
        let claimed: Option<i64> = sqlx::query_scalar(
            "UPDATE invitations SET accepted_at = ?, accepted_by = ?
             WHERE token_hash = ? AND recipient_id = ?
               AND accepted_at IS NULL AND expires_at > ? RETURNING project_id",
        )
        .bind(input.now)
        .bind(input.user_id)
        .bind(token_hash(input.token))
        .bind(input.user_id)
        .bind(input.now)
        .fetch_optional(&mut *tx)
        .await?;
        if claimed.is_none() {
            return Err(sqlx::Error::Protocol(
                "invitation claim lost under write lock".into(),
            ));
        }
        sqlx::query(
            "INSERT INTO memberships (project_id, user_id, role) VALUES (?, ?, ?)
             ON CONFLICT (project_id, user_id) DO NOTHING",
        )
        .bind(project_id)
        .bind(input.user_id)
        .bind(role)
        .execute(&mut *tx)
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

pub fn is_busy(error: &sqlx::Error) -> bool {
    error
        .as_database_error()
        .and_then(|e| e.code())
        .and_then(|code| code.parse::<i32>().ok())
        .is_some_and(|code| code & 0xff == 5)
}
