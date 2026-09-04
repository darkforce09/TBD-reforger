# T-465 verify log — Wave 25 Class-R harden (T-447 false-green)

**pwd:** `/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-465`  
**branch:** `slice/T-465`

## IT

`cms_list_includes_draft_public_feed_excludes_non_admin_forbidden` in `apps/website/api/tests/cms_announcement_body.rs`:

- Admin POST draft → 201, `status=draft`
- `GET /api/v1/cms/announcements` pages → draft id present
- `GET /api/v1/announcements` pages → draft id absent
- enlisted + mission_maker `GET /cms/announcements` → **403** (`AdminUser` / `insufficient role`; not 401)

RED: published-only CMS SQL → draft missing from CMS list assert.

## Class-R

**cms.rs** `list_cms_announcements_is_drafts_plus_published_not_public_feed`:

- Handler-slice pin `_a: AdminUser` (M1)
- Filter `status IN ('draft', 'published')` count == 2 + windows on `query_scalar` / `query_as` (B1)

**content.rs** `content_boots_from_cms_list_not_mock_docs`:

- `page.data.iter().filter_map(doc_from_announcement)` + `docs.set(mapped)` (B2)
- retains LocalResource / api_get / no mock_docs

## RED→GREEN

| Perturbation | Result |
|---|---|
| B1 bait comment + published-only SQL | RED (`count` 1≠2) → GREEN restored |
| M1 drop `_a: AdminUser` | RED (handler-slice) → GREEN restored |
| B2 Effect ignores `opt` / `docs.set(Vec::new())` | RED (map needle) → GREEN restored |
| M2 missing IT | closed by new IT |

## Gate

`bash scripts/platform/wave.sh gate --slice T-465` → **PASS**

## Owns

No widen beyond ticket: `cms.rs`, `cms_announcement_body.rs` (under `tests/`), `content.rs`.

## Residuals

None.
