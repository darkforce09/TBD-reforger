use super::*;

#[test]
fn expand_covers_web_and_compose_and_nothing_else() {
    assert_eq!(
        expand_make_vars("cd $(WEB) && $(COMPOSE) up -d db"),
        "cd apps/website/api_v2 && podman compose up -d db"
    );
    assert_eq!(expand_make_vars("$(CURDIR)/x"), "$(CURDIR)/x");
}

#[test]
fn recipe_body_strips_make_prefixes_and_comments() {
    let mk = "seed: ## doc\n\t@# comment\n\t-cmd one\n\t@cmd two\nnext:\n\tcmd three\n";
    assert_eq!(recipe_body(mk, "seed"), vec!["cmd one", "cmd two"]);
}

/// `seed:` must not swallow a following target's recipe, and `seed-dev:` must not open it.
#[test]
fn recipe_body_stops_at_the_next_target() {
    let mk = "seed-dev:\n\tnope\nseed:\n\tyes\nnext:\n\tno\n";
    assert_eq!(recipe_body(mk, "seed"), vec!["yes"]);
}

/// The rendered `seed` lane is the contract `verify wiki-seeds` pins: five files, wiki last.
#[test]
fn seed_recipe_keeps_all_five_appliers_in_order() {
    let all = rendered_recipes();
    let (_, seed) = all.iter().find(|(t, _)| *t == "seed").expect("seed lane");
    assert_eq!(seed.len(), 5);
    assert!(seed[0].ends_with("< seeds/discord_roles.sql"));
    assert!(
        seed[4].ends_with("< seeds/wiki_pages.sql"),
        "`verify wiki-seeds` pins the wiki seed to this lane"
    );
}
