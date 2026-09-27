//! Smoke test for the development binary: flag enforcement, startup and the
//! served session and domain boundary. The browser workflow covers the rest.
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
    let mut command = Command::new(env!("CARGO_BIN_EXE_reference-dev"));
    if flag {
        command.arg("--local-oidc-demo");
    }
    let mut child = Owned(
        command
            .env("IRIS_PUBLIC_ORIGIN", ORIGIN)
            .env("IRIS_OIDC_ISSUER", issuer)
            .env("IRIS_LISTEN", "127.0.0.1:0")
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    );
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
