use super::*;
use crate::app::connect;
use sha2::{Digest, Sha256};
use sqlx::{AssertSqlSafe, Connection};
use std::path::{Path, PathBuf};

pub(super) const NOW: i64 = 1_000_000;
pub(super) const BOB: Actor = Actor(29);
pub(super) const ALICE: Actor = Actor(11);
/// Bob owns project 43 and Alice is not a member: S19's happy case.
pub(super) const PROJECT: i64 = 43;

pub(super) async fn database() -> (tempfile::TempDir, PathBuf, SqliteConnection) {
    let (dir, path) = crate::read::tests::database().await;
    let conn = connect(&path).await.unwrap();
    (dir, path, conn)
}

pub(super) async fn contact(conn: &mut SqliteConnection, user_id: i64, email: &str) {
    sqlx::query("INSERT OR REPLACE INTO user_contacts (user_id, email) VALUES (?, ?)")
        .bind(user_id)
        .bind(email)
        .execute(&mut *conn)
        .await
        .unwrap();
}

pub(super) async fn exec(conn: &mut SqliteConnection, sql: &str) {
    // Statements are literals in this file.
    sqlx::raw_sql(AssertSqlSafe(sql.to_owned()))
        .execute(&mut *conn)
        .await
        .unwrap();
}

async fn count(conn: &mut SqliteConnection, table: &str) -> i64 {
    // Table names are literals in this file.
    sqlx::query_scalar(AssertSqlSafe(format!("SELECT count(*) FROM {table}")))
        .fetch_one(&mut *conn)
        .await
        .unwrap()
}

async fn role(conn: &mut SqliteConnection, project_id: i64, user_id: i64) -> Option<String> {
    sqlx::query_scalar("SELECT role FROM memberships WHERE project_id=? AND user_id=?")
        .bind(project_id)
        .bind(user_id)
        .fetch_optional(&mut *conn)
        .await
        .unwrap()
}

pub(super) async fn invite(
    conn: &mut SqliteConnection,
    actor: &Actor,
    project_id: i64,
    recipient_id: i64,
    now: i64,
) -> Result<Issued, ActionError<IssueRejection>> {
    issue(
        conn,
        actor,
        IssueInvitation {
            project_id,
            recipient_id,
        },
        now,
    )
    .await
}

pub(super) async fn accept_as(
    conn: &mut SqliteConnection,
    actor: &Actor,
    token: &str,
    now: i64,
) -> Result<Accepted, ActionError<AcceptRejection>> {
    accept(
        conn,
        actor,
        AcceptInvitation {
            token: token.to_owned(),
        },
        now,
    )
    .await
}

/// The newest outbox entry's plaintext credential, as delivery would read it.
async fn latest_token(conn: &mut SqliteConnection) -> String {
    sqlx::query_scalar("SELECT token FROM invitation_outbox ORDER BY id DESC LIMIT 1")
        .fetch_one(&mut *conn)
        .await
        .unwrap()
}

