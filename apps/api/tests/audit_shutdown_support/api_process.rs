//! Runs the real `api` binary as a child process over this binary's private database: starts it on
//! a free port with an explicit environment, waits until it answers, sends it SIGTERM, and waits
//! for it to exit, keeping its log for the failure messages.

use std::fs::File;
use std::path::PathBuf;
use std::process::{ExitStatus, Stdio};
use std::time::{Duration, Instant};

use tokio::process::{Child, Command};
use tokio::time::timeout;
use uuid::Uuid;

/// How long the binary may take to answer its first request.
const START_BOUND: Duration = Duration::from_secs(60);

/// One running `api` process.
pub(crate) struct ApiProcess {
    child: Child,
    base_url: String,
    scratch: PathBuf,
}

impl ApiProcess {
    /// Starts the `api` binary against `database_url`, already migrated by the caller, and waits
    /// until it answers HTTP. Its working directory is a fresh scratch directory, so no `.env`
    /// file reaches it, and its environment holds only what is set here.
    pub(crate) async fn start(
        database_url: &str,
        jwt_secret: &str,
        discord_guild_id: &str,
    ) -> Self {
        let scratch = std::env::temp_dir().join(format!(
            "audit-replay-shutdown-{}-{}",
            std::process::id(),
            Uuid::new_v4()
        ));
        std::fs::create_dir_all(&scratch).expect("create the api process scratch directory");
        let log = File::create(scratch.join("api.log")).expect("create the api process log");
        let port = free_port();
        let child = Command::new(env!("CARGO_BIN_EXE_api"))
            .current_dir(&scratch)
            .env_clear()
            .env("APP_ENV", "development")
            .env("DATABASE_URL", database_url)
            .env("JWT_SECRET", jwt_secret)
            .env("DISCORD_GUILD_ID", discord_guild_id)
            .env("PORT", port.to_string())
            .env("SKIP_MIGRATE", "1")
            .env("UPLOAD_DIR", scratch.join("uploads"))
            .env("EQUIPMENT_DATA_DIR", scratch.join("equipment"))
            .env("RUST_LOG", "info")
            .env("NO_COLOR", "1")
            .stdin(Stdio::null())
            .stdout(log.try_clone().expect("share the api process log"))
            .stderr(log)
            .kill_on_drop(true)
            .spawn()
            .expect("spawn the api binary");
        let mut process = Self {
            child,
            base_url: format!("http://127.0.0.1:{port}"),
            scratch,
        };
        process.wait_answering().await;
        process
    }

    /// The absolute URL of `path` on this process.
    pub(crate) fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base_url)
    }

    /// Polls `GET /healthz` until any HTTP answer arrives; the process exiting first fails the
    /// case with its log.
    async fn wait_answering(&mut self) {
        let client = http_client(Some(Duration::from_secs(2)));
        let deadline = Instant::now() + START_BOUND;
        loop {
            if let Some(status) = self.child.try_wait().expect("poll the api process") {
                panic!(
                    "the api process exited with {status} before answering\n{}",
                    self.log()
                );
            }
            if client.get(self.url("/healthz")).send().await.is_ok() {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "the api process did not answer within {START_BOUND:?}\n{}",
                self.log()
            );
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// Sends SIGTERM, the signal a service manager stops the API with; answers when it was sent.
    pub(crate) fn terminate(&self) -> Instant {
        let pid = self.child.id().expect("the api process is running");
        let status = std::process::Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .status()
            .expect("run kill");
        assert!(status.success(), "kill -TERM {pid} failed: {status}");
        Instant::now()
    }

    /// Waits up to `bound` for the process to exit; answers its status. A clean exit removes the
    /// scratch directory; on a timeout the process is killed when dropped and the case fails with
    /// its log.
    pub(crate) async fn wait_exit(&mut self, bound: Duration) -> ExitStatus {
        let status = match timeout(bound, self.child.wait()).await {
            Ok(status) => status.expect("wait for the api process"),
            Err(_) => panic!(
                "the api process did not exit within {bound:?} of SIGTERM\n{}",
                self.log()
            ),
        };
        if status.success() {
            let _ = std::fs::remove_dir_all(&self.scratch);
        }
        status
    }

    /// The process log with the path it is kept at, for failure messages.
    pub(crate) fn log(&self) -> String {
        let path = self.scratch.join("api.log");
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        let tail: Vec<&str> = text.lines().rev().take(40).collect();
        let tail: Vec<&str> = tail.into_iter().rev().collect();
        format!(
            "api process log {} (last 40 lines):\n{}",
            path.display(),
            tail.join("\n")
        )
    }
}

/// An HTTP client for the process's routes, with an optional per-request timeout. The crate
/// builds reqwest without a bundled TLS provider, so the process-wide rustls provider is
/// installed first; a second install is refused and changes nothing.
pub(crate) fn http_client(request_timeout: Option<Duration>) -> reqwest::Client {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let builder = reqwest::Client::builder();
    let builder = match request_timeout {
        Some(bound) => builder.timeout(bound),
        None => builder,
    };
    builder.build().expect("build the HTTP client")
}

/// A TCP port nothing listens on at the moment of the call.
fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .and_then(|listener| listener.local_addr())
        .map(|address| address.port())
        .expect("find a free port")
}
