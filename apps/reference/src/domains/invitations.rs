//! Invitations (S19 stage 1): issuing and accepting within one write
//! transaction each. Private application code: no route, worker or client
//! exposes these operations yet.
use super::failure_kind;
pub use super::{Cleanup, FailureKind, Stage};
use crate::{domains::memberships::Descriptor, identity::Actor};
use sha2::{Digest, Sha256};
use sqlx::{Connection, SqliteConnection};

pub struct IssueInvitation {
    pub project_id: i64,
    pub recipient_id: i64,
}

pub struct AcceptInvitation {
    pub token: String,
}

/// Issuance committed: the invitation and its outbox entry, not a delivery.
#[derive(Debug, PartialEq, Eq)]
pub struct Issued {
    pub project_id: i64,
    pub recipient_id: i64,
    pub expires_at: i64,
}

/// Acceptance committed; the membership it implies may have existed already.
#[derive(Debug, PartialEq, Eq)]
pub struct Accepted {
    pub project_id: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, strum::VariantArray)]
pub enum IssueRejection {
    Forbidden,
    RecipientNotFound,
    AlreadyMember,
    InvitationPending,
    RecipientUnavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, strum::VariantArray)]
pub enum AcceptRejection {
    NotFound,
    AlreadyAccepted,
    Expired,
}

impl IssueRejection {
    pub fn descriptor(self) -> Descriptor {
        let (code, summary) = match self {
            Self::Forbidden => ("invitations.forbidden", "This operation is not permitted."),
            Self::RecipientNotFound => ("invitations.recipient_not_found", "Recipient not found."),
            Self::AlreadyMember => (
                "invitations.already_member",
                "The recipient is already a member of the project.",
            ),
            Self::InvitationPending => (
                "invitations.invitation_pending",
                "An invitation for this recipient is already pending.",
            ),
            Self::RecipientUnavailable => (
                "invitations.recipient_unavailable",
                "The recipient cannot be invited.",
            ),
        };
        Descriptor {
            code,
            summary,
            rule: None,
            prerequisite: None,
        }
    }
}

impl AcceptRejection {
    pub fn descriptor(self) -> Descriptor {
        let (code, summary) = match self {
            Self::NotFound => ("invitations.not_found", "Invitation not found."),
            Self::AlreadyAccepted => (
                "invitations.already_accepted",
                "The invitation has already been accepted.",
            ),
            Self::Expired => ("invitations.expired", "The invitation has expired."),
        };
        Descriptor {
            code,
            summary,
            rule: None,
            prerequisite: None,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum StopReason<R> {
    Rejected(R),
    Execution { stage: Stage, kind: FailureKind },
}

#[derive(Debug, PartialEq, Eq)]
pub enum ActionError<R> {
    Rejected(R),
    Failed {
        primary: StopReason<R>,
        cleanup: Cleanup,
    },
}

/// How long an issued invitation stays acceptable.
pub const LIFETIME_SECONDS: i64 = 3600;

fn execution<R>(stage: Stage, error: sqlx::Error) -> StopReason<R> {
    StopReason::Execution {
        stage,
        kind: failure_kind(&error),
    }
}

/// A failure before the transaction exists or while committing it: nothing
/// confirms what the database kept.
fn unconfirmed<R>(stage: Stage, error: sqlx::Error) -> ActionError<R> {
    ActionError::Failed {
        primary: execution(stage, error),
        cleanup: Cleanup::Unconfirmed {
            rollback_error: None,
        },
    }
}

// Finalization only, as in `memberships`. No raw driver text is retained.
fn finalize<R>(primary: StopReason<R>, rollback: Result<(), sqlx::Error>) -> ActionError<R> {
    match (primary, rollback) {
        (StopReason::Rejected(r), Ok(())) => ActionError::Rejected(r),
        (primary, result) => ActionError::Failed {
            primary,
            cleanup: match result {
                Ok(()) => Cleanup::RollbackAcknowledged,
                Err(error) => Cleanup::Unconfirmed {
                    rollback_error: Some(failure_kind(&error)),
                },
            },
        },
    }
}

/// Lowercase hex of `bytes` random bytes. An entropy failure is an execution
/// failure of the body, never a panic.
fn random_hex<R>(bytes: usize) -> Result<String, StopReason<R>> {
    let mut buffer = vec![0u8; bytes];
    getrandom::fill(&mut buffer).map_err(|_| StopReason::Execution {
        stage: Stage::Body,
        kind: FailureKind::Other,
    })?;
    Ok(hex(&buffer))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// The stored form of a credential: lowercase hex SHA-256 of its text.
fn token_hash(token: &str) -> String {
    hex(&Sha256::digest(token.as_bytes()))
}

/// A deliberately narrow local policy, not a deliverability check: one `@`,
/// a non-empty local part, no whitespace, and a domain ending in `.test`
/// with a non-empty label before it, ignoring case.
fn usable_contact(email: &str) -> bool {
    let email = email.to_ascii_lowercase();
    let Some((local, domain)) = email.split_once('@') else {
        return false;
    };
    let label = domain.strip_suffix(".test").unwrap_or_default();
    !local.is_empty()
        && !domain.contains('@')
        && !email.chars().any(char::is_whitespace)
        && !label.is_empty()
        && !label.starts_with('.')
        && !label.ends_with('.')
}

/// Issues an invitation and enqueues its delivery in one write transaction.
/// Checks run in S19's order and the first failure wins. Callers must dispose
/// of the connection after failures.
pub async fn issue(
    conn: &mut SqliteConnection,
    actor: &Actor,
    input: IssueInvitation,
    now: i64,
) -> Result<Issued, ActionError<IssueRejection>> {
    let IssueInvitation {
        project_id,
        recipient_id,
    } = input;
    let mut tx = conn
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|e| unconfirmed(Stage::Begin, e))?;
    let expires_at = now + LIFETIME_SECONDS;
    let body: Result<(), StopReason<IssueRejection>> = async {
        let body = |e| execution(Stage::Body, e);
        let owner: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM memberships WHERE project_id=? AND user_id=? AND role='owner')")
            .bind(project_id).bind(actor.0).fetch_one(&mut *tx).await.map_err(body)?;
        if !owner { return Err(StopReason::Rejected(IssueRejection::Forbidden)); }
        let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE id=?)")
            .bind(recipient_id).fetch_one(&mut *tx).await.map_err(body)?;
        if !exists { return Err(StopReason::Rejected(IssueRejection::RecipientNotFound)); }
        let member: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM memberships WHERE project_id=? AND user_id=?)")
            .bind(project_id).bind(recipient_id).fetch_one(&mut *tx).await.map_err(body)?;
        if member { return Err(StopReason::Rejected(IssueRejection::AlreadyMember)); }
        let pending: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM invitations WHERE project_id=? AND recipient_id=? AND accepted_at IS NULL AND expires_at > ?)")
            .bind(project_id).bind(recipient_id).bind(now).fetch_one(&mut *tx).await.map_err(body)?;
        if pending { return Err(StopReason::Rejected(IssueRejection::InvitationPending)); }
        let email: Option<String> = sqlx::query_scalar("SELECT email FROM user_contacts WHERE user_id=?")
            .bind(recipient_id).fetch_optional(&mut *tx).await.map_err(body)?;
        let email = email.filter(|email| usable_contact(email))
            .ok_or(StopReason::Rejected(IssueRejection::RecipientUnavailable))?;
        let token = random_hex(32)?;
        let message_id = format!("<{}@reference.iris.test>", random_hex::<IssueRejection>(16)?);
        let invitation_id: i64 = sqlx::query_scalar("INSERT INTO invitations (project_id, recipient_id, issuer_id, role, token_hash, created_at, expires_at) VALUES (?, ?, ?, 'editor', ?, ?, ?) RETURNING id")
            .bind(project_id).bind(recipient_id).bind(actor.0).bind(token_hash(&token)).bind(now).bind(expires_at)
            .fetch_one(&mut *tx).await.map_err(body)?;
        sqlx::query("INSERT INTO invitation_outbox (invitation_id, recipient_email, token, message_id, created_at) VALUES (?, ?, ?, ?, ?)")
            .bind(invitation_id).bind(email).bind(token).bind(message_id).bind(now)
            .execute(&mut *tx).await.map_err(body)?;
        Ok(())
    }.await;
    match body {
        Ok(()) => tx
            .commit()
            .await
            .map(|()| Issued {
                project_id,
                recipient_id,
                expires_at,
            })
            .map_err(|e| unconfirmed(Stage::Commit, e)),
        Err(primary) => Err(finalize(primary, tx.rollback().await)),
    }
}

