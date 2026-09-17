//! NDJSON voxel-dump reader. Fails loudly: the dumper is the one piece of math still running
//! in-engine, so every convention it promises (normalized coordinates, march order, end line)
//! is asserted here rather than trusted — a convention bug poisons every downstream heuristic
//! and must die on the first real dump, not surface as a subtly wrong wall.

use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde_json::Value;

use super::types::{DumpMeta, FurnRec, ScanMap, VoxelDump};

pub fn parse_dump(path: &Path) -> Result<VoxelDump> {
    let file = File::open(path).with_context(|| format!("open {}", path.display()))?;
    let reader: Box<dyn Read> = if path.extension().and_then(|e| e.to_str()) == Some("gz") {
        Box::new(flate2::read::GzDecoder::new(file))
    } else {
        Box::new(file)
    };
    parse_reader(BufReader::new(reader), &path.display().to_string())
}

pub fn parse_reader<R: BufRead>(reader: R, label: &str) -> Result<VoxelDump> {
    let mut dump = VoxelDump::default();
    let mut data_lines = 0usize;
    let mut end_seen: Option<(usize, i64)> = None;

    for (idx, line) in reader.lines().enumerate() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let lineno = idx + 1;
        if end_seen.is_some() {
            bail!("{label}:{lineno}: data after the end line");
        }
        let value: Value =
            serde_json::from_str(trimmed).with_context(|| format!("{label}:{lineno}: bad JSON"))?;

        if lineno == 1 {
            let meta: DumpMeta = serde_json::from_value(value)
                .with_context(|| format!("{label}:1: bad meta object"))?;
            if meta.v != super::types::DUMP_VERSION {
                bail!(
                    "{label}:1: dump version '{}' (want {})",
                    meta.v,
                    super::types::DUMP_VERSION
                );
            }
            dump.meta = Some(meta);
            continue;
        }
        if dump.meta.is_none() {
            bail!("{label}: first line is not the meta object");
        }

        match &value {
            Value::Array(_) => {
                parse_scanline(&value, &mut dump, label, lineno)?;
                data_lines += 1;
            }
            Value::Object(obj) if obj.contains_key("furn") => {
                let rec: FurnRec = serde_json::from_value(obj["furn"].clone())
                    .with_context(|| format!("{label}:{lineno}: bad furn record"))?;
                dump.furniture.push(rec);
                data_lines += 1;
            }
            Value::Object(obj) if obj.contains_key("end") => {
                let lines = obj["end"]["lines"].as_u64().unwrap_or(0) as usize;
                let ms = obj["end"]["ms"].as_i64().unwrap_or(-1);
                end_seen = Some((lines, ms));
            }
            _ => bail!("{label}:{lineno}: unrecognized line shape"),
        }
    }

    let Some((declared, _ms)) = end_seen else {
        bail!("{label}: no end line - the dump is truncated");
    };
    if declared != data_lines {
        bail!("{label}: end line declares {declared} lines, parsed {data_lines}");
    }
    if dump.meta.is_none() {
        bail!("{label}: empty dump");
    }
    Ok(dump)
}

