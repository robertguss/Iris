use sqlx::{Row, SqlitePool};
use tower_sessions::{
    SessionStore,
    session::{Id, Record},
    session_store,
};

#[derive(Clone, Debug)]
pub struct Store {
    pub pool: SqlitePool,
    pub now: fn() -> i64,
}

fn backend(error: impl std::fmt::Display) -> session_store::Error {
    session_store::Error::Backend(error.to_string())
}

impl Store {
    pub async fn migrate(&self) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        sqlx::raw_sql(include_str!("schema.sql"))
            .execute(&mut *tx)
            .await?;
        tx.commit().await
    }

    pub async fn cleanup(&self) -> Result<(), sqlx::Error> {
        for query in [
            "DELETE FROM iris_sessions WHERE expires_at <= ?",
            "DELETE FROM iris_login_attempts WHERE expires_at <= ?",
        ] {
            sqlx::query(query)
                .bind((self.now)())
                .execute(&self.pool)
                .await?;
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl SessionStore for Store {
    async fn create(&self, record: &mut Record) -> session_store::Result<()> {
        loop {
            let inserted = sqlx::query("INSERT INTO iris_sessions(id,data,expires_at) VALUES(?,?,?) ON CONFLICT(id) DO NOTHING")
                .bind(record.id.to_string()).bind(serde_json::to_string(&record.data).map_err(backend)?)
                .bind(record.expiry_date.unix_timestamp()).execute(&self.pool).await.map_err(backend)?;
            if inserted.rows_affected() == 1 {
                return Ok(());
            }
            record.id = Id::default();
        }
    }

    async fn save(&self, record: &Record) -> session_store::Result<()> {
        let updated = sqlx::query("UPDATE iris_sessions SET data=?,expires_at=MIN(expires_at,?) WHERE id=? AND expires_at>?")
            .bind(serde_json::to_string(&record.data).map_err(backend)?)
            .bind(record.expiry_date.unix_timestamp()).bind(record.id.to_string())
            .bind((self.now)()).execute(&self.pool).await.map_err(backend)?;
        if updated.rows_affected() != 1 {
            return Err(backend("session no longer exists"));
        }
        Ok(())
    }

    async fn load(&self, id: &Id) -> session_store::Result<Option<Record>> {
        let row =
            sqlx::query("SELECT data,expires_at FROM iris_sessions WHERE id=? AND expires_at>?")
                .bind(id.to_string())
                .bind((self.now)())
                .fetch_optional(&self.pool)
                .await
                .map_err(backend)?;
        row.map(|row| {
            Ok(Record {
                id: *id,
                data: serde_json::from_str(row.get::<&str, _>("data")).map_err(backend)?,
                expiry_date: time::OffsetDateTime::from_unix_timestamp(row.get("expires_at"))
                    .map_err(backend)?,
            })
        })
        .transpose()
    }

    async fn delete(&self, id: &Id) -> session_store::Result<()> {
        sqlx::query("DELETE FROM iris_sessions WHERE id=?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(backend)?;
        Ok(())
    }
}
