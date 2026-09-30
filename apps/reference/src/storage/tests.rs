// Each test owns a single-threaded runtime, so holding `PROCESSES` across an
// await blocks only that test's thread, which is the intent.
#![allow(clippy::await_holding_lock)]

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
    let _processes = shared();
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
    let _processes = shared();
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
    let _processes = shared();
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
    let _processes = shared();
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
    let _processes = shared();
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
    let _processes = shared();
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
    let _processes = shared();
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
    let _processes = shared();
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
    let _processes = shared();
    for sidecar in ["-journal", "-wal", "-shm"] {
        let (_dir, root) = canonical_dir();
        let path = root.join("dev.db");
        let stale = root.join(format!("dev.db{sidecar}"));
        fs::write(&stale, b"stale").unwrap();
        refused(
            Storage::open(&path, ISSUER).await,
            &format!("dev.db{sidecar}"),
        );
        refused(Storage::open(&path, ISSUER).await, "--reset");
        assert!(!path.exists() && !staging(&path).exists());
        assert_eq!(fs::read(&stale).unwrap(), b"stale");
    }
}

#[tokio::test]
async fn reserved_names_are_refused_in_any_case() {
    let _processes = shared();
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
    let _processes = shared();
    let (_dir, root) = canonical_dir();
    refused(
        Storage::open(&root.join("absent/dev.db"), ISSUER).await,
        "parent directory",
    );
    assert!(!root.join("absent").exists());
}

#[tokio::test]
async fn unrecognized_staging_occupants_are_refused_and_kept() {
    let _processes = shared();
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
    let _processes = shared();
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
    let _processes = shared();
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
    let _processes = shared();
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
    let _processes = shared();
    let (_dir, root) = canonical_dir();
    let path = root.join("dev.db");
    fs::write(&path, b"someone else's").unwrap();
    // Bypasses the lock, as a race between two initializers would.
    let paths = resolve(&path).unwrap();
    assert!(initialize(&paths, ISSUER, &MIGRATOR).await.is_err());
    assert_eq!(fs::read(&path).unwrap(), b"someone else's");
}

#[tokio::test]
async fn a_failed_initialization_releases_the_path_for_the_next_start() {
    let _processes = shared();
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
    let _processes = exclusive();
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

/// Selects what `interrupted_child` runs; the initializer by default.
const MODE: &str = "IRIS_STORAGE_CHILD_MODE";

/// Runs `mode` in a child process of this test binary until it prints
/// `barrier <stage>`, and returns the child still blocked there.
fn blocked_child(mode: &str, stage: &str, path: &Path) -> Owned {
    let mut child = Owned(
        Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "storage::tests::interrupted_child",
                "--nocapture",
                "--test-threads=1",
            ])
            .env(CHILD_PATH, path)
            .env(MODE, mode)
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
    child
}

/// Kills a blocked child without letting anything clean up.
fn kill(mut child: Owned) {
    child.0.kill().unwrap();
    child.0.wait().unwrap();
}

/// Runs the initializer in a child process until it reaches `stage`, then
/// kills it.
fn kill_at(stage: &str, path: &Path) {
    kill(blocked_child("open", stage, path));
}

/// What a binary from before the sidecar-name rule did for a new database:
/// the same lock and initialization, without refusing the name.
async fn legacy_open(path: &Path) -> Result<fs::File, StorageError> {
    let paths = derive(path)?;
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&paths.lock)?;
    lock.try_lock().unwrap();
    initialize(&paths, ISSUER, &MIGRATOR).await?;
    Ok(lock)
}

