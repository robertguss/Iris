use super::*;
use crate::app::connect;
use crate::domains::invitations::tests::{
    ALICE, BOB, NOW, PROJECT, accept_as, busy_at_begin, contact, database, exec, failed_at_commit,
    invite, issued, racing, sha256_hex,
};
use sqlx::Connection;

type Row = (
    i64,
    i64,
    Option<i64>,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
);

async fn row(conn: &mut SqliteConnection) -> Row {
    sqlx::query_as("SELECT claims, next_claim_at, lease_until, outcome, token, recipient_email, message_id FROM invitation_outbox ORDER BY id LIMIT 1")
        .fetch_one(&mut *conn)
        .await
        .unwrap()
}

async fn got(conn: &mut SqliteConnection, now: i64) -> Claim {
    claim(conn, now).await.unwrap().expect("a claim")
}

async fn finish(conn: &mut SqliteConnection, c: &Claim, r: Completion, now: i64) -> bool {
    complete(conn, c.outbox_id, c.attempt, r, now)
        .await
        .unwrap()
}

fn assert_terminal(r: &Row, outcome: &str) {
    assert_eq!(r.3.as_deref(), Some(outcome), "outcome");
    assert!(
        r.2.is_none() && r.4.is_none() && r.5.is_none(),
        "payload or lease kept"
    );
}

#[tokio::test]
async fn claim_returns_the_issued_payload_with_a_30_second_lease() {
    let (_dir, _path, mut conn) = database().await;
    let token = issued(&mut conn).await;
    let c = got(&mut conn, NOW).await;
    assert_eq!(
        (c.attempt, c.lease_until, c.project_id),
        (1, NOW + 30, PROJECT)
    );
    assert!(c.token == token, "claimed credential is not the issued one");
    let (hash, expires): (String, i64) =
        sqlx::query_as("SELECT token_hash, expires_at FROM invitations WHERE id=?")
            .bind(c.invitation_id)
            .fetch_one(&mut conn)
            .await
            .unwrap();
    assert!(
        sha256_hex(&c.token) == hash,
        "credential does not hash to the invitation's"
    );
    assert_eq!(c.expires_at, expires);
    let r = row(&mut conn).await;
    assert!(
        Some(&c.recipient_email) == r.5.as_ref(),
        "claimed address differs from the outbox row"
    );
    assert!(
        c.message_id == r.6,
        "claimed Message-ID differs from the outbox row"
    );
    assert!(
        c.recipient_email == "alice@example.test",
        "claimed address is not the issued one"
    );
}

#[tokio::test]
async fn a_live_lease_blocks_a_second_claim_and_an_expired_one_allows_it() {
    let (_dir, _path, mut conn) = database().await;
    issued(&mut conn).await;
    let first = got(&mut conn, NOW).await;
    assert!(claim(&mut conn, NOW + 29).await.unwrap().is_none());
    let second = got(&mut conn, NOW + 30).await;
    assert_eq!(second.attempt, 2);
    assert!(
        (&first.token, &first.recipient_email, &first.message_id)
            == (&second.token, &second.recipient_email, &second.message_id),
        "a later claim changed the credential, address or Message-ID"
    );
    let expires: i64 = sqlx::query_scalar("SELECT expires_at FROM invitations")
        .fetch_one(&mut conn)
        .await
        .unwrap();
    assert_eq!((expires, second.expires_at), (NOW + 3600, NOW + 3600));
}

