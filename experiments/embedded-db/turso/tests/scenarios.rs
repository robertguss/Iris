use iris_turso_spike::{
    AcceptInvitation, Outcome, accept, connect, is_busy, migrate, open, token_hash,
};

fn is_constraint(error: &turso::Error) -> bool {
    matches!(error, turso::Error::Constraint(_))
}

struct Fixture {
    db: Option<turso::Database>,
    dir: tempfile::TempDir,
}

struct Conn(turso::Connection);

impl Fixture {
    async fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let db = open(&dir.path().join("app.db")).await.unwrap();
        let fixture = Self { db: Some(db), dir };
        let mut conn = fixture.connect().await;
        migrate(&mut conn.0).await.unwrap();
        migrate(&mut conn.0).await.unwrap();
        conn.execute("INSERT INTO users VALUES (11), (29); INSERT INTO projects VALUES (7), (19)")
            .await
            .unwrap();
        fixture
    }

    async fn connect(&self) -> Conn {
        let conn = connect(self.db.as_ref().unwrap()).await.unwrap();
        Conn(conn)
    }

    async fn reopen(&mut self) {
        self.db.take();
        self.db = Some(open(&self.dir.path().join("app.db")).await.unwrap());
    }
}

impl Conn {
    async fn execute(&mut self, sql: &str) -> Result<(), turso::Error> {
        self.0.execute_batch(sql).await?;
        Ok(())
    }

    async fn scalar(&mut self, sql: &str) -> i64 {
        let mut rows = self.0.query(sql, ()).await.unwrap();
        rows.next().await.unwrap().unwrap().get(0).unwrap()
    }

    async fn invite(&mut self, token: &str, project: i64, recipient: i64, expires: i64) {
        self.0.execute("INSERT INTO invitations (token_hash, project_id, recipient_id, role, expires_at) VALUES (?, ?, ?, 'editor', ?)",
            turso::params![token_hash(token), project, recipient, expires]).await.unwrap();
    }

    async fn accept(&mut self, token: &str, user: i64, now: i64) -> Result<Outcome, turso::Error> {
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
        drop(self);
    }
}

#[path = "../../shared/scenarios.rs"]
mod scenarios;
