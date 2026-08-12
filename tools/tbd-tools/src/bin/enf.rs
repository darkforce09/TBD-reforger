//! `enf` — T-181 Enfusion oracle CLI.
//!
//! Builds and queries the mechanical indexes that let a session answer "how does CRF do X"
//! (and, from T-181.3, "what does vanilla actually do") in seconds, with a real `file:line`
//! instead of a plausible-sounding invention.
//!
//!   enf index crf --root apps/mod/crf_framework --out .ai/artifacts/enf-index
//!   enf lookup CRF_EGamemodeState --index .ai/artifacts/enf-index/crf_symbols.tsv
//!   enf dirs --index .ai/artifacts/enf-index/crf_symbols.tsv --depth 4

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use tbd_tools::enf::{apidoc, capability, carve, citations, index, source};
use tbd_tools::world::pak::PakVfs;

#[derive(Parser)]
#[command(
    name = "enf",
    about = "T-181 Enfusion oracle: index + query CRF and vanilla scripts"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Build a symbol index over a tree of Enfusion `.c` sources.
    Index {
        /// `crf` or `vanilla` — selects the output filename prefix.
        lane: String,
        #[arg(long)]
        root: PathBuf,
        #[arg(long, default_value = ".ai/artifacts/enf-index")]
        out: PathBuf,
    },
    /// Carve vanilla Enfusion source out of the shipped .pak archives.
    /// Output is BI copyrighted content: gitignored, never committed.
    Carve {
        #[arg(long)]
        game: PathBuf,
        #[arg(long, default_value = "apps/mod/vanilla_reference")]
        out: PathBuf,
    },
    /// Parse the cached official Script API docs into the oracle index.
    /// Fetch them first with `cargo xtask fetch vanilla-api`.
    Apidoc {
        #[arg(long, default_value = "apps/mod/vanilla_reference/apidoc")]
        src: PathBuf,
        #[arg(long, default_value = ".ai/artifacts/enf-index")]
        out: PathBuf,
    },
    /// Verify every `@idx lane#Symbol` citation in the docs resolves against an index.
    /// Exits 1 on any unresolved citation — hallucinated APIs fail the build.
    Citations {
        #[arg(long, default_value = "docs/mod")]
        docs: PathBuf,
        #[arg(long, default_value = ".ai/artifacts/enf-index")]
        index_dir: PathBuf,
    },
    /// Extract vanilla scripts from the paks BY NAME via the pak file table.
    /// Supersedes `carve` — real paths, complete files, no byte-scanning.
    Extract {
        #[arg(long, default_value = "apps/mod/vanilla_reference/Scripts")]
        out: PathBuf,
        /// Only extract paths starting with this prefix.
        #[arg(long, default_value = "scripts/")]
        prefix: String,
    },
    /// Dump one pak entry's RAW compressed bytes — codec identification (T-181.3.3).
    DumpEntry {
        path: String,
        #[arg(long, default_value = "/tmp/entry.bin")]
        out: PathBuf,
    },
    /// Reconstruct vanilla .c source (WITH method bodies) from cached Doxygen source pages.
    Source {
        #[arg(long, default_value = "apps/mod/vanilla_reference/source_html")]
        src: PathBuf,
        #[arg(long, default_value = "apps/mod/vanilla_reference/Source")]
        out: PathBuf,
    },
    /// Resolve a symbol to `file:line`. Exits 1 when it does not exist.
    Lookup {
        symbol: String,
        #[arg(long, default_value = ".ai/artifacts/enf-index/crf_symbols.tsv")]
        index: PathBuf,
    },
    /// Join the CRF index against the hand-authored verdict table.
    /// Exits 1 if any CRF file has no verdict (UNTRIAGED) — a forgotten capability
    /// must be a build error, not an oversight.
    Capability {
        #[arg(long, default_value = ".ai/artifacts/enf-index")]
        index_dir: PathBuf,
        #[arg(long, default_value = "docs/mod/capability_verdicts.tsv")]
        verdicts: PathBuf,
    },
    /// Symbol counts per directory — where a subsystem actually lives.
    Dirs {
        #[arg(long, default_value = ".ai/artifacts/enf-index/crf_symbols.tsv")]
        index: PathBuf,
        #[arg(long, default_value_t = 4)]
        depth: usize,
        /// Only show directories with at least this many symbols.
        #[arg(long, default_value_t = 1)]
        min: usize,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("enf: {e:#}");
            ExitCode::from(2)
        }
    }
}

