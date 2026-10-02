-- Doctrine wiki seed for `cargo xtask db seed`.
-- The pages match the content_golden §5 wiki_pages block, so a fresh DB has the same manuals the
-- GET /wiki golden was captured against (incl. the V-suite `field-manual` slug). The fifth,
-- `wiki-formatting-guide`, is the authors' reference for every construct a wiki page can carry.
-- Every page's history starts at revision 1. Re-seeding refreshes a page and its revision 1 only
-- while the page is still at revision 1, so a page saved since it was seeded keeps its content
-- and its history. Idempotent on primary key.

INSERT INTO wiki_pages (id, slug, category, title, icon, body_md, nav_order, updated_by, updated_at)
VALUES
  ('00000000-0000-4000-2000-000000000001', 'field-manual', 'Doctrine', 'Field Manual', 'menu_book',
   E'# TBD Field Manual\n\nThe field manual is the single source of truth for how this unit fights. Where a mission briefing contradicts it, the briefing wins for that operation only.\n\n## 1. Chain of command\n\nPlatoon staff issue intent, not instructions. Squad leads own execution inside their assigned boundary.\n\n## 2. Movement\n\n- Default formation is a staggered column on roads, wedge in the open.\n- Bounding overwatch inside 400 m of a suspected contact.\n- Nobody crosses a linear danger area without near-side security set.\n\n## 3. Contact drills\n\nOn contact: return fire, take cover, report. In that order. The contact report is `CONTACT — direction — distance — description`.\n\n## 4. Casualties\n\nStabilise where the casualty falls only if the position is covered. Otherwise drag to cover first. Medics do not move forward of the base of fire.',
   1, '000000000000000001', '2026-07-14 10:12:00+00'),
  ('00000000-0000-4000-2000-000000000002', 'radio-procedure', 'Doctrine', 'Radio Procedure', 'radio',
   E'# Radio Procedure\n\n## Nets\n\n| Net | Users | Channel |\n| --- | --- | --- |\n| Command | Platoon staff + squad leads | 1 |\n| Squad | Inside a squad | 2–5 |\n| Air | Rotary + JTAC | 8 |\n\n## Format\n\nAlways: `<callsign you want> this is <your callsign>, <message>, over.`\n\nBrevity beats politeness. If the net is busy, wait — do not step on a contact report.',
   2, '000000000000000002', '2026-07-06 19:45:00+00'),
  ('00000000-0000-4000-2000-000000000003', 'medical-sop', 'Doctrine', 'Medical SOP', 'medical_services',
   E'# Medical SOP\n\nTourniquet high and tight, then reassess. Morphine only after bleeding is controlled — it masks the shock that tells you the bleeding is not controlled.\n\nEvery rifleman carries two tourniquets. One is not for you.',
   3, '000000000000000002', '2026-06-29 14:20:00+00'),
  -- No icon: the nav has to render a row with an empty icon slot.
  ('00000000-0000-4000-2000-000000000004', 'server-rules', 'Administration', 'Server Rules', NULL,
   E'# Server Rules\n\n1. No team-killing. Two warnings then a ban; see the audit log for precedent.\n2. Modpack must match the announced version.\n3. Zeus is a privilege, not a rank.',
   10, '000000000000000001', '2026-05-30 08:00:00+00'),
  -- Renders every block, inline and callout kind with no save finding.
  ('00000000-0000-4000-2000-000000000005', 'wiki-formatting-guide', 'Administration', 'Wiki Formatting Guide', 'edit_note',
   E'# Wiki Formatting Guide\n\nThis page shows every kind of formatting a doctrine page can carry. Each section shows the result, then the markdown that makes it, so you can copy it into your own page. Pages are plain markdown: when you save, the wiki refuses raw HTML, links and images whose address it does not trust, and anything nested more than 16 levels deep, and it tells you the line of each problem.\n\n## Contents\n\n- [Headings](#headings)\n- [Text](#text)\n- [Links](#links)\n- [Images](#images)\n- [Tables](#tables)\n- [Checklists](#checklists)\n- [Callouts](#callouts)\n- [Quotes](#quotes)\n- [Code](#code)\n- [Rules](#rules)\n\n## Headings\n\nStart a line with one to six `#` characters and a space. Every heading gets an anchor made from its text: `## Radio nets` is linked as `#radio-nets`, and a repeated heading gets `-2`, `-3` and so on.\n\n# Heading level 1\n## Heading level 2\n### Heading level 3\n#### Heading level 4\n##### Heading level 5\n###### Heading level 6\n\n## Text\n\nWrite **bold** as `**bold**`, *italic* as `*italic*`, ~~struck-through~~ text as `~~struck-through~~` and `inline code` between backticks. End a line with a backslash to break it\\\nwithout starting a new paragraph. Leave a blank line between paragraphs.\n\n## Links\n\n- Another wiki page: [Field Manual](/wiki/field-manual), written `[Field Manual](/wiki/field-manual)`.\n- Another site: [Arma Reforger](https://reforger.armaplatform.com), written with the full `https://` address.\n- A heading on this page: [back to Contents](#contents), written `[back to Contents](#contents)`.\n- An email address: [the staff inbox](mailto:staff@example.com), written `[the staff inbox](mailto:staff@example.com)`.\n\nA link address starts with `https://`, `http://`, `mailto:`, `/` or `#`. Any other address, such as a bare `field-manual` or a `javascript:` address, is refused when you save.\n\n## Images\n\n![Everon grid map](/uploads/wiki-formatting-guide-map.png "Everon grid map")\n\nWritten `![Everon grid map](/uploads/wiki-formatting-guide-map.png "Everon grid map")`. Upload the picture through the content manager and paste the `/uploads/...` address it gives you, or use an `https://` address. The text in the square brackets is shown when the picture cannot be, and the quoted title is optional.\n\n## Tables\n\n| Net | Users | Channel | Notes |\n| :--- | :---: | ---: | --- |\n| Command | Platoon staff and squad leads | 1 | Always monitored |\n| Squad | Inside a squad | 2 | One per squad |\n| Air | Rotary wing and JTAC | 8 | **Brevity** first |\n\nSeparate the cells with `|`. The second line sets each column''s alignment: `:---` left, `:---:` centred, `---:` right and `---` none.\n\n## Checklists\n\n- [x] Radio checked\n- [x] Batteries packed\n- [ ] Map marked\n\nWrite `- [x]` for a ticked box and `- [ ]` for an empty one.\n\n## Callouts\n\n> [!NOTE]\n> A note adds background the reader may skip.\n\n> [!TIP]\n> A tip shows a better way to do something.\n\n> [!IMPORTANT]\n> Important marks something the reader must not miss.\n\n> [!INFO]\n> Info gives neutral reference detail.\n\n> [!WARNING]\n> A warning flags a risk to the mission.\n\n> [!CAUTION]\n> Caution flags a risk to people or equipment.\n\n> [!CRITICAL]\n> Critical marks a rule that must never be broken.\n\nStart a quote with `> [!NOTE]`, `> [!TIP]`, `> [!IMPORTANT]`, `> [!INFO]`, `> [!WARNING]`, `> [!CAUTION]` or `> [!CRITICAL]` on its own line, then write the callout on the lines below, each starting with `>`.\n\n## Quotes\n\n> Brevity beats politeness. If the net is busy, wait.\n\nA quote is a run of lines starting with `>` and no callout marker.\n\n## Code\n\n```text\nCONTACT - direction - distance - description\n```\n\nFence a block with three backticks on the lines above and below it, and name its language after the opening fence.\n\n## Rules\n\nThree dashes on a line of their own draw a rule:\n\n---\n\nLeave a blank line above the dashes, or the line above them becomes a heading.',
   20, '000000000000000001', '2026-09-26 12:00:00+00')
