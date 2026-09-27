use super::failure_kind;
pub use super::{Cleanup, FailureKind, Stage};
use crate::{
    identity::Actor,
    read::{self, Page, ReadError, Stop},
};
use sqlx::{Connection, SqliteConnection};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MemberRole {
    Owner,
    Editor,
    Viewer,
}

impl MemberRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Editor => "editor",
            Self::Viewer => "viewer",
        }
    }

    pub(crate) fn parse(stored: &str) -> Option<Self> {
        match stored {
            "owner" => Some(Self::Owner),
            "editor" => Some(Self::Editor),
            "viewer" => Some(Self::Viewer),
            _ => None,
        }
    }
}

pub struct ChangeRole {
    pub project_id: i64,
    pub user_id: i64,
    pub role: MemberRole,
}

pub struct RemoveMember {
    pub project_id: i64,
    pub user_id: i64,
}

pub struct ListMembers {
    pub project_id: i64,
    pub limit: u32,
    /// Position only: the page starts after this user ID.
    pub after: Option<i64>,
}

#[derive(Debug, PartialEq)]
pub struct MemberSummary {
    pub user_id: i64,
    pub display_name: String,
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

/// Listing permits only this refusal, so it has its own type (S17: types
/// split where permitted sets differ).
#[derive(Clone, Copy, Debug, PartialEq, Eq, strum::VariantArray)]
pub enum ListMembersRejection {
    Forbidden,
}

pub struct Descriptor {
    pub code: &'static str,
    pub summary: &'static str,
    pub rule: Option<&'static str>,
    pub prerequisite: Option<&'static str>,
}

/// The one definition of `memberships.forbidden`, whichever type refuses.
const FORBIDDEN: Descriptor = Descriptor {
    code: "memberships.forbidden",
    summary: "This operation is not permitted.",
    rule: None,
    prerequisite: None,
};

impl Rejection {
    pub fn descriptor(self) -> Descriptor {
        match self {
            Self::Forbidden => FORBIDDEN,
            Self::MemberNotFound => Descriptor {
                code: "memberships.member_not_found",
                summary: "Member not found.",
                rule: None,
                prerequisite: None,
            },
            Self::LastOwner => Descriptor {
                code: "memberships.last_owner",
                summary: "The project must retain an owner.",
                rule: Some("memberships.at_least_one_owner"),
                prerequisite: Some("memberships.another_owner_required"),
            },
        }
    }
}

impl ListMembersRejection {
    pub fn descriptor(self) -> Descriptor {
        match self {
            Self::Forbidden => FORBIDDEN,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum StopReason {
    Rejected(Rejection),
    Execution { stage: Stage, kind: FailureKind },
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

pub async fn change_role(
    conn: &mut SqliteConnection,
    actor: &Actor,
    input: ChangeRole,
) -> Result<Acknowledged, ActionError> {
    apply(
        conn,
        actor,
        input.project_id,
        input.user_id,
        Some(input.role),
    )
    .await
}

pub async fn remove_member(
    conn: &mut SqliteConnection,
    actor: &Actor,
    input: RemoveMember,
) -> Result<Acknowledged, ActionError> {
    apply(conn, actor, input.project_id, input.user_id, None).await
}

/// Owns one SQLite transaction. Callers must dispose of the fresh connection
/// after failures; no safe-reuse or task-loss guarantee is made here.
/// `None` removes the membership; that encoding stays private.
async fn apply(
    conn: &mut SqliteConnection,
    actor: &Actor,
    project_id: i64,
    user_id: i64,
    target: Option<MemberRole>,
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
            .bind(project_id).bind(actor.0).fetch_one(&mut *tx).await.map_err(|e| execution(Stage::Body, e))?;
        if !owner { return Err(StopReason::Rejected(Rejection::Forbidden)); }
        let role: Option<String> = sqlx::query_scalar("SELECT role FROM memberships WHERE project_id=? AND user_id=?")
            .bind(project_id).bind(user_id).fetch_optional(&mut *tx).await.map_err(|e| execution(Stage::Body, e))?;
        let role = role.ok_or(StopReason::Rejected(Rejection::MemberNotFound))?;
        if role == "owner" && target != Some(MemberRole::Owner) {
            let owners: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM memberships WHERE project_id=? AND role='owner'")
                .bind(project_id).fetch_one(&mut *tx).await.map_err(|e| execution(Stage::Body, e))?;
            if owners == 1 { return Err(StopReason::Rejected(Rejection::LastOwner)); }
        }
        if let Some(role) = target {
            sqlx::query("UPDATE memberships SET role=? WHERE project_id=? AND user_id=?")
                .bind(role.as_str()).bind(project_id).bind(user_id).execute(&mut *tx).await.map_err(|e| execution(Stage::Body, e))?;
        } else {
            sqlx::query("DELETE FROM memberships WHERE project_id=? AND user_id=?")
                .bind(project_id).bind(user_id).execute(&mut *tx).await.map_err(|e| execution(Stage::Body, e))?;
        }
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

/// Any member, in any role, may list; an unknown project and a non-member are
/// refused alike. Visibility is re-checked for every page, in the same read
/// transaction as the page itself.
pub async fn list_members(
    conn: SqliteConnection,
    actor: &Actor,
    query: ListMembers,
) -> Result<Page<MemberSummary>, ReadError<ListMembersRejection>> {
    read::run(conn, async |tx| {
        let member: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM memberships WHERE project_id=? AND user_id=?)",
        )
        .bind(query.project_id)
        .bind(actor.0)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| read::execution(Stage::Body, e))?;
        if !member {
            return Err(Stop::Rejected(ListMembersRejection::Forbidden));
        }
        let rows: Vec<(i64, String, String)> = sqlx::query_as(
            "SELECT m.user_id, u.display_name, m.role FROM memberships m JOIN users u ON u.id = m.user_id
             WHERE m.project_id=? AND m.user_id>? ORDER BY m.user_id LIMIT ?",
        )
        .bind(query.project_id)
        .bind(query.after.unwrap_or(0))
        .bind(i64::from(query.limit) + 1)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| read::execution(Stage::Body, e))?;
        let members = rows
            .into_iter()
            .map(|(user_id, display_name, role)| {
                Some(MemberSummary {
                    user_id,
                    display_name,
                    role: MemberRole::parse(&role)?,
                })
            })
            .collect::<Option<Vec<_>>>()
            .ok_or(Stop::Execution {
                stage: Stage::Body,
                kind: FailureKind::Other,
            })?;
        Ok(Page::new(members, query.limit, |m| m.user_id))
    })
    .await
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

    #[tokio::test]
    async fn failed_page_query_rolls_back_and_releases_the_writer() {
        let (_dir, path) = crate::read::tests::database().await;
        let mut conn = crate::app::connect(&path).await.unwrap();
        // Visibility reads only memberships, so the page query fails alone.
        sqlx::query("ALTER TABLE users RENAME TO users_gone")
            .execute(&mut conn)
            .await
            .unwrap();
        let listed = list_members(
            crate::app::connect(&path).await.unwrap(),
            &Actor(11),
            ListMembers {
                project_id: 41,
                limit: 50,
                after: None,
            },
        )
        .await;
        assert_eq!(
            listed,
            Err(ReadError::Failed {
                primary: Stop::Execution {
                    stage: Stage::Body,
                    kind: FailureKind::Other
                },
                cleanup: Cleanup::RollbackAcknowledged,
            })
        );
        let mut tx = conn.begin_with("BEGIN IMMEDIATE").await.unwrap();
        sqlx::query("UPDATE projects SET name='Written' WHERE id=41")
            .execute(&mut *tx)
            .await
            .unwrap();
        tx.commit().await.unwrap();
    }
}
