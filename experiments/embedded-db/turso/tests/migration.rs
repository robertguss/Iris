#[tokio::test]
async fn upgrades_existing_memberships_and_can_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let db = iris_turso_spike::open(&dir.path().join("app.db"))
        .await
        .unwrap();
    let mut conn = iris_turso_spike::connect(&db).await.unwrap();
    conn.execute_batch(include_str!("../../shared/migrations/0001_invitations.sql"))
        .await
        .unwrap();
    conn.execute_batch(
        "INSERT INTO users VALUES(11),(29); INSERT INTO projects VALUES(41);
        INSERT INTO memberships VALUES(41,11,'viewer'),(41,29,'editor'); PRAGMA user_version=1;",
    )
    .await
    .unwrap();
    iris_turso_spike::migrate(&mut conn).await.unwrap();
    let mut rows = conn
        .query("SELECT role FROM memberships ORDER BY user_id", ())
        .await
        .unwrap();
    assert_eq!(
        rows.next()
            .await
            .unwrap()
            .unwrap()
            .get::<String>(0)
            .unwrap(),
        "viewer"
    );
    assert_eq!(
        rows.next()
            .await
            .unwrap()
            .unwrap()
            .get::<String>(0)
            .unwrap(),
        "editor"
    );
    drop(rows);
    conn.execute("UPDATE memberships SET role='owner' WHERE user_id=11", ())
        .await
        .unwrap();
    assert!(
        conn.execute("UPDATE memberships SET role='administrator'", ())
            .await
            .is_err()
    );
    drop(conn);
    let mut conn = iris_turso_spike::connect(&db).await.unwrap();
    iris_turso_spike::migrate(&mut conn).await.unwrap();
}
