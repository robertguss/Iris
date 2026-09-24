// Compiled unchanged against both concrete test fixtures. This is test reuse,
// not a runtime database abstraction. Expectations are independent of accept().
use super::{Fixture, Outcome, is_busy, is_constraint};

#[tokio::test]
async fn accepts_and_persists_after_reopen() {
    let mut f = Fixture::new().await;
    let mut c = f.connect().await;
    c.invite("secret-a", 7, 11, 100).await;
    assert_eq!(
        c.accept("secret-a", 11, 99).await.unwrap(),
        Outcome::Accepted { project_id: 7 }
    );
    c.close().await;
    f.reopen().await;
    let mut c = f.connect().await;
    assert_eq!(
        c.scalar(
            "SELECT COUNT(*) FROM memberships WHERE project_id=7 AND user_id=11 AND role='editor'"
        )
        .await,
        1
    );
    assert_eq!(
        c.scalar("SELECT COUNT(*) FROM invitations WHERE accepted_at=99 AND accepted_by=11")
            .await,
        1
    );
    assert_eq!(
        c.accept("secret-a", 11, 101).await.unwrap(),
        Outcome::AlreadyAccepted
    );
    assert_eq!(c.scalar("SELECT COUNT(*) FROM memberships").await, 1);
}

#[tokio::test]
async fn recipient_and_token_are_both_required() {
    let f = Fixture::new().await;
    let mut c = f.connect().await;
    c.invite("secret-a", 7, 11, 100).await;
    c.invite("secret-b", 19, 29, 100).await;
    assert_eq!(
        c.accept("secret-a", 29, 90).await.unwrap(),
        Outcome::NotFound
    );
    assert_eq!(
        c.accept("missing", 11, 90).await.unwrap(),
        Outcome::NotFound
    );
    assert_eq!(c.scalar("SELECT COUNT(*) FROM memberships").await, 0);
    assert_eq!(
        c.scalar("SELECT COUNT(*) FROM invitations WHERE accepted_at IS NOT NULL")
            .await,
        0
    );
    assert_eq!(
        c.accept("secret-b", 29, 90).await.unwrap(),
        Outcome::Accepted { project_id: 19 }
    );
    assert_eq!(
        c.scalar("SELECT COUNT(*) FROM memberships WHERE project_id=19 AND user_id=29")
            .await,
        1
    );
    assert_eq!(
        c.scalar("SELECT COUNT(*) FROM memberships WHERE project_id=7")
            .await,
        0
    );
    assert_eq!(
        c.accept("secret-b", 11, 90).await.unwrap(),
        Outcome::NotFound
    );
}

#[tokio::test]
async fn expiration_boundary() {
    // Distinct fresh databases: a successful earlier request cannot mask expiry.
    for (now, expected) in [
        (99, Outcome::Accepted { project_id: 7 }),
        (100, Outcome::Expired),
        (101, Outcome::Expired),
    ] {
        let f = Fixture::new().await;
        let mut c = f.connect().await;
        c.invite("secret-a", 7, 11, 100).await;
        assert_eq!(c.accept("secret-a", 11, now).await.unwrap(), expected);
        let count = if now == 99 { 1 } else { 0 };
        assert_eq!(c.scalar("SELECT COUNT(*) FROM memberships").await, count);
        assert_eq!(
            c.scalar("SELECT COUNT(*) FROM invitations WHERE accepted_at IS NOT NULL")
                .await,
            count
        );
    }
}

