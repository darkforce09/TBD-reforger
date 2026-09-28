//! The verify and accept loops of `gate v-suite` over the selected routes.
//!
//! - **Role:** captures each selected route, then diffs it against its frozen golden (`verify`)
//!   or re-sources the golden from the capture with the operator's note (`accept`).
//! - **Position:** called by `routes::run` after the argument checks and the browser launch; in
//!   accept mode it writes goldens, PNGs and `manifest.json` in the oracle-freeze folder.
//! - **Signals & state:** none held; accept mode rewrites files on disk.
//! - **Invariants:** a run that covered no route exits 2, a failed route makes it exit 1, and
//!   accept refuses an implausible capture before it overwrites a golden.

use super::*;

pub(super) async fn run_modes(
    browser: &Browser,
    gold: &Path,
    args: &VSuiteArgs,
    routes: &[&Route],
) -> Result<u8> {
    let mut rows: Vec<Value> = Vec::new();
    let mut fail = 0usize;
    for route in routes {
        match args.mode.as_str() {
            "accept" => {
                // Preserve the React capture as the historical reference, then re-golden.
                let gold_file = gold.join(format!("{}.dom.json", route.slug));
                let react_ref = gold.join(format!("{}.react.dom.json", route.slug));
                if gold_file.exists() && !react_ref.exists() {
                    std::fs::copy(&gold_file, &react_ref)?;
                }
                let cap = capture_route(browser, &args.leptos_dir, 5197, route).await?;
                // Verify parses before accept writes — refuse `"null"` /
                // undersized DOM before overwriting a committed golden.
                validate_accept_dom(&cap.dom)?;
                std::fs::write(&gold_file, &cap.dom)?;
                std::fs::write(gold.join(format!("{}.png", route.slug)), &cap.png)?;
                let manifest_path = gold.join("manifest.json");
                let mut manifest: Value =
                    serde_json::from_str(&std::fs::read_to_string(&manifest_path)?)?;
                let digest = sha_hex(cap.dom.as_bytes());
                if let Some(row) = manifest["routes"]
                    .as_array_mut()
                    .and_then(|a| a.iter_mut().find(|r| r["slug"] == route.slug))
                    && let Some(obj) = row.as_object_mut()
                {
                    obj.insert("goldenSource".into(), json!("leptos"));
                    obj.insert("bytes".into(), json!(js_len(&cap.dom)));
                    obj.insert("sha256".into(), json!(digest));
                    obj.insert("acceptedDelta".into(), json!(args.note));
                }
                std::fs::write(
                    &manifest_path,
                    serde_json::to_string_pretty(&manifest)? + "\n",
                )?;
                println!(
                    "accept {:<14} {:>8} B  {}  (react ref kept)",
                    route.slug,
                    js_len(&cap.dom),
                    &digest[..12]
                );
            }
            _ => {
                let gold_file = gold.join(format!("{}.dom.json", route.slug));
                if !gold_file.exists() {
                    rows.push(
                        json!({ "slug": route.slug, "pass": false, "error": "missing golden" }),
                    );
                    fail += 1;
                    continue;
                }
                let golden = std::fs::read_to_string(&gold_file)?;
                // A route that cannot be captured is one failing route, not the end of the run:
                // stopping here would report the first broken route and hide the rest of the debt.
                let cap = match capture_route(browser, &args.leptos_dir, 5196, route).await {
                    Ok(cap) => cap,
                    Err(e) => {
                        println!("FAIL   {:<14} {e}", route.slug);
                        rows.push(json!({
                            "slug": route.slug, "path": route.path,
                            "pass": false, "error": e.to_string(),
                        }));
                        fail += 1;
                        continue;
                    }
                };
                let mut out = Vec::new();
                diff_node(
                    &serde_json::from_str(&golden)?,
                    &serde_json::from_str(&cap.dom)?,
                    "approot",
                    &mut out,
                    40,
                );
                let pass = out.is_empty();
                if !pass {
                    fail += 1;
                }
                println!(
                    "{}   {:<14} diffs={}  {}→{} B",
                    if pass { "PASS" } else { "FAIL" },
                    route.slug,
                    out.len(),
                    js_len(&golden),
                    js_len(&cap.dom)
                );
                rows.push(json!({
                    "slug": route.slug, "path": route.path, "pass": pass, "diffs": out.len(),
                    "goldenBytes": js_len(&golden), "leptosBytes": js_len(&cap.dom),
                    "first": out.iter().take(5).collect::<Vec<_>>(),
                }));
            }
        }
    }

    match args.mode.as_str() {
        "accept" => {
            println!(
                "\naccepted {} route(s) — golden re-sourced from Leptos, React reference kept",
                routes.len()
            );
        }
        _ => {
            println!(
                "\n{}/{} routes match the frozen oracle",
                rows.len() - fail,
                rows.len()
            );
            if fail > 0 {
                for r in rows.iter().filter(|x| x["pass"] == false) {
                    println!("{}", serde_json::to_string_pretty(r)?);
                }
            }
        }
    }
    Ok(run_exit_code(routes.len(), fail))
}

/// The exit code of a finished run over `covered` routes of which `failed` failed: 2 when it
/// covered no route (an empty selection is a usage error, never a pass), 1 when any route failed,
/// 0 otherwise.
pub(super) fn run_exit_code(covered: usize, failed: usize) -> u8 {
    if covered == 0 {
        2
    } else {
        u8::from(failed > 0)
    }
}