/// The child half of `blocked_child`; a no-op in an ordinary test run.
#[tokio::test]
async fn interrupted_child() {
    let Some(path) = std::env::var_os(CHILD_PATH) else {
        return;
    };
    let path = Path::new(&path);
    match std::env::var(MODE).as_deref() {
        Ok("reset") => drop(Storage::reset(path, ISSUER).await),
        Ok("legacy") => drop(legacy_open(path).await),
        // Leaves a committed change only in the write-ahead log.
        Ok("wal") => {
            let mut conn = connect(path).await.unwrap();
            for sql in [
                "PRAGMA journal_mode=WAL",
                "CREATE TABLE IF NOT EXISTS probe (v TEXT)",
                "DELETE FROM probe",
                "INSERT INTO probe VALUES ('after')",
            ] {
                sqlx::query(sql).fetch_all(&mut conn).await.unwrap();
            }
            println!("barrier wal");
            loop {
                std::thread::sleep(Duration::from_secs(1));
            }
        }
        _ => drop(Storage::open(path, ISSUER).await),
    }
    unreachable!("the barrier blocks until the parent kills this process");
}

#[tokio::test]
async fn a_kill_before_publication_leaves_no_database() {
    let _processes = exclusive();
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
    let _processes = exclusive();
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

/// Every entry under `root`: its type, link count and a digest of its bytes.
fn tree(root: &Path) -> Vec<(PathBuf, String)> {
    use std::hash::{Hash, Hasher};
    let mut entries = Vec::new();
    let mut pending = vec![root.to_owned()];
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            let meta = fs::symlink_metadata(&path).unwrap();
            let what = if meta.is_dir() {
                pending.push(path.clone());
                "directory".to_owned()
            } else if meta.file_type().is_symlink() {
                format!("link to {}", fs::read_link(&path).unwrap().display())
            } else {
                let bytes = fs::read(&path).unwrap();
                let mut digest = std::collections::hash_map::DefaultHasher::new();
                bytes.hash(&mut digest);
                format!(
                    "file, {} links, {} bytes, {:x}",
                    meta.nlink(),
                    bytes.len(),
                    digest.finish()
                )
            };
            entries.push((path, what));
        }
    }
    entries.sort();
    entries
}

/// Both `open` and `reset` refuse `path`, and nothing under `root` changes.
async fn refused_and_kept(root: &Path, path: &Path, needle: &str) {
    let before = tree(root);
    refused(Storage::open(path, ISSUER).await, needle);
    assert_eq!(tree(root), before, "open changed something");
    refused(Storage::reset(path, ISSUER).await, needle);
    assert_eq!(tree(root), before, "reset changed something");
}

/// The changes `a_restart_keeps_changes_and_does_not_reseed` makes.
async fn change(path: &Path) {
    let mut conn = connect(path).await.unwrap();
    sqlx::raw_sql(
        "UPDATE memberships SET role='viewer' WHERE project_id=41 AND user_id=29;
        DELETE FROM memberships WHERE project_id=43",
    )
    .execute(&mut conn)
    .await
    .unwrap();
    conn.close().await.unwrap();
}

async fn bob_role(path: &Path) -> String {
    let mut conn = connect(path).await.unwrap();
    let role =
        sqlx::query_scalar("SELECT role FROM memberships WHERE project_id=41 AND user_id=29")
            .fetch_one(&mut conn)
            .await
            .unwrap();
    conn.close().await.unwrap();
    role
}

fn sidecars(path: &Path) -> Vec<PathBuf> {
    SIDECARS
        .iter()
        .map(|suffix| with_suffix(path, suffix))
        .collect()
}