#[tokio::test]
async fn five_claims_spend_the_budget_even_without_completion() {
    let (_dir, _path, mut conn) = database().await;
    issued(&mut conn).await;
    let mut at = NOW;
    for attempt in 1..=5 {
        assert_eq!(got(&mut conn, at).await.attempt, attempt);
        at += 30;
        if attempt == 4 {
            // A crashed claim 4 with one claim left is not finished.
            assert_eq!(sweep(&mut conn, at).await.unwrap(), 0);
            assert!(
                row(&mut conn).await.4.is_some(),
                "sweep cleared a job with budget left"
            );
        }
    }
    // `at` is now the fifth lease's end.
    assert!(claim(&mut conn, at).await.unwrap().is_none());
    assert!(claim(&mut conn, at + 1).await.unwrap().is_none());
    let r = row(&mut conn).await;
    assert_eq!(r.0, 5);
    assert!(r.3.is_none() && r.4.is_some() && r.5.is_some());
    assert_eq!(sweep(&mut conn, at - 1).await.unwrap(), 0);
    assert!(row(&mut conn).await.4.is_some());
    assert_eq!(sweep(&mut conn, at).await.unwrap(), 1);
    assert_terminal(&row(&mut conn).await, "exhausted");
}

#[tokio::test]
async fn retryable_completions_back_off_5_10_20_40_seconds() {
    let (_dir, _path, mut conn) = database().await;
    issued(&mut conn).await;
    let mut now = NOW;
    for (i, backoff) in [5, 10, 20, 40].into_iter().enumerate() {
        let c = got(&mut conn, now).await;
        assert_eq!(c.attempt, i as i64 + 1);
        now += 1;
        assert!(finish(&mut conn, &c, Completion::Retryable, now).await);
        assert!(claim(&mut conn, now + backoff - 1).await.unwrap().is_none());
        // Waiting out a backoff is not a reason to finish the job.
        assert_eq!(sweep(&mut conn, now + backoff - 1).await.unwrap(), 0);
        assert!(
            row(&mut conn).await.4.is_some(),
            "sweep cleared a job waiting to retry"
        );
        now += backoff;
    }
    let c = got(&mut conn, now).await;
    assert_eq!(c.attempt, 5);
    assert!(finish(&mut conn, &c, Completion::Retryable, now + 1).await);
    assert_terminal(&row(&mut conn).await, "exhausted");
}

#[tokio::test]
async fn sent_and_permanent_completions_are_terminal_and_clear_the_payload() {
    for (result, outcome) in [
        (Completion::Sent, "sent"),
        (Completion::Permanent, "permanent"),
    ] {
        let (_dir, _path, mut conn) = database().await;
        issued(&mut conn).await;
        let before = row(&mut conn).await.6;
        let c = got(&mut conn, NOW).await;
        assert!(finish(&mut conn, &c, result, NOW + 1).await);
        let r = row(&mut conn).await;
        assert_terminal(&r, outcome);
        assert!(r.6 == before, "Message-ID changed");
        assert!(claim(&mut conn, NOW + 1000).await.unwrap().is_none());
        assert_terminal(&row(&mut conn).await, outcome);
    }
}

#[tokio::test]
async fn a_stale_completion_returns_false_and_changes_nothing() {
    let (_dir, _path, mut conn) = database().await;
    issued(&mut conn).await;
    let first = got(&mut conn, NOW).await;
    let before = row(&mut conn).await;
    assert!(
        !finish(&mut conn, &first, Completion::Sent, NOW + 30).await,
        "half (a): completion at the lease's end must be refused"
    );
    assert!(row(&mut conn).await == before, "half (a): row changed");

    let second = got(&mut conn, NOW + 30).await;
    let before = row(&mut conn).await;
    assert!(
        !finish(&mut conn, &first, Completion::Permanent, NOW + 31).await,
        "half (b): a replaced attempt's completion must be refused"
    );
    assert!(row(&mut conn).await == before, "half (b): row changed");
    assert_eq!(before.2, Some(NOW + 60));
    assert!(before.4.is_some());
    assert!(finish(&mut conn, &second, Completion::Sent, NOW + 31).await);
}

