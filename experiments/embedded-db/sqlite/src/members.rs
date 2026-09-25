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
}

pub struct ChangeMember {
    pub project_id: i64,
    pub user_id: i64,
    pub actor_id: i64,
    /// None removes the membership; Some changes its role.
    pub role: Option<MemberRole>,
}

#[derive(Debug, PartialEq)]
pub enum MemberOutcome {
    Changed,
    Forbidden,
    MemberNotFound,
    LastOwner,
}

/// Serialize authorization, owner counting and mutation with all other writers.
/// This also prevents a concurrently demoted actor from using stale authority.
pub async fn change_member(
    conn: &mut SqliteConnection,
    input: ChangeMember,
) -> Result<MemberOutcome, sqlx::Error> {
    let mut tx = conn.begin_with("BEGIN IMMEDIATE").await?;
    let result = async {
        let owner: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM memberships WHERE project_id=? AND user_id=? AND role='owner')")
            .bind(input.project_id).bind(input.actor_id).fetch_one(&mut *tx).await?;
        if !owner { return Ok(MemberOutcome::Forbidden); }
        let role: Option<String> = sqlx::query_scalar("SELECT role FROM memberships WHERE project_id=? AND user_id=?")
            .bind(input.project_id).bind(input.user_id).fetch_optional(&mut *tx).await?;
        let Some(role) = role else { return Ok(MemberOutcome::MemberNotFound); };
        if role == "owner" && input.role != Some(MemberRole::Owner) {
            let owners: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM memberships WHERE project_id=? AND role='owner'")
                .bind(input.project_id).fetch_one(&mut *tx).await?;
            if owners == 1 { return Ok(MemberOutcome::LastOwner); }
        }
        if let Some(role) = input.role {
            sqlx::query("UPDATE memberships SET role=? WHERE project_id=? AND user_id=?")
                .bind(role.as_str()).bind(input.project_id).bind(input.user_id).execute(&mut *tx).await?;
        } else {
            sqlx::query("DELETE FROM memberships WHERE project_id=? AND user_id=?")
                .bind(input.project_id).bind(input.user_id).execute(&mut *tx).await?;
        }
        Ok(MemberOutcome::Changed)
    }.await;
    match result {
        Ok(outcome) => {
            tx.commit().await?;
            Ok(outcome)
        }
        Err(error) => {
            tx.rollback().await?;
            Err(error)
        }
    }
}
