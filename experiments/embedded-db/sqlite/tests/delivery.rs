use iris_sqlite_spike::{
    IssueInvitation, IssueOutcome, connect, issue_with_delivery, migrate,
    outbox::{Completion, claim, complete},
};
use sqlx::Row;

#[path = "../../../agent-interface/evidence.rs"]
mod evidence;

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
    // Upgrade a real pre-delivery database, preserving the original ledger.
    sqlx::migrate!("../shared/migrations")
        .run(&mut conn)
        .await
        .unwrap();
    sqlx::raw_sql("INSERT INTO users VALUES(11),(29); INSERT INTO projects VALUES(41); INSERT INTO memberships VALUES(41,11,'owner')").execute(&mut conn).await.unwrap();
    migrate(&mut conn).await.unwrap();
    migrate(&mut conn).await.unwrap();
    sqlx::query("INSERT INTO user_contacts VALUES(29,'bob@example.test')")
        .execute(&mut conn)
        .await
        .unwrap();
    (dir, conn)
}

#[tokio::test]
async fn enqueue_is_atomic_and_authorized() {
    let (_dir, mut conn) = fixture().await;
    sqlx::raw_sql("CREATE TRIGGER fail_enqueue BEFORE INSERT ON invitation_outbox BEGIN SELECT RAISE(ABORT,'injected'); END").execute(&mut conn).await.unwrap();
    assert!(issue_with_delivery(&mut conn, input(100)).await.is_err());
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM invitations")
            .fetch_one(&mut conn)
            .await
            .unwrap(),
        0
    );
    sqlx::query("DROP TRIGGER fail_enqueue")
        .execute(&mut conn)
        .await
        .unwrap();
    sqlx::query("DELETE FROM user_contacts")
        .execute(&mut conn)
        .await
        .unwrap();
    assert!(issue_with_delivery(&mut conn, input(100)).await.is_err());
    let mut forbidden = input(100);
    forbidden.actor_id = 29;
    assert_eq!(
        issue_with_delivery(&mut conn, forbidden).await.unwrap(),
        IssueOutcome::Forbidden
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM invitations")
            .fetch_one(&mut conn)
            .await
            .unwrap(),
        0
    );
    sqlx::query("INSERT INTO user_contacts VALUES(29,'bob@example.test')")
        .execute(&mut conn)
        .await
        .unwrap();
    issue_with_delivery(&mut conn, input(100)).await.unwrap();
    assert_eq!(
        issue_with_delivery(&mut conn, input(101)).await.unwrap(),
        IssueOutcome::InvitationPending
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM invitation_outbox")
            .fetch_one(&mut conn)
            .await
            .unwrap(),
        1
    );
}

