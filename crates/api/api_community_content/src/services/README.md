# Community content services

The pieces of community content that other code shares: the modpack read path that returns a
pack with its mods, and the wiki markup reader that turns a doctrine page's markdown into the
typed tree the wiki page renders. The Discord webhook announcements are pushed through lives in
the `api_discord` crate, `crates/api/api_discord/`, and the equipment data viewer's dataset
import and read queries in the `api_equipment_datasets` crate, `crates/api/api_equipment_datasets/`.

## Contents

```text
crates/api/api_community_content/src/services/
├── mod.rs              the module tree
├── modpack_lookup.rs   `ModpackDto` and its reads: by id, the current pack, and mod hydration
└── wiki_markup/        `read_markup`: a wiki page's markdown as safe typed blocks, and the save's refusal findings
```

## How it works

`modpack_lookup.rs` owns the one pack-plus-mods query: `load_modpack`, `load_current_modpack` and
`with_mods` return a `ModpackDto`, the pack with its ordered mods flattened into one object, and
the `modpack_cols!` projection keeps every modpack read null-tolerant in the same way.
`wiki_markup/` parses a page's `body_md` with pulldown-cmark into headings, paragraphs, lists,
tables, callouts, quotes, code and rules, replacing each unsafe link or image, raw HTML and nesting
deeper than 16 with a safe form and recording it as a finding with its line; its README holds the
tree and the refusal rules.

## Boundaries

- Depends on: the domain's models; `api_foundation::text::content_url_policy`; sqlx; pulldown-cmark
  (`wiki_markup/`).
- Used by: the domain's handlers, the wiki handlers reading and saving through `wiki_markup/`;
  `api_command_center`'s dashboard (`load_current_modpack`) and `api_server_infrastructure`'s server intel
  (`load_modpack`, `ModpackDto`).
- Rules: a domain that needs a modpack calls `modpack_lookup.rs` rather than writing its own query;
  a wiki page's markdown is parsed only by `wiki_markup/`, for the reads and the save alike.
