//! Smoke test for the development binary: flag enforcement, startup, the
//! served session and domain boundary, the database reset, and shutdown on a
//! signal with the session cleanup task. The browser workflow covers the rest.
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::TcpStream,
    process::{Child, Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};

const ORIGIN: &str = "http://127.0.0.1:5175";
const WAIT: Duration = Duration::from_secs(60);

/// Owns a child from spawn, so a failed assertion cannot leak it.
struct Owned(Child);
impl Drop for Owned {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Forwards a child's output lines, so every wait can be bounded.
fn lines(stream: impl std::io::Read + Send + 'static) -> mpsc::Receiver<String> {
    let (send, receive) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stream).lines().map_while(Result::ok) {
            if send.send(line).is_err() {
                break;
            }
        }
    });
    receive
}

fn issuer() -> (Owned, String) {
    let script = format!(
        "import {{startOidcProvider}} from '{}'; const p=await startOidcProvider({{port:0,redirectUri:'{ORIGIN}/api/auth/callback'}}); console.log(p.issuer);",
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../experiments/api-slice/checks/oidc-provider.mjs")
            .display()
    );
    let mut provider = Owned(
        Command::new("node")
            .args(["--input-type=module", "-e", &script])
            .stdout(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let issuer = lines(provider.0.stdout.take().unwrap())
        .recv_timeout(WAIT)
        .expect("issuer did not start");
    (provider, issuer.trim().to_owned())
}

fn server(issuer: &str, flag: bool) -> (Owned, mpsc::Receiver<String>) {
    server_with(issuer, if flag { &["--local-oidc-demo"] } else { &[] })
}

fn server_with(issuer: &str, args: &[&str]) -> (Owned, mpsc::Receiver<String>) {
    spawn(
        args,
        &[
            ("IRIS_PUBLIC_ORIGIN", ORIGIN),
            ("IRIS_OIDC_ISSUER", issuer),
            ("IRIS_LISTEN", "127.0.0.1:0"),
        ],
    )
}

/// Starts the binary with exactly the given `IRIS_` variables.
fn spawn(args: &[&str], environment: &[(&str, &str)]) -> (Owned, mpsc::Receiver<String>) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_reference-dev"));
    command.args(args);
    for name in ["IRIS_PUBLIC_ORIGIN", "IRIS_OIDC_ISSUER", "IRIS_LISTEN"] {
        command.env_remove(name);
    }
    command.envs(environment.iter().copied());
    let mut child = Owned(command.stderr(Stdio::piped()).spawn().unwrap());
    let stderr = lines(child.0.stderr.take().unwrap());
    (child, stderr)
}

#[test]
fn refuses_to_start_without_the_demo_flag() {
    let (_provider, issuer) = issuer();
    let (mut child, stderr) = server(&issuer, false);
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            break status;
        }
        assert!(started.elapsed() < WAIT, "still running without the flag");
        std::thread::sleep(Duration::from_millis(50));
    };
    assert!(!status.success());
    // The child has exited, so its stderr ends and the channel closes.
    let output = stderr.iter().collect::<Vec<_>>().join("\n");
    assert!(output.contains("requires --local-oidc-demo"), "{output}");
}

/// Waits for the child to exit and returns its status and whole stderr.
fn exited(child: &mut Owned, stderr: mpsc::Receiver<String>) -> (bool, String) {
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            break status;
        }
        assert!(started.elapsed() < WAIT, "still running");
        std::thread::sleep(Duration::from_millis(50));
    };
    (
        status.success(),
        stderr.iter().collect::<Vec<_>>().join("\n"),
    )
}

/// Reads stderr until the server reports its address, returning every line.
fn listening(stderr: &mpsc::Receiver<String>) -> Vec<String> {
    let started = Instant::now();
    let mut seen = Vec::new();
    loop {
        let line = stderr
            .recv_timeout(WAIT.saturating_sub(started.elapsed()))
            .unwrap_or_else(|_| panic!("server did not report its address: {seen:?}"));
        let done = line.starts_with("listening on ");
        seen.push(line);
        if done {
            return seen;
        }
    }
}

