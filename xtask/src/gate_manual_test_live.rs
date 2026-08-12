//! Live REST + full-server arms for T-859 `mod manual-test` (SIZE-1 split).

use std::fs;
use std::net::TcpStream;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use tbd_gate::proc::{self, Run};

use super::{Acc, ChildGuard, Paths};

pub(super) fn curl_code(args: &[&str], body_path: Option<&Path>) -> String {
    let mut run = Run::new("curl").arg("-sS");
    if let Some(p) = body_path {
        run = run.arg("-o").arg(p);
    } else {
        run = run.arg("-o").arg("/dev/null");
    }
    run = run.arg("-w").arg("%{http_code}");
    for a in args {
        run = run.arg(*a);
    }
    match run.merged_output() {
        Ok(m) => m.text.trim().to_string(),
        Err(_) => String::new(),
    }
}

fn body_contains(path: &Path, needle: &str) -> bool {
    fs::read_to_string(path)
        .map(|t| t.contains(needle))
        .unwrap_or(false)
}

fn spawn_restspike(p: &Paths) -> Result<ChildGuard, ()> {
    let bin = Path::new("/tmp/tbd-restspike");
    let log = fs::File::create("/tmp/tbd-restspike.log").map_err(|_| ())?;
    let log_err = log.try_clone().map_err(|_| ())?;
    let child = Command::new(bin)
        .env("GAME_SERVER_TOKENS", "test-manual-token")
        .env("MISSIONS_DIR", p.web.join("missions"))
        .env("PORT", "8199")
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(log_err))
        .spawn()
        .map_err(|_| ())?;
    thread::sleep(Duration::from_millis(1200));
    Ok(ChildGuard(Some(child)))
}

pub(super) fn run_restspike_suite(p: &Paths, a: &mut Acc) -> Result<(), u8> {
    let mut guard = match spawn_restspike(p) {
        Ok(g) => g,
        Err(()) => {
            a.fail("build restspike");
            return Err(1);
        }
    };
    let base = "http://127.0.0.1:8199";
    let auth = "Authorization: Bearer test-manual-token";

    let m1 = Path::new("/tmp/m1.json");
    let code = curl_code(
        &[
            "-H",
            auth,
            &format!("{base}/api/missions/msn_8f3a2c/compiled"),
        ],
        Some(m1),
    );
    if code == "200" && body_contains(m1, "\"Bridgehead at Levie\"") {
        a.pass("GET /api/missions/msn_8f3a2c/compiled -> 200 + correct name");
    } else {
        a.fail(&format!("GET mission msn_8f3a2c (code={code})"));
    }

    let m2 = Path::new("/tmp/m2.json");
    let code = curl_code(
        &[
            "-H",
            auth,
            &format!("{base}/api/missions/msn_2d91be/compiled"),
        ],
        Some(m2),
    );
    if code == "200" && body_contains(m2, "\"Last Stand at Montfort\"") {
        a.pass("GET /api/missions/msn_2d91be/compiled -> 200 + correct name");
    } else {
        a.fail(&format!("GET mission msn_2d91be (code={code})"));
    }

    let code = curl_code(
        &[
            "-H",
            auth,
            &format!("{base}/api/missions/msn_nope/compiled"),
        ],
        None,
    );
    if code == "404" {
        a.pass("GET missing mission -> 404");
    } else {
        a.fail(&format!("GET missing mission (code={code})"));
    }

    let code = curl_code(&[&format!("{base}/api/missions/msn_8f3a2c/compiled")], None);
    if code == "401" {
        a.pass("GET without token -> 401");
    } else {
        a.fail(&format!("GET without token (code={code})"));
    }

    let code = curl_code(
        &[
            "-H",
            "Authorization: Bearer wrong",
            &format!("{base}/api/missions/msn_8f3a2c/compiled"),
        ],
        None,
    );
    if code == "401" {
        a.pass("GET bad token -> 401");
    } else {
        a.fail(&format!("GET bad token (code={code})"));
    }

    let res = Path::new("/tmp/res.json");
    let code = curl_code(
        &[
            "-X",
            "POST",
            "-H",
            auth,
            "-H",
            "Content-Type: application/json",
            "-d",
            r#"{"missionId":"msn_8f3a2c","winner":"blufor"}"#,
            &format!("{base}/api/results"),
        ],
        Some(res),
    );
    if code == "202" && body_contains(res, "\"accepted\"") {
        a.pass("POST /api/results -> 202 accepted");
    } else {
        a.fail(&format!("POST /api/results (code={code})"));
    }

    let tel = Path::new("/tmp/tel.json");
    let code = curl_code(
        &[
            "-X",
            "POST",
            "-H",
            auth,
            "-H",
            "Content-Type: application/json",
            "-d",
            r#"{"missionId":"msn_8f3a2c","events":[{"t":1,"type":"capture"}]}"#,
            &format!("{base}/api/telemetry"),
        ],
        Some(tel),
    );
    if code == "202" && body_contains(tel, "\"accepted\"") {
        a.pass("POST /api/telemetry -> 202 accepted");
    } else {
        a.fail(&format!("POST /api/telemetry (code={code})"));
    }

    let code = curl_code(
        &[
            "-X",
            "POST",
            "-H",
            auth,
            "-H",
            "Content-Type: application/json",
            "-d",
            "not-json",
            &format!("{base}/api/telemetry"),
        ],
        None,
    );
    if code == "400" {
        a.pass("POST bad JSON -> 400");
    } else {
        a.fail(&format!("POST bad JSON (code={code})"));
    }

    let code = curl_code(
        &[
            "-H",
            auth,
            &format!("{base}/api/missions/..%2Fsecret/compiled"),
        ],
        None,
    );
    if code == "400" || code == "404" {
        a.pass(&format!("GET traversal-like id rejected ({code})"));
    } else {
        a.fail(&format!("GET traversal-like id (code={code})"));
    }

    let _ = guard.take().map(|mut c| {
        let _ = c.kill();
        let _ = c.wait();
    });
    Ok(())
}

