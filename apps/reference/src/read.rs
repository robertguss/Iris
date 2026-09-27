//! Reads: one deferred SQLite transaction on a `query_only` connection that
//! the read owns, finalized explicitly and then dropped (S17 read
//! conventions). Application code, not an Iris API; other engines need their
//! own proof.
use crate::domains::{Cleanup, FailureKind, Stage, failure_kind};
use sqlx::{Connection, SqliteConnection};

/// Why a read stopped before its page was complete.
#[derive(Debug, PartialEq, Eq)]
pub enum Stop<R> {
    Rejected(R),
    Execution { stage: Stage, kind: FailureKind },
}

/// S15's rejection, failure and cleanup vocabulary without a mutation-effect
/// assessment; an acknowledged rollback says nothing about effects.
#[derive(Debug, PartialEq, Eq)]
pub enum ReadError<R> {
    Rejected(R),
    Failed { primary: Stop<R>, cleanup: Cleanup },
}

pub fn execution<R>(stage: Stage, error: sqlx::Error) -> Stop<R> {
    Stop::Execution {
        stage,
        kind: failure_kind(&error),
    }
}

/// Present state only: `next` is the key after which the following page
/// starts, a position rather than a snapshot.
#[derive(Debug, PartialEq, Eq)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next: Option<i64>,
}

impl<T> Page<T> {
    /// `rows` were fetched in ascending key order with `LIMIT limit + 1`; the
    /// extra row only shows that another page exists.
    pub fn new(mut rows: Vec<T>, limit: u32, key: fn(&T) -> i64) -> Self {
        let more = rows.len() > limit as usize;
        rows.truncate(limit as usize);
        let next = if more { rows.last().map(key) } else { None };
        Self { items: rows, next }
    }
}

/// Runs `body` in one deferred transaction, then awaits `COMMIT` or
/// `ROLLBACK` so cleanup is classified rather than assumed. Takes the
/// connection and drops it, so neither `query_only` nor an unfinished
/// transaction can reach a later mutation. Dropping this future mid-read drops
/// both; SQLx queues that rollback rather than acknowledging it.
pub async fn run<T, R>(
    mut conn: SqliteConnection,
    body: impl AsyncFnOnce(&mut SqliteConnection) -> Result<T, Stop<R>>,
) -> Result<T, ReadError<R>> {
    let unstarted = |error| ReadError::Failed {
        primary: execution(Stage::Begin, error),
        cleanup: Cleanup::Unconfirmed {
            rollback_error: None,
        },
    };
    // Guards mistakes; not a security boundary or compile-time guarantee.
    sqlx::query("PRAGMA query_only = ON")
        .execute(&mut conn)
        .await
        .map_err(unstarted)?;
    let mut tx = conn.begin().await.map_err(unstarted)?;
    match body(&mut tx).await {
        Ok(value) => tx
            .commit()
            .await
            .map(|()| value)
            .map_err(|error| ReadError::Failed {
                primary: execution(Stage::Commit, error),
                cleanup: Cleanup::Unconfirmed {
                    rollback_error: None,
                },
            }),
        Err(primary) => Err(finalize(primary, tx.rollback().await)),
    }
}