#[tokio::test]
async fn a_reset_discards_changes_and_sessions_and_keeps_the_lock() {
    let _processes = shared();
    let (_dir, root) = canonical_dir();
    let path = root.join("dev.db");
    drop(Storage::open(&path, ISSUER).await.unwrap());
    change(&path).await;
    let mut conn = connect(&path).await.unwrap();
    sqlx::raw_sql(
        "INSERT INTO iris_sessions (id, data, expires_at) VALUES ('s', '{}', 9999999999);
        INSERT INTO iris_login_attempts (state, browser_id, nonce, verifier, expires_at)
            VALUES ('a', 'b', 'n', 'v', 9999999999)",
    )
    .execute(&mut conn)
    .await
    .unwrap();
    conn.close().await.unwrap();
    let lock = fs::metadata(root.join("dev.db.iris-lock")).unwrap().ino();

    let storage = Storage::reset(&path, ISSUER).await.unwrap();
    assert_eq!(storage.path(), path);
    assert_eq!(count(&path, "users").await, 2);
    assert_eq!(count(&path, "memberships").await, 3);
    assert_eq!(count(&path, "iris_external_identities").await, 2);
    assert_eq!(count(&path, "iris_sessions").await, 0);
    assert_eq!(count(&path, "iris_login_attempts").await, 0);
    assert_eq!(bob_role(&path).await, "editor");
    assert_eq!(journal_mode(&path).await, "delete");
    assert!(!staging(&path).exists());
    assert_eq!(
        fs::metadata(root.join("dev.db.iris-lock")).unwrap().ino(),
        lock
    );
    assert_eq!(fs::metadata(&path).unwrap().nlink(), 1);
    // The reset owns the path until its `Storage` is dropped.
    assert!(matches!(
        Storage::open(&path, ISSUER).await,
        Err(StorageError::InUse(_))
    ));
    drop(storage);

    change(&path).await;
    drop(Storage::open(&path, ISSUER).await.unwrap());
    assert_eq!(bob_role(&path).await, "viewer");
}

#[tokio::test]
async fn a_reset_is_refused_while_an_owner_holds_the_database() {
    let _processes = shared();
    let (_dir, root) = canonical_dir();
    let path = root.join("dev.db");
    let _owner = Storage::open(&path, ISSUER).await.unwrap();
    change(&path).await;
    let before = tree(&root);
    match Storage::reset(&path, ISSUER).await {
        Err(StorageError::InUse(named)) => assert_eq!(named, path),
        Err(other) => panic!("expected in use, got {other}"),
        Ok(_) => panic!("reset a database that is in use"),
    }
    assert_eq!(tree(&root), before);
    assert_eq!(bob_role(&path).await, "viewer");
}

#[tokio::test]
async fn a_reset_is_refused_while_an_initialization_holds_the_database() {
    let _processes = exclusive();
    let (_dir, root) = canonical_dir();
    let path = root.join("dev.db");
    let child = blocked_child("open", "after-migrate", &path);
    let before = tree(&root);
    assert!(matches!(
        Storage::reset(&path, ISSUER).await,
        Err(StorageError::InUse(_))
    ));
    assert_eq!(tree(&root), before);
    kill(child);
    assert!(staging(&path).join("owner").exists() && !path.exists());
}

#[tokio::test]
async fn a_reset_removes_every_sidecar() {
    let _processes = exclusive();
    // A database left in WAL mode by a killed process, with both WAL files.
    let (_dir, root) = canonical_dir();
    let path = root.join("dev.db");
    drop(Storage::open(&path, ISSUER).await.unwrap());
    kill(blocked_child("wal", "wal", &path));
    // Nothing opens the database in between: a connection that closes would
    // remove the files this reset must remove.
    assert!(root.join("dev.db-wal").exists() && root.join("dev.db-shm").exists());
    drop(Storage::reset(&path, ISSUER).await.unwrap());
    for sidecar in sidecars(&path) {
        assert!(!sidecar.exists(), "{}", sidecar.display());
    }
    assert_eq!(journal_mode(&path).await, "delete");
    assert_eq!(count(&path, "users").await, 2);

    // Each sidecar alone, beside an existing database.
    for sidecar in sidecars(&path) {
        fs::write(&sidecar, b"stale").unwrap();
        drop(Storage::reset(&path, ISSUER).await.unwrap());
        assert!(!sidecar.exists(), "{}", sidecar.display());
        assert_eq!(count(&path, "users").await, 2);
    }
}