/// Accepts an invitation for the session's user in one write transaction. An
/// unknown credential and another user's credential are refused alike, before
/// acceptance or expiry is considered. An existing membership keeps its role.
pub async fn accept(
    conn: &mut SqliteConnection,
    actor: &Actor,
    input: AcceptInvitation,
    now: i64,
) -> Result<Accepted, ActionError<AcceptRejection>> {
    let hash = token_hash(&input.token);
    let mut tx = conn
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|e| unconfirmed(Stage::Begin, e))?;
    let body: Result<i64, StopReason<AcceptRejection>> = async {
        let body = |e| execution(Stage::Body, e);
        let found: Option<(i64, i64, i64, Option<i64>, i64)> = sqlx::query_as("SELECT id, project_id, recipient_id, accepted_at, expires_at FROM invitations WHERE token_hash=?")
            .bind(&hash).fetch_optional(&mut *tx).await.map_err(body)?;
        let (id, project_id, _, accepted_at, expires_at) = found
            .filter(|(_, _, recipient_id, _, _)| *recipient_id == actor.0)
            .ok_or(StopReason::Rejected(AcceptRejection::NotFound))?;
        if accepted_at.is_some() { return Err(StopReason::Rejected(AcceptRejection::AlreadyAccepted)); }
        if now >= expires_at { return Err(StopReason::Rejected(AcceptRejection::Expired)); }
        sqlx::query("UPDATE invitations SET accepted_at=? WHERE id=?")
            .bind(now).bind(id).execute(&mut *tx).await.map_err(body)?;
        sqlx::query("INSERT INTO memberships (project_id, user_id, role) VALUES (?, ?, 'editor') ON CONFLICT (project_id, user_id) DO NOTHING")
            .bind(project_id).bind(actor.0).execute(&mut *tx).await.map_err(body)?;
        Ok(project_id)
    }.await;
    match body {
        Ok(project_id) => tx
            .commit()
            .await
            .map(|()| Accepted { project_id })
            .map_err(|e| unconfirmed(Stage::Commit, e)),
        Err(primary) => Err(finalize(primary, tx.rollback().await)),
    }
}

pub mod delivery;

#[cfg(test)]
mod tests;
