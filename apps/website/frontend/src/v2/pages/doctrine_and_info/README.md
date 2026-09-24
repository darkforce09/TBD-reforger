# Doctrine and info pages

The reference pages the community consults: the doctrine wiki of standard operating procedures and
manuals, the vehicle identification index, and the modpack manifests a server expects a client to
have. Their data comes from the
[community content](/documentation_v2/glossary.md#community-content) domain of the
[API](/documentation_v2/glossary.md#api).

## Contents

```text
apps/website/frontend/src/v2/pages/doctrine_and_info/
├── mod.rs     the module tree; declares the three page modules
├── modpacks/  the `/modpacks` page: pack list, manifest, and the administrator's edit form
├── vehicles/  the `/vehicles` page: faction-grouped vehicle list and identification dossier
└── wiki/      the `/wiki` and `/wiki/:slug` page: manual index, Markdown reader and editor
```

## How it works

The three pages share one shape. Each renders inside `AuthGate`, fetches its whole list once and
lays it out in the `GlassSplit` master-detail view from `crate::v2::core::ui::split_pane`: a
`SidebarSearch` box and `ListDetailItem` rows in the master pane, the selected item in the detail
pane, both reading the one fetched list. The wiki and vehicle rows arrive as untyped JSON, read
through each folder's total `vstr` helper; the modpacks arrive as the typed `ModpackDto`. The wiki
and the modpacks give an administrator a read/edit switch, gated by a memo over
`has_min_role_authed` and the session's [role](/documentation_v2/glossary.md#role), so a
signed-out visitor never sees it; the vehicle index only reads.

## Public surface

- `modpacks::ModpacksPage`, `vehicles::VehicleDatabasePage` and `wiki::WikiPage`: the route
  components `apps/website/frontend/src/app_routes.rs` binds to `/modpacks`, `/vehicles`, `/wiki`
  and `/wiki/:slug`.

## Boundaries

- Depends on: `crate::v2::core::api` (the request client, `DataEnvelope`, `ModpackDto`),
  `crate::v2::core::auth` (`has_min_role_authed`, `Role`, the `AuthStore` context) and
  `crate::v2::core::ui` (`AuthGate`, the `split_pane` primitives, the toast queue); over HTTP, the
  `/api/v1/wiki`, `/api/v1/vehicle-database` and `/api/v1/modpacks` routes of the community content
  domain.
- Used by: the route table in `apps/website/frontend/src/app_routes.rs` and
  `apps/website/frontend/src/router.rs`; the sidebar's "Doctrine & Info" section in
  `apps/website/frontend/src/v2/pages/navigation/nav_config.rs`; `wiki_source` and
  `modpacks_source` in `apps/website/frontend/src/v2/core/test_support/pins.rs`.
- Rules: every write control is gated by `has_min_role_authed`, never by the browse-mode
  `has_min_role` (`admin_affordance_uses_authed_reactive_role` in `modpacks/tests/modpacks.rs` and
  `wiki/tests/wiki.rs`); each page fetches its list once, and both of its panes read that list.

## Related documentation

- [Doctrine wiki page](/documentation_v2/website/frontend/pages/doctrine_and_info/wiki/wiki_page.md)
  — the wiki's behaviour and design.
- [Vehicle database page](/documentation_v2/website/frontend/pages/doctrine_and_info/vehicles/vehicle_database_page.md)
  — the vehicle index's behaviour and design.
- [Modpacks page](/documentation_v2/website/frontend/pages/doctrine_and_info/modpacks/modpacks_page.md)
  — the modpacks page's behaviour and design.
- [Community content domain](/apps/website/api_v2/src/community_content/README.md) — the routes
  these pages read and write.
