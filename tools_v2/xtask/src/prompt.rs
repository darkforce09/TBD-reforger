use anyhow::Result;
use regex::Regex;
use std::sync::LazyLock;

static SECTION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?ms)^##\s+Claude Code prompt[^\n]*\n+(?:[^\n]*\n)*?```(?:\w*\n)?(.*?)```")
        .unwrap()
});

pub fn extract_prompt(markdown: &str) -> Result<String> {
    if let Some(c) = SECTION.captures(markdown) {
        return Ok(c[1].trim().to_string());
    }
    let idx = markdown
        .find("## Claude Code prompt")
        .ok_or_else(|| anyhow::anyhow!("No '## Claude Code prompt' section found"))?;
    let rest = &markdown[idx..];
    let block = Regex::new(r"(?s)```(?:\w*\n)?(.*?)```")
        .unwrap()
        .captures(rest)
        .ok_or_else(|| anyhow::anyhow!("No fenced code block in Claude Code prompt section"))?;
    Ok(block[1].trim().to_string())
}
