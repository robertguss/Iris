use super::*;
use crate::app::connect;
use sqlx::{AssertSqlSafe, Connection};
use std::{
    io::{BufRead, BufReader},
    os::unix::fs::MetadataExt,
    process::{Child, Command, Stdio},
    sync::mpsc,
    time::{Duration, SystemTime},
};

const ISSUER: &str = "http://127.0.0.1:4001";

fn canonical_dir() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = fs::canonicalize(dir.path()).unwrap();
    (dir, path)
}

async fn count(path: &Path, table: &str) -> i64 {
    let mut conn = connect(path).await.unwrap();
    // The table names are literals in this file.
    let n = sqlx::query_scalar(AssertSqlSafe(format!("SELECT count(*) FROM {table}")))
        .fetch_one(&mut conn)
        .await
        .unwrap();
    conn.close().await.unwrap();
    n
}

async fn journal_mode(path: &Path) -> String {
    let mut conn = connect(path).await.unwrap();
    let mode = sqlx::query_scalar("PRAGMA journal_mode")
        .fetch_one(&mut conn)
        .await
        .unwrap();
    conn.close().await.unwrap();
    mode
}

fn staging(path: &Path) -> PathBuf {
    path.with_file_name(format!(
        "{}.iris-init",
        path.file_name().unwrap().to_str().unwrap()
    ))
}

/// Builds an owned staging directory as an interrupted initialization would.
fn owned_staging(path: &Path) -> PathBuf {
    let dir = staging(path);
    fs::create_dir(&dir).unwrap();
    fs::write(dir.join("owner"), format!("{}\n", path.display())).unwrap();
    dir
}

fn snapshot(path: &Path) -> (Vec<u8>, SystemTime) {
    (
        fs::read(path).unwrap(),
        fs::metadata(path).unwrap().modified().unwrap(),
    )
}

fn refused(result: Result<Storage, StorageError>, needle: &str) {
    match result {
        Err(StorageError::Refused(message)) => {
            assert!(message.contains(needle), "{message}")
        }
        Err(other) => panic!("expected a refusal naming {needle}, got {other}"),
        Ok(_) => panic!("expected a refusal naming {needle}, got a database"),
    }
}

#[tokio::test]
async fn a_new_database_is_seeded_once_in_the_rollback_journal() {
    let (_dir, root) = canonical_dir();
    let path = root.join("dev.db");
    let storage = Storage::open(&path, ISSUER).await.unwrap();
    assert_eq!(storage.path(), path);
    assert_eq!(count(&path, "users").await, 2);
    // seed's two owners and the development editor row.
    assert_eq!(count(&path, "memberships").await, 3);
    assert_eq!(count(&path, "iris_external_identities").await, 2);
    assert_eq!(journal_mode(&path).await, "delete");
    assert!(!staging(&path).exists());
    assert!(root.join("dev.db.iris-lock").exists());
    assert_eq!(fs::metadata(&path).unwrap().nlink(), 1);
}

#[tokio::test]
async fn a_restart_keeps_changes_and_does_not_reseed() {
    let (_dir, root) = canonical_dir();
    let path = root.join("dev.db");
    drop(Storage::open(&path, ISSUER).await.unwrap());
    let mut conn = connect(&path).await.unwrap();
    sqlx::raw_sql(
        "UPDATE memberships SET role='viewer' WHERE project_id=41 AND user_id=29;
        DELETE FROM memberships WHERE project_id=43",
    )
    .execute(&mut conn)
    .await
    .unwrap();
    conn.close().await.unwrap();

    let _storage = Storage::open(&path, ISSUER).await.unwrap();
    assert_eq!(count(&path, "users").await, 2);
    assert_eq!(count(&path, "memberships").await, 2);
    let mut conn = connect(&path).await.unwrap();
    let role: String =
        sqlx::query_scalar("SELECT role FROM memberships WHERE project_id=41 AND user_id=29")
            .fetch_one(&mut conn)
            .await
            .unwrap();
    assert_eq!(role, "viewer");
}

#[tokio::test]
async fn a_second_owner_is_refused_before_touching_the_database() {
    let (_dir, root) = canonical_dir();
    let path = root.join("dev.db");
    let _owner = Storage::open(&path, ISSUER).await.unwrap();
    let before = snapshot(&path);
    match Storage::open(&path, ISSUER).await {
        Err(StorageError::InUse(named)) => assert_eq!(named, path),
        Err(other) => panic!("expected in use, got {other}"),
        Ok(_) => panic!("second owner opened the database"),
    }
    assert_eq!(snapshot(&path), before);
    assert!(!staging(&path).exists());
}

