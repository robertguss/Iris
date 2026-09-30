//! Development database lifecycle (S18): one owner per database path, atomic
//! initialization with the development fixtures, the rollback journal, and
//! reset.
use std::{
    fmt, fs,
    io::{self, Read, Write},
    os::unix::{ffi::OsStrExt, fs::MetadataExt},
    path::{Path, PathBuf},
};

use sqlx::{
    Connection, SqliteConnection,
    migrate::{MigrateError, Migrator},
};

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
    /// A migration failure other than a modified applied migration, with the
    /// database it happened on.
    Migrate(PathBuf, MigrateError),
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
            Self::Migrate(path, error) => {
                write!(f, "migration of database {}: {error}", path.display())
            }
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
    /// initializes and seeds a new one. Paths, the staging directory, the
    /// sidecar names and link counts are validated before any staging file is
    /// deleted; the journal check runs after a valid staging directory is
    /// reclaimed.
    pub async fn open(path: &Path, issuer: &str) -> Result<Self, StorageError> {
        Self::open_with(path, issuer, &MIGRATOR).await
    }

    /// Takes ownership of `path`, deletes the database with its sidecars and
    /// builds a seeded one in its place. Refused while another process owns
    /// the path. Everything is validated before anything is deleted, and the
    /// old database is never opened.
    pub async fn reset(path: &Path, issuer: &str) -> Result<Self, StorageError> {
        Self::reset_with(path, issuer, &MIGRATOR).await
    }

    async fn open_with(
        path: &Path,
        issuer: &str,
        migrator: &Migrator,
    ) -> Result<Self, StorageError> {
        let Claim {
            paths,
            lock,
            staging,
            exists,
        } = claim(path)?;
        if exists {
            reclaim(&paths, staging)?;
            if fs::metadata(&paths.target)?.nlink() != 1 {
                return Err(hard_link(&paths.target));
            }
            let mut conn = connect(&paths.target).await?;
            let migrated = migrate_existing(&mut conn, &paths.target, migrator).await;
            let closed = conn.close().await;
            migrated?;
            closed?;
        } else {
            let stale: Vec<String> = sidecars(&paths.target)
                .filter(|sidecar| fs::symlink_metadata(sidecar).is_ok())
                .map(|sidecar| sidecar.display().to_string())
                .collect();
            if !stale.is_empty() {
                return Err(StorageError::Refused(format!(
                    "database {} does not exist, but {} remain from an earlier database; \
                     to delete them and start from a new database, run: {}",
                    paths.target.display(),
                    stale.join(", "),
                    reset_command(&paths.target)
                )));
            }
            reclaim(&paths, staging)?;
            initialize(&paths, issuer, migrator).await?;
        }
        Ok(Self {
            path: paths.target,
            _lock: lock,
            _dir: None,
        })
    }

    async fn reset_with(
        path: &Path,
        issuer: &str,
        migrator: &Migrator,
    ) -> Result<Self, StorageError> {
        let Claim {
            paths,
            lock,
            staging,
            exists,
        } = claim(path)?;
        if exists && !plausible_database(&paths.target)? {
            return Err(StorageError::Refused(format!(
                "{} does not look like a SQLite database (it is neither empty nor starts with \
                 SQLite's header) and was left untouched; remove it by hand if it is not needed",
                paths.target.display()
            )));
        }
        reclaim(&paths, staging)?;
        // The database goes before its sidecars: an interruption then leaves
        // a missing database with stale sidecars, which `open` refuses and a
        // second reset completes. The other order could leave a database
        // without its hot journal.
        remove_if_present(&paths.target)?;
        checkpoint("after-remove", &paths.target)?;
        for sidecar in sidecars(&paths.target) {
            remove_if_present(&sidecar)?;
        }
        initialize(&paths, issuer, migrator).await?;
        Ok(Self {
            path: paths.target,
            _lock: lock,
            _dir: None,
        })
    }
}

