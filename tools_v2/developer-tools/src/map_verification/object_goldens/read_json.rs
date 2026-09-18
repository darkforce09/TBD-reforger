use super::*;

pub(super) fn read_json(p: &PathBuf) -> Result<Value> {
    let raw = fs::read_to_string(p).with_context(|| format!("read {}", p.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("parse {}", p.display()))
}

pub(super) fn is_chunk_tuple(row: &Value) -> bool {
    row.as_array().and_then(|a| a.first()).map(Value::is_number) == Some(true)
}

pub(super) fn inst_id(row: &Value) -> String {
    match row {
        Value::Array(a) => {
            if is_chunk_tuple(row) {
                format!(
                    "[{}]",
                    a.iter()
                        .map(|v| v.to_string())
                        .collect::<Vec<_>>()
                        .join(",")
                )
            } else {
                a.first()
                    .map(|v| v.as_str().unwrap_or("?").to_string())
                    .unwrap_or_default()
            }
        }
        other => other["id"].as_str().unwrap_or("?").to_string(),
    }
}

pub(super) fn inst_prefab_id(row: &Value) -> Option<f64> {
    match row {
        Value::Array(a) => {
            if is_chunk_tuple(row) {
                a.first().and_then(Value::as_f64)
            } else {
                a.get(1).and_then(Value::as_f64)
            }
        }
        other => other["prefabId"].as_f64(),
    }
}

/// One `WorldChunk` f32 column against another, by BITS: `==` calls `-0.0` equal to `+0.0`, and
/// `-0.0` is the exact value `binary_emit.rs` records the JSON round trip as not preserving.
pub(super) fn f32_col(what: &str, bin: &[f32], json: &[f32], errs: &mut Vec<String>) {
    let (nb, nj) = (bin.len(), json.len());
    if nb != nj {
        errs.push(format!("{what}: bin {nb} values, json {nj}"));
        return;
    }
    for (i, (b, j)) in bin.iter().zip(json).enumerate() {
        if b.to_bits() != j.to_bits() {
            errs.push(format!("{what}[{i}]: bin {b} != json {j}"));
        }
    }
}

/// Every way `map-object-chunk-sample.bin` can disagree with `map-object-chunk-sample.json`.
///
/// **A** re-emits the golden through [`write_chunk_bin`] and demands byte identity — the golden is
/// emitter output, never hand-written, and this is the check one flipped byte fails. Three oracles
/// byte identity alone cannot give sit on top: **B** the shipped binary loader must decode to the
/// columns the shipped JSON loader reads out of the golden JSON, and neither of those is the
/// emitter; **C/D** a fixed-offset `from_le_bytes` decode of header AND rows, touching neither
/// `bytemuck` nor `TbdcHeader`/`ObjectInstancePod`, must agree — a field REORDER moves the emitter
/// and every cast-based reader together, so only raw offsets can see it; **E** every position must
/// lie inside the chunk the header itself declares, because a `.bin` framed for the wrong tile is
/// byte-perfect, parses cleanly, and files another tile's objects under this id forever.
pub(super) fn chunk_bin_errors(
    sample: &Value,
    prefabs_sample: &Value,
    committed: &[u8],
) -> Vec<String> {
    let mut errs: Vec<String> = Vec::new();
    let cx = sample["cx"].as_i64().unwrap_or(0);
    let cy = sample["cy"].as_i64().unwrap_or(0);
    let id = format!("{cx}_{cy}");
    let size_m = sample["chunkSizeM"].as_f64().unwrap_or(512.0);
    let raw = &sample["chunk"];
    let rows = raw["instances"].as_array().cloned().unwrap_or_default();
    let doc = json!({ "prefabs": prefabs_sample.clone() });
    let (by_id, _) = build_prefab_maps(narrow_prefab_rows(&doc));
    let Some(oracle) = parse_chunk(&id, raw, &by_id) else {
        return vec![format!("parse_chunk({id}) read no instances")];
    };
    let (n, len) = (oracle.count as usize, committed.len());

    // A. BYTE IDENTITY against a fresh emit from the golden JSON.
    let tmp = std::env::temp_dir().join(format!("t935-12-golden-{}.bin", std::process::id()));
    let pods = pods_from_rows(&rows, &class_code_table(&doc));
    let reemit = (|| -> Result<Vec<u8>> {
        write_chunk_bin(&tmp, cx, cy, &pods)?;
        fs::read(&tmp).map_err(anyhow::Error::from)
    })();
    let _ = fs::remove_file(&tmp);
    match reemit {
        Err(e) => errs.push(format!("re-emit failed: {e:#}")),
        Ok(want) => {
            let w = want.len();
            if w != len {
                errs.push(format!(".bin is {len} B, emitter writes {w} B"));
            }
            if let Some(o) = want.iter().zip(committed).position(|(a, b)| a != b) {
                let (c, w) = (committed[o], want[o]);
                errs.push(format!("byte {o}: {c:#04x} != emitter {w:#04x}"));
            }
        }
    }
    if len < HEADER_BYTES {
        errs.push(format!("{len} B, under the 32 B TBDC header"));
        return errs;
    }

    // C/D. Header, at fixed offsets, and its MEANING as well as its layout: rows that parse at the
    // right stride can still be framed for the wrong tile or by an unread container version.
    let u16at = |o: usize| u16::from_le_bytes([committed[o], committed[o + 1]]);
    let count = u32::from_le_bytes(committed[8..12].try_into().unwrap_or_default()) as usize;
    let (magic, rsv, ver, flags) = (&committed[..4], &committed[16..32], u16at(4), u16at(6));
    let (hcx, hcy) = (u16at(12) as i16, u16at(14) as i16);
    if magic != b"TBDC" {
        errs.push(format!("magic {magic:?}, want TBDC"));
    }
    if ver != CONTAINER_VERSION {
        errs.push(format!("containerVersion {ver} != {CONTAINER_VERSION}"));
    }
    if flags != 0 || rsv.iter().any(|b| *b != 0) {
        errs.push(format!("flags/reserved not zero: {flags}/{rsv:?}"));
    }
    if i64::from(hcx) != cx || i64::from(hcy) != cy {
        errs.push(format!("header frames {hcx}_{hcy}, JSON says {id}"));
    }
    if count != n {
        errs.push(format!("header count {count} != {n} JSON rows"));
    }
    let want_len = HEADER_BYTES + POD_BYTES * count;
    if len != want_len {
        errs.push(format!("file is {len} B, want {want_len}"));
    }
    let payload = &committed[HEADER_BYTES..];
    let names = ["x", "y", "z", "yaw", "pitch", "roll", "scale"];
    for i in 0..n.min(payload.len() / POD_BYTES) {
        let b = &payload[i * POD_BYTES..];
        let f = |o: usize| f32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]);
        let p = &oracle.positions;
        let got = [f(0), f(4), f(8), f(12), f(16), f(20), f(24)];
        let head = [p[2 * i], p[2 * i + 1], oracle.z[i], oracle.rotations[i]];
        let tail = [oracle.pitch[i], oracle.roll[i], oracle.scale[i]];
        for (k, (g, w)) in got.iter().zip(head.iter().chain(&tail)).enumerate() {
            let name = names[k];
            if g.to_bits() != w.to_bits() {
                errs.push(format!("row {i} {name}: bin {g} != json {w}"));
            }
        }
        let (pid, cls, pad) = (u16::from_le_bytes([b[28], b[29]]), b[30], b[31]);
        let (wpid, wcls) = (oracle.prefab_idx[i], oracle.cls_codes[i]);
        if pid != wpid {
            errs.push(format!("row {i} prefabId {pid} != json {wpid}"));
        }
        if cls != wcls {
            errs.push(format!("row {i} classCode {cls} != json {wcls}"));
        }
        if pad != 0 {
            errs.push(format!("row {i}: _pad is {pad}, must be 0"));
        }
    }

    // B. Shipped binary loader vs shipped JSON loader, column for column. E rides on it, because
    // `dec.cx`/`dec.cy` come from the HEADER — not from the file name, not from the golden JSON.
    match parse_chunk_bin_for(&id, committed) {
        Err(e) => errs.push(format!("parse_chunk_bin_for({id}): {e}")),
        Ok(dec) => {
            let (bid, bn, jn) = (&dec.id, dec.count, oracle.count);
            if bid != &oracle.id || bn != jn {
                errs.push(format!("bin {bid}/{bn}, json {id}/{jn}"));
            }
            f32_col("positions", &dec.positions, &oracle.positions, &mut errs);
            f32_col("z", &dec.z, &oracle.z, &mut errs);
            f32_col("rotations", &dec.rotations, &oracle.rotations, &mut errs);
            f32_col("pitch", &dec.pitch, &oracle.pitch, &mut errs);
            f32_col("roll", &dec.roll, &oracle.roll, &mut errs);
            f32_col("scale", &dec.scale, &oracle.scale, &mut errs);
            for (what, same) in [
                ("prefab_idx", dec.prefab_idx == oracle.prefab_idx),
                ("cls_codes", dec.cls_codes == oracle.cls_codes),
                ("rows_by_class", dec.rows_by_class == oracle.rows_by_class),
            ] {
                if !same {
                    errs.push(format!("{what}: bin decode != json decode"));
                }
            }
            let (hx, hy) = (dec.cx, dec.cy);
            for i in 0..dec.count as usize {
                let x = f64::from(dec.positions[2 * i]);
                let y = f64::from(dec.positions[2 * i + 1]);
                let (lo_x, lo_y) = (hx * size_m, hy * size_m);
                if x < lo_x || x >= lo_x + size_m || y < lo_y || y >= lo_y + size_m {
                    errs.push(format!("row {i} ({x},{y}) outside header {hx}_{hy}"));
                }
            }
        }
    }
    errs.truncate(8);
    errs
}