#[tokio::test]
async fn concurrent_opens_of_a_new_path_leave_one_seeded_database() {
    let (_dir, root) = canonical_dir();
    let path = root.join("dev.db");
    let (a, b) = tokio::join!(Storage::open(&path, ISSUER), Storage::open(&path, ISSUER));
    assert!(
        a.is_ok() != b.is_ok(),
        "exactly one may open: {:?} / {:?}",
        a.as_ref().err(),
        b.as_ref().err()
    );
    let refusal = a.as_ref().err().or(b.as_ref().err()).unwrap();
    assert!(matches!(refusal, StorageError::InUse(_)), "{refusal}");
    assert_eq!(count(&path, "users").await, 2);
}

#[tokio::test]
async fn a_directory_alias_contends_for_the_same_lock() {
    let (_dir, root) = canonical_dir();
    fs::create_dir(root.join("real")).unwrap();
    std::os::unix::fs::symlink(root.join("real"), root.join("alias")).unwrap();
    let _owner = Storage::open(&root.join("real/dev.db"), ISSUER)
        .await
        .unwrap();
    assert!(matches!(
        Storage::open(&root.join("alias/dev.db"), ISSUER).await,
        Err(StorageError::InUse(_))
    ));
}

#[tokio::test]
async fn a_symlinked_database_is_refused() {
    let (_dir, root) = canonical_dir();
    let path = root.join("dev.db");
    drop(Storage::open(&path, ISSUER).await.unwrap());
    std::os::unix::fs::symlink(&path, root.join("link.db")).unwrap();
    refused(
        Storage::open(&root.join("link.db"), ISSUER).await,
        "symbolic link",
    );
    assert!(!root.join("link.db.iris-lock").exists());
}

#[tokio::test]
async fn a_foreign_hard_link_is_refused_under_either_name() {
    let (_dir, root) = canonical_dir();
    let path = root.join("dev.db");
    drop(Storage::open(&path, ISSUER).await.unwrap());
    fs::hard_link(&path, root.join("other.db")).unwrap();
    refused(Storage::open(&path, ISSUER).await, "hard link");
    refused(
        Storage::open(&root.join("other.db"), ISSUER).await,
        "hard link",
    );
    assert_eq!(fs::metadata(&path).unwrap().nlink(), 2);
}

#[tokio::test]
async fn a_publication_leftover_with_a_foreign_alias_is_refused() {
    let (_dir, root) = canonical_dir();
    let path = root.join("dev.db");
    drop(Storage::open(&path, ISSUER).await.unwrap());
    let dir = owned_staging(&path);
    fs::hard_link(&path, dir.join("reference.db")).unwrap();
    fs::hard_link(&path, root.join("alias.db")).unwrap();
    refused(Storage::open(&path, ISSUER).await, "hard link");
    assert!(dir.join("reference.db").exists() && dir.join("owner").exists());
    assert!(root.join("alias.db").exists());
    assert_eq!(fs::metadata(&path).unwrap().nlink(), 3);
}

#[tokio::test]
async fn a_missing_database_with_stale_sidecars_is_refused() {
    for sidecar in ["-journal", "-wal", "-shm"] {
        let (_dir, root) = canonical_dir();
        let path = root.join("dev.db");
        let stale = root.join(format!("dev.db{sidecar}"));
        fs::write(&stale, b"stale").unwrap();
        refused(
            Storage::open(&path, ISSUER).await,
            &format!("dev.db{sidecar}"),
        );
        assert!(!path.exists() && !staging(&path).exists());
        assert_eq!(fs::read(&stale).unwrap(), b"stale");
    }
}

#[tokio::test]
async fn reserved_names_are_refused_in_any_case() {
    let (_dir, root) = canonical_dir();
    for name in [
        "example.db.IRIS-INIT",
        "Example.DB.Iris-Lock",
        "dev.db.iris-init-journal",
        "dev.db.iris-lock-WAL",
    ] {
        refused(Storage::open(&root.join(name), ISSUER).await, "reserved");
    }
    fs::create_dir(root.join("dev.db.Iris-Init")).unwrap();
    refused(
        Storage::open(&root.join("dev.db.Iris-Init/inner.db"), ISSUER).await,
        "reserved",
    );
    assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
}

