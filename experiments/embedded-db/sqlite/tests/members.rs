use iris_sqlite_spike::{
    ChangeMember, MemberOutcome as O, MemberRole as R, change_member, connect, migrate,
};

#[path = "../../../agent-interface/evidence.rs"]
mod evidence;

fn input(actor_id: i64, user_id: i64, role: Option<R>) -> ChangeMember {
    ChangeMember {
        project_id: 41,
        actor_id,
        user_id,
        role,
    }
}

async fn fixture() -> (tempfile::TempDir, sqlx::SqliteConnection) {
    let dir = tempfile::tempdir().unwrap();
    let mut conn = connect(&dir.path().join("app.db")).await.unwrap();
    migrate(&mut conn).await.unwrap();
    sqlx::raw_sql("INSERT INTO users VALUES(11),(29),(37),(53),(71); INSERT INTO projects VALUES(41),(43);
        INSERT INTO memberships VALUES(41,11,'owner'),(41,29,'editor'),(41,37,'viewer'),(43,53,'owner'),(43,29,'viewer')")
        .execute(&mut conn).await.unwrap();
    (dir, conn)
}

async fn members(conn: &mut sqlx::SqliteConnection) -> Vec<(i64, i64, String)> {
    sqlx::query_as("SELECT project_id,user_id,role FROM memberships ORDER BY project_id,user_id")
        .fetch_all(conn)
        .await
        .unwrap()
}

#[tokio::test]
async fn policy_roles_project_scope_and_self_service() {
    let (_dir, mut conn) = fixture().await;
    let original = members(&mut conn).await;
    for actor in [29, 37, 53, 71] {
        for user in [11, 999] {
            for role in [None, Some(R::Owner)] {
                assert_eq!(
                    change_member(&mut conn, input(actor, user, role))
                        .await
                        .unwrap(),
                    O::Forbidden
                );
            }
        }
    }
    for role in [None, Some(R::Viewer), Some(R::Editor)] {
        assert_eq!(
            change_member(&mut conn, input(11, 11, role)).await.unwrap(),
            O::LastOwner
        );
    }
    assert_eq!(
        change_member(&mut conn, input(11, 53, None)).await.unwrap(),
        O::MemberNotFound
    );
    assert_eq!(
        members(&mut conn).await,
        original,
        "rejections must not mutate any membership"
    );
    assert_eq!(
        change_member(&mut conn, input(11, 11, Some(R::Owner)))
            .await
            .unwrap(),
        O::Changed
    );
    for (role, expected) in [
        (R::Viewer, "viewer"),
        (R::Editor, "editor"),
        (R::Owner, "owner"),
    ] {
        assert_eq!(
            change_member(&mut conn, input(11, 29, Some(role)))
                .await
                .unwrap(),
            O::Changed
        );
        let actual = members(&mut conn).await;
        assert_eq!(actual[1], (41, 29, expected.into()));
        assert_eq!(actual[3], (43, 29, "viewer".into()));
    }
    assert_eq!(
        change_member(&mut conn, input(11, 11, Some(R::Viewer)))
            .await
            .unwrap(),
        O::Changed
    );
    assert_eq!(
        change_member(&mut conn, input(11, 37, None)).await.unwrap(),
        O::Forbidden
    );
    assert_eq!(
        change_member(&mut conn, input(29, 11, Some(R::Owner)))
            .await
            .unwrap(),
        O::Changed
    );
    assert_eq!(
        change_member(&mut conn, input(29, 29, None)).await.unwrap(),
        O::Changed
    );
    assert_eq!(
        change_member(&mut conn, input(11, 29, None)).await.unwrap(),
        O::MemberNotFound
    );
    assert_eq!(
        change_member(&mut conn, input(11, 37, None)).await.unwrap(),
        O::Changed
    );
    assert_eq!(
        members(&mut conn).await,
        vec![
            (41, 11, "owner".into()),
            (43, 29, "viewer".into()),
            (43, 53, "owner".into())
        ]
    );
}

#[tokio::test]
async fn concurrent_last_owner_and_authority() {
    let mut successes = 0;
    let mut conflicts = 0;
    let mut retained = 0;
    // Removal/removal, demotion/demotion, and mixed operations must serialize.
    for (a, b) in [
        (None, None),
        (Some(R::Editor), Some(R::Viewer)),
        (None, Some(R::Viewer)),
    ] {
        let (dir, mut first) = fixture().await;
        assert_eq!(
            change_member(&mut first, input(11, 29, Some(R::Owner)))
                .await
                .unwrap(),
            O::Changed
        );
        let mut second = connect(&dir.path().join("app.db")).await.unwrap();
        let (a, b) = tokio::join!(
            change_member(&mut first, input(11, 11, a)),
            change_member(&mut second, input(29, 29, b))
        );
        let outcomes = [a.unwrap(), b.unwrap()];
        assert_eq!(outcomes.iter().filter(|o| **o == O::Changed).count(), 1);
        assert_eq!(outcomes.iter().filter(|o| **o == O::LastOwner).count(), 1);
        successes += 1;
        conflicts += 1;
        let owners: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM memberships WHERE project_id=41 AND role='owner'",
        )
        .fetch_one(&mut first)
        .await
        .unwrap();
        assert_eq!(owners, 1);
        retained += owners;
    }
    // Each owner tries to demote the other: the loser must lose authority,
    // rather than authorizing outside the write transaction.
    let (dir, mut first) = fixture().await;
    change_member(&mut first, input(11, 29, Some(R::Owner)))
        .await
        .unwrap();
    let mut second = connect(&dir.path().join("app.db")).await.unwrap();
    let (a, b) = tokio::join!(
        change_member(&mut first, input(11, 29, Some(R::Editor))),
        change_member(&mut second, input(29, 11, None))
    );
    let outcomes = [a.unwrap(), b.unwrap()];
    let forbidden = outcomes.iter().filter(|o| **o == O::Forbidden).count();
    assert_eq!(forbidden, 1);
    assert_eq!(outcomes.iter().filter(|o| **o == O::Changed).count(), 1);
    evidence::record("successful_departures", successes);
    evidence::record("last_owner_conflicts", conflicts);
    evidence::record("retained_owners", retained);
    evidence::record("revoked_actor_rejected", forbidden as i64);
}
