# New mission dialog

The one-step "New Mission" dialog the [mission](/documentation_v2/glossary.md#mission) library
opens: it creates a draft mission from a title and its environment, then hands the author to the
[Mission Creator](/documentation_v2/glossary.md#mission-creator).

## Contents

```text
apps/website/frontend/src/v2/pages/mission_hub/create_dialog/
├── dialog.rs  `CreateMissionDialog`: the form, the create request, the hand-off to the editor
├── mod.rs     the module tree; re-exports `CreateMissionDialog`
└── tests/     unit tests for the request's briefing and the thumbnail field the form omits
```

## How it works

The library's "New Mission" button sets the `open` flag the dialog is bound to; the dialog has no
route of its own. Its fields are the title ("Operation Designation"), the terrain (Everon or
Arland, default Everon), the game mode ("Co-op PvE", "PvP" or "Zeus"), the insertion time (default
14:00), the weather ("Clear (Default)", "Overcast", "Heavy Rain", "Dense Fog"), the player cap (16
to 128 "Operators", default 64) and an optional library blurb. The author is the signed-in session,
not a field, and there is no thumbnail control, because the endpoint drops that key.

Submitting with an empty title shows the toast "Title is required". Otherwise the dialog sends
`POST /api/v1/missions` with `title`, `terrain`, `game_mode`, `weather`, `time_of_day`,
`max_players` and the trimmed `briefing`, reads the reply as `serde_json::Value`, shows "Mission
created", closes, and loads `/missions/{id}/edit` as a full page. A failure shows the
[API](/documentation_v2/glossary.md#api)'s message, or "Failed to create mission". The button reads
"Creating…" while the request is in flight and "Create Mission Draft" otherwise. Every field resets
whenever the dialog closes. The submit runs in the browser build only.

## Boundaries

- Depends on: `crate::v2::core::api::client` (`api_post`, `api_error_message`),
  `crate::v2::core::ui` (`Dialog`, `cn`, the toasts) and the `AuthStore` context.
- Used by: the mission library page in `apps/website/frontend/src/v2/pages/mission_hub/library/`,
  the only surface that opens it; `create_dialog_source` in
  `apps/website/frontend/src/v2/core/test_support/pins.rs` reads its source files.
- Rules: the create request carries only fields the endpoint accepts, the briefing included
  (`the_post_body_carries_the_authored_briefing`), and the form offers no thumbnail it cannot store
  (`the_create_form_offers_no_thumbnail_it_cannot_store`).

## Related documentation

- [Mission library page](/documentation_v2/website/frontend/pages/mission_hub/library/mission_library_page.md#creating-a-mission)
  — the library that opens this dialog, and the create flow with what the API checks.
