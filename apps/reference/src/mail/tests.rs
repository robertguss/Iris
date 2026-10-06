use super::*;
use std::collections::BTreeSet;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

const TOKEN: &str = "tok-0123456789abcdefghijklmnopqrstuvwxyz-ABCDEF";
const EMAIL: &str = "alice@example.test";
const MESSAGE_ID: &str = "<invitation-7f3a9c@reference.iris.test>";
/// 2001-09-09T01:46:40Z.
const EXPIRES: i64 = 1_000_000_000;
const ORIGIN: &str = "http://127.0.0.1:5175";
const LOOPBACK: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);

fn claim_with(email: &str, message_id: &str) -> Claim {
    Claim {
        outbox_id: 1,
        invitation_id: 2,
        project_id: 43,
        expires_at: EXPIRES,
        attempt: 1,
        lease_until: 0,
        recipient_email: email.to_owned(),
        token: TOKEN.to_owned(),
        message_id: message_id.to_owned(),
    }
}

fn claim() -> Claim {
    claim_with(EMAIL, MESSAGE_ID)
}

// ---- A scripted SMTP responder, owned by the test.

#[derive(Clone)]
pub(crate) struct Plan {
    pub(crate) greeting: &'static str,
    pub(crate) rcpt: &'static str,
    pub(crate) end: &'static str,
    pub(crate) delay: Duration,
    pub(crate) silent: bool,
}

impl Plan {
    pub(crate) fn ok() -> Self {
        Self {
            greeting: "220 test ready\r\n",
            rcpt: "250 ok\r\n",
            end: "250 queued\r\n",
            delay: Duration::ZERO,
            silent: false,
        }
    }
}

