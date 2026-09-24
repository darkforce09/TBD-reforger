# Mission write-boundary checks

The predicates the [missions](/documentation_v2/glossary.md#missions) handlers apply to authored
input before a row is written: who may read or change a
[mission](/documentation_v2/glossary.md#mission), the scalar fields of a mission row, the semver of
a version, and whether a version payload is worth becoming the current version.

## Contents

```text
apps/website/api_v2/src/missions/validation/
├── access.rs           `can_view` and `can_edit`: who may read and who may change a mission
├── mission_fields.rs   title, thumbnail URL, terrain, game mode, weather, time of day, player range
├── mod.rs              the module tree
├── semver.rs           SemVer 2.0.0 parsing of a version's semver
├── tests/              unit tests for the field guards, the semver parse and the payload checks
└── version_payload.rs  the title a payload mirrors onto its mission, and the vacuous-payload refusal
```

## How it works

A handler owns the HTTP answer; these functions own the accept set, so the create and the patch of
a mission cannot accept different values. They refuse rather than repair: a value two writers store
must reach the column exactly as the other writer would store it. The one repair is the title,
display text that is stored trimmed.

- `access.rs`: a `live` mission is visible to every signed-in member; before that it belongs to its
  author, and an administrator stands in for the author everywhere.
- `mission_fields.rs`: the title must hold more than whitespace; the thumbnail URL is empty or an
  absolute `http://` or `https://` URL, because the library renders it as an image source; the
  terrain, game mode and weather must name a value of their Postgres enum, and a blank weather is
  never `clear`; the time of day is `HH:MM` or `HH:MM:SS` within 23:59:59, which the
  [Mission Creator](/documentation_v2/glossary.md#mission-creator)'s clock can read back, and is
  stored as sent.
- `semver.rs`: the full SemVer 2.0.0 grammar, pre-release and build metadata included, with no
  padding and no leading zeros, so `' 0.1.0 '` never stands beside `0.1.0` as a second version.
- `version_payload.rs`: a payload with no editor graph and no placed content is vacuous and refused
  at save with 400, since as the current version it would leave nothing to compile or seat; an
  explicit empty `editor.slots` array is a draft skeleton and passes.

## Boundaries

- Depends on: `models::mission`; `core` for `AuthUser` and errors; `serde_json` for the payload
  reads.
- Used by: the handlers `mission_library.rs`, `mission_lifecycle.rs`, `mission_versions.rs`,
  `mission_armory.rs`, `mission_reviews.rs` and `mission_export.rs` in
  `apps/website/api_v2/src/missions/handlers/`, and `services::mission_write_lock`; nothing
  outside the domain.
- Rules: a blank weather stays invalid, so a patch never rewrites a stored weather to `clear`
  (`blank_weather_is_not_clear` in `tests/mission_fields.rs`); only real SemVer versions pass
  (`semver_accepts_real_versions_only` in `tests/semver.rs`); an empty `editor.slots` skeleton is
  not vacuous (`version_payload_vacuous_rejects_empty_keeps_editor_skeleton` in
  `tests/version_payload.rs`).
