use super::*;

pub fn gunzip_json(p: &Path) -> Result<Value> {
    let raw = gunzip(&std::fs::read(p).with_context(|| p.display().to_string())?)?;
    Ok(serde_json::from_slice(&raw)?)
}
