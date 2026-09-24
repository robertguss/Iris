use sha2::{Digest, Sha256};

/// Caller supplies a verified identity and a single timestamp for this attempt.
/// This experiment does not implement authentication or automatic retries.
pub struct AcceptInvitation<'a> {
    pub token: &'a str,
    pub user_id: i64,
    pub now: i64,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    Accepted {
        project_id: i64,
    },
    /// Also used for a token belonging to another recipient.
    NotFound,
    Expired,
    AlreadyAccepted,
}

pub fn token_hash(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}