// Finalization only. No raw driver text is retained.
fn finalize<R>(primary: Stop<R>, rollback: Result<(), sqlx::Error>) -> ReadError<R> {
    match (primary, rollback) {
        (Stop::Rejected(r), Ok(())) => ReadError::Rejected(r),
        (primary, result) => ReadError::Failed {
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

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::app::{MIGRATOR, connect, is_busy, seed};
    use std::path::{Path, PathBuf};

    pub(crate) async fn database() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("read.db");
        let mut conn = connect(&path).await.unwrap();
        MIGRATOR.run(&mut conn).await.unwrap();
        seed(&mut conn, "http://127.0.0.1:1").await.unwrap();
        (dir, path)
    }

    /// A writer that must commit on its own connection within the busy timeout.
    async fn writer_commits(path: &Path, name: &str) {
        let mut writer = connect(path).await.unwrap();
        let mut tx = writer.begin_with("BEGIN IMMEDIATE").await.unwrap();
        sqlx::query("UPDATE projects SET name=? WHERE id=41")
            .bind(name)
            .execute(&mut *tx)
            .await
            .unwrap();
        tx.commit().await.unwrap();
    }

    async fn project_name(path: &Path) -> String {
        let mut conn = connect(path).await.unwrap();
        sqlx::query_scalar("SELECT name FROM projects WHERE id=41")
            .fetch_one(&mut conn)
            .await
            .unwrap()
    }

    #[test]
    fn finalize_classifies_rollback_outcomes() {
        let failed = || Err(sqlx::Error::Protocol("secret-canary".into()));
        assert_eq!(finalize(Stop::Rejected(7), Ok(())), ReadError::Rejected(7));
        let unconfirmed = finalize(Stop::Rejected(7), failed());
        assert_eq!(
            unconfirmed,
            ReadError::Failed {
                primary: Stop::Rejected(7),
                cleanup: Cleanup::Unconfirmed {
                    rollback_error: Some(FailureKind::Other)
                },
            },
            "a rejection whose rollback failed is never projected as a rejection"
        );
        assert!(!format!("{unconfirmed:?}").contains("secret-canary"));
        let busy = || Stop::<u8>::Execution {
            stage: Stage::Body,
            kind: FailureKind::Busy,
        };
        assert_eq!(
            finalize(busy(), Ok(())),
            ReadError::Failed {
                primary: busy(),
                cleanup: Cleanup::RollbackAcknowledged,
            }
        );
        assert_eq!(
            finalize(busy(), failed()),
            ReadError::Failed {
                primary: busy(),
                cleanup: Cleanup::Unconfirmed {
                    rollback_error: Some(FailureKind::Other)
                },
            }
        );
    }

    #[test]
    fn pages_report_a_next_position_only_when_more_rows_exist() {
        let key = |n: &i64| *n;
        assert_eq!(
            Page::new(vec![3, 5, 8], 2, key),
            Page {
                items: vec![3, 5],
                next: Some(5)
            }
        );
        assert_eq!(
            Page::new(vec![3, 5], 2, key),
            Page {
                items: vec![3, 5],
                next: None
            }
        );
        assert_eq!(
            Page::new(Vec::new(), 2, key),
            Page {
                items: vec![],
                next: None
            }
        );
    }

    #[tokio::test]
    async fn finished_reads_leave_no_lock() {
        let (_dir, path) = database().await;
        let read = run::<i64, ()>(connect(&path).await.unwrap(), async |conn| {
            sqlx::query_scalar("SELECT COUNT(*) FROM memberships")
                .fetch_one(&mut *conn)
                .await
                .map_err(|e| execution(Stage::Body, e))
        })
        .await;
        assert_eq!(read, Ok(2));
        writer_commits(&path, "After success").await;
        let rejected = run::<(), u8>(connect(&path).await.unwrap(), async |conn| {
            sqlx::query("SELECT 1 FROM memberships")
                .fetch_all(&mut *conn)
                .await
                .map_err(|e| execution(Stage::Body, e))?;
            Err(Stop::Rejected(3))
        })
        .await;
        assert_eq!(rejected, Err(ReadError::Rejected(3)));
        writer_commits(&path, "After rejection").await;
        assert_eq!(project_name(&path).await, "After rejection");
    }

    #[tokio::test]
    async fn query_only_refuses_writes_without_reaching_mutations() {
        let (_dir, path) = database().await;
        let result = run::<(), ()>(connect(&path).await.unwrap(), async |conn| {
            sqlx::query("UPDATE projects SET name='Leaked' WHERE id=41")
                .execute(&mut *conn)
                .await
                .map_err(|e| execution(Stage::Body, e))?;
            Ok(())
        })
        .await;
        assert_eq!(
            result,
            Err(ReadError::Failed {
                primary: Stop::Execution {
                    stage: Stage::Body,
                    kind: FailureKind::Other
                },
                cleanup: Cleanup::RollbackAcknowledged,
            })
        );
        assert_eq!(project_name(&path).await, "Launch plan");
        // The mutation path opens its own connection, as every request does.
        let mut conn = connect(&path).await.unwrap();
        sqlx::query("INSERT INTO users (id, display_name) VALUES (29000, 'Probe')")
            .execute(&mut conn)
            .await
            .unwrap();
        sqlx::query("INSERT INTO memberships VALUES (41, 29000, 'viewer')")
            .execute(&mut conn)
            .await
            .unwrap();
        let changed = crate::domains::memberships::change_role(
            &mut conn,
            &crate::identity::Actor(11),
            crate::domains::memberships::ChangeRole {
                project_id: 41,
                user_id: 29000,
                role: crate::domains::memberships::MemberRole::Editor,
            },
        )
        .await;
        assert!(changed.is_ok(), "{changed:?}");
    }

    #[tokio::test]
    async fn cancelled_read_releases_its_lock() {
        let (_dir, path) = database().await;
        let (ready, holding) = tokio::sync::oneshot::channel();
        let mut read = Box::pin(run::<(), ()>(
            connect(&path).await.unwrap(),
            async move |conn| {
                sqlx::query("SELECT COUNT(*) FROM memberships")
                    .fetch_one(&mut *conn)
                    .await
                    .unwrap();
                ready.send(()).unwrap();
                std::future::pending().await
            },
        ));
        tokio::select! {
            _ = &mut read => panic!("the read never finishes on its own"),
            signal = holding => signal.unwrap(),
        }
        // One writer transaction throughout, driven by explicit statements so
        // a busy COMMIT leaves it open for the retry.
        let mut writer = connect(&path).await.unwrap();
        for statement in [
            "BEGIN IMMEDIATE",
            "UPDATE projects SET name='Written' WHERE id=41",
        ] {
            sqlx::query(statement).execute(&mut writer).await.unwrap();
        }
        let blocked = sqlx::query("COMMIT").execute(&mut writer).await;
        assert!(
            blocked.as_ref().is_err_and(is_busy),
            "the read's shared lock blocks the commit: {blocked:?}"
        );
        drop(read);
        let mut committed = false;
        for _ in 0..50 {
            match sqlx::query("COMMIT").execute(&mut writer).await {
                Ok(_) => {
                    committed = true;
                    break;
                }
                Err(error) if is_busy(&error) => {
                    tokio::time::sleep(std::time::Duration::from_millis(20)).await
                }
                Err(error) => panic!("{error}"),
            }
        }
        assert!(committed, "the writer commits once the read is dropped");
        assert_eq!(project_name(&path).await, "Written");
    }
}