#[tokio::test]
async fn a_reset_initializes_a_missing_database_with_or_without_stale_sidecars() {
    let _processes = shared();
    let (_dir, root) = canonical_dir();
    let path = root.join("dev.db");
    drop(Storage::reset(&path, ISSUER).await.unwrap());
    assert_eq!(count(&path, "memberships").await, 3);

    for sidecar in sidecars(&path) {
        fs::remove_file(&path).unwrap();
        fs::write(&sidecar, b"stale").unwrap();
        refused(Storage::open(&path, ISSUER).await, "--reset");
        drop(Storage::reset(&path, ISSUER).await.unwrap());
        assert!(!sidecar.exists(), "{}", sidecar.display());
        assert_eq!(count(&path, "memberships").await, 3);
    }
}

#[tokio::test]
async fn a_reset_refuses_what_it_cannot_recognize_and_keeps_every_file() {
    let _processes = shared();
    // Refused before the lock file is created.
    let (_dir, root) = canonical_dir();
    let path = root.join("dev.db");
    drop(Storage::open(&path, ISSUER).await.unwrap());
    std::os::unix::fs::symlink(&path, root.join("link.db")).unwrap();
    let before = tree(&root);
    refused(
        Storage::reset(&root.join("link.db"), ISSUER).await,
        "symbolic link",
    );
    refused(
        Storage::reset(&root.join("other.db.Iris-Lock"), ISSUER).await,
        "reserved",
    );
    refused(
        Storage::reset(&root.join("absent/dev.db"), ISSUER).await,
        "parent directory",
    );
    assert_eq!(tree(&root), before);

    // A foreign hard link.
    fs::hard_link(&path, root.join("other.db")).unwrap();
    let before = tree(&root);
    refused(Storage::reset(&path, ISSUER).await, "hard link");
    assert_eq!(tree(&root), before);

    // Files that are not plausibly a SQLite database, and a directory.
    let (_dir, root) = canonical_dir();
    let path = root.join("dev.db");
    drop(Storage::open(&path, ISSUER).await.unwrap());
    let mut damaged = fs::read(&path).unwrap();
    damaged[0] = b'X';
    fs::write(&path, damaged).unwrap();
    fs::write(root.join("notes.txt"), b"notes").unwrap();
    fs::write(root.join("notes.txt.iris-lock"), b"").unwrap();
    fs::create_dir(root.join("folder")).unwrap();
    fs::write(root.join("folder.iris-lock"), b"").unwrap();
    let before = tree(&root);
    refused(Storage::reset(&path, ISSUER).await, "SQLite database");
    refused(
        Storage::reset(&root.join("notes.txt"), ISSUER).await,
        "SQLite database",
    );
    refused(
        Storage::reset(&root.join("folder"), ISSUER).await,
        "not a regular file",
    );
    assert_eq!(tree(&root), before);

    // An unrecognized staging occupant, beside an intact database.
    let (_dir, root) = canonical_dir();
    let path = root.join("dev.db");
    drop(Storage::open(&path, ISSUER).await.unwrap());
    let dir = owned_staging(&path);
    fs::write(dir.join("notes.txt"), b"keep").unwrap();
    let before = tree(&root);
    refused(Storage::reset(&path, ISSUER).await, "unrecognized");
    assert_eq!(tree(&root), before);
}

#[tokio::test]
async fn a_reset_replaces_an_empty_file() {
    let _processes = shared();
    let (_dir, root) = canonical_dir();
    let path = root.join("dev.db");
    fs::write(&path, b"").unwrap();
    drop(Storage::reset(&path, ISSUER).await.unwrap());
    assert_eq!(count(&path, "users").await, 2);
}

#[tokio::test]
async fn a_reset_reclaims_an_interrupted_initialization() {
    let _processes = exclusive();
    for stage in ["after-seed", "after-link"] {
        let (_dir, root) = canonical_dir();
        let path = root.join("dev.db");
        kill_at(stage, &path);
        assert!(staging(&path).join("owner").exists(), "{stage}");
        drop(Storage::reset(&path, ISSUER).await.unwrap());
        assert!(!staging(&path).exists(), "{stage}");
        assert_eq!(fs::metadata(&path).unwrap().nlink(), 1, "{stage}");
        assert_eq!(count(&path, "memberships").await, 3, "{stage}");
    }
}

