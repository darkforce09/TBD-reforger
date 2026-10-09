//! The `import-item-registry` binary: ingests registry envelopes (items and compatibility edges)
//! into Postgres.
//!
//! ```text
//! import-item-registry [--items <path>] [--compat <path>] [--modpack <uuid>] [--prune]
//! ```
//!
//! **Role:** at least one of `--items` / `--compat` is required. `--modpack` overrides the
//! envelope `modpackId`; `--prune` deletes modpack-scoped rows absent from the envelope
//! (full-scan-set semantics). Reads `DATABASE_URL` from the environment (`.env` honored); runs
//! migrations first so a fresh database works out of the box.
//! **Position:** the registry import binary of the `api_server` crate; `cargo xtask db
//! registry-import` runs it over the committed Workbench exports.
//! **Signals & state:** none; one pass over the named files, then exit.
//! **Invariants:** a refused command line, an unset `DATABASE_URL`, an unreadable file or a failed
//! import is an [`api_server::Error`], printed as `Error: <message>` with its causes, and exits 1.

use api_identifiers::ModpackId;
use api_missions::services::registry_import::{ImportCounts, import_compat, import_items};
use api_server::Error;

struct Args {
    items: Option<String>,
    compat: Option<String>,
    modpack: Option<ModpackId>,
    prune: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        items: None,
        compat: None,
        modpack: None,
        prune: false,
    };
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--items" => args.items = Some(it.next().ok_or("--items needs a path")?),
            "--compat" => args.compat = Some(it.next().ok_or("--compat needs a path")?),
            "--modpack" => {
                let raw = it.next().ok_or("--modpack needs a uuid")?;
                args.modpack = Some(
                    raw.parse()
                        .map_err(|_| format!("bad --modpack uuid: {raw}"))?,
                );
            }
            "--prune" => args.prune = true,
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    if args.items.is_none() && args.compat.is_none() {
        return Err("nothing to do: pass --items <path> and/or --compat <path>".into());
    }
    Ok(args)
}

fn print_counts(label: &str, c: &ImportCounts) {
    println!(
        "{label}: total={} unique={} inserted={} updated={} pruned={}",
        c.total, c.unique, c.inserted, c.updated, c.pruned
    );
    for (k, n) in &c.histogram {
        println!("  {k}: {n}");
    }
}

#[tokio::main]
async fn main() -> api_server::Result<()> {
    dotenvy::dotenv().ok();
    let args = parse_args().map_err(Error::CommandLine)?;
    let url = std::env::var("DATABASE_URL").map_err(|_| Error::DatabaseUrlMissing)?;

    let pool = api_database::connect(&url).await?;
    api_database::migrate(&pool).await?;

    if let Some(path) = &args.items {
        let raw = std::fs::read(path)?;
        let c = import_items(&pool, &raw, args.modpack, args.prune).await?;
        print_counts("items", &c);
    }
    if let Some(path) = &args.compat {
        let raw = std::fs::read(path)?;
        let c = import_compat(&pool, &raw, args.modpack, args.prune).await?;
        print_counts("compat", &c);
    }
    Ok(())
}