#[tokio::test]
async fn snapshot_backoff_crash_recovery_and_stale_ack() {
    let (dir, mut conn) = fixture().await;
    let IssueOutcome::Issued { token, .. } =
        issue_with_delivery(&mut conn, input(100)).await.unwrap()
    else {
        panic!()
    };
    sqlx::query("UPDATE user_contacts SET email='changed@example.test'")
        .execute(&mut conn)
        .await
        .unwrap();
    let first = claim(&mut conn, 100).await.unwrap().unwrap();
    evidence::record("first_attempt", first.attempt);
    assert_eq!(first.token, token);
    assert_eq!(first.recipient, "bob@example.test");
    let mut other = connect(&dir.path().join("app.db")).await.unwrap();
    assert!(claim(&mut other, 129).await.unwrap().is_none());
    // Simulate SMTP success then process death without acknowledgement.
    let recovered = claim(&mut other, 130).await.unwrap().unwrap();
    evidence::record("recovered_attempt", recovered.attempt);
    assert_eq!(recovered.attempt, 2);
    assert_eq!(recovered.token, first.token);
    let stale_ack = complete(&mut conn, &first, Completion::Sent, 131)
        .await
        .unwrap();
    evidence::record("stale_ack_accepted", i64::from(stale_ack));
    assert!(!stale_ack);
    assert!(
        complete(&mut other, &recovered, Completion::Retry, 131)
            .await
            .unwrap()
    );
    assert!(claim(&mut conn, 140).await.unwrap().is_none());
    let third = claim(&mut conn, 141).await.unwrap().unwrap();
    evidence::record("retry_attempt", third.attempt);
    assert_eq!(third.attempt, 3);
    assert!(
        complete(&mut conn, &third, Completion::Sent, 142)
            .await
            .unwrap()
    );
    let row = sqlx::query("SELECT state,token,recipient FROM invitation_outbox")
        .fetch_one(&mut conn)
        .await
        .unwrap();
    assert_eq!(row.get::<String, _>("state"), "sent");
    evidence::record(
        "payload_cleared",
        i64::from(
            row.get::<Option<String>, _>("token").is_none()
                && row.get::<Option<String>, _>("recipient").is_none(),
        ),
    );
    assert!(row.get::<Option<String>, _>("token").is_none());
    assert!(row.get::<Option<String>, _>("recipient").is_none());
    assert!(claim(&mut conn, 200).await.unwrap().is_none());
}

#[tokio::test]
async fn expiry_acceptance_and_final_crash_clear_payloads() {
    for scenario in ["expired", "accepted", "exhausted", "permanent"] {
        let (_dir, mut conn) = fixture().await;
        let IssueOutcome::Issued { token, .. } =
            issue_with_delivery(&mut conn, input(100)).await.unwrap()
        else {
            panic!()
        };
        match scenario {
            "expired" => {
                assert!(claim(&mut conn, 3700).await.unwrap().is_none());
            }
            "accepted" => {
                iris_sqlite_spike::accept(
                    &mut conn,
                    iris_sqlite_spike::AcceptInvitation {
                        token: &token,
                        user_id: 29,
                        now: 101,
                    },
                )
                .await
                .unwrap();
                assert!(claim(&mut conn, 102).await.unwrap().is_none());
            }
            "permanent" => {
                let job = claim(&mut conn, 100).await.unwrap().unwrap();
                complete(&mut conn, &job, Completion::PermanentFailure, 101)
                    .await
                    .unwrap();
            }
            _ => {
                // Each worker crashes. The fifth expired lease must not stick forever.
                for attempt in 1..=5 {
                    assert_eq!(
                        claim(&mut conn, 100 + (attempt - 1) * 30)
                            .await
                            .unwrap()
                            .unwrap()
                            .attempt,
                        attempt
                    );
                }
                assert!(claim(&mut conn, 250).await.unwrap().is_none());
            }
        }
        let row = sqlx::query("SELECT state,token,recipient FROM invitation_outbox")
            .fetch_one(&mut conn)
            .await
            .unwrap();
        assert_eq!(row.get::<String, _>("state"), "dead", "{scenario}");
        assert!(row.get::<Option<String>, _>("token").is_none());
        assert!(row.get::<Option<String>, _>("recipient").is_none());
    }
}

#[tokio::test]
async fn concurrent_issuance_and_claim_have_one_winner() {
    let (dir, mut a) = fixture().await;
    let mut b = connect(&dir.path().join("app.db")).await.unwrap();
    let (x, y) = tokio::join!(
        issue_with_delivery(&mut a, input(100)),
        issue_with_delivery(&mut b, input(100))
    );
    let results = [x.unwrap(), y.unwrap()];
    assert_eq!(
        results
            .iter()
            .filter(|r| matches!(r, IssueOutcome::Issued { .. }))
            .count(),
        1
    );
    let (x, y) = tokio::join!(claim(&mut a, 100), claim(&mut b, 100));
    assert_eq!(
        [x.unwrap(), y.unwrap()]
            .iter()
            .filter(|r| r.is_some())
            .count(),
        1
    );
}