#[tokio::test]
async fn an_interrupted_reset_is_completed_by_the_next_reset() {
    let _processes = exclusive();
    // Killed between deleting the database and deleting its sidecars.
    let (_dir, root) = canonical_dir();
    let path = root.join("dev.db");
    drop(Storage::open(&path, ISSUER).await.unwrap());
    change(&path).await;
    let journal = root.join("dev.db-journal");
    fs::write(&journal, b"stale").unwrap();
    kill(blocked_child("reset", "after-remove", &path));
    assert!(!path.exists());
    assert_eq!(fs::read(&journal).unwrap(), b"stale");
    refused(Storage::open(&path, ISSUER).await, "--reset");
    assert!(!path.exists() && !staging(&path).exists());
    drop(Storage::reset(&path, ISSUER).await.unwrap());
    assert!(!journal.exists());
    assert_eq!(bob_role(&path).await, "editor");

    // Killed while the reset initializes the replacement.
    change(&path).await;
    kill(blocked_child("reset", "after-seed", &path));
    assert!(!path.exists());
    assert!(staging(&path).join("owner").exists());
    drop(Storage::open(&path, ISSUER).await.unwrap());
    assert!(!staging(&path).exists());
    assert_eq!(bob_role(&path).await, "editor");
    assert_eq!(count(&path, "memberships").await, 3);
}

#[tokio::test]
async fn a_sidecar_name_is_refused_as_a_database() {
    let _processes = shared();
    let (_dir, root) = canonical_dir();
    for name in ["dev.db-wal", "dev.db-JOURNAL", "dev.db-shm"] {
        refused(Storage::open(&root.join(name), ISSUER).await, "reserved");
        refused(Storage::reset(&root.join(name), ISSUER).await, "reserved");
    }
    assert_eq!(fs::read_dir(&root).unwrap().count(), 0);
}

#[tokio::test]
async fn a_database_at_a_sidecar_name_is_refused_and_kept() {
    let _processes = shared();
    for suffix in SIDECARS {
        // Owned by a binary from before the name rule, and in use.
        let (_dir, root) = canonical_dir();
        let path = root.join("dev.db");
        drop(Storage::open(&path, ISSUER).await.unwrap());
        let legacy = with_suffix(&path, suffix);
        drop(legacy_open(&legacy).await.unwrap());
        let mut live = connect(&legacy).await.unwrap();
        refused_and_kept(&root, &path, "separate database").await;
        let users: i64 = sqlx::query_scalar("SELECT count(*) FROM users")
            .fetch_one(&mut live)
            .await
            .unwrap();
        assert_eq!(users, 2, "{suffix}");
        live.close().await.unwrap();

        // A SQLite database with no ownership sibling: the header alone.
        let (_dir, root) = canonical_dir();
        let path = root.join("dev.db");
        drop(Storage::open(&path, ISSUER).await.unwrap());
        let mut conn = connect(&with_suffix(&path, suffix)).await.unwrap();
        sqlx::query("CREATE TABLE theirs (x)")
            .execute(&mut conn)
            .await
            .unwrap();
        conn.close().await.unwrap();
        refused_and_kept(&root, &path, "separate database").await;

        // An empty file with a lock sibling: the sibling alone.
        let (_dir, root) = canonical_dir();
        let path = root.join("dev.db");
        drop(Storage::open(&path, ISSUER).await.unwrap());
        let sidecar = with_suffix(&path, suffix);
        fs::write(&sidecar, b"").unwrap();
        fs::write(with_suffix(&sidecar, ".iris-lock"), b"").unwrap();
        refused_and_kept(&root, &path, "separate database").await;
    }
}