pub(super) fn sha256_hex(token: &str) -> String {
    Sha256::digest(token.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Alice has a usable contact and Bob has issued her an invitation to 43.
pub(super) async fn issued(conn: &mut SqliteConnection) -> String {
    contact(conn, 11, "alice@example.test").await;
    invite(conn, &BOB, PROJECT, 11, NOW).await.unwrap();
    latest_token(conn).await
}

fn rejected<T: std::fmt::Debug, R: std::fmt::Debug + PartialEq>(
    result: Result<T, ActionError<R>>,
    expected: R,
) {
    match result {
        Err(ActionError::Rejected(r)) => assert_eq!(r, expected),
        other => panic!("expected rejection {expected:?}, got {other:?}"),
    }
}

#[tokio::test]
async fn bob_invites_alice_who_accepts_as_editor() {
    let (_dir, _path, mut conn) = database().await;
    contact(&mut conn, 11, "alice@example.test").await;
    let memberships = count(&mut conn, "memberships").await;

    let issued = invite(&mut conn, &BOB, PROJECT, 11, NOW).await.unwrap();
    assert_eq!(
        issued,
        Issued {
            project_id: PROJECT,
            recipient_id: 11,
            expires_at: NOW + LIFETIME_SECONDS,
        }
    );
    // Issuance changes no membership.
    assert_eq!(count(&mut conn, "memberships").await, memberships);
    assert_eq!(role(&mut conn, PROJECT, 11).await, None);

    let (invitation_id, hash, issuer, invited_role, created, expires, accepted): (
        i64,
        String,
        i64,
        String,
        i64,
        i64,
        Option<i64>,
    ) = sqlx::query_as(
        "SELECT id, token_hash, issuer_id, role, created_at, expires_at, accepted_at FROM invitations",
    )
    .fetch_one(&mut conn)
    .await
    .unwrap();
    let (outbox_invitation, email, token, message_id, outbox_created): (
        i64,
        String,
        String,
        String,
        i64,
    ) = sqlx::query_as(
        "SELECT invitation_id, recipient_email, token, message_id, created_at FROM invitation_outbox",
    )
    .fetch_one(&mut conn)
    .await
    .unwrap();
    assert_eq!(outbox_invitation, invitation_id);
    assert!(
        email == "alice@example.test",
        "the outbox address is not the stored contact"
    );
    assert_eq!(token.len(), 64);
    assert!(
        token
            .chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
    );
    // The invitation holds the hash, never the credential.
    assert!(
        hash == sha256_hex(&token),
        "the invitation hash does not match the outbox credential"
    );
    assert!(hash != token, "the invitation stores the credential itself");
    assert_eq!((issuer, invited_role.as_str()), (29, "editor"));
    assert_eq!((created, expires, accepted), (NOW, NOW + 3600, None));
    assert_eq!(outbox_created, NOW);
    assert!(message_id.starts_with('<') && message_id.ends_with("@reference.iris.test>"));
    assert_eq!(message_id.len(), "<@reference.iris.test>".len() + 32);

    let accepted = accept_as(&mut conn, &ALICE, &token, NOW + 10)
        .await
        .unwrap();
    assert_eq!(
        accepted,
        Accepted {
            project_id: PROJECT
        }
    );
    assert_eq!(
        role(&mut conn, PROJECT, 11).await.as_deref(),
        Some("editor")
    );
    let accepted_at: Option<i64> = sqlx::query_scalar("SELECT accepted_at FROM invitations")
        .fetch_one(&mut conn)
        .await
        .unwrap();
    assert_eq!(accepted_at, Some(NOW + 10));
}

/// Each case also fails every later check, so only the order explains the
/// code returned; no rejection writes anything.
#[tokio::test]
async fn issue_checks_run_in_order_and_write_nothing_when_refused() {
    let (_dir, _path, mut conn) = database().await;
    // Alice owns 41; Bob owns 43. User 77 does not exist.
    let cases: Vec<(Actor, i64, i64, IssueRejection)> = vec![
        // Unknown project, unknown recipient.
        (BOB, 999, 77, IssueRejection::Forbidden),
        // Non-owner of an existing project, unknown recipient.
        (ALICE, PROJECT, 77, IssueRejection::Forbidden),
        // Unknown recipient (who also has no contact).
        (BOB, PROJECT, 77, IssueRejection::RecipientNotFound),
        // Bob is a member of his own project and has no contact.
        (BOB, PROJECT, 29, IssueRejection::AlreadyMember),
    ];
    for (actor, project_id, recipient_id, expected) in cases {
        rejected(
            invite(&mut conn, &actor, project_id, recipient_id, NOW).await,
            expected,
        );
    }
    assert_eq!(count(&mut conn, "invitations").await, 0);

    // A member with a pending invitation and no contact: membership wins.
    contact(&mut conn, 11, "alice@example.test").await;
    invite(&mut conn, &BOB, PROJECT, 11, NOW).await.unwrap();
    exec(&mut conn, "DELETE FROM user_contacts WHERE user_id=11").await;
    exec(
        &mut conn,
        "INSERT INTO memberships (project_id, user_id, role) VALUES (43, 11, 'viewer')",
    )
    .await;
    rejected(
        invite(&mut conn, &BOB, PROJECT, 11, NOW + 1).await,
        IssueRejection::AlreadyMember,
    );
    // A pending invitation and no contact: pending wins.
    exec(
        &mut conn,
        "DELETE FROM memberships WHERE project_id=43 AND user_id=11",
    )
    .await;
    rejected(
        invite(&mut conn, &BOB, PROJECT, 11, NOW + 1).await,
        IssueRejection::InvitationPending,
    );
    assert_eq!(count(&mut conn, "invitations").await, 1);
    assert_eq!(count(&mut conn, "invitation_outbox").await, 1);
}

#[tokio::test]
async fn only_a_usable_local_test_contact_permits_issuance() {
    let (_dir, _path, mut conn) = database().await;
    rejected(
        invite(&mut conn, &BOB, PROJECT, 11, NOW).await,
        IssueRejection::RecipientUnavailable,
    );
    for unusable in [
        "alice@example.com",
        "a@b@x.test",
        "@x.test",
        "a b@x.test",
        "alice@example.test ",
        "a@x.test.com",
        "a@x.tests",
        "a@.test",
        "",
    ] {
        contact(&mut conn, 11, unusable).await;
        rejected(
            invite(&mut conn, &BOB, PROJECT, 11, NOW).await,
            IssueRejection::RecipientUnavailable,
        );
    }
    assert_eq!(count(&mut conn, "invitations").await, 0);
    assert_eq!(count(&mut conn, "invitation_outbox").await, 0);

    // Case does not matter, and the snapshot keeps the stored bytes.
    contact(&mut conn, 11, "ALICE@Example.TEST").await;
    invite(&mut conn, &BOB, PROJECT, 11, NOW).await.unwrap();
    let email: String = sqlx::query_scalar("SELECT recipient_email FROM invitation_outbox")
        .fetch_one(&mut conn)
        .await
        .unwrap();
    assert!(
        email == "ALICE@Example.TEST",
        "the outbox did not keep the stored address byte for byte"
    );
}

#[tokio::test]
async fn an_expired_invitation_does_not_block_a_new_one() {
    let (_dir, _path, mut conn) = database().await;
    let first = issued(&mut conn).await;
    // At expiry equality the first is no longer pending.
    invite(&mut conn, &BOB, PROJECT, 11, NOW + LIFETIME_SECONDS)
        .await
        .unwrap();
    let second = latest_token(&mut conn).await;
    assert!(first != second, "a fresh issuance reused the credential");
    assert_eq!(count(&mut conn, "invitations").await, 2);
    // One second earlier, the first would still have been pending.
    rejected(
        invite(&mut conn, &BOB, PROJECT, 11, NOW + LIFETIME_SECONDS + 1).await,
        IssueRejection::InvitationPending,
    );
}

/// An accepted invitation is not pending, even before it expires: after the
/// recipient is removed, a fresh issuance succeeds.
#[tokio::test]
async fn an_accepted_invitation_does_not_block_reinvitation_after_removal() {
    let (_dir, _path, mut conn) = database().await;
    let first = issued(&mut conn).await;
    accept_as(&mut conn, &ALICE, &first, NOW + 1).await.unwrap();
    exec(
        &mut conn,
        "DELETE FROM memberships WHERE project_id=43 AND user_id=11",
    )
    .await;
    invite(&mut conn, &BOB, PROJECT, 11, NOW + 2).await.unwrap();
    let second = latest_token(&mut conn).await;
    assert!(first != second, "a fresh issuance reused the credential");
    assert_eq!(count(&mut conn, "invitations").await, 2);
    assert_eq!(count(&mut conn, "invitation_outbox").await, 2);
}

#[tokio::test]
async fn expiry_equality_is_expired() {
    let (_dir, _path, mut conn) = database().await;
    let token = issued(&mut conn).await;
    let expires = NOW + LIFETIME_SECONDS;
    rejected(
        accept_as(&mut conn, &ALICE, &token, expires).await,
        AcceptRejection::Expired,
    );
    rejected(
        accept_as(&mut conn, &ALICE, &token, expires + 100).await,
        AcceptRejection::Expired,
    );
    assert_eq!(role(&mut conn, PROJECT, 11).await, None);
    accept_as(&mut conn, &ALICE, &token, expires - 1)
        .await
        .unwrap();
    assert_eq!(
        role(&mut conn, PROJECT, 11).await.as_deref(),
        Some("editor")
    );
}

#[tokio::test]
async fn accepted_wins_over_expired_with_no_second_effect() {
    let (_dir, _path, mut conn) = database().await;
    let token = issued(&mut conn).await;
    accept_as(&mut conn, &ALICE, &token, NOW + 1).await.unwrap();
    exec(
        &mut conn,
        "UPDATE memberships SET role='viewer' WHERE project_id=43 AND user_id=11",
    )
    .await;
    rejected(
        accept_as(&mut conn, &ALICE, &token, NOW + LIFETIME_SECONDS + 50).await,
        AcceptRejection::AlreadyAccepted,
    );
    assert_eq!(
        role(&mut conn, PROJECT, 11).await.as_deref(),
        Some("viewer")
    );
    let accepted_at: Option<i64> = sqlx::query_scalar("SELECT accepted_at FROM invitations")
        .fetch_one(&mut conn)
        .await
        .unwrap();
    assert_eq!(accepted_at, Some(NOW + 1));
}

/// Unknown credentials and the wrong recipient are not found, whatever the
/// invitation's state: no acceptance or expiry is disclosed.
#[tokio::test]
async fn unknown_credentials_and_wrong_recipients_are_not_found_in_every_state() {
    let (_dir, _path, mut conn) = database().await;
    let token = issued(&mut conn).await;
    let unknown = "0".repeat(64);
    let probes = |token: &str| {
        vec![
            (BOB, token.to_owned()),
            (ALICE, unknown.clone()),
            (ALICE, String::new()),
            (ALICE, "not-hex".to_owned()),
            (ALICE, token.to_uppercase()),
        ]
    };
    // Pending, then expired, then accepted.
    for now in [NOW + 1, NOW + LIFETIME_SECONDS + 1] {
        for (actor, candidate) in probes(&token) {
            rejected(
                accept_as(&mut conn, &actor, &candidate, now).await,
                AcceptRejection::NotFound,
            );
        }
    }
    accept_as(&mut conn, &ALICE, &token, NOW + 2).await.unwrap();
    for now in [NOW + 3, NOW + LIFETIME_SECONDS + 1] {
        for (actor, candidate) in probes(&token) {
            rejected(
                accept_as(&mut conn, &actor, &candidate, now).await,
                AcceptRejection::NotFound,
            );
        }
    }
    assert_eq!(role(&mut conn, PROJECT, 29).await.as_deref(), Some("owner"));
}

#[tokio::test]
async fn acceptance_keeps_an_existing_role() {
    for existing in ["viewer", "owner"] {
        let (_dir, _path, mut conn) = database().await;
        let token = issued(&mut conn).await;
        sqlx::query("INSERT INTO memberships (project_id, user_id, role) VALUES (43, 11, ?)")
            .bind(existing)
            .execute(&mut conn)
            .await
            .unwrap();
        accept_as(&mut conn, &ALICE, &token, NOW + 1).await.unwrap();
        assert_eq!(
            role(&mut conn, PROJECT, 11).await.as_deref(),
            Some(existing)
        );
        let accepted_at: Option<i64> = sqlx::query_scalar("SELECT accepted_at FROM invitations")
            .fetch_one(&mut conn)
            .await
            .unwrap();
        assert_eq!(accepted_at, Some(NOW + 1), "{existing}");
    }
}

#[tokio::test]
async fn a_consumed_invitation_cannot_restore_a_removed_membership() {
    let (_dir, _path, mut conn) = database().await;
    let token = issued(&mut conn).await;
    accept_as(&mut conn, &ALICE, &token, NOW + 1).await.unwrap();
    exec(
        &mut conn,
        "DELETE FROM memberships WHERE project_id=43 AND user_id=11",
    )
    .await;
    rejected(
        accept_as(&mut conn, &ALICE, &token, NOW + 2).await,
        AcceptRejection::AlreadyAccepted,
    );
    assert_eq!(role(&mut conn, PROJECT, 11).await, None);
}

#[tokio::test]
async fn an_invitation_survives_its_issuer_losing_ownership() {
    let (_dir, _path, mut conn) = database().await;
    let token = issued(&mut conn).await;
    // Another owner first, so the project keeps one.
    exec(
        &mut conn,
        "INSERT INTO users (id, display_name) VALUES (30, 'Carol Example');
         INSERT INTO memberships (project_id, user_id, role) VALUES (43, 30, 'owner');
         UPDATE memberships SET role='viewer' WHERE project_id=43 AND user_id=29",
    )
    .await;
    accept_as(&mut conn, &ALICE, &token, NOW + 1).await.unwrap();
    assert_eq!(
        role(&mut conn, PROJECT, 11).await.as_deref(),
        Some("editor")
    );
}

pub(super) fn busy_at_begin<R>() -> ActionError<R> {
    ActionError::Failed {
        primary: StopReason::Execution {
            stage: Stage::Begin,
            kind: FailureKind::Busy,
        },
        cleanup: Cleanup::Unconfirmed {
            rollback_error: None,
        },
    }
}

pub(super) fn failed_at_commit<R>() -> ActionError<R> {
    ActionError::Failed {
        primary: StopReason::Execution {
            stage: Stage::Commit,
            kind: FailureKind::Other,
        },
        cleanup: Cleanup::Unconfirmed {
            rollback_error: None,
        },
    }
}

pub(super) async fn racing(path: &Path) -> (SqliteConnection, SqliteConnection) {
    let mut first = connect(path).await.unwrap();
    let mut second = connect(path).await.unwrap();
    for conn in [&mut first, &mut second] {
        exec(conn, "PRAGMA busy_timeout=5000").await;
    }
    (first, second)
}

#[tokio::test]
async fn concurrent_issues_leave_one_invitation() {
    let (_dir, path, mut conn) = database().await;
    contact(&mut conn, 11, "alice@example.test").await;
    let (mut first, mut second) = racing(&path).await;
    let barrier = tokio::sync::Barrier::new(2);
    let (a, b) = tokio::join!(
        async {
            barrier.wait().await;
            invite(&mut first, &BOB, PROJECT, 11, NOW).await
        },
        async {
            barrier.wait().await;
            invite(&mut second, &BOB, PROJECT, 11, NOW).await
        }
    );
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    rejected(
        if a.is_err() { a } else { b },
        IssueRejection::InvitationPending,
    );
    assert_eq!(count(&mut conn, "invitations").await, 1);
    assert_eq!(count(&mut conn, "invitation_outbox").await, 1);
}

#[tokio::test]
async fn concurrent_acceptances_take_effect_once() {
    let (_dir, path, mut conn) = database().await;
    let token = issued(&mut conn).await;
    let (mut first, mut second) = racing(&path).await;
    let barrier = tokio::sync::Barrier::new(2);
    let (a, b) = tokio::join!(
        async {
            barrier.wait().await;
            accept_as(&mut first, &ALICE, &token, NOW + 1).await
        },
        async {
            barrier.wait().await;
            accept_as(&mut second, &ALICE, &token, NOW + 1).await
        }
    );
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    rejected(
        if a.is_err() { a } else { b },
        AcceptRejection::AlreadyAccepted,
    );
    let rows: i64 =
        sqlx::query_scalar("SELECT count(*) FROM memberships WHERE project_id=43 AND user_id=11")
            .fetch_one(&mut conn)
            .await
            .unwrap();
    assert_eq!(rows, 1);
}

/// The invitation and its outbox entry commit or roll back together.
#[tokio::test]
async fn a_failed_enqueue_rolls_back_the_invitation() {
    let (_dir, _path, mut conn) = database().await;
    contact(&mut conn, 11, "alice@example.test").await;
    exec(
        &mut conn,
        "ALTER TABLE invitation_outbox RENAME TO outbox_gone",
    )
    .await;
    assert_eq!(
        invite(&mut conn, &BOB, PROJECT, 11, NOW).await.unwrap_err(),
        ActionError::Failed {
            primary: StopReason::Execution {
                stage: Stage::Body,
                kind: FailureKind::Other
            },
            cleanup: Cleanup::RollbackAcknowledged,
        }
    );
    assert_eq!(count(&mut conn, "invitations").await, 0);
    assert_eq!(count(&mut conn, "outbox_gone").await, 0);
}

#[tokio::test]
async fn begin_failures_are_not_rejections_and_write_nothing() {
    let (_dir, path, mut conn) = database().await;
    let token = issued(&mut conn).await;
    let mut locked = connect(&path).await.unwrap();
    let tx = locked.begin_with("BEGIN IMMEDIATE").await.unwrap();
    let mut issuing = connect(&path).await.unwrap();
    assert_eq!(
        invite(&mut issuing, &BOB, PROJECT, 11, NOW + 1)
            .await
            .unwrap_err(),
        busy_at_begin()
    );
    let mut accepting = connect(&path).await.unwrap();
    assert_eq!(
        accept_as(&mut accepting, &ALICE, &token, NOW + 1)
            .await
            .unwrap_err(),
        busy_at_begin()
    );
    tx.rollback().await.unwrap();
    assert_eq!(count(&mut conn, "invitations").await, 1);
    assert_eq!(role(&mut conn, PROJECT, 11).await, None);
}

/// A real deferred foreign-key violation fails COMMIT rather than the body,
/// following `http/memberships/tests.rs`. Disposal and a fresh connection
/// observe what persisted; this does not establish an acknowledged rollback.
#[tokio::test]
async fn commit_failures_are_not_acknowledged_and_persist_nothing() {
    let (_dir, path, mut conn) = database().await;
    contact(&mut conn, 11, "alice@example.test").await;
    exec(
        &mut conn,
        "CREATE TABLE deferred_fault (user_id INTEGER REFERENCES users(id) DEFERRABLE INITIALLY DEFERRED);
         CREATE TRIGGER issue_fault AFTER INSERT ON invitations BEGIN INSERT INTO deferred_fault VALUES (9999); END;",
    )
    .await;
    let mut issuing = connect(&path).await.unwrap();
    assert_eq!(
        invite(&mut issuing, &BOB, PROJECT, 11, NOW)
            .await
            .unwrap_err(),
        failed_at_commit()
    );
    issuing.close().await.unwrap();
    assert_eq!(count(&mut conn, "invitations").await, 0);
    assert_eq!(count(&mut conn, "invitation_outbox").await, 0);

    exec(&mut conn, "DROP TRIGGER issue_fault").await;
    invite(&mut conn, &BOB, PROJECT, 11, NOW).await.unwrap();
    let token = latest_token(&mut conn).await;
    exec(
        &mut conn,
        "CREATE TRIGGER accept_fault AFTER UPDATE ON invitations BEGIN INSERT INTO deferred_fault VALUES (9999); END;",
    )
    .await;
    let mut accepting = connect(&path).await.unwrap();
    assert_eq!(
        accept_as(&mut accepting, &ALICE, &token, NOW + 1)
            .await
            .unwrap_err(),
        failed_at_commit()
    );
    accepting.close().await.unwrap();
    let mut fresh = connect(&path).await.unwrap();
    let accepted_at: Option<i64> = sqlx::query_scalar("SELECT accepted_at FROM invitations")
        .fetch_one(&mut fresh)
        .await
        .unwrap();
    assert_eq!(accepted_at, None);
    assert_eq!(role(&mut fresh, PROJECT, 11).await, None);
}

/// Injected rollback observations, not SQLite rollback failures: the
/// finalizer keeps either primary and drops the driver's text.
#[test]
fn finalization_keeps_the_primary_when_rollback_fails() {
    for primary in [
        StopReason::Rejected(IssueRejection::InvitationPending),
        StopReason::Execution {
            stage: Stage::Body,
            kind: FailureKind::Busy,
        },
    ] {
        let expected = format!("{primary:?}");
        let result = finalize(primary, Err(sqlx::Error::Protocol("secret-canary".into())));
        assert!(!format!("{result:?}").contains("secret-canary"));
        match result {
            ActionError::Failed {
                primary,
                cleanup:
                    Cleanup::Unconfirmed {
                        rollback_error: Some(FailureKind::Other),
                    },
            } => assert_eq!(format!("{primary:?}"), expected),
            other => panic!("unexpected {other:?}"),
        }
    }
    assert_eq!(
        finalize(StopReason::Rejected(AcceptRejection::Expired), Ok(())),
        ActionError::Rejected(AcceptRejection::Expired)
    );
    assert_eq!(
        finalize::<AcceptRejection>(
            StopReason::Execution {
                stage: Stage::Body,
                kind: FailureKind::Other
            },
            Ok(())
        ),
        ActionError::Failed {
            primary: StopReason::Execution {
                stage: Stage::Body,
                kind: FailureKind::Other
            },
            cleanup: Cleanup::RollbackAcknowledged,
        }
    );
}

#[tokio::test]
async fn results_and_errors_never_carry_the_credential_or_address() {
    let (_dir, _path, mut conn) = database().await;
    contact(&mut conn, 11, "alice-canary@example.test").await;
    let issued = invite(&mut conn, &BOB, PROJECT, 11, NOW).await.unwrap();
    let token = latest_token(&mut conn).await;
    let mut seen = vec![format!("{issued:?}")];
    seen.push(format!(
        "{:?}",
        invite(&mut conn, &BOB, PROJECT, 11, NOW).await
    ));
    seen.push(format!(
        "{:?}",
        accept_as(&mut conn, &BOB, &token, NOW).await
    ));
    seen.push(format!(
        "{:?}",
        accept_as(&mut conn, &ALICE, &token, NOW).await
    ));
    seen.push(format!(
        "{:?}",
        accept_as(&mut conn, &ALICE, &token, NOW).await
    ));
    for text in seen {
        assert!(
            !text.contains(&token),
            "Debug output contains the credential"
        );
        assert!(
            !text.contains(&sha256_hex(&token)),
            "Debug output contains the credential hash"
        );
        assert!(
            !text.contains("canary"),
            "Debug output contains the address"
        );
    }
}

#[test]
fn codes_are_the_s19_codes() {
    use strum::VariantArray;
    let issue: Vec<&str> = IssueRejection::VARIANTS
        .iter()
        .map(|r| r.descriptor().code)
        .collect();
    assert_eq!(
        issue,
        [
            "invitations.forbidden",
            "invitations.recipient_not_found",
            "invitations.already_member",
            "invitations.invitation_pending",
            "invitations.recipient_unavailable",
        ]
    );
    let accept: Vec<&str> = AcceptRejection::VARIANTS
        .iter()
        .map(|r| r.descriptor().code)
        .collect();
    assert_eq!(
        accept,
        [
            "invitations.not_found",
            "invitations.already_accepted",
            "invitations.expired",
        ]
    );
}