/// An owned path whose surroundings passed every check, with nothing changed
/// yet except the lock file's creation. A refusal while resolving the path
/// comes before even that; a later one can leave a new, empty lock file.
struct Claim {
    paths: Paths,
    lock: fs::File,
    staging: Staging,
    /// Whether a database exists at the target.
    exists: bool,
}

/// The front half `open` and `reset` share: one identity for the path, the
/// ownership lock, then validation of the staging directory, the sidecar
/// names and the target.
fn claim(path: &Path) -> Result<Claim, StorageError> {
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
    inspect_sidecars(&paths.target)?;
    let exists = match fs::symlink_metadata(&paths.target) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => false,
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
            true
        }
    };
    Ok(Claim {
        paths,
        lock,
        staging,
        exists,
    })
}

/// SQLite's sidecar suffixes; the staging directory may hold them too.
const SIDECARS: [&str; 3] = ["-journal", "-wal", "-shm"];
/// The initializer's own names; a database may not use them, in any case.
const RESERVED: [&str; 2] = [".iris-lock", ".iris-init"];
const STAGED: &str = "reference.db";
const OWNER: &str = "owner";
/// The first bytes of every SQLite database file.
const HEADER: &[u8; 16] = b"SQLite format 3\0";
/// Where the guide explains what to do about a database at a sidecar name.
const GUIDE: &str = "see \"A database at a sidecar name\" in apps/reference/README.md";

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

fn sidecars(target: &Path) -> impl Iterator<Item = PathBuf> {
    SIDECARS
        .iter()
        .map(move |suffix| with_suffix(target, suffix))
}

/// The command a refusal names for discarding a database, with the path
/// quoted for a POSIX shell so that it can be pasted as printed.
fn reset_command(target: &Path) -> String {
    format!(
        "reference-dev --local-oidc-demo --database {} --reset",
        shell_quote(&target.display().to_string())
    )
}