#[tokio::test]
async fn an_unexpected_occupant_of_a_sidecar_name_is_refused_and_kept() {
    let _processes = shared();
    for suffix in SIDECARS {
        for kind in ["directory", "symbolic link", "second link"] {
            let (_dir, root) = canonical_dir();
            let path = root.join("dev.db");
            drop(Storage::open(&path, ISSUER).await.unwrap());
            let sidecar = with_suffix(&path, suffix);
            fs::write(root.join("notes.txt"), b"notes").unwrap();
            match kind {
                "directory" => fs::create_dir(&sidecar).unwrap(),
                "symbolic link" => {
                    std::os::unix::fs::symlink(root.join("notes.txt"), &sidecar).unwrap()
                }
                _ => fs::hard_link(root.join("notes.txt"), &sidecar).unwrap(),
            }
            refused_and_kept(&root, &path, "separate database").await;
            assert!(path.exists(), "{suffix} {kind}");
        }
    }
}

#[tokio::test]
async fn ownership_siblings_of_an_absent_sidecar_are_refused_and_kept() {
    let _processes = exclusive();
    for suffix in SIDECARS {
        // Only the lock file of a database at the sidecar name.
        let (_dir, root) = canonical_dir();
        let path = root.join("dev.db");
        drop(Storage::open(&path, ISSUER).await.unwrap());
        let sidecar = with_suffix(&path, suffix);
        fs::write(with_suffix(&sidecar, ".iris-lock"), b"").unwrap();
        refused_and_kept(&root, &path, "separate database").await;

        // Only its staging directory.
        let (_dir, root) = canonical_dir();
        let path = root.join("dev.db");
        drop(Storage::open(&path, ISSUER).await.unwrap());
        let sidecar = with_suffix(&path, suffix);
        fs::create_dir(with_suffix(&sidecar, ".iris-init")).unwrap();
        refused_and_kept(&root, &path, "separate database").await;

        // A live initializer from before the name rule, not yet published.
        let (_dir, root) = canonical_dir();
        let path = root.join("dev.db");
        drop(Storage::open(&path, ISSUER).await.unwrap());
        let sidecar = with_suffix(&path, suffix);
        let child = blocked_child("legacy", "after-migrate", &sidecar);
        assert!(!sidecar.exists(), "{suffix}");
        refused_and_kept(&root, &path, "separate database").await;
        kill(child);
        assert!(staging(&sidecar).join("owner").exists(), "{suffix}");
        assert!(staging(&sidecar).join("reference.db").exists(), "{suffix}");
    }
}

/// The guide's procedure for a database at a sidecar name: the whole file set
/// moves under matching names. Moving the database alone loses what its
/// write-ahead log holds.
#[tokio::test]
async fn moving_a_database_with_its_recovery_files_keeps_committed_changes() {
    let _processes = exclusive();
    async fn probe(path: &Path) -> String {
        let mut conn = connect(path).await.unwrap();
        let value = sqlx::query_scalar("SELECT v FROM probe")
            .fetch_one(&mut conn)
            .await
            .unwrap();
        conn.close().await.unwrap();
        value
    }
    for whole in [true, false] {
        let (_dir, root) = canonical_dir();
        let legacy = root.join("dev.db-wal");
        let mut conn = connect(&legacy).await.unwrap();
        sqlx::raw_sql("CREATE TABLE probe (v TEXT); INSERT INTO probe VALUES ('before')")
            .execute(&mut conn)
            .await
            .unwrap();
        conn.close().await.unwrap();
        kill(blocked_child("wal", "wal", &legacy));
        assert!(root.join("dev.db-wal-wal").exists());

        let moved = root.join("moved.db");
        fs::rename(&legacy, &moved).unwrap();
        if whole {
            for suffix in SIDECARS {
                let from = with_suffix(&legacy, suffix);
                if from.exists() {
                    fs::rename(from, with_suffix(&moved, suffix)).unwrap();
                }
            }
        }
        assert_eq!(probe(&moved).await, if whole { "after" } else { "before" });
    }
}

const INITIAL: &str = include_str!("../../migrations/0001_initial.sql");

/// A migrator resolved at run time from the given files.
async fn migrator(files: &[(&str, &str)]) -> sqlx::migrate::Migrator {
    let dir = tempfile::tempdir().unwrap();
    for (name, sql) in files {
        fs::write(dir.path().join(name), sql).unwrap();
    }
    sqlx::migrate::Migrator::new(dir.path()).await.unwrap()
}

