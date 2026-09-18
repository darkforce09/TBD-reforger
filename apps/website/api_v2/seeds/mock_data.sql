-- Mock Data for TBD Reforger

-- 1. Users (Personnel Roster)
INSERT INTO users (discord_id, username, discord_handle, avatar_url, arma_character, role, total_deployments, attendance_rate, created_at, updated_at) VALUES
('111111111111111111', 'Admin Dave', 'Dave#1234', 'https://cdn.discordapp.com/embed/avatars/0.png', '[TBD] Admin Dave', 'admin', 42, 94.5, NOW(), NOW()),
('222222222222222222', 'Mission Maker Mike', 'Mike#5678', 'https://cdn.discordapp.com/embed/avatars/1.png', '[TBD] MM Mike', 'mission_maker', 24, 88.0, NOW(), NOW()),
('333333333333333333', 'Leader Luke', 'Luke#9012', 'https://cdn.discordapp.com/embed/avatars/2.png', '[TBD] Leader Luke', 'leader', 60, 99.0, NOW(), NOW()),
('444444444444444444', 'Enlisted Ethan', 'Ethan#3456', 'https://cdn.discordapp.com/embed/avatars/3.png', '[TBD] Pvt. Ethan', 'enlisted', 5, 100.0, NOW(), NOW())
ON CONFLICT (discord_id) DO NOTHING;

-- 2. Modpacks
INSERT INTO modpacks (id, name, version, total_size_bytes, workshop_url, is_current, created_at) VALUES
('00000000-0000-4000-a000-000000000001', 'Core Modern Expansion', '2.1', 48532275200, 'https://steamcommunity.com/sharedfiles/filedetails/?id=123456789', true, NOW()),
('00000000-0000-4000-a000-000000000002', 'Vietnam War Era', '1.0', 15032275200, 'https://steamcommunity.com/sharedfiles/filedetails/?id=987654321', false, NOW())
ON CONFLICT (id) DO NOTHING;

INSERT INTO modpack_mods (id, modpack_id, name, is_key_dependency, sort_order) VALUES
('00000000-0000-4000-b000-000000000001', '00000000-0000-4000-a000-000000000001', 'RHS: Status Quo', true, 1),
('00000000-0000-4000-b000-000000000002', '00000000-0000-4000-a000-000000000001', 'TFAR', true, 2),
('00000000-0000-4000-b000-000000000003', '00000000-0000-4000-a000-000000000001', 'ACE3', true, 3),
('00000000-0000-4000-b000-000000000004', '00000000-0000-4000-a000-000000000002', 'SOG Prairie Fire', true, 1)
ON CONFLICT (id) DO NOTHING;

-- 3. Missions
INSERT INTO missions (id, title, author_id, terrain, game_mode, weather, time_of_day, max_players, status, briefing, created_at, updated_at) VALUES
('00000000-0000-4000-c000-000000000001', 'Operation Red Dawn', '222222222222222222', 'everon', 'pve_coop', 'overcast', '05:30', 64, 'live', 'Soviet forces have invaded Everon. Repel them.', NOW(), NOW()),
('00000000-0000-4000-c000-000000000002', 'Operation Blue Storm', '222222222222222222', 'arland', 'pvp', 'clear', '14:00', 32, 'pending_approval', 'US vs USSR on Arland.', NOW(), NOW()),
('00000000-0000-4000-c000-000000000003', 'Draft Mission Beta', '333333333333333333', 'everon', 'zeus', 'heavy_rain', '22:00', 40, 'draft', 'Testing zeus mechanics in the rain.', NOW(), NOW()),
('00000000-0000-4000-c000-000000000004', 'Operation Desert Fox', '222222222222222222', 'custom', 'pve_coop', 'clear', '12:00', 50, 'live', 'Desert engagement.', NOW(), NOW())
ON CONFLICT (id) DO NOTHING;
