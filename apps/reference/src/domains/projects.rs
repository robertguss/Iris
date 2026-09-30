//! Projects as seen by their members. Application code, not an Iris API.
use super::{FailureKind, Stage, memberships::MemberRole};
use crate::{
    identity::Actor,
    read::{self, Page, ReadError, Stop},
};

pub struct ListMine {
    pub limit: u32,
    /// Position only: the page starts after this project ID.
    pub after: Option<i64>,
}

/// The actor's own membership in one project.
#[derive(Debug, PartialEq)]
pub struct ProjectSummary {
    pub project_id: i64,
    pub name: String,
    pub role: MemberRole,
}

/// Listing one's own projects refuses nothing: rows are filtered by actor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListMineRejection {}

/// Filter-style visibility: the page query itself selects only the actor's
/// memberships, so there is nothing to refuse.
pub async fn list_mine(
    conn: impl read::OwnedConnection,
    actor: &Actor,
    query: ListMine,
) -> Result<Page<ProjectSummary>, ReadError<ListMineRejection>> {
    read::run(conn, async |tx| {
        let rows: Vec<(i64, String, String)> = sqlx::query_as(
            "SELECT p.id, p.name, m.role FROM memberships m JOIN projects p ON p.id = m.project_id
             WHERE m.user_id=? AND m.project_id>? ORDER BY m.project_id LIMIT ?",
        )
        .bind(actor.0)
        .bind(query.after.unwrap_or(0))
        .bind(i64::from(query.limit) + 1)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| read::execution(Stage::Body, e))?;
        let projects = rows
            .into_iter()
            .map(|(project_id, name, role)| {
                Some(ProjectSummary {
                    project_id,
                    name,
                    role: MemberRole::parse(&role)?,
                })
            })
            .collect::<Option<Vec<_>>>()
            .ok_or(Stop::Execution {
                stage: Stage::Body,
                kind: FailureKind::Other,
            })?;
        Ok(Page::new(projects, query.limit, |p| p.project_id))
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domains::Cleanup;
    use sqlx::Connection;

    #[tokio::test]
    async fn failed_page_query_rolls_back_and_releases_the_writer() {
        let (_dir, path) = crate::read::tests::database().await;
        let mut conn = crate::app::connect(&path).await.unwrap();
        sqlx::query("ALTER TABLE projects RENAME TO projects_gone")
            .execute(&mut conn)
            .await
            .unwrap();
        let listed = list_mine(
            crate::app::connect(&path).await.unwrap(),
            &Actor(11),
            ListMine {
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
        sqlx::query("UPDATE users SET display_name='Written' WHERE id=11")
            .execute(&mut *tx)
            .await
            .unwrap();
        tx.commit().await.unwrap();
    }
}
