//! Role: assets.
//! Position: `doc/operations` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

/// Domain representation of place payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlacePayload {
    /// Asset id.
    pub asset_id: String,

    /// Role.
    pub role: String,
}

/// Derive a schema `#/$defs/alias` for a placed object from its ResourceName + display name.
#[must_use]
pub fn derive_object_alias(resource_name: &str, display_name: &str) -> String {
    const KNOWN: &[(&str, &str)] = &[(
        "{E1D01D77D7F47EF3}PrefabsEditable/Auto/Compositions/Misc/SubCompositions/E_Sandbag_Barricade_US_04.et",
        "comp:checkpoint_small",
    )];
    for (guid, alias) in KNOWN {
        if resource_name == *guid {
            return (*alias).to_string();
        }
    }
    let prefix = if resource_name.contains("Composition") || resource_name.contains("Compositions")
    {
        "comp"
    } else {
        "prop"
    };
    let slug = object_alias_slug(display_name);
    format!("{prefix}:{slug}")
}

/// Object alias slug using the supplied domain data.
pub fn object_alias_slug(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut prev_repl = false;
    for c in raw.to_lowercase().chars() {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            out.push(c);
            prev_repl = false;
        } else if !prev_repl {
            out.push('_');
            prev_repl = true;
        }
    }
    let trimmed = out.trim_matches('_');
    if trimmed.is_empty() {
        "object".to_string()
    } else {
        trimmed.to_string()
    }
}

/// The CLASSNAME TAIL of an Enfusion `resource_name`: the last path segment with its extension dropped — `{26A9756790131354}Prefabs/…/Character_US_Rifleman.et` → `Character_US_Rifleman`.
#[must_use]
pub fn classname_tail(id: &str) -> &str {
    let seg = id.rsplit('/').next().unwrap_or(id);
    match seg.rfind('.') {
        Some(i) if i > 0 => &seg[..i],
        _ => seg,
    }
}