#[tokio::test]
async fn a_missing_parent_directory_is_refused() {
    let (_dir, root) = canonical_dir();
    refused(
        Storage::open(&root.join("absent/dev.db"), ISSUER).await,
        "parent directory",
    );
    assert!(!root.join("absent").exists());
}

#[tokio::test]
async fn unrecognized_staging_occupants_are_refused_and_kept() {
    // A file where the staging directory belongs.
    let (_dir, root) = canonical_dir();
    let path = root.join("dev.db");
    fs::write(staging(&path), b"not ours").unwrap();
    refused(Storage::open(&path, ISSUER).await, "unrecognized");
    assert_eq!(fs::read(staging(&path)).unwrap(), b"not ours");

    // A directory whose owner record names another database.
    let (_dir, root) = canonical_dir();
    let path = root.join("dev.db");
    let dir = staging(&path);
    fs::create_dir(&dir).unwrap();
    fs::write(
        dir.join("owner"),
        format!("{}\n", root.join("x.db").display()),
    )
    .unwrap();
    refused(Storage::open(&path, ISSUER).await, "unrecognized");
    assert!(dir.join("owner").exists());

    // An owned directory holding something the initializer never writes.
    let (_dir, root) = canonical_dir();
    let path = root.join("dev.db");
    let dir = owned_staging(&path);
    fs::write(dir.join("notes.txt"), b"keep").unwrap();
    refused(Storage::open(&path, ISSUER).await, "unrecognized");
    assert_eq!(fs::read(dir.join("notes.txt")).unwrap(), b"keep");
    assert!(dir.join("owner").exists());

    // A directory with files but no owner record.
    let (_dir, root) = canonical_dir();
    let path = root.join("dev.db");
    let dir = staging(&path);
    fs::create_dir(&dir).unwrap();
    fs::write(dir.join("reference.db"), b"whose?").unwrap();
    refused(Storage::open(&path, ISSUER).await, "unrecognized");
    assert!(dir.join("reference.db").exists());
}

#[tokio::test]
async fn an_empty_or_owned_staging_directory_is_reclaimed_and_siblings_survive() {
    let (_dir, root) = canonical_dir();
    let path = root.join("dev.db");
    for sibling in ["dev.db.init", "dev.db2", "notes.txt"] {
        fs::write(root.join(sibling), sibling).unwrap();
    }
    fs::create_dir(staging(&path)).unwrap();
    drop(Storage::open(&path, ISSUER).await.unwrap());
    assert!(!staging(&path).exists());

    let dir = owned_staging(&path);
    fs::write(dir.join("reference.db"), b"leftover").unwrap();
    fs::write(dir.join("reference.db-journal"), b"leftover").unwrap();
    let _storage = Storage::open(&path, ISSUER).await.unwrap();
    assert!(!staging(&path).exists());
    assert_eq!(count(&path, "users").await, 2);
    for sibling in ["dev.db.init", "dev.db2", "notes.txt"] {
        assert_eq!(fs::read_to_string(root.join(sibling)).unwrap(), sibling);
    }
}

#[tokio::test]
async fn an_existing_file_is_migrated_but_never_seeded() {
    let (_dir, root) = canonical_dir();
    let path = root.join("dev.db");
    let mut conn = connect(&path).await.unwrap();
    sqlx::raw_sql("CREATE TABLE unrelated (x); INSERT INTO unrelated VALUES (7)")
        .execute(&mut conn)
        .await
        .unwrap();
    conn.close().await.unwrap();

    let _storage = Storage::open(&path, ISSUER).await.unwrap();
    let mut conn = connect(&path).await.unwrap();
    let versions: Vec<i64> = sqlx::query_scalar("SELECT version FROM _sqlx_migrations")
        .fetch_all(&mut conn)
        .await
        .unwrap();
    assert_eq!(versions, [1]);
    let x: i64 = sqlx::query_scalar("SELECT x FROM unrelated")
        .fetch_one(&mut conn)
        .await
        .unwrap();
    assert_eq!(x, 7);
    conn.close().await.unwrap();
    assert_eq!(count(&path, "users").await, 0);
    assert_eq!(count(&path, "memberships").await, 0);
}

