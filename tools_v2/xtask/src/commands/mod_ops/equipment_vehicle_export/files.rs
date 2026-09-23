use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};
use serde::de::{Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};
use sha2::{Digest, Sha256};

use super::FileDigest;

pub(super) fn child(root: &Path, relative: &str) -> Result<PathBuf> {
    ensure!(
        !relative.contains('\\'),
        "non-portable export path: {relative}"
    );
    let path = Path::new(relative);
    ensure!(
        !relative.is_empty() && !path.is_absolute(),
        "invalid export path: {relative}"
    );
    let mut result = root.to_path_buf();
    for component in path.components() {
        let Component::Normal(segment) = component else {
            bail!("unsafe export path: {relative}")
        };
        result.push(segment);
        if let Ok(metadata) = fs::symlink_metadata(&result) {
            ensure!(
                !metadata.file_type().is_symlink(),
                "symlink in export path: {relative}"
            );
        }
    }
    Ok(result)
}

pub(super) fn digest(bytes: &[u8]) -> FileDigest {
    FileDigest {
        bytes: bytes.len() as u64,
        sha256: format!("{:x}", Sha256::digest(bytes)),
    }
}

pub(super) fn read(root: &Path, relative: &str) -> Result<(Value, FileDigest)> {
    let path = child(root, relative)?;
    let metadata = fs::metadata(&path).with_context(|| format!("reading {relative}"))?;
    ensure!(metadata.is_file(), "not a file: {relative}");
    ensure!(
        metadata.len() <= 128 * 1024 * 1024,
        "export document exceeds 128 MiB: {relative}"
    );
    let bytes = fs::read(&path)?;
    let mut deserializer = serde_json::Deserializer::from_slice(&bytes);
    let value = UniqueValue::deserialize(&mut deserializer)
        .with_context(|| format!("invalid JSON in {relative}"))?
        .0;
    deserializer.end()?;
    Ok((value, digest(&bytes)))
}

// JSON's duplicate keys otherwise overwrite source evidence silently.
struct UniqueValue(Value);
impl<'de> Deserialize<'de> for UniqueValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct UniqueVisitor;
        impl<'de> Visitor<'de> for UniqueVisitor {
            type Value = UniqueValue;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("JSON without duplicate keys")
            }
            fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::Bool(v)))
            }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Self::Value, E> {
                Ok(UniqueValue(v.into()))
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Self::Value, E> {
                Ok(UniqueValue(v.into()))
            }
            fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Self::Value, E> {
                Number::from_f64(v)
                    .map(|n| UniqueValue(Value::Number(n)))
                    .ok_or_else(|| E::custom("non-finite source number"))
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(UniqueValue(v.into()))
            }
            fn visit_string<E: serde::de::Error>(self, v: String) -> Result<Self::Value, E> {
                Ok(UniqueValue(v.into()))
            }
            fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::Null))
            }
            fn visit_none<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                self.visit_unit()
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                while let Some(v) = seq.next_element::<UniqueValue>()? {
                    values.push(v.0);
                }
                Ok(UniqueValue(Value::Array(values)))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut values = Map::new();
                let mut keys = BTreeSet::new();
                while let Some((key, value)) = map.next_entry::<String, UniqueValue>()? {
                    if !keys.insert(key.clone()) {
                        return Err(serde::de::Error::custom(format!(
                            "duplicate JSON key: {key}"
                        )));
                    }
                    values.insert(key, value.0);
                }
                Ok(UniqueValue(Value::Object(values)))
            }
        }
        deserializer.deserialize_any(UniqueVisitor)
    }
}
