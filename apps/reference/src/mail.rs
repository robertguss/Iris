//! Invitation mail (S19 stage 2, step 2): composing a claimed delivery and
//! sending it over plain SMTP to a dedicated loopback `.test` capture. No task
//! or route uses it yet. Nothing here stores or prints a credential, address,
//! Message-ID, body or raw SMTP error: callers get a bounded category.
use crate::domains::invitations::delivery::{Claim, Completion};
use crate::domains::invitations::usable_contact;
use crate::identity::canonical_origin;
use lettre::message::header::{ContentTransferEncoding, ContentType};
use lettre::message::{Body, Mailbox};
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use std::fmt;
use std::net::SocketAddr;
use std::time::Duration;

/// The bound on a whole send, connect included. It is the outer timeout around
/// the send, and the sole bound once connected: lettre's async transport applies
/// `.timeout(SEND_TIMEOUT)` to the TCP connect only, with no timeout on reads or
/// writes, so without the outer one a silent server holds the send forever.
/// After a stop the supervisor admits at most one send, so the worst case in the
/// drain is this plus one `complete` on a 100 ms busy timeout.
pub const SEND_TIMEOUT: Duration = Duration::from_secs(2);

const SENDER: &str = "Iris reference <invitations@reference.iris.test>";

/// Why a mailer could not be built. Carries no value from the caller.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MailConfigError {
    /// Not a canonical HTTPS origin or explicit HTTP loopback origin.
    Origin,
    /// The SMTP address is not loopback.
    SmtpAddress,
}

/// The bounded reason for a send's result; the label is fixed text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    Accepted,
    Rejected,
    Transient,
    Timeout,
    Connection,
    Malformed,
}

impl Category {
    pub fn label(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
            Self::Transient => "transient",
            Self::Timeout => "timeout",
            Self::Connection => "connection",
            Self::Malformed => "malformed",
        }
    }
}

/// What one send came to: what step 1 records and a category for diagnostics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sent {
    pub completion: Completion,
    pub category: Category,
}

const fn sent(completion: Completion, category: Category) -> Sent {
    Sent {
        completion,
        category,
    }
}

pub struct Mailer {
    origin: String,
    smtp: SocketAddr,
    transport: AsyncSmtpTransport<Tokio1Executor>,
}

impl fmt::Debug for Mailer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Mailer")
            .field("origin", &self.origin)
            .field("smtp", &self.smtp)
            .finish()
    }
}

impl Mailer {
    /// `origin` is where invitation links point; `smtp` must be loopback.
    pub fn new(origin: impl Into<String>, smtp: SocketAddr) -> Result<Self, MailConfigError> {
        let origin = origin.into();
        if !canonical_origin(&origin) {
            return Err(MailConfigError::Origin);
        }
        if !smtp.ip().is_loopback() {
            return Err(MailConfigError::SmtpAddress);
        }
        let transport =
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(smtp.ip().to_string())
                .port(smtp.port())
                .timeout(Some(SEND_TIMEOUT))
                .build();
        Ok(Self {
            origin,
            smtp,
            transport,
        })
    }

    /// Sends one claim. A recipient that is not a usable `.test` contact is
    /// refused before any connection is made.
    pub async fn send(&self, claim: &Claim) -> Sent {
        let Some(message) = self.compose(claim) else {
            return sent(Completion::Permanent, Category::Malformed);
        };
        match tokio::time::timeout(SEND_TIMEOUT, self.transport.send(message)).await {
            Err(_) => sent(Completion::Retryable, Category::Timeout),
            Ok(Ok(_)) => sent(Completion::Sent, Category::Accepted),
            Ok(Err(e)) if e.is_permanent() => sent(Completion::Permanent, Category::Rejected),
            Ok(Err(e)) if e.is_transient() => sent(Completion::Retryable, Category::Transient),
            // Only a connect timeout can reach here; the outer one is above.
            Ok(Err(e)) if e.is_timeout() => sent(Completion::Retryable, Category::Timeout),
            // The fallthrough: lettre has no public connection predicate, so
            // refused connections, resets and unparsable replies land here.
            Ok(Err(_)) => sent(Completion::Retryable, Category::Connection),
        }
    }

    fn compose(&self, claim: &Claim) -> Option<Message> {
        // The token and Message-ID go into the body and a header as given, so
        // a corrupt row must not be able to add lines to either.
        if !usable_contact(&claim.recipient_email)
            || claim.token.is_empty()
            || !claim.token.bytes().all(|b| b.is_ascii_graphic())
            || claim
                .message_id
                .chars()
                .any(|c| c.is_whitespace() || c.is_control())
        {
            return None;
        }
        let to = Mailbox::new(None, claim.recipient_email.parse().ok()?);
        let text = format!(
            "You have been invited to Iris project {project}.\r\n\
             \r\n\
             Sign in first, then open or reopen this link and accept explicitly:\r\n\
             \r\n\
             {origin}/#invitation={token}\r\n\
             \r\n\
             This invitation expires at {expires}.\r\n\
             \r\n\
             Duplicates of this message may arrive; they share one invitation.\r\n",
            project = claim.project_id,
            origin = self.origin,
            token = claim.token,
            expires = rfc3339_utc(claim.expires_at)?,
        );
        // lettre would quote-print any line over 76 characters, splitting the
        // link. The text is ASCII with CRLF lines far under the 998 limit, so
        // it is sent as 7bit as it stands.
        if !text.is_ascii() || text.split("\r\n").any(|line| line.len() > 900) {
            return None;
        }
        let body =
            Body::dangerous_pre_encoded(text.into_bytes(), ContentTransferEncoding::SevenBit);
        Message::builder()
            .from(SENDER.parse().ok()?)
            .to(to)
            .message_id(Some(claim.message_id.clone()))
            .subject(format!("Iris invitation to project {}", claim.project_id))
            .header(ContentType::TEXT_PLAIN)
            .body(body)
            .ok()
    }
}

fn rfc3339_utc(unix_seconds: i64) -> Option<String> {
    let t = time::OffsetDateTime::from_unix_timestamp(unix_seconds).ok()?;
    Some(format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        t.year(),
        u8::from(t.month()),
        t.day(),
        t.hour(),
        t.minute(),
        t.second()
    ))
}

#[cfg(test)]
mod tests;
