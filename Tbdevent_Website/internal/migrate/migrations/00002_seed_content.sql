-- +goose Up
INSERT INTO pages (id, slug, title, published) VALUES
    ('a0000000-0000-4000-8000-000000000001', 'rules', 'Rules', TRUE),
    ('a0000000-0000-4000-8000-000000000002', 'compliance', 'Monetization Compliance', TRUE),
    ('a0000000-0000-4000-8000-000000000003', 'server-info', 'Server Information', TRUE),
    ('a0000000-0000-4000-8000-000000000004', 'mods', 'Mods Used', TRUE);

INSERT INTO page_sections (page_id, section_key, heading, content, sort_order) VALUES
    ('a0000000-0000-4000-8000-000000000001', 'intro', '', E'We''re all here to enjoy the game we play, so the biggest rule is to **NOT BREAK THE SPIRIT OF THE GAME**. If you realize you''re doing something that isn''t covered by the rules, but you wouldn''t like it if an opponent was doing it and it made you really mad, don''t do it.\n\nIn case you''re not sure, you can always contact the administrator, as they will have the final say. This is here just in case something that isn''t covered by the rules happens, so the game administrator can put a stop to it.', 0),

    ('a0000000-0000-4000-8000-000000000001', 'teamkilling', 'Teamkilling', E'- It''s forbidden to intentionally teamkill or destroy friendly vehicles and equipment\n- Teamkills are something that happens, it''s part of the game\n- There is no reason to be abusive towards people that teamkilled you, as it could be a genuine mistake\n- It''s wanted that a person that teamkilled apologizes to the victim\n- Killing a bot during warmup to take its equipment is forbidden (striping him is allowed)', 1),

    ('a0000000-0000-4000-8000-000000000001', 'misc', 'Misc', E'- It''s allowed to steal enemy equipment\n  - This includes radios, weapons, GPSs, uniforms etc.\n- It''s forbidden to intentionally reveal the positions of friendly squads\n  - Be it by saying where they are to the enemy\n- Infantry and ground vehicles can not leave the map\n- Planes and helicopters, even when carrying infantry, are allowed to leave the map', 2),

    ('a0000000-0000-4000-8000-000000000001', 'cheats', 'Cheats', E'- It''s forbidden to use any exploits or external and internal tools which grant the user an advantage over others\n  - This includes classic cheats\n  - This includes Arma bugs\n  - This includes textures disappearing\n- It''s forbidden to communicate with anyone also playing the game elsewhere than by the normal in game channels\n  - Even when communicating with someone not playing the game, it''s forbidden for them to give you information about the game', 3),

    ('a0000000-0000-4000-8000-000000000002', 'monetized', 'What is monetized', E'- Reserved slots — once applicable\n- Priority queue — once applicable\n- Cosmetic perks — custom patches\n- Ingame donator tag', 0),

    ('a0000000-0000-4000-8000-000000000002', 'pvp-note', 'Server Information', E'Monetization only applies to the 24/7 Conflict server. **TBD PvP Events are not monetizable** and the features are not available.', 1),

    ('a0000000-0000-4000-8000-000000000003', 'overview', '', E'TBD PvP Events run on dedicated event servers. Check Discord for upcoming dates, maps, and sign-up information.', 0),

    ('a0000000-0000-4000-8000-000000000004', 'mods-list', '', E'**ACE** and all its parts.\n\nWe respect intellectual property rights of community mods and we will only use mods that allow usage on monetized servers.', 0);

-- +goose Down
DELETE FROM page_sections;
DELETE FROM pages;