fn tcp_open(port: u16) -> bool {
    TcpStream::connect_timeout(
        &format!("127.0.0.1:{port}").parse().unwrap(),
        Duration::from_millis(200),
    )
    .is_ok()
}

fn maybe_start_postgres() {
    // bash: command -v podman && names match → start || true
    if proc::which("podman").is_err() {
        return;
    }
    let Ok(out) = Run::new("podman")
        .arg("ps")
        .arg("-a")
        .arg("--format")
        .arg("{{.Names}}")
        .merged_output()
    else {
        return;
    };
    if !out.text.lines().any(|l| l.trim() == "tbdevent-postgres") {
        return;
    }
    let _ = Run::new("podman")
        .arg("start")
        .arg("tbdevent-postgres")
        .merged_output();
    thread::sleep(Duration::from_secs(1));
}

fn load_database_url(web: &Path) -> Option<String> {
    if let Ok(v) = std::env::var("DATABASE_URL")
        && !v.is_empty()
    {
        return Some(v);
    }
    let text = fs::read_to_string(web.join(".env")).ok()?;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(rest) = line.strip_prefix("DATABASE_URL=") {
            let v = rest.trim().trim_matches('"').trim_matches('\'');
            if !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }
    None
}

pub(super) fn run_full_server_suite(p: &Paths, a: &mut Acc) {
    maybe_start_postgres();

    let mut has_pg = false;
    for pgport in [5433_u16, 5432] {
        if tcp_open(pgport) {
            has_pg = true;
            break;
        }
    }
    let db_url = load_database_url(&p.web);

    if !(has_pg && db_url.is_some()) {
        a.skip("Full website API — Postgres not available");
        return;
    }
    let db_url = db_url.unwrap();

    let Ok(log) = fs::File::create("/tmp/tbd-server.log") else {
        a.fail("Full website API — could not open server log");
        return;
    };
    let Ok(log_err) = log.try_clone() else {
        a.fail("Full website API — could not open server log");
        return;
    };
    let child = Command::new("go")
        .args(["run", "./cmd/server"])
        .current_dir(&p.web)
        .env("SESSION_SECRET", "manual-test-secret")
        .env("GAME_SERVER_TOKENS", "test-manual-token")
        .env("MISSIONS_DIR", p.web.join("missions"))
        .env("PORT", "8198")
        .env("ENV", "development")
        .env("DATABASE_URL", &db_url)
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(log_err))
        .spawn();
    let Ok(child) = child else {
        a.fail("Full website API — go run ./cmd/server failed to spawn");
        return;
    };
    let mut guard = ChildGuard(Some(child));
    thread::sleep(Duration::from_secs(3));
    let fb = "http://127.0.0.1:8198";
    let auth = "Authorization: Bearer test-manual-token";

    let pages = Path::new("/tmp/pages.json");
    let code = curl_code(&[&format!("{fb}/api/pages")], Some(pages));
    if code == "200" {
        a.pass("GET /api/pages -> 200");
    } else {
        a.fail(&format!("GET /api/pages (code={code})"));
    }

    let rules = Path::new("/tmp/rules.json");
    let code = curl_code(&[&format!("{fb}/api/pages/rules")], Some(rules));
    if code == "200" {
        a.pass("GET /api/pages/rules -> 200");
    } else {
        a.fail(&format!("GET /api/pages/rules (code={code})"));
    }

    let events = Path::new("/tmp/events.json");
    let code = curl_code(&[&format!("{fb}/api/events?upcoming=true")], Some(events));
    if code == "200" {
        a.pass("GET /api/events?upcoming=true -> 200");
    } else {
        a.fail(&format!("GET /api/events (code={code})"));
    }

    let ev = Path::new("/tmp/ev.json");
    let code = curl_code(&[&format!("{fb}/api/events/tbd-pvp-1")], Some(ev));
    if code == "200" {
        a.pass("GET /api/events/tbd-pvp-1 -> 200");
    } else {
        a.fail(&format!("GET event detail (code={code})"));
    }

    let roster = Path::new("/tmp/roster.json");
    let code = curl_code(
        &[&format!("{fb}/api/events/tbd-pvp-1/roster")],
        Some(roster),
    );
    if code == "200" {
        a.pass("GET /api/events/tbd-pvp-1/roster -> 200");
    } else {
        a.fail(&format!("GET roster (code={code})"));
    }

    let ann = Path::new("/tmp/ann.json");
    let code = curl_code(&[&format!("{fb}/api/announcements")], Some(ann));
    if code == "200" {
        a.pass("GET /api/announcements -> 200");
    } else {
        a.fail(&format!("GET announcements (code={code})"));
    }

    let code = curl_code(&[&format!("{fb}/api/auth/me")], None);
    if code == "401" {
        a.pass("GET /api/auth/me without session -> 401");
    } else {
        a.fail(&format!("GET /api/auth/me (code={code})"));
    }

    let code = curl_code(&[&format!("{fb}/api/admin/pages/rules")], None);
    if code == "401" {
        a.pass("GET /api/admin/pages/rules without session -> 401");
    } else {
        a.fail(&format!("GET admin (code={code})"));
    }

    let code = curl_code(
        &[
            "-H",
            auth,
            &format!("{fb}/api/missions/msn_8f3a2c/compiled"),
        ],
        None,
    );
    if code == "200" {
        a.pass("Full server: GET game mission -> 200");
    } else {
        a.fail(&format!("Full server game mission (code={code})"));
    }

    let res = Path::new("/tmp/res.json");
    let code = curl_code(
        &[
            "-X",
            "POST",
            "-H",
            auth,
            "-H",
            "Content-Type: application/json",
            "-d",
            r#"{"missionId":"msn_8f3a2c","winner":"blufor"}"#,
            &format!("{fb}/api/results"),
        ],
        Some(res),
    );
    if code == "202" {
        a.pass("Full server: POST /api/results -> 202");
    } else {
        a.fail(&format!("Full server POST results (code={code})"));
    }

    let tel = Path::new("/tmp/tel.json");
    let code = curl_code(
        &[
            "-X",
            "POST",
            "-H",
            auth,
            "-H",
            "Content-Type: application/json",
            "-d",
            r#"{"missionId":"msn_8f3a2c","events":[]}"#,
            &format!("{fb}/api/telemetry"),
        ],
        Some(tel),
    );
    if code == "202" {
        a.pass("Full server: POST /api/telemetry -> 202");
    } else {
        a.fail(&format!("Full server POST telemetry (code={code})"));
    }

    let code = curl_code(&[&format!("{fb}/")], None);
    if code == "200" {
        a.pass("GET / (embedded SPA) -> 200");
    } else {
        a.fail(&format!("GET / static (code={code})"));
    }

    let _ = guard.take().map(|mut c| {
        let _ = c.kill();
        let _ = c.wait();
    });
}
