use iris_sqlite_spike::{AcceptInvitation, Outcome, accept, connect, is_busy, migrate, token_hash};
use sqlx::SqliteConnection;

fn is_constraint(error: &sqlx::Error) -> bool {
    error
        .as_database_error()
        .and_then(|e| e.code())
        .and_then(|code| code.parse::<i32>().ok())
        .is_some_and(|code| code & 0xff == 19)
}

struct Fixture {
    dir: tempfile::TempDir,
}

struct Conn(SqliteConnection);

impl Fixture {
    async fn new() -> Self {
        let fixture = Self {
            dir: tempfile::tempdir().unwrap(),
        };
        let mut conn = fixture.connect().await;
        migrate(&mut conn.0).await.unwrap();
        migrate(&mut conn.0).await.unwrap();
        conn.execute("INSERT INTO users VALUES (11), (29); INSERT INTO projects VALUES (7), (19)")
            .await
            .unwrap();
        fixture
    }

    async fn connect(&self) -> Conn {
        Conn(connect(&self.dir.path().join("app.db")).await.unwrap())
    }

    async fn reopen(&mut self) {
        // No engine handle: each connection opens the same file independently.
    }
}

impl Conn {
    async fn execute(&mut self, sql: &'static str) -> Result<(), sqlx::Error> {
        sqlx::raw_sql(sql).execute(&mut self.0).await?;
        Ok(())
    }

    async fn scalar(&mut self, sql: &'static str) -> i64 {
        sqlx::query_scalar(sql)
            .fetch_one(&mut self.0)
            .await
            .unwrap()
    }

    async fn invite(&mut self, token: &str, project: i64, recipient: i64, expires: i64) {
        sqlx::query("INSERT INTO invitations (token_hash, project_id, recipient_id, role, expires_at) VALUES (?, ?, ?, 'editor', ?)")
            .bind(token_hash(token)).bind(project).bind(recipient).bind(expires)
            .execute(&mut self.0).await.unwrap();
    }

    async fn accept(&mut self, token: &str, user: i64, now: i64) -> Result<Outcome, sqlx::Error> {
        accept(
            &mut self.0,
            AcceptInvitation {
                token,
                user_id: user,
                now,
            },
        )
        .await
    }

    async fn close(self) {
        use sqlx::Connection;
        self.0.close().await.unwrap();
    }
}

#[path = "../../shared/scenarios.rs"]
mod scenarios;