#[tokio::test]
async fn database_errors_are_errors_not_false_none_or_zero() {
    for which in ["claim", "complete", "sweep"] {
        let (_dir, path, mut conn) = database().await;
        issued(&mut conn).await;
        // The live claim `complete` needs, taken before any trigger exists.
        let live = if which == "complete" {
            let c = got(&mut conn, NOW).await;
            Some((c.outbox_id, c.attempt))
        } else {
            None
        };
        // Sweep's row is sweepable once the invitation has expired.
        let now = if which == "sweep" {
            NOW + 3600
        } else {
            NOW + 1
        };
        macro_rules! call {
            ($c:expr) => {
                match which {
                    "claim" => claim($c, now).await.map(|c| c.is_some()),
                    "complete" => {
                        let (id, attempt) = live.unwrap();
                        complete($c, id, attempt, Completion::Sent, now).await
                    }
                    _ => sweep($c, now).await.map(|n| n > 0),
                }
            };
        }
        let before = row(&mut conn).await;

        let mut locked = connect(&path).await.unwrap();
        let tx = locked.begin_with("BEGIN IMMEDIATE").await.unwrap();
        let mut busy = connect(&path).await.unwrap();
        assert_eq!(call!(&mut busy).unwrap_err(), busy_at_begin(), "{which}");
        tx.rollback().await.unwrap();
        assert!(row(&mut conn).await == before, "{which}: row changed");

        exec(
            &mut conn,
            "CREATE TABLE deferred_fault (user_id INTEGER REFERENCES users(id) DEFERRABLE INITIALLY DEFERRED);
             CREATE TRIGGER delivery_fault AFTER UPDATE ON invitation_outbox BEGIN INSERT INTO deferred_fault VALUES (9999); END;",
        )
        .await;
        let mut failing = connect(&path).await.unwrap();
        assert_eq!(
            call!(&mut failing).unwrap_err(),
            failed_at_commit(),
            "{which}"
        );
        failing.close().await.unwrap();
        let mut fresh = connect(&path).await.unwrap();
        assert!(row(&mut fresh).await == before, "{which}: row changed");
    }
}

#[tokio::test]
async fn expired_or_accepted_invitations_are_not_claimed_and_are_swept() {
    let (_dir, _path, mut conn) = database().await;
    issued(&mut conn).await;
    assert!(claim(&mut conn, NOW + 3600).await.unwrap().is_none());
    assert_eq!(sweep(&mut conn, NOW + 3600).await.unwrap(), 1);
    assert_terminal(&row(&mut conn).await, "expired");

    let (_dir, _path, mut conn) = database().await;
    let token = issued(&mut conn).await;
    accept_as(&mut conn, &ALICE, &token, NOW + 1).await.unwrap();
    assert!(claim(&mut conn, NOW + 2).await.unwrap().is_none());
    assert_eq!(sweep(&mut conn, NOW + 2).await.unwrap(), 1);
    assert_terminal(&row(&mut conn).await, "accepted");
}

#[tokio::test]
async fn expiry_or_acceptance_during_a_live_lease_does_not_stop_the_send() {
    let (_dir, _path, mut conn) = database().await;
    let token = issued(&mut conn).await;
    let c = got(&mut conn, NOW).await;
    accept_as(&mut conn, &ALICE, &token, NOW + 5).await.unwrap();
    assert_eq!(sweep(&mut conn, NOW + 10).await.unwrap(), 0);
    assert!(row(&mut conn).await.4.is_some());
    assert!(finish(&mut conn, &c, Completion::Sent, NOW + 20).await);
    assert_terminal(&row(&mut conn).await, "sent");

    let (_dir, _path, mut conn) = database().await;
    issued(&mut conn).await;
    let c = got(&mut conn, NOW + 3590).await;
    assert_eq!(sweep(&mut conn, NOW + 3600).await.unwrap(), 0);
    assert!(row(&mut conn).await.4.is_some());
    assert!(finish(&mut conn, &c, Completion::Sent, NOW + 3610).await);
    assert_terminal(&row(&mut conn).await, "sent");
}