#[tokio::test]
async fn a_modified_applied_migration_stops_startup_and_keeps_the_data() {
    let _processes = shared();
    let (_dir, root) = canonical_dir();
    let path = root.join("dev.db");
    drop(Storage::open(&path, ISSUER).await.unwrap());
    change(&path).await;
    let modified = migrator(&[("0001_initial.sql", &format!("{INITIAL}\n-- edited\n"))]).await;

    let before = snapshot(&path);
    let message = match Storage::open_with(&path, ISSUER, &modified).await {
        Err(StorageError::Refused(message)) => message,
        Err(other) => panic!("expected a refusal, got {other}"),
        Ok(_) => panic!("opened a database whose migration was modified"),
    };
    assert!(message.contains(&path.display().to_string()), "{message}");
    assert!(message.contains("migration 1"), "{message}");
    let restore = message.find("restore").expect(&message);
    let reset = message.find("--reset").expect(&message);
    assert!(restore < reset, "{message}");
    assert_eq!(snapshot(&path), before);
    assert!(!staging(&path).exists());

    // With the migration restored, the retained data is intact.
    drop(Storage::open(&path, ISSUER).await.unwrap());
    assert_eq!(bob_role(&path).await, "viewer");
    assert_eq!(count(&path, "memberships").await, 2);
    assert_eq!(count(&path, "users").await, 2);

    // The refusal does not block the reset it names.
    drop(Storage::reset_with(&path, ISSUER, &modified).await.unwrap());
    assert_eq!(bob_role(&path).await, "editor");
}

#[tokio::test]
async fn an_added_migration_applies_to_an_existing_database() {
    let _processes = shared();
    let (_dir, root) = canonical_dir();
    let path = root.join("dev.db");
    drop(Storage::open(&path, ISSUER).await.unwrap());
    change(&path).await;
    let added = migrator(&[
        ("0001_initial.sql", INITIAL),
        ("0002_added.sql", "CREATE TABLE added (x INTEGER);"),
    ])
    .await;

    drop(Storage::open_with(&path, ISSUER, &added).await.unwrap());
    assert_eq!(count(&path, "added").await, 0);
    let mut conn = connect(&path).await.unwrap();
    let versions: Vec<i64> =
        sqlx::query_scalar("SELECT version FROM _sqlx_migrations ORDER BY version")
            .fetch_all(&mut conn)
            .await
            .unwrap();
    conn.close().await.unwrap();
    assert_eq!(versions, [1, 2]);
    assert_eq!(bob_role(&path).await, "viewer");
    assert_eq!(count(&path, "memberships").await, 2);

    // Any other migration failure names the database.
    match Storage::open(&path, ISSUER).await {
        Err(error @ StorageError::Migrate(..)) => {
            let message = error.to_string();
            assert!(message.contains(&path.display().to_string()), "{message}");
            assert!(message.contains("migration 2"), "{message}");
        }
        Err(other) => panic!("expected a migration error, got {other}"),
        Ok(_) => panic!("opened a database with an unknown applied migration"),
    }
}

/// The quoting alone; `tests/dev_binary.rs` runs a refusal's command through
/// a shell.
#[test]
fn the_reset_command_quotes_its_path_as_one_shell_word() {
    assert_eq!(shell_quote("/tmp/dev-1.db"), "/tmp/dev-1.db");
    assert_eq!(
        shell_quote("/tmp/space dir/dev.db"),
        "'/tmp/space dir/dev.db'"
    );
    assert_eq!(shell_quote("it's"), r"'it'\''s'");
    assert_eq!(shell_quote("$HOME;x"), "'$HOME;x'");
    assert_eq!(shell_quote(""), "''");
    assert_eq!(
        reset_command(Path::new("/tmp/a b.db")),
        "reference-dev --local-oidc-demo --database '/tmp/a b.db' --reset"
    );
}