#[test]
fn refuses_an_unknown_argument() {
    let (_provider, issuer) = issuer();
    let (mut child, stderr) = server_with(&issuer, &["--local-oidc-demo", "--persist"]);
    let (success, output) = exited(&mut child, stderr);
    assert!(!success);
    assert!(output.contains("usage: reference-dev"), "{output}");
}

#[test]
fn a_database_path_persists_across_a_kill_and_admits_one_process() {
    let (_provider, issuer) = issuer();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("dev.db");
    let path = path.to_str().unwrap();
    let args = ["--local-oidc-demo", "--database", path];

    let (mut first, stderr) = server_with(&issuer, &args);
    let lines = listening(&stderr);
    assert!(
        lines.iter().any(|line| line.contains("persistent data at")),
        "{lines:?}"
    );
    let (mut second, second_err) = server_with(&issuer, &args);
    let (success, output) = exited(&mut second, second_err);
    assert!(!success);
    assert!(
        output.contains("in use by another reference-dev process"),
        "{output}"
    );

    // SIGKILL runs no destructors; the operating system releases the lock.
    first.0.kill().unwrap();
    first.0.wait().unwrap();
    let (_third, stderr) = server_with(&issuer, &args);
    listening(&stderr);
    assert!(!dir.path().join("dev.db.iris-init").exists());
}

#[tokio::test]
async fn serves_the_session_and_domain_boundary() {
    let (_provider, issuer) = issuer();
    let (_server, stderr) = server(&issuer, true);
    let started = Instant::now();
    let address = loop {
        let line = stderr
            .recv_timeout(WAIT.saturating_sub(started.elapsed()))
            .expect("server did not report its address");
        if let Some(address) = line.strip_prefix("listening on ") {
            break address.to_owned();
        }
    };
    let http = reqwest::Client::new();

    let session = http
        .get(format!("{address}/api/auth/session"))
        .send()
        .await
        .unwrap();
    assert_eq!(session.status(), 200);
    let cookie = session.headers()["set-cookie"]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    assert!(cookie.starts_with("iris-session-dev="), "{cookie}");
    let info: Value = session.json().await.unwrap();
    assert_eq!(info["user_id"], Value::Null);
    let csrf = info["csrf_token"].as_str().unwrap();

    let refused = http
        .post(format!("{address}/api/memberships/role"))
        .header("origin", ORIGIN)
        .header("x-iris-csrf", csrf)
        .header("cookie", &cookie)
        .json(&json!({"project_id": "41", "user_id": "29", "role": "viewer"}))
        .send()
        .await
        .unwrap();
    assert_eq!(refused.status(), 401);
    let body: Value = refused.json().await.unwrap();
    assert_eq!(body["operation"], "memberships.change_role");
    assert_eq!(body["kind"], "refused");
    assert_eq!(body["code"], "http.unauthenticated");

    let missing = http
        .get(format!("{address}/api/unknown"))
        .send()
        .await
        .unwrap();
    assert_eq!(missing.status(), 404);
    assert!(missing.bytes().await.unwrap().is_empty());
}

/// The address a started server reported.
fn address(lines: &[String]) -> String {
    lines
        .last()
        .and_then(|line| line.strip_prefix("listening on "))
        .expect("no address line")
        .to_owned()
}

