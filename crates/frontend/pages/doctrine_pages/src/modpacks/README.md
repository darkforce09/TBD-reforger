# Modpacks page

The `/modpacks` page: the modpacks a server can require, listed beside the manifest of the selected
one, with its version, size, addons and Workshop collection link. An administrator also creates,
edits, activates and deletes packs here.

## Contents

```text
crates/frontend/pages/doctrine_pages/src/modpacks/
├── mod.rs          the module tree; re-exports `ModpacksPage`
├── mod_table.rs    the read dossier: totals, addon rows, launch and Workshop links, admin actions
├── mode_toggle.rs  `MpMode` and the read/edit switch both detail views share
├── pack_edit.rs    `PackEdit`: the edit draft, its reader and request body
├── pack_editor.rs  the administrator's edit form: pack fields, addon rows, save and cancel
├── page.rs         `ModpacksPage`: the list fetch, the selection, the mode and the split view
└── preset_list.rs  the master pane: title, create button, search box and one row per pack
```

## How it works

`ModpacksPage` renders inside `AuthGate` and fetches the pack list once; both panes of the
`GlassSplit` read that one list. The selection starts on the first pack, and choosing another
returns the detail pane to reading. The search box matches a pack's name or any of its addon names.
The detail pane shows the read dossier (`mod_table.rs`), or the edit form (`pack_editor.rs`) once an
administrator switches to editing.

`is_admin` is a memo over `has_min_role_authed` and the session's
[role](/documentation/glossary/n_to_z.md#role), so the create, edit, activate and delete controls
appear only for a signed-in administrator, never for a signed-out visitor or while the session
restores. The edit form seeds its signals from the pack once, so an edit in progress survives a
refetch of the list; an empty name falls back to the stored name and an empty version to `0.0.0`.
`PackEdit::to_put_body` stamps each addon row's index as its `sort_order`, so the draft's order is
the stored order. Every write to the [API](/documentation/glossary/a_to_f.md#api) refetches the list,
and a create selects the new pack. Sizes print through `format_download_size` of
`frontend_ui::byte_formatting`: `x.x GB` from a gibibyte up and whole `MB` below.

## Routes

| Route | Component | Access | Layout |
|---|---|---|---|
| `/modpacks` | `ModpacksPage` | route tier `none`; the list renders only for a signed-in viewer, the write controls only for `admin` | full-bleed inside the navigation frame; breadcrumb Doctrine & Info / Modpacks |

## Data

- `GET /api/v1/modpacks`: read as `DataEnvelope<ModpackDto>`: each pack's `id`, `name`,
  `version`, `total_size_bytes`, `workshop_url` and `is_current`, and its `mods` rows as
  `ModpackMod` (`name`, `is_key_dependency`, `workshop_id`, `mod_guid`, `version`; the last three
  read empty when the wire leaves them out).
- `POST /api/v1/modpacks`: sends a blank pack (`New Modpack`, version `0.1.0`, no addons), read
  back as `ModpackDto`.
- `PUT /api/v1/modpacks/{id}`: sends the draft from `PackEdit::to_put_body`, read back as
  `ModpackDto`.
- `POST /api/v1/modpacks/{id}/set-current` with `{}`: makes the pack the current one.
- `DELETE /api/v1/modpacks/{id}`: deletes the pack.
- The page reads the `AuthStore` and the toast queue from context. The launch button sends
  nothing; it shows a toast. The requests run in the browser build only; the views that run them exist in that build only.

## States

| State | What the viewer sees |
|---|---|
| session restoring | "Loading session…" |
| signed out | "Sign in to load live data from the platform." and a "Sign in with Discord" link to `/login` |
| loading | "Loading modpacks…" |
| failed | "Failed to load modpacks." |
| no pack selected | "No modpack selected." |
| list | "Modpacks", the "Search packs & mods…" box, and rows reading "v<version> · <n> mods · <size>", with an "Active" chip on the current pack; an administrator also gets "+ New" ("…" while it creates) |
| reading | the pack name, "v<version>", "<size> total" and "<n> mods included"; each addon with its Workshop id, and "[ REQUIRED ]" when required; "[ Launch Game & Auto-Download ]"; "View collection in Reforger Workshop ↗" when the pack has a Workshop URL |
| reading, administrator | also "[ read ]" and "[ edit ]", "Set current" ("Setting…") unless the pack is current, and "Delete" ("Deleting…") |
| editing | "Modpack name", "Version", "Workshop URL" and "Mark as current modpack"; per addon a "[ REQUIRED ]" toggle, a remove button and "Workshop id (game.mods[].modId)"; "No mods yet — add one below." when there are none; "Add a mod (e.g. ACE Reforged)…", "Workshop id" and "Add"; "Save Changes" ("Saving…") and "Cancel" |
| save failed | the API's message, or "Failed to save modpack", above the save button |
| toasts | "Launch requires the Reforger client", `Created "<name>"`, `Saved "<name>"`, "Set as current modpack" and "Modpack deleted"; a failure shows the API's message, or "Failed to create modpack", "Failed to set current" or "Failed to delete modpack" |

## Boundaries

- Depends on: `frontend_transport` (`api_get`, `api_post`, `api_put`, `api_post_ok`,
  `api_delete`, `Error::message_or`, and `ModpackDto`, `ModpackMod` and `DataEnvelope` from
  `crates/frontend/foundation/frontend_api_dtos/src/content.rs` and
  `crates/frontend/foundation/frontend_api_dtos/src/common.rs`), `frontend_session`
  (`has_min_role_authed`, `Role`, the `AuthStore` context), `frontend_ui` (`AuthGate`,
  the `split_pane` primitives, `MaterialIcon`, the toast queue) and `frontend_ui`
  (`format_download_size` for the pack and addon sizes).
- Used by: the `/modpacks` route in `crates/frontend/shell/frontend_application/src/app_routes.rs` and
  `crates/frontend/foundation/frontend_route_table/src/routes.rs`; the sidebar's "Modpacks" link in
  `crates/frontend/foundation/frontend_route_table/src/navigation_menu.rs`; `modpacks_source` in
  `crates/frontend/pages/doctrine_pages/src/tests/source_pins.rs`, which joins the seven source files for
  the page's test; the DOM oracle's `modpacks` capture in
  `tools/browser_testing/browser_gate_suites/src/dom_oracle/routes.rs`.
- Rules: the write controls are gated by the `is_admin` memo over `has_min_role_authed`, never by
  the browse-mode `has_min_role` (`admin_affordance_uses_authed_reactive_role` in
  `tests/modpacks.rs`); both panes read the one fetched list; the make-current button never shows
  on the current pack.

## Related documentation

- [Modpacks page](/documentation/crates/frontend/pages/doctrine_pages/modpacks/modpacks_page.md)
  — the page's behaviour and design.
- [Community content domain](/crates/api/api_community_content/src/README.md) — the modpack
  routes this page calls.
