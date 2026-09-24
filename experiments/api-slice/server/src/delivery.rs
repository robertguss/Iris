//! Local capture only: fixed loopback SMTP, no relay or real recipient support.
use iris_sqlite_spike::{
    connect,
    outbox::{self, Completion, Delivery},
    token_hash,
};
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

pub struct Mailer {
    smtp: AsyncSmtpTransport<Tokio1Executor>,
    origin: String,
}

impl Mailer {
    pub fn new(origin: String, port: u16) -> Result<Self, Box<dyn std::error::Error>> {
        let url = openidconnect::reqwest::Url::parse(&origin)?;
        if url.origin().ascii_serialization() != origin
            || !(url.scheme() == "https"
                || (url.scheme() == "http"
                    && matches!(url.host_str(), Some("127.0.0.1" | "localhost"))))
        {
            return Err("mail origin must be canonical HTTPS or HTTP loopback".into());
        }
        Ok(Self {
            smtp: AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous("127.0.0.1")
                .port(port)
                .timeout(Some(Duration::from_secs(5)))
                .build(),
            origin,
        })
    }

    fn message(&self, job: &Delivery) -> Result<Message, Box<dyn std::error::Error>> {
        let address: lettre::Address = job.recipient.parse()?;
        if !address.domain().ends_with(".test") {
            return Err("local capture requires .test recipient".into());
        }
        let digest: String = token_hash(&job.token)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        Ok(Message::builder().from("Iris <no-reply@iris.test>".parse()?)
            .to(lettre::message::Mailbox::new(None, address))
            .message_id(Some(format!("<invitation-{digest}@iris.test>")))
            .subject(format!("Iris: invitation to project {}", job.project_id))
            .header(lettre::message::header::ContentType::TEXT_PLAIN)
            .body(format!("Local Iris experiment — no real email was sent.\n\nYou have been invited to project {}. Sign in as the recipient, then open or reopen this link and explicitly accept:\n\n{}/#invitation={}\n\nThis invitation expires one hour after issuance. Retries may deliver duplicate messages; they share the same invitation.\n", job.project_id, self.origin, job.token))?)
    }

    pub async fn send(&self, job: &Delivery) -> Completion {
        let Ok(message) = self.message(job) else {
            return Completion::PermanentFailure;
        };
        match tokio::time::timeout(Duration::from_secs(10), self.smtp.send(message)).await {
            Ok(Ok(_)) => Completion::Sent,
            Ok(Err(error)) if error.is_permanent() => Completion::PermanentFailure,
            _ => Completion::Retry,
        }
    }

    pub async fn tick(&self, database: &Path) -> Result<(), sqlx::Error> {
        let mut conn = connect(database).await?;
        let job = outbox::claim(&mut conn, crate::unix_time()).await?;
        drop(conn);
        if let Some(job) = job {
            let outcome = self.send(&job).await;
            let mut conn = connect(database).await?;
            outbox::complete(&mut conn, &job, outcome, crate::unix_time()).await?;
        }
        Ok(())
    }

    pub async fn run(self, database: PathBuf) {
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        loop {
            interval.tick().await;
            if self.tick(&database).await.is_err() {
                eprintln!("delivery database operation failed; will retry");
            }
        }
    }
}