#[tokio::test]
async fn an_issuer_losing_ownership_does_not_suppress_delivery() {
    let (_dir, _path, mut conn) = database().await;
    issued(&mut conn).await;
    exec(
        &mut conn,
        "UPDATE memberships SET role='viewer' WHERE project_id=43 AND user_id=29",
    )
    .await;
    assert_eq!(got(&mut conn, NOW).await.attempt, 1);
}

#[tokio::test]
async fn malformed_payloads_become_terminal_without_being_claimed() {
    for malformed in ["token=NULL", "token=''", "recipient_email=NULL"] {
        let (_dir, _path, mut conn) = database().await;
        contact(&mut conn, 11, "alice@example.test").await;
        invite(&mut conn, &BOB, PROJECT, 11, NOW).await.unwrap();
        // The first row is malformed; a second, good row sorts after it.
        exec(
            &mut conn,
            &format!("UPDATE invitation_outbox SET {malformed}"),
        )
        .await;
        exec(
            &mut conn,
            "INSERT INTO invitations (project_id, recipient_id, issuer_id, role, token_hash, created_at, expires_at) VALUES (43, 29, 29, 'editor', 'h2', 1000000, 1003600);
             INSERT INTO invitation_outbox (invitation_id, recipient_email, token, message_id, created_at) VALUES (last_insert_rowid(), 'bob@example.test', 'second', '<m2@reference.iris.test>', 1000000);",
        )
        .await;
        let c = got(&mut conn, NOW).await;
        assert!(
            c.token == "second",
            "{malformed}: the malformed row was claimed"
        );
        let all: Vec<(Option<String>, Option<String>)> =
            sqlx::query_as("SELECT outcome, token FROM invitation_outbox ORDER BY id")
                .fetch_all(&mut conn)
                .await
                .unwrap();
        assert!(
            all[0] == (Some("malformed".to_owned()), None),
            "{malformed}: not terminal malformed and cleared"
        );
        assert!(all[1].0.is_none(), "{malformed}: the good row was finished");
    }

    let (_dir, _path, mut conn) = database().await;
    issued(&mut conn).await;
    exec(
        &mut conn,
        "UPDATE invitation_outbox SET recipient_email='nobody@example.com'",
    )
    .await;
    assert!(claim(&mut conn, NOW).await.unwrap().is_none());
    assert_terminal(&row(&mut conn).await, "malformed");
    // A terminal row stays as it is.
    assert!(claim(&mut conn, NOW + 1).await.unwrap().is_none());
    assert_terminal(&row(&mut conn).await, "malformed");
}

#[tokio::test]
async fn sweep_leaves_finished_rows_and_orders_its_reasons() {
    // A sent row is not rewritten when its invitation is later accepted or has
    // expired, and is not counted.
    let (_dir, _path, mut conn) = database().await;
    let token = issued(&mut conn).await;
    let c = got(&mut conn, NOW).await;
    assert!(finish(&mut conn, &c, Completion::Sent, NOW + 1).await);
    accept_as(&mut conn, &ALICE, &token, NOW + 2).await.unwrap();
    assert_eq!(sweep(&mut conn, NOW + 3).await.unwrap(), 0);
    assert_eq!(sweep(&mut conn, NOW + 4000).await.unwrap(), 0);
    assert_terminal(&row(&mut conn).await, "sent");

    // Accepted and expired: accepted wins.
    let (_dir, _path, mut conn) = database().await;
    let token = issued(&mut conn).await;
    accept_as(&mut conn, &ALICE, &token, NOW + 1).await.unwrap();
    assert_eq!(sweep(&mut conn, NOW + 3600).await.unwrap(), 1);
    assert_terminal(&row(&mut conn).await, "accepted");

    // Five claims and expired: expired wins over exhausted.
    let (_dir, _path, mut conn) = database().await;
    issued(&mut conn).await;
    for i in 0..5 {
        got(&mut conn, NOW + 30 * i).await;
    }
    assert_eq!(sweep(&mut conn, NOW + 3600).await.unwrap(), 1);
    assert_terminal(&row(&mut conn).await, "expired");
}

