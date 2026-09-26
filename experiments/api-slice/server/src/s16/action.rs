use crate::Actor;
use iris_sqlite_spike::MemberRole;
use sqlx::{Connection, SqliteConnection};

pub struct ChangeRole {
    pub project_id: i64,
    pub user_id: i64,
    pub role: MemberRole,
}

#[derive(Debug)]
pub struct Acknowledged;

#[derive(Clone, Copy, Debug, PartialEq, Eq, strum::VariantArray)]
pub enum Rejection {
    Forbidden,
    MemberNotFound,
    LastOwner,
}

pub struct Descriptor {
    pub code: &'static str,
    pub summary: &'static str,
    pub rule: Option<&'static str>,
    pub prerequisite: Option<&'static str>,
}

impl Rejection {
    pub fn descriptor(self) -> Descriptor {
        let (code, summary, rule, prerequisite) = match self {
            Self::Forbidden => (
                "memberships.forbidden",
                "This operation is not permitted.",
                None,
                None,
            ),
            Self::MemberNotFound => (
                "memberships.member_not_found",
                "Member not found.",
                None,
                None,
            ),
            Self::LastOwner => (
                "memberships.last_owner",
                "The project must retain an owner.",
                Some("memberships.at_least_one_owner"),
                Some("memberships.another_owner_required"),
            ),
        };
        Descriptor {
            code,
            summary,
            rule,
            prerequisite,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Stage {
    Begin,
    Body,
    Commit,
}
#[derive(Debug, PartialEq, Eq)]
pub enum FailureKind {
    Busy,
    Other,
}
#[derive(Debug, PartialEq, Eq)]
pub enum StopReason {
    Rejected(Rejection),
    Execution { stage: Stage, kind: FailureKind },
}
#[derive(Debug, PartialEq, Eq)]
pub enum Cleanup {
    RollbackAcknowledged,
    Unconfirmed { rollback_error: Option<FailureKind> },
}
#[derive(Debug, PartialEq, Eq)]
pub enum ActionError {
    Rejected(Rejection),
    Failed {
        primary: StopReason,
        cleanup: Cleanup,
    },
}

fn execution(stage: Stage, error: sqlx::Error) -> StopReason {
    StopReason::Execution {
        stage,
        kind: failure_kind(&error),
    }
}

fn failure_kind(error: &sqlx::Error) -> FailureKind {
    if iris_sqlite_spike::is_busy(error) {
        FailureKind::Busy
    } else {
        FailureKind::Other
    }
}

// Finalization only, not a transaction executor. No raw driver text is retained.
fn finalize(primary: StopReason, rollback: Result<(), sqlx::Error>) -> ActionError {
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

/// Owns one SQLite transaction. Callers must dispose of the fresh connection
/// after failures; no safe-reuse or task-loss guarantee is made here.
pub async fn change_role(
    conn: &mut SqliteConnection,
    actor: &Actor,
    input: ChangeRole,
) -> Result<Acknowledged, ActionError> {
    let mut tx = conn
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|e| ActionError::Failed {
            primary: execution(Stage::Begin, e),
            cleanup: Cleanup::Unconfirmed {
                rollback_error: None,
            },
        })?;
    let body: Result<(), StopReason> = async {
        let owner: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM memberships WHERE project_id=? AND user_id=? AND role='owner')")
            .bind(input.project_id).bind(actor.0).fetch_one(&mut *tx).await.map_err(|e| execution(Stage::Body, e))?;
        if !owner { return Err(StopReason::Rejected(Rejection::Forbidden)); }
        let role: Option<String> = sqlx::query_scalar("SELECT role FROM memberships WHERE project_id=? AND user_id=?")
            .bind(input.project_id).bind(input.user_id).fetch_optional(&mut *tx).await.map_err(|e| execution(Stage::Body, e))?;
        let role = role.ok_or(StopReason::Rejected(Rejection::MemberNotFound))?;
        if role == "owner" && input.role != MemberRole::Owner {
            let owners: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM memberships WHERE project_id=? AND role='owner'")
                .bind(input.project_id).fetch_one(&mut *tx).await.map_err(|e| execution(Stage::Body, e))?;
            if owners == 1 { return Err(StopReason::Rejected(Rejection::LastOwner)); }
        }
        sqlx::query("UPDATE memberships SET role=? WHERE project_id=? AND user_id=?")
            .bind(input.role.as_str()).bind(input.project_id).bind(input.user_id).execute(&mut *tx).await.map_err(|e| execution(Stage::Body, e))?;
        Ok(())
    }.await;
    match body {
        Ok(()) => tx
            .commit()
            .await
            .map(|()| Acknowledged)
            .map_err(|e| ActionError::Failed {
                primary: execution(Stage::Commit, e),
                cleanup: Cleanup::Unconfirmed {
                    rollback_error: None,
                },
            }),
        Err(primary) => Err(finalize(primary, tx.rollback().await)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cleanup_preserves_both_primary_kinds() {
        for primary in [
            StopReason::Rejected(Rejection::LastOwner),
            StopReason::Execution {
                stage: Stage::Body,
                kind: FailureKind::Busy,
            },
        ] {
            // Injected observation instead of calling the driver. This tests
            // finalization, not SQLite rollback-failure behavior.
            let result = finalize(primary, Err(sqlx::Error::Protocol("secret-canary".into())));
            assert!(matches!(
                result,
                ActionError::Failed {
                    cleanup: Cleanup::Unconfirmed {
                        rollback_error: Some(FailureKind::Other)
                    },
                    ..
                }
            ));
            assert!(!format!("{result:?}").contains("secret-canary"));
            match result {
                ActionError::Failed {
                    primary: StopReason::Rejected(r),
                    ..
                } => assert_eq!(r, Rejection::LastOwner),
                ActionError::Failed {
                    primary: StopReason::Execution { stage, kind },
                    ..
                } => {
                    assert_eq!(stage, Stage::Body);
                    assert_eq!(kind, FailureKind::Busy);
                }
                _ => unreachable!(),
            }
        }
    }
}
