//! T-181.3 — carve vanilla Enfusion script out of the shipped `.pak` archives.
//!
//! WHY THIS EXISTS
//! ---------------
//! `.ai/artifacts/slot_materialization_handoff.md` calls getting vanilla sources greppable
//! *"the highest-leverage tooling task available"*, and records one loading-screen hunt
//! costing 4 Workbench restarts + 3 operator round-trips that a single `grep` would have
//! answered. `api_search` returns signatures, never bodies.
//!
//! THE HANDOFF'S BLOCKING ASSUMPTION IS FALSE (measured 2026-07-25)
//! It states there is "no vanilla `.c` source on disk". There is:
//! `addons/data/data007.pak` is a FORM/PAC1 IFF container holding *uncompressed* script
//! chunks — 209 printable blobs >= 2 KB, 33.9 MB, 1,859 class declarations (914 `SCR_*`).
//! Verified by reading real bodies (`class SCR_AISpawnerGroupFaction: AISpawnerGroup`,
//! `class SCR_AutotestToolPlugin : WorkbenchPlugin`).
//!
//! WHY A CARVER AND NOT `PakVfs::read_file`
//! The pak FILE tree lists `.et`/`.conf` names but **zero `.c` names** — scripts are not
//! name-addressable, so `read_file("…/SCR_BaseGameMode.c")` can never resolve. We therefore
//! scan raw bytes for printable runs and keep the ones that parse as Enfusion. `PakVfs`
//! (`src/world/pak.rs`) remains the right tool for `.conf`/`.et`; it is not usable here.
//!
//! LICENCE — output is Bohemia Interactive's copyrighted game content. It is written to a
//! **gitignored** tree and regenerated on demand. Never commit it. Only the derived symbol
//! index (names + coordinates, no bodies) is committable.

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};

/// Minimum printable run to consider.
///
/// Measured, and the reason this is not 2048: `class SCR_AIDangerReaction` lives in a run of
/// only **1,312 bytes**, so a 2 KB floor silently dropped most real script (127 blobs kept
/// vs 397 declarations). Vanilla script ships as many small uncompressed fragments, not a few
/// large ones. The classifier — not the size floor — is what rejects non-script.
const MIN_BLOB: usize = 400;

pub struct CarveStats {
    pub paks: usize,
    pub blobs_seen: usize,
    pub blobs_kept: usize,
    pub bytes_kept: u64,
    pub duplicates: usize,
}

fn printable(b: u8) -> bool {
    (0x20..0x7f).contains(&b) || b == b'\t' || b == b'\n' || b == b'\r'
}

/// Keep a blob only if it reads as Enfusion source.
///
/// Deliberately independent of the depth-tracking scanner: a carved blob begins at an
/// arbitrary byte offset, so it routinely starts *inside* a class body and its braces never
/// balance. Classifying on scanner output alone rejected ~2/3 of real script (measured: 69
/// blobs kept where data007 alone holds 209). Here we just look for declaration keywords at
/// line starts, which is offset-independent.
fn looks_like_script(text: &str) -> bool {
    let mut decls = 0usize;
    for line in text.lines() {
        let t = line.trim_start();
        if t.starts_with("class ")
            || t.starts_with("modded class ")
            || t.starts_with("sealed class ")
            || t.starts_with("enum ")
            || t.starts_with("proto ")
        {
            decls += 1;
            if decls >= 2 {
                return true;
            }
        }
    }
    // A single declaration still counts if the blob also reads like code rather than prose.
    decls == 1 && text.contains(");") && text.contains('{')
}

/// Count declaration lines — used for the manifest, same offset-independent rule.
fn declaration_count(text: &str) -> usize {
    text.lines()
        .filter(|l| {
            let t = l.trim_start();
            t.starts_with("class ")
                || t.starts_with("modded class ")
                || t.starts_with("sealed class ")
                || t.starts_with("enum ")
        })
        .count()
}