#[tokio::test]
async fn concurrent_claims_take_a_job_once() {
    let (_dir, path, mut conn) = database().await;
    issued(&mut conn).await;
    let (mut first, mut second) = racing(&path).await;
    let barrier = tokio::sync::Barrier::new(2);
    let (a, b) = tokio::join!(
        async {
            barrier.wait().await;
            claim(&mut first, NOW).await
        },
        async {
            barrier.wait().await;
            claim(&mut second, NOW).await
        }
    );
    let (a, b) = (a.unwrap(), b.unwrap());
    assert_eq!(usize::from(a.is_some()) + usize::from(b.is_some()), 1);
    assert_eq!(row(&mut conn).await.0, 1);
}

#[tokio::test]
async fn delivery_errors_and_claims_never_print_the_credential_or_address() {
    let (_dir, path, mut conn) = database().await;
    let token = issued(&mut conn).await;
    let c = got(&mut conn, NOW + 100).await;
    let mut shown = vec![format!("{c:?}")];
    let mut locked = connect(&path).await.unwrap();
    let tx = locked.begin_with("BEGIN IMMEDIATE").await.unwrap();
    let mut busy = connect(&path).await.unwrap();
    shown.push(format!("{:?}", claim(&mut busy, NOW).await.unwrap_err()));
    shown.push(format!("{:?}", sweep(&mut busy, NOW).await.unwrap_err()));
    shown.push(format!(
        "{:?}",
        complete(&mut busy, 1, 1, Completion::Sent, NOW)
            .await
            .unwrap_err()
    ));
    tx.rollback().await.unwrap();
    for text in &shown {
        assert!(!text.contains(&token), "debug output shows the credential");
        assert!(
            !text.contains(&sha256_hex(&token)),
            "debug output shows the hash"
        );
        assert!(
            !text.contains("alice@example"),
            "debug output shows the address"
        );
        assert!(
            !text.contains(&c.message_id),
            "debug output shows the Message-ID"
        );
    }
    assert!(shown[0].contains("outbox_id"));
}

/// A database that has only 0001 and 0002 and holds a stage-1 outbox row. The
/// storage guard is held across awaits, as in the storage tests.
#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn a_stage_one_outbox_row_survives_migration_and_is_claimable() {
    use crate::domains::invitations::{IssueInvitation, issue};
    const ISSUER: &str = "http://127.0.0.1:4001";
    let _processes = crate::storage::shared();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().canonicalize().unwrap().join("dev.db");
    let files = tempfile::tempdir().unwrap();
    for (name, sql) in [
        (
            "0001_initial.sql",
            include_str!("../../../../migrations/0001_initial.sql"),
        ),
        (
            "0002_invitations.sql",
            include_str!("../../../../migrations/0002_invitations.sql"),
        ),
    ] {
        std::fs::write(files.path().join(name), sql).unwrap();
    }
    let old = sqlx::migrate::Migrator::new(files.path()).await.unwrap();
    let mut conn = connect(&path).await.unwrap();
    old.run(&mut conn).await.unwrap();
    crate::app::seed(&mut conn, ISSUER).await.unwrap();
    contact(&mut conn, 11, "alice@example.test").await;
    issue(
        &mut conn,
        &BOB,
        IssueInvitation {
            project_id: PROJECT,
            recipient_id: 11,
        },
        NOW,
    )
    .await
    .unwrap();
    conn.close().await.unwrap();

    let _storage = crate::storage::Storage::open(&path, ISSUER).await.unwrap();
    let mut conn = connect(&path).await.unwrap();
    let r = row(&mut conn).await;
    assert_eq!((r.0, r.1, r.2, r.3), (0, 0, None, None));
    assert!(r.4.is_some() && r.5.is_some());
    assert_eq!(got(&mut conn, NOW).await.attempt, 1);
}
