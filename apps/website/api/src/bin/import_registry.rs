//! `import-registry` — ingest T-150 registry envelopes (items + compat edges)
//! into Postgres (T-068.9). Successor of the Go `cmd/import-registry-items`.
//!
//! ```text
//! import-registry [--items <path>] [--compat <path>] [--modpack <uuid>] [--prune]
//! ```
//!
//! At least one of `--items` / `--compat` is required. `--modpack` overrides the
//! envelope `modpackId`; `--prune` deletes modpack-scoped rows absent from the
//! envelope (full-scan-set semantics). Reads `DATABASE_URL` from the environment
//! (`.env` honored); runs migrations first so a fresh DB works out of the box.

use uuid::Uuid;
use website_api::db;
use website_api::services::registry_import::{ImportCounts, import_compat, import_items};

struct Args {
    items: Option<String>,
    compat: Option<String>,
    modpack: Option<Uuid>,
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
                args.modpack =
                    Some(Uuid::parse_str(&raw).map_err(|_| format!("bad --modpack uuid: {raw}"))?);
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
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let args = parse_args().map_err(|e| anyhow::anyhow!("{e}"))?;
    let url = std::env::var("DATABASE_URL")
        .map_err(|_| anyhow::anyhow!("DATABASE_URL is not set (env or .env)"))?;

    let pool = db::connect(&url).await?;
    db::migrate(&pool).await?;

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