/// One shell word: left bare when every character is plainly safe, and
/// otherwise single-quoted, with each embedded single quote closed, escaped
/// and reopened.
fn shell_quote(word: &str) -> String {
    let safe = |c: char| c.is_ascii_alphanumeric() || "/._-+:=@,%".contains(c);
    if !word.is_empty() && word.chars().all(safe) {
        word.to_owned()
    } else {
        format!("'{}'", word.replace('\'', "'\\''"))
    }
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

/// Resolves one identity for the database, and refuses a name that belongs to
/// another database's sidecars: SQLite and a reset of that database would
/// treat the file as theirs.
fn resolve(path: &Path) -> Result<Paths, StorageError> {
    let paths = derive(path)?;
    // `derive` has checked that the file name is UTF-8.
    let name = paths.target.file_name().and_then(|name| name.to_str());
    let name = name.unwrap_or_default();
    let folded = name.to_ascii_lowercase();
    if let Some(suffix) = SIDECARS.iter().find(|suffix| folded.ends_with(*suffix)) {
        let owner = paths
            .target
            .with_file_name(&name[..name.len() - suffix.len()]);
        return Err(StorageError::Refused(format!(
            "database path {}: a file name ending in -journal, -wal or -shm is reserved for the \
             SQLite sidecars of {}; nothing was touched; {GUIDE}",
            path.display(),
            owner.display()
        )));
    }
    Ok(paths)
}

/// Derives the names before any lock is taken: the parent must exist and is
/// canonicalized, and the file itself may not be a symbolic link.
fn derive(path: &Path) -> Result<Paths, StorageError> {
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

/// Refuses recognizable evidence that a sidecar name holds something other
/// than this database's sidecar: the ownership siblings of a database at that
/// name, whether or not the file exists yet, or an occupant that is not a
/// regular single-link file or that starts with SQLite's database header. A
/// file passing these checks is not proven to be a sidecar.
fn inspect_sidecars(target: &Path) -> Result<(), StorageError> {
    let refuse = |occupant: &Path| {
        StorageError::Refused(format!(
            "database {}: {} looks like a separate database or an unrecognized file, not this \
             database's SQLite sidecar; the database and that file were left as they are; \
             {GUIDE}",
            target.display(),
            occupant.display()
        ))
    };
    for sidecar in sidecars(target) {
        for suffix in RESERVED {
            let sibling = with_suffix(&sidecar, suffix);
            match fs::symlink_metadata(&sibling) {
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
                Ok(_) => return Err(refuse(&sibling)),
            }
        }
        match fs::symlink_metadata(&sidecar) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
            Ok(meta) => {
                if !meta.is_file() || meta.nlink() != 1 || starts_with_header(&sidecar)? {
                    return Err(refuse(&sidecar));
                }
            }
        }
    }
    Ok(())
}

/// The first bytes of a regular file, up to the length of SQLite's header.
fn leading_bytes(path: &Path) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(HEADER.len() as u64)
        .read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn starts_with_header(path: &Path) -> io::Result<bool> {
    Ok(leading_bytes(path)? == HEADER)
}

/// An empty file or one with SQLite's header intact. A plausibility check
/// before a reset deletes the file, not proof that this application made it.
fn plausible_database(path: &Path) -> io::Result<bool> {
    let bytes = leading_bytes(path)?;
    Ok(bytes.is_empty() || bytes == HEADER)
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
async fn initialize(paths: &Paths, issuer: &str, migrator: &Migrator) -> Result<(), StorageError> {
    fs::create_dir(&paths.staging)?;
    let mut owner = fs::File::create_new(paths.staging.join(OWNER))?;
    owner.write_all(&owner_record(&paths.target))?;
    owner.sync_all()?;
    drop(owner);
    let staged = paths.staging.join(STAGED);
    let mut conn = connect(&staged).await?;
    let built = build(&mut conn, &paths.target, issuer, migrator).await;
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
    migrator: &Migrator,
) -> Result<(), StorageError> {
    check_journal(conn, target).await?;
    migrate(conn, target, migrator).await?;
    checkpoint("after-migrate", target)?;
    seed_development(conn, issuer).await?;
    checkpoint("after-seed", target)
}

/// An existing database is migrated, never seeded, so a restart keeps its data.
async fn migrate_existing(
    conn: &mut SqliteConnection,
    target: &Path,
    migrator: &Migrator,
) -> Result<(), StorageError> {
    check_journal(conn, target).await?;
    migrate(conn, target, migrator).await
}

/// Applies pending migrations. A modified applied migration stops here and
/// is never answered with a reset: the refusal offers restoring the
/// migration, which keeps the data, before the reset, which discards it.
async fn migrate(
    conn: &mut SqliteConnection,
    target: &Path,
    migrator: &Migrator,
) -> Result<(), StorageError> {
    migrator.run(&mut *conn).await.map_err(|error| match error {
        MigrateError::VersionMismatch(version) => StorageError::Refused(format!(
            "database {} has migration {version} applied, but that migration's source has been \
             modified since (its checksum differs), and startup never resets a database. To keep \
             the data, restore the applied migration's source and put the intended change in a \
             new migration. To discard the data instead, run: {}",
            target.display(),
            reset_command(target)
        )),
        error => StorageError::Migrate(target.to_owned(), error),
    })
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
/// it is killed, and a registered path fails after migration. `after-remove`
/// lies between a reset's deletion of the database and of its sidecars.
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

/// The ownership lock belongs to the open file, so a child process spawned by
/// any thread shares every lock then held until it execs, and a test that
/// releases a path and takes it again in that instant is refused. Tests that
/// spawn a child therefore run alone, for their whole length, and the others run
/// together. Every spawn in the library's tests takes part.
#[cfg(test)]
static PROCESSES: std::sync::RwLock<()> = std::sync::RwLock::new(());

/// Held by a storage test that spawns no child. A failed test must not fail the rest,
/// so a poisoned lock is still taken.
#[cfg(test)]
pub(crate) fn shared() -> std::sync::RwLockReadGuard<'static, ()> {
    PROCESSES
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Held by a storage test that spawns a child process, and by any other
/// library test around its spawn: a child can inherit locks its spawner never
/// took.
#[cfg(test)]
pub(crate) fn exclusive() -> std::sync::RwLockWriteGuard<'static, ()> {
    PROCESSES
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests;
