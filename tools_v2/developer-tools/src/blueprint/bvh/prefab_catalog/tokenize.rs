use super::*;

pub(super) fn tokenize(src: &str) -> Vec<Tok> {
    let mut out = Vec::new();
    let b = src.as_bytes();
    let mut i = 0usize;
    while i < b.len() {
        let c = b[i];
        if c.is_ascii_whitespace() {
            i += 1;
        } else if c == b'/' && b.get(i + 1) == Some(&b'/') {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
        } else if c == b'{' {
            out.push(Tok::LBrace);
            i += 1;
        } else if c == b'}' {
            out.push(Tok::RBrace);
            i += 1;
        } else if c == b':' {
            out.push(Tok::Colon);
            i += 1;
        } else if c == b'"' {
            let start = i + 1;
            let mut j = start;
            while j < b.len() && b[j] != b'"' {
                j += 1;
            }
            out.push(Tok::Str(src[start..j].to_string()));
            i = j + 1;
        } else {
            let start = i;
            while i < b.len()
                && !b[i].is_ascii_whitespace()
                && b[i] != b'{'
                && b[i] != b'}'
                && b[i] != b'"'
            {
                i += 1;
            }
            let word = &src[start..i];
            if word == "$grp" {
                out.push(Tok::Grp);
            } else if word.starts_with(|ch: char| ch.is_ascii_digit())
                || (word.starts_with('-') && word.len() > 1)
                || word.starts_with('.')
            {
                out.push(Tok::Num(word.to_string()));
            } else {
                out.push(Tok::Ident(word.to_string()));
            }
        }
    }
    out
}

/// `{GUID}path` → `path`; anything else unchanged.
pub fn strip_guid(s: &str) -> &str {
    if s.starts_with('{')
        && let Some(end) = s.find('}')
    {
        return &s[end + 1..];
    }
    s
}

/// Parse a whole `.et` file into its root blocks (one per file in practice).
pub fn parse_et(src: &str) -> Result<Vec<Block>> {
    let mut p = Parser {
        toks: tokenize(src),
        pos: 0,
    };
    let mut root = Block::default();
    p.body(&mut root, 0)?;
    if p.pos < p.toks.len() {
        bail!("unbalanced braces in prefab text");
    }
    let mut roots = root.blocks;
    // Props at file level are meaningless; a value list too. Anonymous top-level blocks
    // are not a thing either — keep what parsed and let the resolver complain.
    roots.extend(root.anon);
    Ok(roots)
}

pub(super) fn is_placeholder_mesh(p: &str) -> bool {
    p.ends_with("Common/Models/Default.xob")
}

pub(super) fn read_child(head: &Block, inst: &Block) -> ChildRef {
    let pivot = inst
        .block("components")
        .and_then(|c| c.block("Hierarchy"))
        .and_then(|h| h.prop("PivotID"))
        .map(ToString::to_string);
    ChildRef {
        class: head.name.clone(),
        prefab: head.base.clone().unwrap_or_default(),
        id: inst.prop("ID").map(ToString::to_string),
        pivot_id: pivot,
        coords: inst.prop_vec3("coords").unwrap_or([0.0; 3]),
        angles_deg: inst.prop_vec3("angles").unwrap_or([0.0; 3]),
        scale: inst.prop_f64("scale").unwrap_or(1.0),
    }
}

pub(super) fn enabled(b: &Block) -> bool {
    b.prop("Enabled") != Some("0")
}

pub(super) fn own_facts(root: &Block) -> OwnFacts {
    let mut f = OwnFacts {
        class: root.name.clone(),
        base: root.base.clone(),
        ..OwnFacts::default()
    };
    if let Some(components) = root.block("components") {
        if let Some(m) = components.block("MeshObject") {
            f.mesh = m.prop("Object").map(|s| strip_guid(s).to_string());
        }
        if let Some(d) = components.block("DoorComponent") {
            f.door = Some((
                d.prop_f64("AngleRange"),
                d.prop_f64("ClosedAngle"),
                d.prop_f64("InitialAngle"),
                enabled(d),
            ));
        }
        if let Some(s) = components.block("SlidingDoorComponent") {
            f.sliding = Some((
                s.prop_f64("OpenedDistance"),
                s.prop_f64("InitialDistance"),
                enabled(s),
            ));
        }
        if let Some(h) = components.block("Hierarchy") {
            f.hierarchy_pivot = h.prop("PivotID").map(ToString::to_string);
        }
    }
    if let Some(sb) = root.block("SlotBoneMappings") {
        for m in sb.blocks_named("SlotBoneMappingObject") {
            if let (Some(prefix), Some(prefab)) = (m.prop("BonePrefix"), m.prop("Prefab")) {
                f.slot_bones
                    .push((prefix.to_string(), strip_guid(prefab).to_string()));
            }
        }
    }
    for list in &root.anon {
        for head in &list.blocks {
            if head.base.is_none() {
                continue;
            }
            if head.grp || (!head.anon.is_empty() && head.prop("ID").is_none()) {
                for inst in &head.anon {
                    f.children.push(read_child(head, inst));
                }
            } else {
                f.children.push(read_child(head, head));
            }
        }
    }
    f
}