fn parse_scanline(value: &Value, dump: &mut VoxelDump, label: &str, lineno: usize) -> Result<()> {
    let arr = value.as_array().expect("caller checked");
    if arr.len() < 4 {
        bail!("{label}:{lineno}: scanline needs [code, j, k, entries]");
    }
    let code = arr[0].as_str().unwrap_or("");
    let j = arr[1].as_u64().context("j")? as usize;
    let k = arr[2].as_u64().context("k")? as usize;
    let entries: Vec<f64> = arr[3]
        .as_array()
        .with_context(|| format!("{label}:{lineno}: entries not an array"))?
        .iter()
        .map(|v| v.as_f64().unwrap_or(f64::NAN))
        .collect();
    if entries.iter().any(|e| !e.is_finite()) {
        bail!("{label}:{lineno}: non-finite entry");
    }
    if entries.is_empty() {
        bail!("{label}:{lineno}: empty scanline emitted (dumper contract: omit empties)");
    }
    if arr.len() > 4 {
        dump.truncated += 1;
    }

    // March-order assertion: "+" runs ascend, "-" runs descend after normalization. This is the
    // tripwire for the dumper's span-flip math.
    let ascending = code.ends_with('+');
    for w in entries.windows(2) {
        let ok = if ascending {
            w[1] > w[0] - 1e-6
        } else {
            w[1] < w[0] + 1e-6
        };
        if !ok {
            bail!(
                "{label}:{lineno}: '{code}' entries out of march order ({} then {})",
                w[0],
                w[1]
            );
        }
    }

    let map: &mut ScanMap = match code {
        "x+" => &mut dump.x_pos,
        "x-" => &mut dump.x_neg,
        "y-" => &mut dump.y_down,
        "y+" => &mut dump.y_up,
        "z+" => &mut dump.z_pos,
        "z-" => &mut dump.z_neg,
        other => bail!("{label}:{lineno}: unknown scanline code '{other}'"),
    };
    if map.insert((j, k), entries).is_some() {
        bail!("{label}:{lineno}: duplicate scanline {code} ({j},{k})");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    pub(crate) const META: &str = r#"{"v":"tbd-voxel-dump/1","slug":"t","resource":"r","origin":[0,0,0],"cell":0.1,"dims":[10,10,10],"span":[1,1,1],"bboxMin":[0,0,0],"bboxMax":[1,1,1],"rootYawDeg":0,"excluded":{"doors":0,"glass":0,"furniture":0},"tick":1}"#;

    fn parse_str(s: &str) -> Result<VoxelDump> {
        parse_reader(Cursor::new(s.to_string()), "test")
    }

    #[test]
    fn round_trip_minimal() {
        let s = format!(
            "{META}\n[\"x+\",1,2,[0.25,0.61]]\n[\"x-\",1,2,[0.66,0.30]]\n{{\"furn\":{{\"name\":\"w\",\"res\":\"r\",\"pos\":[1,0,1],\"worldYawDeg\":90,\"size\":[1,2,1],\"boundsMinY\":0}}}}\n{{\"end\":{{\"lines\":3,\"ms\":5}}}}\n"
        );
        let d = parse_str(&s).unwrap();
        assert_eq!(d.x_pos[&(1, 2)], vec![0.25, 0.61]);
        assert_eq!(d.x_neg[&(1, 2)], vec![0.66, 0.30]);
        assert_eq!(d.furniture.len(), 1);
        assert_eq!(d.truncated, 0);
    }

    #[test]
    fn missing_end_line_is_truncation() {
        let s = format!("{META}\n[\"x+\",1,2,[0.25]]\n");
        assert!(
            parse_str(&s)
                .unwrap_err()
                .to_string()
                .contains("no end line")
        );
    }

    #[test]
    fn wrong_line_count_fails() {
        let s = format!("{META}\n[\"x+\",1,2,[0.25]]\n{{\"end\":{{\"lines\":7,\"ms\":5}}}}\n");
        assert!(
            parse_str(&s)
                .unwrap_err()
                .to_string()
                .contains("declares 7")
        );
    }

    #[test]
    fn march_order_violation_fails() {
        let s = format!("{META}\n[\"x+\",1,2,[0.61,0.25]]\n{{\"end\":{{\"lines\":1,\"ms\":5}}}}\n");
        assert!(
            parse_str(&s)
                .unwrap_err()
                .to_string()
                .contains("march order")
        );
    }

    #[test]
    fn descending_required_for_minus_runs() {
        let s = format!("{META}\n[\"y-\",1,2,[0.25,0.61]]\n{{\"end\":{{\"lines\":1,\"ms\":5}}}}\n");
        assert!(
            parse_str(&s)
                .unwrap_err()
                .to_string()
                .contains("march order")
        );
    }
}
