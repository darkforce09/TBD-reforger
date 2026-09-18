-- T-444 — Doctrine wiki seed for `cargo xtask db seed`.
-- Rows match the content_golden §5 wiki_pages block so a fresh DB has
-- the same manuals the GET /wiki golden was captured against (incl. the
-- V-suite `field-manual` slug). Idempotent on primary key.

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
   10, '000000000000000001', '2026-05-30 08:00:00+00')
ON CONFLICT (id) DO UPDATE SET
    slug = EXCLUDED.slug, category = EXCLUDED.category, title = EXCLUDED.title,
    icon = EXCLUDED.icon, body_md = EXCLUDED.body_md, nav_order = EXCLUDED.nav_order,
    updated_by = EXCLUDED.updated_by, updated_at = EXCLUDED.updated_at;
