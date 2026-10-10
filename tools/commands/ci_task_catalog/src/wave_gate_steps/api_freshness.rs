//! `cargo xtask mk preflight-api-freshness`: is the API on `:8080` up, and is it running current
//! code?
//!
//! **Role:** the preflight check the ticket manager's `[[preflight.check]]` names: `/healthz`
//! answers 200 and the listening process started after the newest commit under `crates/api`.
//! **Position:** run by `ttm preflight` (warn severity); reads `curl`, `ss`, `stat` and git.
//! **Signals & state:** none; read-only probes.
//! **Invariants:** prints one detail line; exits 0 only for a healthy, current API, so a stale or
//! wedged API is reported, never passed.

use super::host;
use super::step_context::Ctx;

/// The source folder of the running API.
const API_SOURCE_FOLDER: &str = "crates/api";

fn capture(ctx: &Ctx, argv: &[&str]) -> String {
    let full = if ctx.host.bridge {
        let mut v = host::v(&["distrobox-host-exec"]);
        v.extend(host::v(argv));
        v
    } else {
        host::v(argv)
    };
    match process_runner::Run::new(&full[0]).args(&full[1..]).output() {
        Ok(o) => o.stdout.trim().to_string(),
        Err(_) => String::new(),
    }
}

fn hhmm(ctx: &Ctx, epoch: i64) -> String {
    capture(ctx, &["date", "-d", &format!("@{epoch}"), "+%H:%M"])
}

pub(crate) fn api_freshness(ctx: &Ctx) -> i32 {
    let code = capture(
        ctx,
        &[
            "curl",
            "-s",
            "-o",
            "/dev/null",
            "-w",
            "%{http_code}",
            "-m",
            "4",
            "http://127.0.0.1:8080/healthz",
        ],
    );
    if code != "200" {
        if !code.is_empty() && code != "000" {
            println!("listening but /healthz returned {code} — wedged or mid-restart");
        } else {
            println!("down — editor smokes would report gate-red for an environment reason");
        }
        return 1;
    }
    let pid = capture(ctx, &["ss", "-ltnp"])
        .lines()
        .filter(|l| l.contains(":8080"))
        .find_map(|l| {
            let rest = &l[l.find("pid=")? + 4..];
            let pid: String = rest.chars().take_while(char::is_ascii_digit).collect();
            (!pid.is_empty()).then_some(pid)
        })
        .unwrap_or_default();
    let started: i64 = if pid.is_empty() {
        0
    } else {
        capture(ctx, &["stat", "-c", "%Y", &format!("/proc/{pid}")])
            .parse()
            .unwrap_or(0)
    };
    let newest: i64 = super::step_context::git_stdout(&[
        "-C",
        &ctx.main_root.display().to_string(),
        "log",
        "-1",
        "--format=%ct",
        "--",
        API_SOURCE_FOLDER,
    ])
    .and_then(|s| s.trim().parse().ok())
    .unwrap_or(0);
    if started > 0 && newest > started {
        println!(
            "healthy but STALE — running since {}, API code changed {}. Restart it or verifications lie.",
            hhmm(ctx, started),
            hhmm(ctx, newest)
        );
        return 1;
    }
    println!("healthz 200, binary current");
    0
}