/// What the session endpoint reports for `cookie`: the user ID, or null.
async fn user(http: &reqwest::Client, address: &str, cookie: &str) -> Value {
    let session = http
        .get(format!("{address}/api/auth/session"))
        .header("cookie", cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(session.status(), 200);
    session.json::<Value>().await.unwrap()["user_id"].clone()
}

fn cookie_of(response: &reqwest::Response) -> String {
    response.headers()["set-cookie"]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned()
}

/// Signs in as Alice through the issuer fixture and returns the session
/// cookie the server issued for the signed-in session.
async fn sign_in(http: &reqwest::Client, address: &str) -> String {
    let session = http
        .get(format!("{address}/api/auth/session"))
        .send()
        .await
        .unwrap();
    let cookie = cookie_of(&session);
    let info: Value = session.json().await.unwrap();
    let login = http
        .post(format!("{address}/api/auth/login"))
        .header("origin", ORIGIN)
        .header("x-iris-csrf", info["csrf_token"].as_str().unwrap())
        .header("cookie", &cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(login.status(), 200);
    let login: Value = login.json().await.unwrap();
    let url = reqwest::Url::parse(login["authorization_url"].as_str().unwrap()).unwrap();
    let mut form: Vec<(String, String)> = url
        .query_pairs()
        .map(|(name, value)| (name.into_owned(), value.into_owned()))
        .collect();
    form.push(("identity".into(), "alice".into()));
    let chosen = http
        .post(format!("{}/authorize", url.origin().ascii_serialization()))
        .form(&form)
        .send()
        .await
        .unwrap();
    assert_eq!(chosen.status(), 303);
    // The issuer redirects to the public origin; the server is behind it.
    let callback = reqwest::Url::parse(chosen.headers()["location"].to_str().unwrap()).unwrap();
    let completed = http
        .get(format!(
            "{address}{}?{}",
            callback.path(),
            callback.query().unwrap()
        ))
        .header("cookie", &cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(completed.status(), 303);
    cookie_of(&completed)
}

fn stop(mut child: Owned) {
    child.0.kill().unwrap();
    child.0.wait().unwrap();
}

#[tokio::test]
async fn a_reset_is_refused_while_a_server_runs_and_ends_every_session_afterwards() {
    let (_provider, issuer) = issuer();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("dev.db");
    let path = path.to_str().unwrap();
    let serve = ["--local-oidc-demo", "--database", path];
    let reset = ["--local-oidc-demo", "--database", path, "--reset"];
    let http = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();

    let (server, stderr) = server_with(&issuer, &serve);
    let first = address(&listening(&stderr));
    let cookie = sign_in(&http, &first).await;
    assert_eq!(user(&http, &first, &cookie).await, "11");

    // Refused while the server owns the path, which keeps serving.
    let (mut refused, refused_err) = server_with(&issuer, &reset);
    let (success, output) = exited(&mut refused, refused_err);
    assert!(!success);
    assert!(
        output.contains("in use by another reference-dev process"),
        "{output}"
    );
    assert_eq!(user(&http, &first, &cookie).await, "11");

    // Control: a restart alone keeps the session.
    stop(server);
    let (server, stderr) = server_with(&issuer, &serve);
    let second = address(&listening(&stderr));
    assert_eq!(user(&http, &second, &cookie).await, "11");

    // After a reset, the same cookie is an anonymous browser.
    stop(server);
    let (mut done, done_err) = server_with(&issuer, &reset);
    let (success, output) = exited(&mut done, done_err);
    assert!(success, "{output}");
    assert!(output.contains("reset database at"), "{output}");
    let (_server, stderr) = server_with(&issuer, &serve);
    let third = address(&listening(&stderr));
    assert_eq!(user(&http, &third, &cookie).await, Value::Null);
}

#[tokio::test]
async fn a_reset_needs_only_the_issuer_name_and_never_listens() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("dev.db");
    // An address in use and an issuer nobody serves: neither is contacted.
    let taken = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let listen = taken.local_addr().unwrap().to_string();
    let closed = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let issuer = format!("http://{}", closed.local_addr().unwrap());
    drop(closed);

    let (mut reset, stderr) = spawn(
        &[
            "--local-oidc-demo",
            "--database",
            path.to_str().unwrap(),
            "--reset",
        ],
        &[("IRIS_OIDC_ISSUER", &issuer), ("IRIS_LISTEN", &listen)],
    );
    let (success, output) = exited(&mut reset, stderr);
    assert!(success, "{output}");
    assert!(output.contains("reset database at"), "{output}");
    assert!(!output.contains("listening on"), "{output}");

    let mut conn = iris_reference::app::connect(&path).await.unwrap();
    let issuers: Vec<String> =
        sqlx::query_scalar("SELECT issuer FROM iris_external_identities ORDER BY subject")
            .fetch_all(&mut conn)
            .await
            .unwrap();
    assert_eq!(issuers, [issuer.clone(), issuer]);
}

#[test]
fn a_reset_without_a_database_or_out_of_place_is_a_usage_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("dev.db");
    let path = path.to_str().unwrap();
    for args in [
        &["--local-oidc-demo", "--reset"][..],
        &["--local-oidc-demo", "--reset", "--database", path],
        &["--reset", "--local-oidc-demo", "--database", path],
        &[
            "--local-oidc-demo",
            "--database",
            path,
            "--reset",
            "--reset",
        ],
    ] {
        let (mut child, stderr) = spawn(args, &[("IRIS_OIDC_ISSUER", "http://127.0.0.1:4001")]);
        let (success, output) = exited(&mut child, stderr);
        assert!(!success, "{args:?}");
        assert!(
            output.contains("usage: reference-dev"),
            "{args:?}: {output}"
        );
    }
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}

/// The reset command a refusal prints works as printed: pasted into a shell,
/// the path stays one argument whatever it contains.
#[test]
fn the_reset_command_in_a_refusal_runs_as_printed() {
    for name in [
        "space dir.db",
        "it's.db",
        "$HOME;echo x.db",
        "a\"b`c`\\d*.db",
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(name);
        // A missing database beside a stale sidecar is refused, naming the reset.
        std::fs::write(dir.path().join(format!("{name}-journal")), b"stale").unwrap();
        let (mut server, stderr) = server_with(
            "http://127.0.0.1:4001",
            &["--local-oidc-demo", "--database", path.to_str().unwrap()],
        );
        let (success, output) = exited(&mut server, stderr);
        assert!(!success, "{name}");
        let command = output
            .split_once("run: ")
            .unwrap_or_else(|| panic!("no command in {output}"))
            .1
            .trim();
        assert!(command.starts_with("reference-dev "), "{command}");

        // The command names the binary bare, so the shell finds it on PATH.
        let binary = std::path::Path::new(env!("CARGO_BIN_EXE_reference-dev"));
        assert_eq!(binary.file_name().unwrap(), "reference-dev");
        let search = format!(
            "{}:{}",
            binary.parent().unwrap().display(),
            std::env::var("PATH").unwrap()
        );
        let ran = Command::new("sh")
            .args(["-c", command])
            .env("PATH", search)
            .env("IRIS_OIDC_ISSUER", "http://127.0.0.1:4001")
            .output()
            .unwrap();
        let printed = String::from_utf8_lossy(&ran.stderr);
        assert!(ran.status.success(), "{command}: {printed}");
        assert!(printed.contains("reset database at"), "{printed}");
        assert!(path.is_file(), "{name}");
        assert!(
            !dir.path().join(format!("{name}-journal")).exists(),
            "{name}"
        );
    }
}

/// Sends `name` (TERM or INT) to a running server.
fn signal(child: &Owned, name: &str) {
    let status = Command::new("kill")
        .args([format!("-{name}"), child.0.id().to_string()])
        .status()
        .unwrap();
    assert!(status.success());
}

/// Waits for the child's own exit and returns its code and whole stderr.
fn code(child: &mut Owned, stderr: mpsc::Receiver<String>) -> (Option<i32>, String) {
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            break status;
        }
        assert!(started.elapsed() < WAIT, "still running");
        std::thread::sleep(Duration::from_millis(20));
    };
    (status.code(), stderr.iter().collect::<Vec<_>>().join("\n"))
}