#[tokio::test]
async fn a_database_in_another_journal_mode_is_refused_not_converted() {
    let (_dir, root) = canonical_dir();
    let path = root.join("dev.db");
    drop(Storage::open(&path, ISSUER).await.unwrap());
    let mut conn = connect(&path).await.unwrap();
    sqlx::query("PRAGMA journal_mode=WAL")
        .execute(&mut conn)
        .await
        .unwrap();
    conn.close().await.unwrap();

    refused(Storage::open(&path, ISSUER).await, "wal");
    assert_eq!(journal_mode(&path).await, "wal");
}

#[tokio::test]
async fn an_existing_target_is_never_replaced_by_initialization() {
    let (_dir, root) = canonical_dir();
    let path = root.join("dev.db");
    fs::write(&path, b"someone else's").unwrap();
    // Bypasses the lock, as a race between two initializers would.
    let paths = resolve(&path).unwrap();
    assert!(initialize(&paths, ISSUER).await.is_err());
    assert_eq!(fs::read(&path).unwrap(), b"someone else's");
}

#[tokio::test]
async fn a_failed_initialization_releases_the_path_for_the_next_start() {
    let (_dir, root) = canonical_dir();
    let path = root.join("dev.db");
    fail_after_migration(&path, true);
    let failed = Storage::open(&path, ISSUER).await;
    fail_after_migration(&path, false);
    assert!(failed.is_err());
    assert!(!path.exists());
    // Lock reacquisition and recovery, not proof that no descriptor lingered.
    let _storage = Storage::open(&path, ISSUER).await.unwrap();
    assert!(!staging(&path).exists());
    assert_eq!(count(&path, "users").await, 2);
}

#[tokio::test]
async fn a_disposable_database_is_seeded_and_removed_on_drop() {
    let storage = Storage::disposable(ISSUER).await.unwrap();
    let path = storage.path().to_owned();
    assert_eq!(count(&path, "users").await, 2);
    drop(storage);
    assert!(!path.parent().unwrap().exists());
}

/// Owns a child from spawn, so a failed assertion cannot leak it.
struct Owned(Child);
impl Drop for Owned {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Runs the initializer in a child process of this test binary until it
/// reaches `stage`, then kills it without letting anything clean up.
fn kill_at(stage: &str, path: &Path) {
    let mut child = Owned(
        Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "storage::tests::interrupted_child",
                "--nocapture",
                "--test-threads=1",
            ])
            .env(CHILD_PATH, path)
            .env(BARRIER, stage)
            .stdout(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let (send, receive) = mpsc::channel();
    let stdout = child.0.stdout.take().unwrap();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if send.send(line).is_err() {
                break;
            }
        }
    });
    let marker = format!("barrier {stage}");
    loop {
        let line = receive
            .recv_timeout(Duration::from_secs(60))
            .unwrap_or_else(|_| panic!("child never reached {stage}"));
        // libtest's own "test ... " prefix shares the marker's line.
        if line.ends_with(&marker) {
            break;
        }
    }
    child.0.kill().unwrap();
    child.0.wait().unwrap();
}

/// The child half of `kill_at`; a no-op in an ordinary test run.
#[tokio::test]
async fn interrupted_child() {
    let Some(path) = std::env::var_os(CHILD_PATH) else {
        return;
    };
    let _ = Storage::open(Path::new(&path), ISSUER).await;
    unreachable!("the barrier blocks until the parent kills this process");
}

#[tokio::test]
async fn a_kill_before_publication_leaves_no_database() {
    for stage in ["after-migrate", "after-seed"] {
        let (_dir, root) = canonical_dir();
        let path = root.join("dev.db");
        kill_at(stage, &path);
        assert!(!path.exists(), "{stage}");
        assert!(staging(&path).join("owner").exists(), "{stage}");
        let _storage = Storage::open(&path, ISSUER).await.unwrap();
        assert!(!staging(&path).exists(), "{stage}");
        assert_eq!(count(&path, "users").await, 2, "{stage}");
        assert_eq!(count(&path, "memberships").await, 3, "{stage}");
    }
}

#[tokio::test]
async fn a_kill_after_publication_is_recovered_without_reseeding() {
    let (_dir, root) = canonical_dir();
    let path = root.join("dev.db");
    kill_at("after-link", &path);
    assert_eq!(fs::metadata(&path).unwrap().nlink(), 2);
    let _storage = Storage::open(&path, ISSUER).await.unwrap();
    assert!(!staging(&path).exists());
    assert_eq!(fs::metadata(&path).unwrap().nlink(), 1);
    assert_eq!(count(&path, "users").await, 2);
    assert_eq!(count(&path, "memberships").await, 3);
}