ON CONFLICT (id) DO UPDATE SET
    slug = EXCLUDED.slug, category = EXCLUDED.category, title = EXCLUDED.title,
    icon = EXCLUDED.icon, body_md = EXCLUDED.body_md, nav_order = EXCLUDED.nav_order,
    updated_by = EXCLUDED.updated_by, updated_at = EXCLUDED.updated_at
WHERE wiki_pages.revision = 1;

-- Revision 1 of each seeded page still at revision 1: its content, editor and update time.
INSERT INTO wiki_page_revisions
    (page_id, revision, slug, category, title, icon, nav_order, body_md, author_id, created_at)
SELECT id, 1, slug, category, title, icon, nav_order, body_md, updated_by, updated_at
FROM wiki_pages
WHERE revision = 1 AND id IN (
    '00000000-0000-4000-2000-000000000001', '00000000-0000-4000-2000-000000000002',
    '00000000-0000-4000-2000-000000000003', '00000000-0000-4000-2000-000000000004',
    '00000000-0000-4000-2000-000000000005')
ON CONFLICT (page_id, revision) DO UPDATE SET
    slug = EXCLUDED.slug, category = EXCLUDED.category, title = EXCLUDED.title,
    icon = EXCLUDED.icon, nav_order = EXCLUDED.nav_order, body_md = EXCLUDED.body_md,
    author_id = EXCLUDED.author_id, created_at = EXCLUDED.created_at;
