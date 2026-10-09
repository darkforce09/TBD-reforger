//! Tests for [`super`] — every public re-export form names its crates, and nothing else does.

use super::*;

#[test]
fn public_reexports_read_only_public_reexport_statements() {
    let first = |text: &str| {
        reexported_crates(text)
            .first()
            .map(|(_, name)| name.clone())
    };
    assert_eq!(
        first("pub use mission_model::Mission;").as_deref(),
        Some("mission_model")
    );
    assert_eq!(
        first("    pub use ::mission_model::Mission;").as_deref(),
        Some("mission_model")
    );
    assert_eq!(first("pub use crate::data::Mission;"), None);
    assert_eq!(first("pub(crate) use mission_model::Mission;"), None);
    assert_eq!(first("use mission_model::Mission;"), None);
}

#[test]
fn public_reexports_read_every_reexport_form_of_a_crate() {
    let one = |name: &str| vec![(1, name.to_owned())];
    let names = |names: &[&str]| -> Vec<(usize, String)> {
        names.iter().map(|name| (1, (*name).to_owned())).collect()
    };
    for (text, expected) in [
        ("pub use mission_model;", one("mission_model")),
        ("pub use mission_model::Mission;", one("mission_model")),
        ("pub use mission_model as model;", one("mission_model")),
        ("pub use ::mission_model as rooted;", one("mission_model")),
        (
            "    pub use ::mission_model as model;",
            one("mission_model"),
        ),
        (
            "pub use mission_model::{self as grouped};",
            one("mission_model"),
        ),
        (
            "pub use {mission_model as braced, ::other_crate::Item};",
            names(&["mission_model", "other_crate"]),
        ),
        (
            "pub use { std::{fmt, io}, mission_model::* };",
            names(&["std", "mission_model"]),
        ),
        (
            "pub use {mission_model::{Mission, Unit}, std::fmt, crate::data};",
            names(&["mission_model", "std"]),
        ),
        (
            "pub use {{mission_model, ::other_crate}, self::local};",
            names(&["mission_model", "other_crate"]),
        ),
        (
            "pub extern crate mission_model as external;",
            one("mission_model"),
        ),
        (
            "pub use mission_model as model; // a re-export\n",
            one("mission_model"),
        ),
        (
            "#[cfg(test)]\npub use satellite_imagery as streamer;",
            vec![(2, "satellite_imagery".to_owned())],
        ),
        ("pub use {crate::data, self::local, super::parent};", vec![]),
        ("pub use crate::data as model;", vec![]),
        ("pub use self::data;\npub use super::data;", vec![]),
        ("pub(crate) use mission_model as private;", vec![]),
        ("pub(super) use mission_model::Mission;", vec![]),
        ("use mission_model as local;", vec![]),
        ("pub user_model: Model,", vec![]),
    ] {
        assert_eq!(reexported_crates(text), expected, "{text}");
    }
}

#[test]
fn public_reexports_read_a_reexport_spread_over_several_lines() {
    let text = "/// Docs.\npub use\n    mission_model\n    as model;\n\npub use {\n    // the item path\n    std::fmt,\n    mission_model::{\n        Mission,\n    },\n};\n";
    assert_eq!(
        reexported_crates(text),
        vec![
            (2, "mission_model".to_owned()),
            (6, "std".to_owned()),
            (6, "mission_model".to_owned()),
        ]
    );
}

/// Every statement of one text is read, each cited at its first line; restricted visibility, a
/// private `use` and a commented-out statement re-export nothing.
#[test]
fn public_reexports_cite_every_statement_of_a_text_at_its_first_line() {
    let text = "pub use mission_model as model;\n\
                pub use ::mission_model as rooted;\n\
                pub use mission_model::{self as grouped};\n\
                pub use {std::fmt, mission_model as braced};\n\
                pub use mission_model;\n\
                pub extern crate mission_model as external;\n\
                pub use {\n    std::io,\n    mission_model as spread,\n};\n\
                pub use\n    mission_model::Unit;\n\
                pub use std as standard;\n\
                pub(crate) use mission_model as private;\n\
                use mission_model as local;\n\
                // pub use mission_model as commented;\n";
    let lines: Vec<usize> = reexported_crates(text)
        .into_iter()
        .filter(|(_, name)| name == "mission_model")
        .map(|(line, _)| line)
        .collect();
    assert_eq!(lines, vec![1, 2, 3, 4, 5, 6, 7, 11]);
}
