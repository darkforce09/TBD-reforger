use super::*;
use std::io::Write as _;
use std::{fs, path::PathBuf};

/// Build a PAC1 image: `files` = (path, bytes, compress?). Directory nesting follows the
/// paths (one level of dirs is enough to exercise the tree walk).
pub(crate) fn synth_pak(files: &[(&str, &[u8], bool)]) -> Vec<u8> {
    // DATA payload + per-file records. Offsets are absolute: FORM header (12) + the
    // HEAD chunk (8 + 3) + the DATA chunk header (8) precede the payload.
    const DATA_PAYLOAD_AT: u32 = 12 + 11 + 8;
    let mut data = Vec::new();
    let mut recs: Vec<(String, u32, u32, u32, bool)> = Vec::new();
    for (path, bytes, compress) in files {
        let off = DATA_PAYLOAD_AT + data.len() as u32;
        let stored: Vec<u8> = if *compress {
            let mut enc =
                flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
            enc.write_all(bytes).unwrap();
            enc.finish().unwrap()
        } else {
            bytes.to_vec()
        };
        data.extend_from_slice(&stored);
        recs.push((
            path.to_string(),
            off,
            stored.len() as u32,
            bytes.len() as u32,
            *compress,
        ));
    }
    // FILE tree: root dir → { top-level files, one dir per distinct first component }.
    let mut tree = Vec::new();
    let mut dirs: Vec<(String, Vec<usize>)> = Vec::new();
    let mut top: Vec<usize> = Vec::new();
    for (i, r) in recs.iter().enumerate() {
        match r.0.split_once('/') {
            Some((d, _)) => match dirs.iter_mut().find(|x| x.0 == d) {
                Some(x) => x.1.push(i),
                None => dirs.push((d.to_string(), vec![i])),
            },
            None => top.push(i),
        }
    }
    let file_rec = |t: &mut Vec<u8>, name: &str, r: &(String, u32, u32, u32, bool)| {
        t.push(1);
        t.push(name.len() as u8);
        t.extend_from_slice(name.as_bytes());
        t.extend_from_slice(&r.1.to_le_bytes());
        t.extend_from_slice(&r.2.to_le_bytes());
        t.extend_from_slice(&r.3.to_le_bytes());
        t.extend_from_slice(&[0u8; 6]);
        t.push(u8::from(r.4));
        t.extend_from_slice(&[0u8; 5]);
    };
    tree.push(0);
    tree.push(0); // root: empty name
    tree.extend_from_slice(&((top.len() + dirs.len()) as u32).to_le_bytes());
    for i in &top {
        file_rec(&mut tree, &recs[*i].0, &recs[*i]);
    }
    for (d, members) in &dirs {
        tree.push(0);
        tree.push(d.len() as u8);
        tree.extend_from_slice(d.as_bytes());
        tree.extend_from_slice(&(members.len() as u32).to_le_bytes());
        for i in members {
            let leaf = recs[*i].0.split_once('/').unwrap().1;
            file_rec(&mut tree, leaf, &recs[*i]);
        }
    }
    let mut img = Vec::new();
    img.extend_from_slice(b"FORM");
    img.extend_from_slice(&0u32.to_be_bytes()); // patched below
    img.extend_from_slice(b"PAC1");
    let head = [7u8, 7, 7]; // an opaque HEAD chunk the reader must skip
    img.extend_from_slice(b"HEAD");
    img.extend_from_slice(&(head.len() as u32).to_be_bytes());
    img.extend_from_slice(&head);
    img.extend_from_slice(b"DATA");
    img.extend_from_slice(&(data.len() as u32).to_be_bytes());
    img.extend_from_slice(&data);
    img.extend_from_slice(b"FILE");
    img.extend_from_slice(&(tree.len() as u32).to_be_bytes());
    img.extend_from_slice(&tree);
    let total = (img.len() - 8) as u32;
    img[4..8].copy_from_slice(&total.to_be_bytes());
    img
}

