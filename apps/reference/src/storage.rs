//! Development database lifecycle (S18): one owner per database path, atomic
//! initialization with the development fixtures, and the rollback journal.
use std::{
    fmt, fs,
    io::{self, Write},
    os::unix::{ffi::OsStrExt, fs::MetadataExt},
    path::{Path, PathBuf},
};

use sqlx::{Connection, SqliteConnection};

use crate::app::{MIGRATOR, connect, seed};

/// An owned development database. Dropping it releases the ownership lock,
/// and for a disposable database removes its directory.
pub struct Storage {
    path: PathBuf,
    _lock: fs::File,
    _dir: Option<tempfile::TempDir>,
}

#[derive(Debug)]
pub enum StorageError {
    /// Another process owns the database path.
    InUse(PathBuf),
    /// The path or its surroundings are unsafe to open. A refusal before the
    /// staging directory is reclaimed changes nothing.
    Refused(String),
    Io(io::Error),
    Database(sqlx::Error),
    Migrate(sqlx::migrate::MigrateError),
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InUse(path) => write!(
                f,
                "database {} is in use by another reference-dev process",
                path.display()
            ),
            Self::Refused(message) => f.write_str(message),
            Self::Io(error) => write!(f, "database storage: {error}"),
            Self::Database(error) => write!(f, "database: {error}"),
            Self::Migrate(error) => write!(f, "migration: {error}"),
        }
    }
}

impl std::error::Error for StorageError {}

impl From<io::Error> for StorageError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<sqlx::Error> for StorageError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

impl From<sqlx::migrate::MigrateError> for StorageError {
    fn from(error: sqlx::migrate::MigrateError) -> Self {
        Self::Migrate(error)
    }
}

impl Storage {
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// A database in a fresh temporary directory, initialized like any other.
    pub async fn disposable(issuer: &str) -> Result<Self, StorageError> {
        let dir = tempfile::tempdir()?;
        let mut storage = Self::open(&dir.path().join("reference.db"), issuer).await?;
        storage._dir = Some(dir);
        Ok(storage)
    }

    /// Takes ownership of `path`, then migrates an existing database or
    /// initializes and seeds a new one. Paths, the staging directory and link
    /// counts are validated before any staging file is deleted; the journal
    /// check runs after a valid staging directory is reclaimed.
    pub async fn open(path: &Path, issuer: &str) -> Result<Self, StorageError> {
        let paths = resolve(path)?;
        let lock = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&paths.lock)?;
        match lock.try_lock() {
            Ok(()) => {}
            Err(fs::TryLockError::WouldBlock) => return Err(StorageError::InUse(paths.target)),
            Err(fs::TryLockError::Error(error)) => return Err(error.into()),
        }
        let staging = inspect_staging(&paths)?;
        match fs::symlink_metadata(&paths.target) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let stale: Vec<String> = SIDECARS
                    .iter()
                    .map(|suffix| with_suffix(&paths.target, suffix))
                    .filter(|sidecar| fs::symlink_metadata(sidecar).is_ok())
                    .map(|sidecar| sidecar.display().to_string())
                    .collect();
                if !stale.is_empty() {
                    return Err(StorageError::Refused(format!(
                        "database {} does not exist, but {} remain from an earlier database; \
                         use a fresh path",
                        paths.target.display(),
                        stale.join(", ")
                    )));
                }
                reclaim(&paths, staging)?;
                initialize(&paths, issuer).await?;
            }
            Err(error) => return Err(error.into()),
            Ok(meta) => {
                if !meta.is_file() {
                    return Err(StorageError::Refused(format!(
                        "database {} is not a regular file",
                        paths.target.display()
                    )));
                }
                let published = meta.nlink() == 2
                    && matches!(&staging, Staging::Owned(Some(staged))
                        if staged.dev() == meta.dev() && staged.ino() == meta.ino());
                if meta.nlink() != 1 && !published {
                    return Err(hard_link(&paths.target));
                }
                reclaim(&paths, staging)?;
                if fs::metadata(&paths.target)?.nlink() != 1 {
                    return Err(hard_link(&paths.target));
                }
                let mut conn = connect(&paths.target).await?;
                let migrated = migrate_existing(&mut conn, &paths.target).await;
                let closed = conn.close().await;
                migrated?;
                closed?;
            }
        }
        Ok(Self {
            path: paths.target,
            _lock: lock,
            _dir: None,
        })
    }
}

