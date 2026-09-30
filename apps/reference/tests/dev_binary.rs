//! Smoke test for the development binary: flag enforcement, startup, the
//! served session and domain boundary, and the database reset. The browser
//! workflow covers the rest.
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader},
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