#[test]
fn synthetic_pak_lists_and_reads_stored_and_zlib_files() {
    let big: Vec<u8> = (0..5000u32).map(|i| (i % 7) as u8).collect();
    let img = synth_pak(&[
        ("readme.txt", b"hello", false),
        ("Prefabs/Door.et", b"GenericEntity {\n}\n", true),
        ("Prefabs/Table.et", &big, true),
    ]);
    let entries = parse_pak_bytes(&img).expect("parse");
    let paths: Vec<&str> = entries.iter().map(|e| e.path.as_str()).collect();
    assert_eq!(paths, ["readme.txt", "Prefabs/Door.et", "Prefabs/Table.et"]);
    assert!(!entries[0].compressed && entries[1].compressed);
    assert_eq!(entries[2].decompressed_len, 5000);
    assert!(entries[2].compressed_len < 5000, "zlib shrank the pattern");
    // Offsets are absolute: the DATA payload starts after FORM(12) + HEAD chunk(8+3).
    assert_eq!(entries[0].offset, 12 + 11 + 8);
    // A DATA-relative offset (below the payload start) is rejected by name.
    let mut bad = img.clone();
    // FILE chunk is last; find its entry record by searching from the end.
    let needle = (12u32 + 11 + 8).to_le_bytes();
    let at = bad
        .windows(4)
        .rposition(|w| w == needle)
        .expect("first entry offset in the FILE tree");
    bad[at..at + 4].copy_from_slice(&0u32.to_le_bytes());
    let err = parse_pak_bytes(&bad).unwrap_err().to_string();
    assert!(err.contains("outside the DATA chunk"), "{err}");
    let dir = std::env::temp_dir().join(format!("tbd-pak-test-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let p = dir.join("b_second.pak");
    fs::write(&p, &img).unwrap();
    let idx = PakIndex::open(&p).expect("open");
    assert_eq!(idx.read(&idx.entries[0]).unwrap(), b"hello");
    assert_eq!(idx.read(&idx.entries[1]).unwrap(), b"GenericEntity {\n}\n");
    assert_eq!(idx.read(&idx.entries[2]).unwrap(), big);
    // A second pak (sorted first) shadows the duplicate path.
    let first = synth_pak(&[("Prefabs/Door.et", b"FIRST WINS", false)]);
    fs::write(dir.join("a_first.pak"), &first).unwrap();
    let set = PakSet::from_dir(&dir).expect("set");
    assert_eq!(set.pak_count(), 2);
    assert_eq!(set.file_count(), 3);
    assert_eq!(set.read("prefabs/door.et").unwrap(), b"FIRST WINS");
    assert_eq!(set.read("Prefabs/Table.et").unwrap(), big);
    assert!(set.exists("README.TXT") && !set.exists("nope.et"));
    assert_eq!(
        set.paths_under("Prefabs/"),
        ["Prefabs/Door.et", "Prefabs/Table.et"]
    );
    assert!(set.read("missing.et").is_err());
    // Loose-directory source with case-insensitive lookup, layered under the paks.
    fs::create_dir_all(dir.join("loose/Assets")).unwrap();
    fs::write(dir.join("loose/Assets/Thing.xob"), b"xob").unwrap();
    let loose = DirSource {
        root: dir.join("loose"),
    };
    assert_eq!(loose.read("assets/thing.XOB").unwrap(), b"xob");
    let layered = LayeredSource {
        layers: vec![Box::new(set), Box::new(loose)],
    };
    assert_eq!(layered.read("Prefabs/Door.et").unwrap(), b"FIRST WINS");
    assert_eq!(layered.read("Assets/Thing.xob").unwrap(), b"xob");
    assert!(layered.read("Assets/Other.xob").is_err());
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn malformed_pak_images_are_rejected() {
    assert!(parse_pak_bytes(b"FORM\0\0\0\0XOB9").is_err());
    let img = synth_pak(&[("a.txt", b"x", false)]);
    // Drop the FILE chunk.
    let cut = img[..img.len() - 40].to_vec();
    assert!(parse_pak_bytes(&cut).is_err());
    // Wrong inflated length is caught.
    let e = PakEntry {
        path: "z".into(),
        offset: 0,
        compressed_len: 0,
        decompressed_len: 99,
        compressed: true,
        method: [0; 6],
    };
    let mut enc = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
    enc.write_all(b"short").unwrap();
    assert!(inflate_entry(&enc.finish().unwrap(), &e).is_err());
}

/// Census of the real install: entries per pak (the MCP log reports 7319 for data002
/// and 5372 for data004 — a lower count here means a truncated tree walk) and every
/// holder of the farmhouse XOB with the first byte that differs from the extract.
#[test]
#[ignore = "needs ~/.cache/enfusion-mcp-root/addons"]
fn real_pak_census() {
    let Some(dir) = PakSet::default_dir().filter(|d| d.is_dir()) else {
        return;
    };
    let home = std::env::var("HOME").unwrap();
    let want = fs::read(PathBuf::from(home).join(
        "ReforgerExtract/unpacked/Assets/Structures/Houses/Farm/FarmHouse_E_1L01/FarmHouse_E_1L01.xob",
    ))
    .unwrap_or_default();
    let mut paths: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "pak"))
        .collect();
    paths.sort();
    for p in paths {
        let idx = PakIndex::open(&p).expect("open pak");
        let farm: Vec<&PakEntry> = idx
            .entries
            .iter()
            .filter(|e| e.path.to_ascii_lowercase().contains("farmhouse_e_1l01.xob"))
            .collect();
        eprintln!(
            "{}: {} entries; farmhouse xob entries: {:?}",
            p.file_name().unwrap().to_string_lossy(),
            idx.entries.len(),
            farm.iter()
                .map(|e| (
                    e.path.as_str(),
                    e.compressed,
                    e.compressed_len,
                    e.decompressed_len
                ))
                .collect::<Vec<_>>()
        );
        for e in farm {
            let got = idx.read(e).expect("read");
            let first_diff = got.iter().zip(want.iter()).position(|(a, b)| a != b);
            eprintln!(
                "  {} → {} bytes, first diff vs extract at {:?} (extract {} bytes); pak {} bytes, entry.offset {}",
                e.path,
                got.len(),
                first_diff,
                want.len(),
                fs::metadata(&p).map(|m| m.len()).unwrap_or(0),
                e.offset
            );
            eprintln!("  got[..16]  = {:02x?}", &got[..16.min(got.len())]);
            eprintln!("  want[..16] = {:02x?}", &want[..16.min(want.len())]);
            // Where does the extract's first 16 bytes actually sit in this pak?
            let pak_bytes = fs::read(&p).unwrap();
            if want.len() >= 16 {
                let needle = &want[..16];
                let hits: Vec<usize> = pak_bytes
                    .windows(16)
                    .enumerate()
                    .filter(|(_, w)| *w == needle)
                    .map(|(i, _)| i)
                    .take(4)
                    .collect();
                eprintln!("  extract head found in pak at {hits:?}");
            }
        }
    }
}

/// The real install: the farmhouse XOB read through the pak set must byte-equal the
/// operator's hand-extracted copy. Needs the MCP symlink farm + the extract — skipped
/// (not failed) elsewhere.
#[test]
#[ignore = "needs ~/.cache/enfusion-mcp-root/addons + ~/ReforgerExtract"]
fn real_pak_farmhouse_xob_matches_extract() {
    let Some(dir) = PakSet::default_dir().filter(|d| d.is_dir()) else {
        return;
    };
    let home = std::env::var("HOME").unwrap();
    let extract = PathBuf::from(home).join(
        "ReforgerExtract/unpacked/Assets/Structures/Houses/Farm/FarmHouse_E_1L01/FarmHouse_E_1L01.xob",
    );
    let Ok(want) = fs::read(&extract) else {
        return;
    };
    let set = PakSet::from_dir(&dir).expect("pak set");
    let rel = "Assets/Structures/Houses/Farm/FarmHouse_E_1L01/FarmHouse_E_1L01.xob";
    // Diagnostic: which paks carry the path, and which copy equals the extract.
    let mut holders = Vec::new();
    for pak in &set.paks {
        if let Some(e) = pak
            .entries
            .iter()
            .find(|e| normalize_path(&e.path) == normalize_path(rel))
        {
            let bytes = pak.read(e).expect("read");
            let name = pak.path.file_name().unwrap().to_string_lossy().into_owned();
            holders.push(format!(
                "{name}: {} bytes, {}",
                bytes.len(),
                if bytes == want {
                    "== extract"
                } else {
                    "differs"
                }
            ));
        }
    }
    eprintln!("farmhouse xob holders: {holders:?}");
    let got = set.read(rel).expect("farmhouse xob in a pak");
    assert_eq!(got.len(), want.len());
    assert!(
        got == want,
        "pak read differs from the extracted file ({holders:?})"
    );
}