/// Bob's role in project 41, read directly from the database.
async fn role(path: &str) -> String {
    use sqlx::Connection;
    let mut conn = iris_reference::app::connect(std::path::Path::new(path))
        .await
        .unwrap();
    let role =
        sqlx::query_scalar("SELECT role FROM memberships WHERE project_id=41 AND user_id=29")
            .fetch_one(&mut conn)
            .await
            .unwrap();
    conn.close().await.unwrap();
    role
}

async fn csrf(http: &reqwest::Client, address: &str, cookie: &str) -> String {
    let session = http
        .get(format!("{address}/api/auth/session"))
        .header("cookie", cookie)
        .send()
        .await
        .unwrap();
    session.json::<Value>().await.unwrap()["csrf_token"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn no_redirects() -> reqwest::Client {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap()
}

#[tokio::test]
async fn a_signal_stops_an_idle_server_cleanly_and_releases_its_database() {
    let (_provider, issuer) = issuer();
    for name in ["TERM", "INT"] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("dev.db");
        let path = path.to_str().unwrap();
        let args = ["--local-oidc-demo", "--database", path];
        let http = no_redirects();

        let (mut server, stderr) = server_with(&issuer, &args);
        let address = address(&listening(&stderr));
        let cookie = sign_in(&http, &address).await;
        let changed = http
            .post(format!("{address}/api/memberships/role"))
            .header("origin", ORIGIN)
            .header("x-iris-csrf", csrf(&http, &address, &cookie).await)
            .header("cookie", &cookie)
            .json(&json!({"project_id": "41", "user_id": "29", "role": "viewer"}))
            .send()
            .await
            .unwrap();
        assert_eq!(changed.status(), 200);

        let signalled = Instant::now();
        signal(&server, name);
        let (code, output) = code(&mut server, stderr);
        let elapsed = signalled.elapsed();
        assert_eq!(code, Some(0), "{name}: {output}");
        assert!(elapsed < Duration::from_secs(2), "{name}: {elapsed:?}");
        assert!(output.contains(&format!("received SIG{name}")), "{output}");
        assert!(output.contains("stopped after draining"), "{output}");
        assert!(!dir.path().join("dev.db-journal").exists());
        assert!(!dir.path().join("dev.db.iris-init").exists());

        // The lock is free at once, and the change was kept.
        let (_server, stderr) = server_with(&issuer, &args);
        listening(&stderr);
        assert_eq!(role(path).await, "viewer");
    }
}

/// A role change whose headers the server has admitted, shown by its
/// `100 Continue`, and whose body is still partly unsent. What the interim
/// response shows is that the server began reading the body; it says nothing
/// about authentication or the transaction.
struct Admitted {
    stream: TcpStream,
    rest: Vec<u8>,
}

fn admit(address: &str, cookie: &str, csrf: &str) -> Admitted {
    let host = address.strip_prefix("http://").unwrap();
    let body = json!({"project_id": "41", "user_id": "29", "role": "viewer"}).to_string();
    let mut stream = TcpStream::connect(host).unwrap();
    stream.set_read_timeout(Some(WAIT)).unwrap();
    write!(
        stream,
        "POST /api/memberships/role HTTP/1.1\r\nhost: {host}\r\norigin: {ORIGIN}\r\n\
         x-iris-csrf: {csrf}\r\ncookie: {cookie}\r\ncontent-type: application/json\r\n\
         content-length: {}\r\nexpect: 100-continue\r\n\r\n",
        body.len()
    )
    .unwrap();
    // The interim response, read to its end and parsed on its own.
    let mut interim = Vec::new();
    let mut byte = [0];
    while !interim.ends_with(b"\r\n\r\n") {
        assert_eq!(stream.read(&mut byte).unwrap(), 1, "no interim response");
        interim.push(byte[0]);
    }
    let interim = String::from_utf8(interim).unwrap();
    assert!(
        interim.starts_with("HTTP/1.1 100 Continue\r\n"),
        "{interim}"
    );
    let (first, rest) = body.as_bytes().split_at(10);
    stream.write_all(first).unwrap();
    Admitted {
        stream,
        rest: rest.to_owned(),
    }
}

/// Everything the server sends from here until it closes the connection.
fn remainder(stream: &mut TcpStream) -> String {
    let mut bytes = Vec::new();
    // A reset counts as the end: whatever arrived before it is kept.
    let _ = stream.read_to_end(&mut bytes);
    String::from_utf8_lossy(&bytes).into_owned()
}

/// Waits for a stderr line containing `needle`.
fn reported(stderr: &mpsc::Receiver<String>, needle: &str) {
    let started = Instant::now();
    loop {
        let line = stderr
            .recv_timeout(WAIT.saturating_sub(started.elapsed()))
            .unwrap_or_else(|_| panic!("the server never reported {needle}"));
        if line.contains(needle) {
            return;
        }
    }
}

/// Waits, bounded, until a new connection to the server fails.
fn refused(address: &str) {
    let host = address.strip_prefix("http://").unwrap();
    let started = Instant::now();
    while TcpStream::connect(host).is_ok() {
        assert!(started.elapsed() < WAIT, "still accepting connections");
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[tokio::test]
async fn a_mutation_in_flight_at_the_signal_completes_and_is_acknowledged() {
    let (_provider, issuer) = issuer();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("dev.db");
    let path = path.to_str().unwrap();
    let args = ["--local-oidc-demo", "--database", path];
    let http = no_redirects();
    let (mut server, stderr) = server_with(&issuer, &args);
    let address = address(&listening(&stderr));
    let cookie = sign_in(&http, &address).await;
    let token = csrf(&http, &address, &cookie).await;
    drop(http);

    let mut admitted = admit(&address, &cookie, &token);
    signal(&server, "TERM");
    reported(&stderr, "received SIGTERM");
    refused(&address);
    assert!(server.0.try_wait().unwrap().is_none());

    admitted.stream.write_all(&admitted.rest).unwrap();
    let response = remainder(&mut admitted.stream);
    assert!(response.starts_with("HTTP/1.1 200 "), "{response}");
    let envelope: Value = serde_json::from_str(response.split_once("\r\n\r\n").unwrap().1).unwrap();
    assert_eq!(envelope["kind"], "success");
    assert_eq!(envelope["operation"], "memberships.change_role");

    let (code, output) = code(&mut server, stderr);
    assert_eq!(code, Some(0), "{output}");
    assert!(output.contains("stopped after draining"), "{output}");
    let (_server, stderr) = server_with(&issuer, &args);
    listening(&stderr);
    assert_eq!(role(path).await, "viewer");
}

#[tokio::test]
async fn a_mutation_never_finished_is_abandoned_at_the_deadline_without_an_answer() {
    let (_provider, issuer) = issuer();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("dev.db");
    let path = path.to_str().unwrap();
    let args = ["--local-oidc-demo", "--database", path];
    let http = no_redirects();
    let (mut server, stderr) = server_with(&issuer, &args);
    let address = address(&listening(&stderr));
    let cookie = sign_in(&http, &address).await;
    let token = csrf(&http, &address, &cookie).await;
    drop(http);

    let mut admitted = admit(&address, &cookie, &token);
    let signalled = Instant::now();
    signal(&server, "TERM");
    reported(&stderr, "received SIGTERM");
    refused(&address);
    // A second signal neither ends the drain nor extends it.
    std::thread::sleep(Duration::from_secs(1));
    signal(&server, "TERM");

    let (code, output) = code(&mut server, stderr);
    let elapsed = signalled.elapsed();
    assert_eq!(code, Some(1), "{output}");
    assert!(elapsed >= Duration::from_millis(2900), "{elapsed:?}");
    // Inside the supervisor's 5 s kill, with a margin.
    assert!(elapsed < Duration::from_millis(4500), "{elapsed:?}");
    assert!(output.contains("unconfirmed"), "{output}");
    assert!(output.contains("closure was not established"), "{output}");
    assert!(
        !output.contains("received SIGTERM"),
        "a second drain: {output}"
    );
    // No final response follows the interim one: never acknowledged.
    let response = remainder(&mut admitted.stream);
    assert!(!response.contains("HTTP/1.1"), "{response}");

    let (_server, stderr) = server_with(&issuer, &args);
    listening(&stderr);
    assert_eq!(role(path).await, "editor");
}

/// A Busy from either read invalidates the entire observation.
async fn startup_rows(conn: &mut sqlx::SqliteConnection) -> Result<Vec<String>, sqlx::Error> {
    let mut rows: Vec<String> = sqlx::query_scalar("SELECT id FROM iris_sessions")
        .fetch_all(&mut *conn)
        .await?;
    rows.extend(
        sqlx::query_scalar::<_, String>("SELECT state FROM iris_login_attempts")
            .fetch_all(&mut *conn)
            .await?,
    );
    Ok(rows)
}

async fn wait_for_startup_cleanup(
    conn: &mut sqlx::SqliteConnection,
    deadline: tokio::time::Instant,
) -> Result<(), String> {
    let mut last = "no completed observation".to_owned();
    // One absolute budget includes both reads and every asynchronous retry sleep.
    let result = tokio::time::timeout_at(deadline, async {
        loop {
            if tokio::time::Instant::now() >= deadline {
                return Ok(false);
            }
            match startup_rows(conn).await {
                Ok(rows) => {
                    last = format!("unexpected rows {rows:?}");
                    // timeout_at may poll an immediately ready future after expiry.
                    if tokio::time::Instant::now() >= deadline {
                        return Ok(false);
                    }
                    if rows == ["live", "live"] {
                        return Ok(true);
                    }
                }
                Err(error) if iris_reference::app::is_busy(&error) => last = error.to_string(),
                Err(error) => return Err(error),
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await;
    match result {
        Ok(Ok(true)) => Ok(()),
        Ok(Err(error)) => Err(format!("startup cleanup: {error}")),
        _ => Err(format!(
            "startup cleanup: observation deadline expired; last: {last}"
        )),
    }
}

async fn startup_observer_fixture() -> (tempfile::TempDir, sqlx::SqliteConnection) {
    let dir = tempfile::tempdir().unwrap();
    let mut conn = iris_reference::app::connect(&dir.path().join("dev.db"))
        .await
        .unwrap();
    iris_reference::app::MIGRATOR.run(&mut conn).await.unwrap();
    sqlx::raw_sql(
        "INSERT INTO iris_sessions VALUES('live','{}',4102444800);
         INSERT INTO iris_login_attempts VALUES('live','b','n','v',4102444800)",
    )
    .execute(&mut conn)
    .await
    .unwrap();
    (dir, conn)
}

#[tokio::test]
async fn startup_observer_transient_busy_waits_for_release() {
    use sqlx::Connection;
    let (dir, mut conn) = startup_observer_fixture().await;
    let mut writer = iris_reference::app::connect(&dir.path().join("dev.db"))
        .await
        .unwrap();
    let tx = writer.begin_with("BEGIN EXCLUSIVE").await.unwrap();
    assert!(iris_reference::app::is_busy(
        &startup_rows(&mut conn).await.unwrap_err()
    ));
    let observation = wait_for_startup_cleanup(
        &mut conn,
        tokio::time::Instant::now() + Duration::from_secs(2),
    );
    tokio::pin!(observation);
    assert!(
        tokio::time::timeout(Duration::from_millis(250), &mut observation)
            .await
            .is_err(),
        "observer returned before explicit lock release"
    );
    tx.rollback().await.unwrap();
    tokio::time::timeout(Duration::from_secs(3), observation)
        .await
        .expect("outer watchdog")
        .unwrap();
}

#[tokio::test]
async fn startup_observer_persistent_busy_expires_with_context() {
    use sqlx::Connection;
    let (dir, mut conn) = startup_observer_fixture().await;
    let mut writer = iris_reference::app::connect(&dir.path().join("dev.db"))
        .await
        .unwrap();
    let tx = writer.begin_with("BEGIN EXCLUSIVE").await.unwrap();
    assert!(iris_reference::app::is_busy(
        &startup_rows(&mut conn).await.unwrap_err()
    ));
    let deadline = tokio::time::Instant::now() + Duration::from_millis(250);
    let error = tokio::time::timeout(
        Duration::from_secs(2),
        wait_for_startup_cleanup(&mut conn, deadline),
    )
    .await
    .expect("outer watchdog")
    .unwrap_err();
    assert!(
        tokio::time::Instant::now() >= deadline,
        "returned before deadline: {error}"
    );
    assert!(
        error.contains("startup cleanup")
            && error.contains("deadline expired")
            && error.contains("database is locked"),
        "{error}"
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn startup_observer_missing_second_table_fails_immediately() {
    let (_dir, mut conn) = startup_observer_fixture().await;
    sqlx::query("DROP TABLE iris_login_attempts")
        .execute(&mut conn)
        .await
        .unwrap();
    let first: Vec<String> = sqlx::query_scalar("SELECT id FROM iris_sessions")
        .fetch_all(&mut conn)
        .await
        .unwrap();
    assert_eq!(first, ["live"]);
    let original = startup_rows(&mut conn).await.unwrap_err();
    assert!(!iris_reference::app::is_busy(&original));
    let error = tokio::time::timeout(
        Duration::from_secs(1),
        wait_for_startup_cleanup(
            &mut conn,
            tokio::time::Instant::now() + Duration::from_secs(10),
        ),
    )
    .await
    .expect("non-Busy must not wait for the row deadline")
    .unwrap_err();
    assert!(error.contains(&original.to_string()), "{error}");
    assert!(
        error.contains("no such table: iris_login_attempts"),
        "{error}"
    );
}

#[tokio::test]
async fn startup_observer_wrong_rows_cannot_succeed() {
    let (_dir, mut conn) = startup_observer_fixture().await;
    for (sql, expected) in [
        ("INSERT INTO iris_sessions VALUES('old','{}',1)", "old"),
        ("DELETE FROM iris_sessions", "[\"live\"]"),
        ("DELETE FROM iris_login_attempts", "[]"),
    ] {
        sqlx::raw_sql(sqlx::AssertSqlSafe(sql))
            .execute(&mut conn)
            .await
            .unwrap();
        let deadline = tokio::time::Instant::now() + Duration::from_millis(150);
        let error = tokio::time::timeout(
            Duration::from_secs(2),
            wait_for_startup_cleanup(&mut conn, deadline),
        )
        .await
        .expect("outer watchdog")
        .unwrap_err();
        assert!(tokio::time::Instant::now() >= deadline);
        assert!(
            error.contains("deadline expired") && error.contains(expected),
            "{error}"
        );
    }
}

#[tokio::test]
async fn startup_observer_expired_deadline_rejects_even_valid_rows() {
    let (_dir, mut conn) = startup_observer_fixture().await;
    assert_eq!(startup_rows(&mut conn).await.unwrap(), ["live", "live"]);
    let error = wait_for_startup_cleanup(
        &mut conn,
        tokio::time::Instant::now() - Duration::from_millis(1),
    )
    .await
    .unwrap_err();
    assert!(error.contains("deadline expired"), "{error}");
}

#[tokio::test]
async fn a_start_deletes_expired_sessions_and_login_attempts_and_keeps_live_ones() {
    use sqlx::Connection;
    let (_provider, issuer) = issuer();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("dev.db");
    let args = ["--local-oidc-demo", "--database", path.to_str().unwrap()];
    let (mut server, stderr) = server_with(&issuer, &args);
    listening(&stderr);
    signal(&server, "TERM");
    assert_eq!(code(&mut server, stderr).0, Some(0));

    let far = "4102444800";
    let mut conn = iris_reference::app::connect(&path).await.unwrap();
    sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
        "INSERT INTO iris_sessions(id,data,expires_at) VALUES('old','{{}}',1),('live','{{}}',{far});
         INSERT INTO iris_login_attempts(state,browser_id,nonce,verifier,expires_at)
         VALUES('old','b','n','v',1),('live','b','n','v',{far})"
    )))
    .execute(&mut conn)
    .await
    .unwrap();

    let (_server, stderr) = server_with(&issuer, &args);
    listening(&stderr);
    // The first tick, at start; the next is a minute away.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    wait_for_startup_cleanup(&mut conn, deadline).await.unwrap();
    conn.close().await.unwrap();
}