pub(crate) struct Responder {
    pub(crate) addr: SocketAddr,
    connections: Arc<AtomicUsize>,
    pub(crate) active: Arc<AtomicUsize>,
    data: Arc<Mutex<Vec<Vec<u8>>>>,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for Responder {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl Responder {
    pub(crate) async fn start(plan: Plan) -> Self {
        Self::gated(plan, None).await
    }

    pub(crate) async fn gated(plan: Plan, gate: Option<Arc<crate::lifecycle::Gate>>) -> Self {
        let listener = TcpListener::bind((LOOPBACK, 0)).await.unwrap();
        let addr = listener.local_addr().unwrap();
        let connections = Arc::new(AtomicUsize::new(0));
        let data = Arc::new(Mutex::new(Vec::new()));
        let active = Arc::new(AtomicUsize::new(0));
        let task = tokio::spawn({
            let (connections, data, active) = (connections.clone(), data.clone(), active.clone());
            async move {
                let mut sessions = tokio::task::JoinSet::new();
                while let Ok((stream, _)) = listener.accept().await {
                    connections.fetch_add(1, Ordering::SeqCst);
                    active.fetch_add(1, Ordering::SeqCst);
                    let guard = Active(active.clone());
                    let session = session(stream, plan.clone(), data.clone(), gate.clone());
                    sessions.spawn(async move {
                        let _guard = guard;
                        session.await;
                    });
                    while sessions.try_join_next().is_some() {}
                }
            }
        });
        Self {
            addr,
            connections,
            active,
            data,
            task,
        }
    }

    pub(crate) fn mailer(&self) -> Mailer {
        Mailer::new(ORIGIN, self.addr).unwrap()
    }

    pub(crate) fn connections(&self) -> usize {
        self.connections.load(Ordering::SeqCst)
    }

    pub(crate) fn messages(&self) -> Vec<Vec<u8>> {
        self.data.lock().unwrap().clone()
    }
}

struct Active(Arc<AtomicUsize>);
impl Drop for Active {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

async fn session(
    stream: TcpStream,
    plan: Plan,
    data: Arc<Mutex<Vec<Vec<u8>>>>,
    gate: Option<Arc<crate::lifecycle::Gate>>,
) {
    let (read, mut write) = stream.into_split();
    let mut read = BufReader::new(read);
    if plan.silent {
        std::future::pending::<()>().await;
    }
    tokio::time::sleep(plan.delay).await;
    if write.write_all(plan.greeting.as_bytes()).await.is_err() {
        return;
    }
    loop {
        let mut line = Vec::new();
        if read.read_until(b'\n', &mut line).await.unwrap_or(0) == 0 {
            return;
        }
        let command = String::from_utf8_lossy(&line).to_ascii_uppercase();
        tokio::time::sleep(plan.delay).await;
        let reply = if command.starts_with("DATA") {
            if write.write_all(b"354 go\r\n").await.is_err() {
                return;
            }
            let mut body = Vec::new();
            loop {
                let mut part = Vec::new();
                if read.read_until(b'\n', &mut part).await.unwrap_or(0) == 0 {
                    return;
                }
                if part == b".\r\n" {
                    break;
                }
                body.extend_from_slice(&part);
            }
            data.lock().unwrap().push(body);
            if let Some(gate) = &gate {
                gate.pass().await;
            }
            tokio::time::sleep(plan.delay).await;
            plan.end
        } else if command.starts_with("RCPT") {
            plan.rcpt
        } else if command.starts_with("QUIT") {
            let _ = write.write_all(b"221 bye\r\n").await;
            return;
        } else {
            "250 ok\r\n"
        };
        if write.write_all(reply.as_bytes()).await.is_err() {
            return;
        }
    }
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("message is not UTF-8")
}

#[tokio::test]
async fn a_claim_composes_the_s19_message() {
    let responder = Responder::start(Plan::ok()).await;
    let sent = responder.mailer().send(&claim()).await;
    assert_eq!(sent, super::sent(Completion::Sent, Category::Accepted));
    let messages = responder.messages();
    assert!(messages.len() == 1, "expected exactly one message");
    let raw = text(&messages[0]);
    let (head, body) = raw.split_once("\r\n\r\n").expect("no header/body split");
    let lines: Vec<String> = head.split("\r\n").map(|l| l.replace('"', "")).collect();
    let has = |l: &str| lines.iter().any(|x| x == l);
    assert!(
        has("From: Iris reference <invitations@reference.iris.test>"),
        "From header is wrong"
    );
    assert!(has("To: alice@example.test"), "To header is wrong");
    assert!(
        has("Subject: Iris invitation to project 43"),
        "Subject does not name the project"
    );
    assert!(
        has(&format!("Message-ID: {MESSAGE_ID}")),
        "Message-ID is not the stored one, byte for byte"
    );
    assert!(
        lines
            .iter()
            .filter(|l| l.starts_with("Message-ID:"))
            .count()
            == 1,
        "more than one Message-ID header"
    );
    assert!(!head.contains("<<"), "a doubled angle bracket");
    assert!(
        has("Content-Type: text/plain; charset=utf-8"),
        "body is not text/plain UTF-8"
    );
    assert!(
        body.contains(&format!("{ORIGIN}/#invitation={TOKEN}\r\n")),
        "link is missing or altered"
    );
    assert!(
        body.contains("2001-09-09T01:46:40Z"),
        "RFC 3339 expiry is missing"
    );
    assert!(
        body.contains("Sign in first, then open or reopen this link and accept explicitly"),
        "sign-in instruction is missing"
    );
    assert!(
        body.contains("Duplicates of this message may arrive; they share one invitation."),
        "duplicate notice is missing"
    );
}

#[tokio::test]
async fn send_classifies_smtp_replies() {
    let cases = [
        (Plan::ok(), Completion::Sent, Category::Accepted),
        (
            Plan {
                rcpt: "451 try later\r\n",
                ..Plan::ok()
            },
            Completion::Retryable,
            Category::Transient,
        ),
        (
            Plan {
                rcpt: "550 no such user\r\n",
                ..Plan::ok()
            },
            Completion::Permanent,
            Category::Rejected,
        ),
        (
            Plan {
                end: "554 transaction failed\r\n",
                ..Plan::ok()
            },
            Completion::Permanent,
            Category::Rejected,
        ),
        (
            Plan {
                greeting: "421 busy\r\n",
                ..Plan::ok()
            },
            Completion::Retryable,
            Category::Transient,
        ),
        (
            Plan {
                greeting: "this is not smtp\r\n",
                ..Plan::ok()
            },
            Completion::Retryable,
            Category::Connection,
        ),
    ];
    for (n, (plan, completion, category)) in cases.into_iter().enumerate() {
        let responder = Responder::start(plan).await;
        let got = responder.mailer().send(&claim()).await;
        assert!(
            got == sent(completion, category),
            "case {n} was classified wrongly"
        );
    }
}

async fn timed(mailer: &Mailer) -> (Sent, Duration) {
    let start = Instant::now();
    // A missing outer bound must fail the test, not hang it.
    let got = tokio::time::timeout(Duration::from_secs(10), mailer.send(&claim()))
        .await
        .expect("a send ran far past its bound");
    (got, start.elapsed())
}

#[tokio::test]
async fn send_bounds_the_whole_send() {
    let bound = SEND_TIMEOUT + Duration::from_millis(500);

    // (a) Nothing listens.
    let closed = TcpListener::bind((LOOPBACK, 0)).await.unwrap();
    let addr = closed.local_addr().unwrap();
    drop(closed);
    let (got, took) = timed(&Mailer::new(ORIGIN, addr).unwrap()).await;
    assert!(
        got == sent(Completion::Retryable, Category::Connection),
        "a refused connection was classified wrongly"
    );
    assert!(took < bound, "a refused connection outlasted the bound");

    // (b) Accepts and never replies.
    let silent = Responder::start(Plan {
        silent: true,
        ..Plan::ok()
    })
    .await;
    let (got, took) = timed(&silent.mailer()).await;
    assert!(
        got == sent(Completion::Retryable, Category::Timeout),
        "a silent server was classified wrongly"
    );
    assert!(took < bound, "a silent server outlasted the bound");

    // (c) Every reply is correct but late: no single read times out.
    let slow = Responder::start(Plan {
        delay: Duration::from_millis(500),
        ..Plan::ok()
    })
    .await;
    let (got, took) = timed(&slow.mailer()).await;
    assert!(
        got == sent(Completion::Retryable, Category::Timeout),
        "a slow server was classified wrongly"
    );
    assert!(took < bound, "a slow server outlasted the bound");
}

#[tokio::test]
async fn an_unusable_recipient_is_permanent_without_connecting() {
    let responder = Responder::start(Plan::ok()).await;
    let mailer = responder.mailer();
    let mut claims: Vec<Claim> = ["alice@example.com", "not an address", "a@@b.test", ""]
        .into_iter()
        .map(|email| claim_with(email, MESSAGE_ID))
        .collect();
    // A corrupt token or Message-ID must not add lines to the body or headers.
    for token in ["", "tok\r\nBcc: x@example.test", "tok en", "tok\0", "tök"] {
        claims.push(Claim {
            token: token.to_owned(),
            ..claim()
        });
    }
    for id in [
        "<a b@reference.iris.test>",
        "<a\r\nBcc: x@example.test>",
        "<a\0@x>",
    ] {
        claims.push(claim_with(EMAIL, id));
    }
    for claim in &claims {
        let got = mailer.send(claim).await;
        assert!(
            got == sent(Completion::Permanent, Category::Malformed),
            "an unusable claim was not refused as malformed"
        );
    }
    assert!(responder.connections() == 0, "a connection was made");
}

#[test]
fn the_mailer_refuses_non_loopback_smtp_and_non_canonical_origins() {
    let smtp = |s: &str| s.parse::<SocketAddr>().unwrap();
    let local = smtp("127.0.0.1:25");
    for refused in ["192.0.2.1:25", "[2001:db8::1]:25"] {
        assert!(
            Mailer::new("https://example.test", smtp(refused)).err()
                == Some(MailConfigError::SmtpAddress),
            "a non-loopback SMTP address was accepted"
        );
    }
    for accepted in ["127.0.0.1:25", "[::1]:25"] {
        assert!(
            Mailer::new("https://example.test", smtp(accepted)).is_ok(),
            "a loopback SMTP address was refused"
        );
    }
    for refused in [
        "https://example.test/app",
        "http://example.test",
        "http://example.test:5175",
        "http://127.0.0.1:5175/",
        "https://example.test/",
        "ftp://127.0.0.1",
        "example.test",
    ] {
        assert!(
            Mailer::new(refused, local).err() == Some(MailConfigError::Origin),
            "a non-canonical origin was accepted"
        );
    }
    for accepted in ["https://example.test", "http://127.0.0.1:5175"] {
        assert!(
            Mailer::new(accepted, local).is_ok(),
            "a canonical origin was refused"
        );
    }
}

#[test]
fn send_fits_the_shutdown_drain() {
    assert!(SEND_TIMEOUT + Duration::from_millis(500) < crate::lifecycle::DRAIN);
}

#[tokio::test]
async fn mail_types_never_print_the_payload() {
    let responder = Responder::start(Plan::ok()).await;
    let mailer = responder.mailer();
    let mut shown = vec![format!("{:?}", mailer.send(&claim()).await)];
    shown.push(format!("{mailer:?}"));
    for category in [
        Category::Accepted,
        Category::Rejected,
        Category::Transient,
        Category::Timeout,
        Category::Connection,
        Category::Malformed,
    ] {
        shown.push(format!("{category:?} {}", category.label()));
    }
    for error in [MailConfigError::Origin, MailConfigError::SmtpAddress] {
        shown.push(format!("{error:?}"));
    }
    for (kind, text) in [
        ("credential", TOKEN),
        ("address", "alice"),
        ("message id", "invitation-7f3a9c"),
        ("body", "Sign in first"),
    ] {
        assert!(
            shown.iter().all(|s| !s.contains(text)),
            "a printed form holds the {kind}"
        );
    }
}

// ---- The real capture, started only through the launcher development uses.

const VERSION: &str = "v1.31.2";
const BIN: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/.dev/bin/mailpit");
const SCRIPT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/scripts/mailpit.sh");
const WAIT: Duration = Duration::from_secs(90);

fn reserve_port() -> u16 {
    std::net::TcpListener::bind((LOOPBACK, 0))
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

/// A running capture: killed and reaped on every path, including a panic.
struct Capture {
    child: Child,
    smtp: SocketAddr,
    ui: String,
    database: std::path::PathBuf,
    http: reqwest::Client,
    _dir: tempfile::TempDir,
    _exclusive: std::sync::RwLockWriteGuard<'static, ()>,
}

impl Drop for Capture {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Capture {
    pub(crate) async fn start(env: &[(&str, &str)]) -> Self {
        let exclusive = crate::storage::exclusive();
        assert!(
            std::path::Path::new(BIN).exists(),
            "mailpit {VERSION} is not installed; run: bash apps/reference/scripts/mailpit.sh install"
        );
        let dir = tempfile::tempdir().unwrap();
        let database = dir.path().join("capture.db");
        let (smtp_port, ui_port) = (reserve_port(), reserve_port());
        let child = Command::new("bash")
            .arg(SCRIPT)
            .args(["run", &smtp_port.to_string(), &ui_port.to_string()])
            .arg(&database)
            .envs(env.iter().copied())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let mut capture = Self {
            child,
            smtp: SocketAddr::new(LOOPBACK, smtp_port),
            ui: format!("http://127.0.0.1:{ui_port}"),
            database,
            http: reqwest::Client::new(),
            _dir: dir,
            _exclusive: exclusive,
        };
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            assert!(
                capture.child.try_wait().unwrap().is_none(),
                "capture exited"
            );
            let ready = capture
                .http
                .get(format!("{}/readyz", capture.ui))
                .send()
                .await;
            if ready.is_ok_and(|r| r.status().is_success()) {
                break;
            }
            assert!(Instant::now() < deadline, "capture was not ready in time");
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        let info = capture.get("/api/v1/info").await;
        assert!(info["Version"] == VERSION, "capture is not {VERSION}");
        capture
    }

    pub(crate) fn mailer(&self) -> Mailer {
        Mailer::new(ORIGIN, self.smtp).unwrap()
    }

    async fn get(&self, path: &str) -> serde_json::Value {
        let response = self
            .http
            .get(format!("{}{path}", self.ui))
            .send()
            .await
            .unwrap();
        assert!(response.status().is_success(), "capture API call failed");
        response.json().await.unwrap()
    }

    async fn raw(&self, id: &str) -> String {
        let url = format!("{}/api/v1/message/{id}/raw", self.ui);
        self.http
            .get(url)
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap()
    }

    async fn total(&self) -> u64 {
        self.get("/api/v1/messages?limit=1").await["total"]
            .as_u64()
            .unwrap()
    }

    pub(crate) async fn messages(&self) -> Vec<serde_json::Value> {
        self.get("/api/v1/messages?limit=1000").await["messages"]
            .as_array()
            .unwrap()
            .clone()
    }

    /// Waits until `done` holds of the total, or fails after `WAIT`.
    async fn wait_total(&self, done: impl Fn(u64) -> bool) {
        let deadline = Instant::now() + WAIT;
        while !done(self.total().await) {
            assert!(
                Instant::now() < deadline,
                "the count did not settle in time"
            );
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    }
}

fn accepted(got: Sent) -> bool {
    got == sent(Completion::Sent, Category::Accepted)
}

#[ignore = "needs the pinned Mailpit: bash apps/reference/scripts/mailpit.sh install"]
#[tokio::test]
async fn the_capture_receives_the_message_exactly() {
    let capture = Capture::start(&[]).await;
    assert!(
        accepted(capture.mailer().send(&claim()).await),
        "send failed"
    );
    capture.wait_total(|n| n == 1).await;
    let listed = capture.messages().await;
    let id = listed[0]["ID"].as_str().unwrap().to_owned();
    let message = capture.get(&format!("/api/v1/message/{id}")).await;
    assert!(
        message["To"][0]["Address"] == EMAIL && message["To"].as_array().unwrap().len() == 1,
        "To is wrong"
    );
    assert!(
        message["From"]["Address"] == "invitations@reference.iris.test"
            && message["From"]["Name"] == "Iris reference",
        "From is wrong"
    );
    assert!(
        message["Subject"] == "Iris invitation to project 43",
        "Subject is wrong"
    );
    assert!(
        message["MessageID"] == MESSAGE_ID.trim_matches(['<', '>']),
        "Message-ID is wrong"
    );
    assert!(
        message["Text"]
            .as_str()
            .unwrap()
            .contains(&format!("{ORIGIN}/#invitation={TOKEN}")),
        "link is missing"
    );
    assert!(
        capture
            .raw(&id)
            .await
            .contains(&format!("Message-ID: {MESSAGE_ID}\r\n")),
        "stored Message-ID header is not byte-identical"
    );
}

#[ignore = "needs the pinned Mailpit: bash apps/reference/scripts/mailpit.sh install"]
#[tokio::test]
async fn a_resend_is_captured_twice_with_one_message_id() {
    let capture = Capture::start(&[]).await;
    let mailer = capture.mailer();
    assert!(accepted(mailer.send(&claim()).await), "first send failed");
    assert!(accepted(mailer.send(&claim()).await), "resend failed");
    capture.wait_total(|n| n == 2).await;
    let listed = capture.messages().await;
    assert!(listed.len() == 2, "duplicates were collapsed");
    assert!(
        listed[0]["ID"] != listed[1]["ID"],
        "one message stored twice"
    );
    assert!(
        listed
            .iter()
            .all(|m| m["MessageID"] == MESSAGE_ID.trim_matches(['<', '>'])),
        "the copies do not share the Message-ID"
    );
}

fn listeners(pid: u32) -> BTreeSet<String> {
    let out = Command::new("lsof")
        .args([
            "-nP",
            "-a",
            "-p",
            &pid.to_string(),
            "-iTCP",
            "-sTCP:LISTEN",
            "-Fn",
        ])
        .output()
        .unwrap();
    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .filter_map(|l| l.strip_prefix('n'))
        .map(str::to_owned)
        .collect()
}

#[ignore = "needs the pinned Mailpit: bash apps/reference/scripts/mailpit.sh install"]
#[tokio::test]
async fn the_capture_is_isolated_despite_a_hostile_environment() {
    let dir = tempfile::tempdir().unwrap();
    let auth = dir.path().join("pop3-auth");
    let relay = dir.path().join("relay.yaml");
    std::fs::write(&auth, "user:pass\n").unwrap();
    std::fs::write(&relay, "host: 127.0.0.1\nport: 9\n").unwrap();
    let pop3 = format!("127.0.0.1:{}", reserve_port());
    let capture = Capture::start(&[
        ("MP_ENABLE_CHAOS", "true"),
        ("MP_SMTP_RELAY_ALL", "true"),
        ("MP_POP3_AUTH_FILE", auth.to_str().unwrap()),
        ("MP_POP3_BIND_ADDR", &pop3),
        ("MP_SMTP_RELAY_CONFIG", relay.to_str().unwrap()),
    ])
    .await;
    let expected: BTreeSet<String> = [
        capture.smtp.to_string(),
        capture.ui.trim_start_matches("http://").to_owned(),
    ]
    .into();
    assert!(
        listeners(capture.child.id()) == expected,
        "the capture listens somewhere other than the two given loopback ports"
    );
    let webui = capture.get("/api/v1/webui").await;
    assert!(webui["ChaosEnabled"] == false, "chaos is enabled");
    assert!(
        webui["MessageRelay"]["Enabled"] == false,
        "relay is enabled"
    );
    let chaos = capture
        .http
        .get(format!("{}/api/v1/chaos", capture.ui))
        .send()
        .await
        .unwrap();
    assert!(chaos.status().as_u16() != 200, "the chaos API is served");
}

#[ignore = "needs the pinned Mailpit: bash apps/reference/scripts/mailpit.sh install"]
#[tokio::test]
async fn the_capture_keeps_at_most_500_messages() {
    let capture = Capture::start(&[]).await;
    let mailer = capture.mailer();
    for n in 0..505 {
        let id = format!("<cap-{n}@reference.iris.test>");
        assert!(
            accepted(mailer.send(&claim_with(EMAIL, &id)).await),
            "a send failed"
        );
    }
    capture.wait_total(|n| n == 500).await;
    tokio::time::sleep(Duration::from_secs(2)).await;
    assert!(capture.total().await == 500, "the count moved off 500");
}

#[ignore = "needs the pinned Mailpit: bash apps/reference/scripts/mailpit.sh install"]
#[tokio::test]
async fn the_capture_prunes_messages_older_than_24_hours() {
    use sqlx::Connection;
    const HOUR_MS: i64 = 3_600_000;
    let capture = Capture::start(&[]).await;
    let mailer = capture.mailer();
    let (old, recent, fresh) = (
        "<age-25h@reference.iris.test>",
        "<age-23h@reference.iris.test>",
        "<age-fresh@reference.iris.test>",
    );
    for id in [old, recent] {
        assert!(
            accepted(mailer.send(&claim_with(EMAIL, id)).await),
            "send failed"
        );
    }
    capture.wait_total(|n| n == 2).await;
    let options = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(&capture.database)
        .busy_timeout(Duration::from_secs(5));
    let mut conn = sqlx::SqliteConnection::connect_with(&options)
        .await
        .unwrap();
    for (id, hours) in [(old, 25), (recent, 23)] {
        let moved = sqlx::query("UPDATE mailbox SET Created = Created - ? WHERE MessageID = ?")
            .bind(hours * HOUR_MS)
            .bind(id.trim_matches(['<', '>']))
            .execute(&mut conn)
            .await
            .unwrap()
            .rows_affected();
        assert!(moved == 1, "the capture's own row was not found");
    }
    drop(conn);
    assert!(
        accepted(mailer.send(&claim_with(EMAIL, fresh)).await),
        "send failed"
    );
    let deadline = Instant::now() + WAIT;
    while capture.total().await != 2 {
        assert!(
            Instant::now() < deadline,
            "the old message was not pruned in time"
        );
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    let mut kept: Vec<String> = capture
        .messages()
        .await
        .iter()
        .map(|m| m["MessageID"].as_str().unwrap().to_owned())
        .collect();
    kept.sort();
    let mut want = vec![
        recent.trim_matches(['<', '>']).to_owned(),
        fresh.trim_matches(['<', '>']).to_owned(),
    ];
    want.sort();
    assert!(kept == want, "the wrong messages remain");
}