#[tokio::test]
async fn existing_role_is_preserved() {
    let f = Fixture::new().await;
    let mut c = f.connect().await;
    c.execute("INSERT INTO memberships VALUES (7, 11, 'viewer')")
        .await
        .unwrap();
    c.invite("secret-a", 7, 11, 100).await;
    assert_eq!(
        c.accept("secret-a", 11, 90).await.unwrap(),
        Outcome::Accepted { project_id: 7 }
    );
    assert_eq!(c.scalar("SELECT COUNT(*) FROM memberships").await, 1);
    assert_eq!(
        c.scalar("SELECT COUNT(*) FROM memberships WHERE role='viewer'")
            .await,
        1
    );
    assert_eq!(
        c.scalar("SELECT COUNT(*) FROM invitations WHERE accepted_at=90 AND accepted_by=11")
            .await,
        1
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn concurrent_acceptance() {
    let f = Fixture::new().await;
    let mut a = f.connect().await;
    let mut b = f.connect().await;
    a.invite("secret-a", 7, 11, 100).await;
    let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(2));
    let other = barrier.clone();
    let first = tokio::spawn(async move {
        barrier.wait().await;
        a.accept("secret-a", 11, 90).await
    });
    let second = tokio::spawn(async move {
        other.wait().await;
        b.accept("secret-a", 11, 90).await
    });
    let results = [
        first.await.unwrap().unwrap(),
        second.await.unwrap().unwrap(),
    ];
    assert_eq!(
        results
            .iter()
            .filter(|r| **r == Outcome::Accepted { project_id: 7 })
            .count(),
        1
    );
    assert_eq!(
        results
            .iter()
            .filter(|r| **r == Outcome::AlreadyAccepted)
            .count(),
        1
    );
    let mut c = f.connect().await;
    assert_eq!(
        c.scalar("SELECT COUNT(*) FROM memberships WHERE project_id=7 AND user_id=11")
            .await,
        1
    );
    assert_eq!(
        c.scalar("SELECT COUNT(*) FROM invitations WHERE accepted_at=90 AND accepted_by=11")
            .await,
        1
    );
}

#[tokio::test]
async fn rolls_back_claim_when_membership_write_fails() {
    let f = Fixture::new().await;
    let mut c = f.connect().await;
    c.invite("secret-a", 7, 11, 100).await;
    // Fault injection in the database, not an application-only failure flag.
    c.execute(
        "DROP TABLE memberships;
        CREATE TABLE memberships (
            project_id INTEGER NOT NULL REFERENCES projects(id),
            user_id INTEGER NOT NULL REFERENCES users(id),
            role TEXT NOT NULL CHECK (role='viewer'),
            PRIMARY KEY (project_id, user_id))",
    )
    .await
    .unwrap();
    let error = c.accept("secret-a", 11, 90).await.unwrap_err();
    assert!(
        is_constraint(&error),
        "expected constraint failure, got {error:?}"
    );
    let mut observer = f.connect().await;
    assert_eq!(observer.scalar("SELECT COUNT(*) FROM memberships").await, 0);
    assert_eq!(
        observer
            .scalar(
                "SELECT COUNT(*) FROM invitations WHERE accepted_at IS NULL AND accepted_by IS NULL"
            )
            .await,
        1
    );
    observer.close().await;
    c.execute(
        "DROP TABLE memberships;
        CREATE TABLE memberships (
            project_id INTEGER NOT NULL REFERENCES projects(id),
            user_id INTEGER NOT NULL REFERENCES users(id),
            role TEXT NOT NULL CHECK (role IN ('viewer', 'editor')),
            PRIMARY KEY (project_id, user_id))",
    )
    .await
    .unwrap();
    assert_eq!(
        c.accept("secret-a", 11, 91).await.unwrap(),
        Outcome::Accepted { project_id: 7 }
    );
    assert_eq!(
        c.scalar("SELECT COUNT(*) FROM invitations WHERE accepted_at=91")
            .await,
        1
    );
}

#[tokio::test]
async fn foreign_keys_on_every_connection() {
    let f = Fixture::new().await;
    for _ in 0..2 {
        let mut c = f.connect().await;
        assert_eq!(c.scalar("PRAGMA foreign_keys").await, 1);
        assert!(
            c.execute("INSERT INTO memberships VALUES (999, 11, 'viewer')")
                .await
                .is_err()
        );
        assert!(
            c.execute("INSERT INTO memberships VALUES (7, 999, 'viewer')")
                .await
                .is_err()
        );
        assert_eq!(c.scalar("SELECT COUNT(*) FROM memberships").await, 0);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn contention_is_bounded_and_connection_recovers() {
    let f = Fixture::new().await;
    let mut holder = f.connect().await;
    let mut contender = f.connect().await;
    holder.invite("secret-a", 7, 11, 100).await;
    holder.execute("BEGIN IMMEDIATE").await.unwrap();
    let start = std::time::Instant::now();
    let error = contender.accept("secret-a", 11, 90).await.unwrap_err();
    assert!(
        is_busy(&error),
        "expected busy classification, got {error:?}"
    );
    assert!(start.elapsed() < std::time::Duration::from_secs(2));
    holder.execute("ROLLBACK").await.unwrap();
    assert_eq!(
        contender.scalar("SELECT COUNT(*) FROM memberships").await,
        0
    );
    assert_eq!(
        contender.accept("secret-a", 11, 91).await.unwrap(),
        Outcome::Accepted { project_id: 7 }
    );
}

#[tokio::test]
async fn independent_databases_do_not_share_state() {
    let a = Fixture::new().await;
    let b = Fixture::new().await;
    let mut first = a.connect().await;
    let mut second = b.connect().await;
    first.invite("secret-a", 7, 11, 100).await;
    assert_eq!(
        second.accept("secret-a", 11, 90).await.unwrap(),
        Outcome::NotFound
    );
    assert_eq!(second.scalar("SELECT COUNT(*) FROM invitations").await, 0);
}

#[tokio::test]
async fn schema_rejects_partial_or_wrong_recipient_claims() {
    let f = Fixture::new().await;
    let mut c = f.connect().await;
    c.invite("secret-a", 7, 11, 100).await;
    for sql in [
        "UPDATE invitations SET accepted_at=90",
        "UPDATE invitations SET accepted_by=11",
        "UPDATE invitations SET accepted_at=90, accepted_by=29",
    ] {
        let error = c.execute(sql).await.unwrap_err();
        assert!(is_constraint(&error), "{error:?}");
    }
    assert_eq!(
        c.scalar(
            "SELECT COUNT(*) FROM invitations WHERE accepted_at IS NULL AND accepted_by IS NULL"
        )
        .await,
        1
    );
}
