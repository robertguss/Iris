//! Domain operations and the S15 execution vocabulary that mutations and reads
//! share. Application code, not an Iris API.
pub mod memberships;

#[derive(Debug, PartialEq, Eq)]
pub enum Stage {
    Begin,
    Body,
    Commit,
}

#[derive(Debug, PartialEq, Eq)]
pub enum FailureKind {
    Busy,
    Other,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Cleanup {
    RollbackAcknowledged,
    Unconfirmed { rollback_error: Option<FailureKind> },
}

pub(crate) fn failure_kind(error: &sqlx::Error) -> FailureKind {
    if crate::app::is_busy(error) {
        FailureKind::Busy
    } else {
        FailureKind::Other
    }
}