fn sha8(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    h.finalize()
        .iter()
        .take(4)
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Carve every `*.pak` under `game_root/addons/**` into `out_dir/Carved/<pak>/`.
pub fn carve(game_root: &Path, out_dir: &Path) -> Result<CarveStats> {
    let addons = game_root.join("addons");
    let mut paks: Vec<PathBuf> = Vec::new();
    for sub in ["data", "core"] {
        let d = addons.join(sub);
        if let Ok(rd) = std::fs::read_dir(&d) {
            for e in rd.flatten() {
                let p = e.path();
                if p.extension().and_then(|s| s.to_str()) == Some("pak") {
                    paks.push(p);
                }
            }
        }
    }
    paks.sort();
    anyhow::ensure!(
        !paks.is_empty(),
        "no .pak files under {} — is this an Arma Reforger install?",
        addons.display()
    );

    let carved_root = out_dir.join("Carved");
    // Regenerate from scratch so stale fragments can never linger in the index.
    let _ = std::fs::remove_dir_all(&carved_root);
    std::fs::create_dir_all(&carved_root)?;

    let mut st = CarveStats {
        paks: paks.len(),
        blobs_seen: 0,
        blobs_kept: 0,
        bytes_kept: 0,
        duplicates: 0,
    };
    let mut manifest = String::from("carved_file\tpak\tbyte_offset\tlen\tsha8\tdeclarations\n");
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    for pak in &paks {
        let pak_name = pak
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("pak")
            .to_string();
        let dir = carved_root.join(&pak_name);
        let mut made_dir = false;
        let mut seq = 0usize;

        let f = File::open(pak).with_context(|| format!("opening {}", pak.display()))?;
        let mut rd = BufReader::with_capacity(1 << 22, f);

        // Streaming run accumulator: a blob may straddle any number of read chunks, so the
        // in-progress run is carried across them rather than using a fixed overlap window.
        let mut run: Vec<u8> = Vec::new();
        let mut run_start: u64 = 0;
        let mut pos: u64 = 0;
        let mut buf = vec![0u8; 1 << 22];

        loop {
            let n = rd.read(&mut buf)?;
            if n == 0 {
                break;
            }
            for &b in &buf[..n] {
                if printable(b) {
                    if run.is_empty() {
                        run_start = pos;
                    }
                    run.push(b);
                } else if !run.is_empty() {
                    if run.len() >= MIN_BLOB {
                        st.blobs_seen += 1;
                        flush_blob(
                            &run,
                            run_start,
                            &pak_name,
                            &dir,
                            &mut made_dir,
                            &mut seq,
                            &mut seen,
                            &mut manifest,
                            &mut st,
                        )?;
                    }
                    run.clear();
                }
                pos += 1;
            }
        }
        if run.len() >= MIN_BLOB {
            st.blobs_seen += 1;
            flush_blob(
                &run,
                run_start,
                &pak_name,
                &dir,
                &mut made_dir,
                &mut seq,
                &mut seen,
                &mut manifest,
                &mut st,
            )?;
        }
    }

    std::fs::write(out_dir.join("_MANIFEST.tsv"), manifest)?;
    std::fs::write(out_dir.join("REFERENCE-ONLY.md"), REFERENCE_ONLY_MD)?;
    Ok(st)
}

#[allow(clippy::too_many_arguments)]
fn flush_blob(
    run: &[u8],
    offset: u64,
    pak_name: &str,
    dir: &Path,
    made_dir: &mut bool,
    seq: &mut usize,
    seen: &mut std::collections::HashSet<String>,
    manifest: &mut String,
    st: &mut CarveStats,
) -> Result<()> {
    use std::fmt::Write as _;

    let text = match std::str::from_utf8(run) {
        Ok(t) => t,
        Err(_) => return Ok(()),
    };
    if !looks_like_script(text) {
        return Ok(());
    }
    let h = sha8(run);
    // The same script blob appears in several paks; keep one copy, record it once.
    if !seen.insert(h.clone()) {
        st.duplicates += 1;
        return Ok(());
    }
    if !*made_dir {
        std::fs::create_dir_all(dir)?;
        *made_dir = true;
    }
    let name = format!("{:05}_{}.c", *seq, h);
    std::fs::write(dir.join(&name), run)?;

    let decls = declaration_count(text);
    let _ = writeln!(
        manifest,
        "Carved/{}/{}\t{}\t{}\t{}\t{}\t{}",
        pak_name,
        name,
        pak_name,
        offset,
        run.len(),
        h,
        decls
    );

    *seq += 1;
    st.blobs_kept += 1;
    st.bytes_kept += run.len() as u64;
    Ok(())
}

const REFERENCE_ONLY_MD: &str = r#"# Reference only — carved vanilla Enfusion source

**GENERATED. GITIGNORED. NEVER COMMIT.**

This tree is Arma Reforger's own script source, carved out of the shipped `.pak` archives by
`enf carve` (T-181.3). It is **Bohemia Interactive's copyrighted game content** — it exists
purely so a developer or agent can `rg` the vanilla implementation instead of guessing at APIs
that `api_search` only exposes as signatures.

Regenerate with:

    cargo run -q -p tbd-tools --bin enf -- carve \
      --game "$HOME/.local/share/Steam/steamapps/common/Arma Reforger" \
      --out apps/mod/vanilla_reference

Filenames are `Carved/<pak>/<seq>_<sha8>.c` because scripts are **not name-addressable** inside
the pak FILE tree — there are no `.c` names to recover. `_MANIFEST.tsv` records the pak, byte
offset and length each blob came from. Only the derived symbol index
(`.ai/artifacts/enf-index/vanilla_*.tsv` — names and coordinates, no bodies) is committed.
"#;