fn run(cli: Cli) -> anyhow::Result<ExitCode> {
    match cli.cmd {
        Cmd::Index { lane, root, out } => {
            let prefix = match lane.as_str() {
                "crf" => "crf",
                "vanilla" => "vanilla",
                other => anyhow::bail!("unknown lane '{other}' (expected crf|vanilla)"),
            };
            if !root.exists() {
                anyhow::bail!(
                    "source root {} does not exist.\n\
                     (crf_framework is gitignored — it must be present locally to reindex)",
                    root.display()
                );
            }
            let st = index::build(&root, &out, prefix)?;
            println!("indexed {} ({prefix} lane)", root.display());
            println!("  files    {}", st.files);
            println!("  loc      {}", st.loc);
            println!("  symbols  {}  ({} declarations)", st.symbols, st.classes);
            println!("  rplprops {}", st.rpl_props);
            println!("  -> {}/{prefix}_*.tsv", out.display());
            if st.files == 0 {
                eprintln!("enf: no .c files found — nothing indexed");
                return Ok(ExitCode::from(1));
            }
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Carve { game, out } => {
            if !game.join("addons").is_dir() {
                anyhow::bail!(
                    "{} has no addons/ — expected an Arma Reforger install",
                    game.display()
                );
            }
            let st = carve::carve(&game, &out)?;
            println!("carved {} pak(s) -> {}", st.paks, out.display());
            println!("  blobs >=2KB   {}", st.blobs_seen);
            println!(
                "  kept as script {}  ({:.1} MB)",
                st.blobs_kept,
                st.bytes_kept as f64 / 1e6
            );
            println!("  cross-pak dupes {}", st.duplicates);
            if st.blobs_kept == 0 {
                eprintln!("enf: carved nothing — pak layout may have changed");
                return Ok(ExitCode::from(1));
            }
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Apidoc { src, out } => {
            let st = apidoc::build(&src, &out)?;
            println!("parsed official Script API docs");
            println!("  classes       {}", st.classes);
            println!("  member pages  {}", st.member_pages);
            println!("  signatures    {}", st.members);
            println!("  -> {}/vanilla_api_*.tsv", out.display());
            if st.classes == 0 {
                eprintln!("enf: parsed no classes — doc layout may have changed");
                return Ok(ExitCode::from(1));
            }
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Citations { docs, index_dir } => {
            let rep = citations::verify(&docs, &index_dir)?;
            println!(
                "checked {} citation(s) across {} doc(s)",
                rep.checked, rep.docs
            );
            if !rep.unresolved.is_empty() {
                eprintln!("\nUNRESOLVED CITATIONS:");
                for (c, why) in &rep.unresolved {
                    eprintln!(
                        "  {}:{}: @idx {}#{} — {}",
                        c.file, c.line, c.lane, c.symbol, why
                    );
                }
                eprintln!(
                    "\nEither the symbol does not exist (the claim is wrong) or the index is stale"
                );
                eprintln!(
                    "(rebuild: cargo run -q -p tbd-tools --bin enf -- index crf / cargo run -q -p tbd-tools --bin enf -- carve / cargo run -q -p tbd-tools --bin enf -- apidoc)."
                );
                return Ok(ExitCode::from(1));
            }
            println!("all citations resolve.");
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Extract { out, prefix } => {
            let vfs = PakVfs::open_default()?;
            let paths: Vec<String> = vfs
                .all_file_paths()
                .into_iter()
                .filter(|p| p.starts_with(&prefix) && p.ends_with(".c"))
                .map(|s| s.to_string())
                .collect();
            println!(
                "{} script(s) in the pak file table under '{}'",
                paths.len(),
                prefix
            );
            let _ = std::fs::remove_dir_all(&out);
            let (mut ok, mut failed, mut bytes) = (0usize, 0usize, 0u64);
            let mut first_err = String::new();
            for p in &paths {
                match vfs.read_file(p) {
                    Ok(data) => {
                        let dest = out.join(p.strip_prefix(&prefix).unwrap_or(p));
                        if let Some(d) = dest.parent() {
                            std::fs::create_dir_all(d)?;
                        }
                        bytes += data.len() as u64;
                        std::fs::write(&dest, data)?;
                        ok += 1;
                    }
                    Err(e) => {
                        failed += 1;
                        if first_err.is_empty() {
                            first_err = format!("{p}: {e:#}");
                        }
                    }
                }
            }
            println!("  extracted {ok}  ({:.1} MB)", bytes as f64 / 1e6);
            println!("  failed    {failed}");
            if !first_err.is_empty() {
                println!("  first failure: {first_err}");
            }
            println!("  -> {}", out.display());
            // Identify the second codec: compare entry method bytes for a file that
            // inflated against one that did not.
            let mut good = None;
            let mut bad = None;
            for p in &paths {
                let m = vfs.entry_method(p);
                if vfs.read_file(p).is_ok() {
                    if good.is_none() {
                        good = m.map(|x| (p.clone(), x));
                    }
                } else if bad.is_none() {
                    bad = m.map(|x| (p.clone(), x));
                }
                if good.is_some() && bad.is_some() {
                    break;
                }
            }
            if let (Some((gp, (gm, gc))), Some((bp, (bm, bc)))) = (good, bad) {
                println!("  OK   {gp}\n       method={gm:02x?} compressed={gc}");
                println!("  FAIL {bp}\n       method={bm:02x?} compressed={bc}");
            }
            if ok == 0 {
                return Ok(ExitCode::from(1));
            }
            Ok(ExitCode::SUCCESS)
        }
        Cmd::DumpEntry { path, out } => {
            let vfs = PakVfs::open_default()?;
            let (raw, dlen) = vfs.read_raw(&path)?;
            std::fs::write(&out, &raw)?;
            println!(
                "{path}\n  compressed_len {} -> decompressed_len {}",
                raw.len(),
                dlen
            );
            println!("  wrote {}", out.display());
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Source { src, out } => {
            let st = source::build(&src, &out)?;
            println!("reconstructed vanilla source (with bodies)");
            println!("  pages  {}", st.pages);
            println!("  files  {}", st.files);
            println!("  lines  {}", st.lines);
            println!("  -> {}", out.display());
            if st.files == 0 {
                eprintln!("enf: nothing reconstructed — fetch pages first");
                return Ok(ExitCode::from(1));
            }
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Lookup { symbol, index } => {
            let hits = index::lookup(&index, &symbol)?;
            if hits.is_empty() {
                println!("{symbol}: NOT FOUND");
                println!("(if a doc or a plan cites this symbol, the citation is wrong)");
                return Ok(ExitCode::from(1));
            }
            for h in &hits {
                println!("{symbol}\t{h}");
            }
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Capability {
            index_dir,
            verdicts,
        } => {
            let rules = capability::load_rules(&verdicts)?;
            let rep = capability::build(
                &index_dir.join("crf_files.tsv"),
                &index_dir.join("crf_symbols.tsv"),
                &rules,
            )?;
            let out = index_dir.join("capability_matrix.tsv");
            std::fs::write(&out, &rep.matrix_tsv)?;
            print!("{}", rep.matrix_tsv);
            println!("\n{} capabilities -> {}", rep.capabilities, out.display());
            if !rep.untriaged.is_empty() {
                eprintln!(
                    "\nUNTRIAGED ({} files with no TBD verdict):",
                    rep.untriaged.len()
                );
                for f in rep.untriaged.iter().take(40) {
                    eprintln!("  {f}");
                }
                if rep.untriaged.len() > 40 {
                    eprintln!("  ... {} more", rep.untriaged.len() - 40);
                }
                eprintln!("\nAdd a rule to {} for each.", verdicts.display());
                return Ok(ExitCode::from(1));
            }
            println!("UNTRIAGED: none — every CRF capability has a TBD verdict.");
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Dirs { index, depth, min } => {
            let hist = index::dir_histogram(&index, depth)?;
            let mut rows: Vec<_> = hist.into_iter().filter(|(_, n)| *n >= min).collect();
            // Busiest directory first. `Reverse` == the flipped `cmp` it replaces, and both sorts
            // are stable, so equal counts keep dir_histogram's order.
            rows.sort_by_key(|r| std::cmp::Reverse(r.1));
            for (dir, n) in rows {
                println!("{n:>6}  {dir}");
            }
            Ok(ExitCode::SUCCESS)
        }
    }
}