/// SQLite's sidecar suffixes; the staging directory may hold them too.
const SIDECARS: [&str; 3] = ["-journal", "-wal", "-shm"];
/// The initializer's own names; a database may not use them, in any case.
const RESERVED: [&str; 2] = [".iris-lock", ".iris-init"];
const STAGED: &str = "reference.db";
const OWNER: &str = "owner";

/// The canonical database path and the names derived from it.
struct Paths {
    target: PathBuf,
    /// Never deleted, so it stays stable across initialization and reset.
    lock: PathBuf,
    /// A directory holding `OWNER` and `STAGED` while a database is built.
    staging: PathBuf,
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

fn reserved(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    let base = SIDECARS
        .iter()
        .find_map(|suffix| name.strip_suffix(suffix))
        .unwrap_or(&name);
    RESERVED.iter().any(|suffix| base.ends_with(suffix))
}

fn hard_link(target: &Path) -> StorageError {
    StorageError::Refused(format!(
        "database {} has another hard link; open it by a single name",
        target.display()
    ))
}

/// Resolves one identity for the database before any lock is derived: the
/// parent must exist and is canonicalized, and the file itself may not be a
/// symbolic link.
fn resolve(path: &Path) -> Result<Paths, StorageError> {
    let refuse =
        |why: String| StorageError::Refused(format!("database path {}: {why}", path.display()));
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| refuse("needs a UTF-8 file name".into()))?;
    if reserved(name) {
        return Err(refuse(
            "the file name ends in a reserved .iris-lock or .iris-init suffix".into(),
        ));
    }
    let parent = match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    let parent = fs::canonicalize(parent)
        .map_err(|error| refuse(format!("its parent directory cannot be resolved: {error}")))?;
    if !parent.is_dir() {
        return Err(refuse("its parent directory is not a directory".into()));
    }
    if parent
        .components()
        .any(|part| part.as_os_str().to_str().is_some_and(reserved))
    {
        return Err(refuse(
            "it lies inside a reserved .iris-init directory".into(),
        ));
    }
    let target = parent.join(name);
    match fs::symlink_metadata(&target) {
        Ok(meta) if meta.file_type().is_symlink() => {
            return Err(refuse(
                "it is a symbolic link; give the path of the database itself".into(),
            ));
        }
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    Ok(Paths {
        lock: parent.join(format!("{name}.iris-lock")),
        staging: parent.join(format!("{name}.iris-init")),
        target,
    })
}

fn owner_record(target: &Path) -> Vec<u8> {
    let mut record = target.as_os_str().as_bytes().to_vec();
    record.push(b'\n');
    record
}

/// What occupies the staging name, validated without changing anything.
enum Staging {
    Absent,
    Empty,
    /// Written by this path's initializer, with the staged database if any.
    Owned(Option<fs::Metadata>),
}

fn inspect_staging(paths: &Paths) -> Result<Staging, StorageError> {
    let unrecognized = || {
        StorageError::Refused(format!(
            "{} is an unrecognized occupant of the reserved staging name; \
             remove it by hand if it is not needed",
            paths.staging.display()
        ))
    };
    match fs::symlink_metadata(&paths.staging) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Staging::Absent),
        Err(error) => return Err(error.into()),
        Ok(meta) if !meta.is_dir() => return Err(unrecognized()),
        Ok(_) => {}
    }
    let mut names = Vec::new();
    for entry in fs::read_dir(&paths.staging)? {
        names.push(entry?.file_name());
    }
    if names.is_empty() {
        return Ok(Staging::Empty);
    }
    let known = |name: &std::ffi::OsStr| {
        name == OWNER
            || name == STAGED
            || SIDECARS
                .iter()
                .any(|suffix| name.to_str() == Some(&format!("{STAGED}{suffix}")))
    };
    if !names.iter().all(|name| known(name)) {
        return Err(unrecognized());
    }
    let owner = paths.staging.join(OWNER);
    match fs::symlink_metadata(&owner) {
        Ok(meta) if meta.is_file() => {}
        _ => return Err(unrecognized()),
    }
    if fs::read(&owner)? != owner_record(&paths.target) {
        return Err(unrecognized());
    }
    match fs::symlink_metadata(paths.staging.join(STAGED)) {
        Ok(meta) if meta.is_file() => Ok(Staging::Owned(Some(meta))),
        Ok(_) => Err(unrecognized()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Staging::Owned(None)),
        Err(error) => Err(error.into()),
    }
}

fn remove_if_present(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}

