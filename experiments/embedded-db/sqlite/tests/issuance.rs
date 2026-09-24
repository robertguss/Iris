use iris_sqlite_spike::{
    AcceptInvitation, IssueInvitation, IssueOutcome, Outcome, accept, connect, issue, migrate,
    token_hash,
};

fn input(now: i64) -> IssueInvitation {
    IssueInvitation {
        project_id: 41,
        recipient_id: 29,
        actor_id: 11,
        now,
    }
}

async fn fixture() -> (tempfile::TempDir, sqlx::SqliteConnection) {
    let dir = tempfile::tempdir().unwrap();
    let mut conn = connect(&dir.path().join("app.db")).await.unwrap();
    migrate(&mut conn).await.unwrap();
    sqlx::raw_sql(
        "INSERT INTO users VALUES(11),(29); INSERT INTO projects VALUES(41);
        INSERT INTO memberships VALUES(41,11,'owner')",
    )
    .execute(&mut conn)
    .await
    .unwrap();
    (dir, conn)
}

#[tokio::test]
async fn expiration_reissue_hash_storage_and_revocation() {
    let (_dir, mut conn) = fixture().await;
    let IssueOutcome::Issued { token, expires_at } = issue(&mut conn, input(100)).await.unwrap()
    else {
        panic!("not issued")
    };
    assert_eq!(expires_at, 3700);
    assert_eq!(token.len(), 64);
    assert!(token.bytes().all(|b| b.is_ascii_hexdigit()));
    let stored: Vec<u8> = sqlx::query_scalar("SELECT token_hash FROM invitations")
        .fetch_one(&mut conn)
        .await
        .unwrap();
    assert_eq!(
        stored.len(),
        32,
        "Store the SHA-256 digest, not the 64-byte token"
    );
    assert_eq!(stored, token_hash(&token));
    assert_eq!(
        issue(&mut conn, input(3699)).await.unwrap(),
        IssueOutcome::InvitationPending
    );
    let IssueOutcome::Issued { token: second, .. } = issue(&mut conn, input(3700)).await.unwrap()
    else {
        panic!("expired invitation prevented replacement")
    };
    assert_ne!(token, second);
    assert_eq!(
        accept(
            &mut conn,
            AcceptInvitation {
                token: &token,
                user_id: 29,
                now: 3700
            }
        )
        .await
        .unwrap(),
        Outcome::Expired
    );
    let members: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM memberships")
        .fetch_one(&mut conn)
        .await
        .unwrap();
    assert_eq!(members, 1);
    // Authorization is checked anew, before recipient/duplicate information.
    for role in ["editor", "viewer"] {
        sqlx::query("UPDATE memberships SET role=? WHERE user_id=11")
            .bind(role)
            .execute(&mut conn)
            .await
            .unwrap();
        assert_eq!(
            issue(&mut conn, input(7300)).await.unwrap(),
            IssueOutcome::Forbidden
        );
    }
    sqlx::raw_sql("DELETE FROM memberships")
        .execute(&mut conn)
        .await
        .unwrap();
    assert_eq!(
        issue(&mut conn, input(7300)).await.unwrap(),
        IssueOutcome::Forbidden
    );
}

#[tokio::test]
async fn concurrent_issuance_produces_one_pending_invitation() {
    let (dir, mut first) = fixture().await;
    let mut second = connect(&dir.path().join("app.db")).await.unwrap();
    let (a, b) = tokio::join!(
        issue(&mut first, input(100)),
        issue(&mut second, input(100))
    );
    let outcomes = [a.unwrap(), b.unwrap()];
    assert_eq!(
        outcomes
            .iter()
            .filter(|v| matches!(v, IssueOutcome::Issued { .. }))
            .count(),
        1
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|v| **v == IssueOutcome::InvitationPending)
            .count(),
        1
    );
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM invitations")
        .fetch_one(&mut first)
        .await
        .unwrap();
    assert_eq!(count, 1);
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM memberships WHERE user_id=29")
        .fetch_one(&mut first)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn owner_migration_preserves_existing_membership_and_constraints() {
    let dir = tempfile::tempdir().unwrap();
    let mut conn = connect(&dir.path().join("app.db")).await.unwrap();
    sqlx::raw_sql(include_str!("../../shared/migrations/0001_invitations.sql"))
        .execute(&mut conn)
        .await
        .unwrap();
    sqlx::raw_sql(
        "INSERT INTO users VALUES(11),(29); INSERT INTO projects VALUES(41);
        INSERT INTO memberships VALUES(41,11,'viewer'),(41,29,'editor')",
    )
    .execute(&mut conn)
    .await
    .unwrap();
    sqlx::raw_sql(include_str!("../../shared/migrations/0002_owners.sql"))
        .execute(&mut conn)
        .await
        .unwrap();
    let roles: Vec<String> = sqlx::query_scalar("SELECT role FROM memberships ORDER BY user_id")
        .fetch_all(&mut conn)
        .await
        .unwrap();
    assert_eq!(roles, ["viewer", "editor"]);
    sqlx::raw_sql("UPDATE memberships SET role='owner' WHERE user_id=11")
        .execute(&mut conn)
        .await
        .unwrap();
    for sql in [
        "UPDATE memberships SET role='administrator'",
        "INSERT INTO memberships VALUES(41,999,'owner')",
        "INSERT INTO memberships VALUES(41,11,'owner')",
    ] {
        assert!(sqlx::raw_sql(sql).execute(&mut conn).await.is_err());
    }
}