/// Removes a validated staging directory: database files first and the owner
/// record last, so an interruption leaves it recognizable or empty. Never
/// recursive.
fn reclaim(paths: &Paths, staging: Staging) -> io::Result<()> {
    if let Staging::Absent = staging {
        return Ok(());
    }
    let staged = paths.staging.join(STAGED);
    remove_if_present(&staged)?;
    for suffix in SIDECARS {
        remove_if_present(&with_suffix(&staged, suffix))?;
    }
    remove_if_present(&paths.staging.join(OWNER))?;
    fs::remove_dir(&paths.staging)
}

/// Builds a database in the staging directory and publishes it under the
/// target name only if that name is still free. Callers hold the lock.
async fn initialize(paths: &Paths, issuer: &str) -> Result<(), StorageError> {
    fs::create_dir(&paths.staging)?;
    let mut owner = fs::File::create_new(paths.staging.join(OWNER))?;
    owner.write_all(&owner_record(&paths.target))?;
    owner.sync_all()?;
    drop(owner);
    let staged = paths.staging.join(STAGED);
    let mut conn = connect(&staged).await?;
    let built = build(&mut conn, &paths.target, issuer).await;
    // Closed, and its closure awaited, before publication or any return.
    let closed = conn.close().await;
    built?;
    closed?;
    fs::hard_link(&staged, &paths.target).map_err(|error| {
        if error.kind() == io::ErrorKind::AlreadyExists {
            StorageError::Refused(format!(
                "database {} appeared during initialization and was left untouched",
                paths.target.display()
            ))
        } else {
            error.into()
        }
    })?;
    checkpoint("after-link", &paths.target)?;
    reclaim(paths, Staging::Owned(None))?;
    Ok(())
}

async fn build(
    conn: &mut SqliteConnection,
    target: &Path,
    issuer: &str,
) -> Result<(), StorageError> {
    check_journal(conn, target).await?;
    MIGRATOR.run(&mut *conn).await?;
    checkpoint("after-migrate", target)?;
    seed_development(conn, issuer).await?;
    checkpoint("after-seed", target)
}

/// An existing database is migrated, never seeded, so a restart keeps its data.
async fn migrate_existing(conn: &mut SqliteConnection, target: &Path) -> Result<(), StorageError> {
    check_journal(conn, target).await?;
    MIGRATOR.run(&mut *conn).await?;
    Ok(())
}

/// Refuses rather than converts: a file switched to WAL elsewhere keeps WAL.
async fn check_journal(conn: &mut SqliteConnection, target: &Path) -> Result<(), StorageError> {
    let mode: String = sqlx::query_scalar("PRAGMA journal_mode")
        .fetch_one(&mut *conn)
        .await?;
    if mode.eq_ignore_ascii_case("delete") {
        Ok(())
    } else {
        Err(StorageError::Refused(format!(
            "database {} uses journal mode {mode}; the reference application requires the \
             rollback journal (delete) and does not convert a database",
            target.display()
        )))
    }
}

/// The development fixture set, applied only to a database this initializer
/// created.
async fn seed_development(conn: &mut SqliteConnection, issuer: &str) -> Result<(), sqlx::Error> {
    seed(conn, issuer).await?;
    // Without invitations, a successful role change or removal needs a
    // non-owner member; the membership tests insert the same row.
    sqlx::query("INSERT INTO memberships (project_id, user_id, role) VALUES (41, 29, 'editor')")
        .execute(&mut *conn)
        .await?;
    Ok(())
}

#[cfg(not(test))]
fn checkpoint(_stage: &str, _target: &Path) -> Result<(), StorageError> {
    Ok(())
}

#[cfg(test)]
const CHILD_PATH: &str = "IRIS_STORAGE_CHILD_PATH";
#[cfg(test)]
const BARRIER: &str = "IRIS_STORAGE_BARRIER";
#[cfg(test)]
static FAILURES: std::sync::Mutex<Vec<PathBuf>> = std::sync::Mutex::new(Vec::new());

/// Test-only interruption points: a child process blocks at `BARRIER` until
/// it is killed, and a registered path fails after migration.
#[cfg(test)]
fn checkpoint(stage: &str, target: &Path) -> Result<(), StorageError> {
    if std::env::var(BARRIER).as_deref() == Ok(stage) {
        println!("barrier {stage}");
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }
    if stage == "after-migrate" && FAILURES.lock().unwrap().iter().any(|path| path == target) {
        return Err(StorageError::Refused(
            "injected failure after migration".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
fn fail_after_migration(path: &Path, on: bool) {
    let mut failures = FAILURES.lock().unwrap();
    failures.retain(|failing| failing != path);
    if on {
        failures.push(path.to_owned());
    }
}

#[cfg(test)]
mod tests;
