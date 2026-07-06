-- T-145 G5 differential seed. Fixed timestamps with varied fractional seconds so the
-- Go `time.Time.MarshalJSON` vs Rust `go_time` serializer are compared on the .789 /
-- .5 / whole-second paths. Data-only (schema already migrated).

-- Users: numeric attendance_rate + an arma-linked leaderboard player.
INSERT INTO users (discord_id, username, discord_handle, avatar_url, arma_id, arma_character, role, is_banned, ban_reason, total_deployments, attendance_rate, last_login_at, created_at, updated_at)
VALUES
 ('700000000000000001', 'Author One', 'authorone', 'https://cdn/a.png', NULL, '', 'mission_maker', false, '', 3, 87.5, '2026-07-01 09:00:00.5+00', '2026-06-01 12:34:56.789+00', '2026-06-02 12:34:56+00'),
 ('700000000000000002', 'Player Two', 'playertwo', '', '76561190000000002', '[TBD] Two', 'enlisted', false, '', 5, 100.0, '2026-07-01 09:00:00+00', '2026-06-01 12:00:00+00', '2026-06-02 12:00:00+00')
ON CONFLICT (discord_id) DO NOTHING;

-- Announcement: published, tag, snippet (omitempty), published_at timestamp.
INSERT INTO announcements (id, title, body, snippet, tag, thumbnail_url, author_id, status, is_pinned, pushed_to_discord, discord_message_id, published_at, created_at, updated_at)
VALUES ('70000000-0000-0000-0000-000000000020', 'Operation Redwood', '<p>Briefing</p>', 'Short preview', 'event', '', '700000000000000001', 'published', true, false, '', '2026-06-15 18:30:00.25+00', '2026-06-15 18:00:00.789+00', '2026-06-15 18:30:00+00')
ON CONFLICT (id) DO NOTHING;

-- Modpack (current) + mods (bigint sizes, ordered).
INSERT INTO modpacks (id, name, version, is_current, total_size_bytes, workshop_url, created_at)
VALUES ('70000000-0000-0000-0000-000000000030', 'TBD Core', '1.4.2', true, 1610612736, 'https://steamcommunity.com/x', '2026-05-01 00:00:00+00')
ON CONFLICT (id) DO NOTHING;
INSERT INTO modpack_mods (id, modpack_id, name, is_key_dependency, sort_order)
VALUES
 ('70000000-0000-0000-0000-000000000031', '70000000-0000-0000-0000-000000000030', 'ACE', true, 0),
 ('70000000-0000-0000-0000-000000000032', '70000000-0000-0000-0000-000000000030', 'CBA', false, 1)
ON CONFLICT (id) DO NOTHING;

-- Server + status: numeric server_fps + inet ip.
INSERT INTO servers (id, name, ip, port, required_modpack_id, is_active)
VALUES ('70000000-0000-0000-0000-000000000040', 'Primary', '203.0.113.7'::inet, 2302, '70000000-0000-0000-0000-000000000030', true)
ON CONFLICT (id) DO NOTHING;
INSERT INTO server_statuses (server_id, is_online, player_count, max_players, server_fps, uptime_seconds, current_match_id, ingame_time, ingame_weather, updated_at)
VALUES ('70000000-0000-0000-0000-000000000040', true, 42, 64, 58.7, 3600, NULL, '08:00', 'clear', '2026-07-01 08:00:00.123+00')
ON CONFLICT (server_id) DO NOTHING;

-- Mission (live) + version with a jsonb editor payload (passthrough) + time_of_day.
INSERT INTO missions (id, title, author_id, terrain, custom_terrain_name, game_mode, weather, time_of_day, max_players, status, thumbnail_url, briefing, rejection_reason, created_at, updated_at)
VALUES ('70000000-0000-0000-0000-000000000001', 'Redwood Assault', '700000000000000001', 'everon', '', 'pve_coop', 'overcast', '06:30', 32, 'live', '', 'Take the ridge.', '', '2026-06-10 10:00:00.5+00', '2026-06-11 10:00:00+00')
ON CONFLICT (id) DO NOTHING;
INSERT INTO mission_versions (id, mission_id, semver, json_payload, editor_notes, created_by, created_at)
VALUES ('70000000-0000-0000-0000-000000000010', '70000000-0000-0000-0000-000000000001', '1.0.0', '{"editor":{"factions":[{"key":"USA","squadIds":["s1"]}],"squads":[{"id":"s1","name":"Alpha","slotIds":["x0"]}],"slots":[{"id":"x0","index":0,"role":"SL"}]},"meta":{"n":1,"f":2.5}}'::jsonb, 'v1', '700000000000000001', '2026-06-10 10:00:00+00')
ON CONFLICT (id) DO NOTHING;
UPDATE missions SET current_version_id = '70000000-0000-0000-0000-000000000010' WHERE id = '70000000-0000-0000-0000-000000000001';

-- Leave requests: date columns → serialized as midnight-UTC timestamps.
INSERT INTO leave_requests (id, discord_id, starts_on, ends_on, reason, status, reviewed_by, created_at)
VALUES ('70000000-0000-0000-0000-000000000070', '700000000000000002', '2026-08-01', '2026-08-10', 'holiday', 'pending', NULL, '2026-07-20 14:00:00.5+00')
ON CONFLICT (id) DO NOTHING;

-- Match + player stats → leaderboard MV.
INSERT INTO matches (id, source_match_id, event_id, mission_id, terrain, started_at, ended_at, outcome, winning_faction, aar_replay_url, created_at)
VALUES ('70000000-0000-0000-0000-000000000060', 'src-1', NULL, '70000000-0000-0000-0000-000000000001', 'everon', '2026-06-20 20:00:00+00', '2026-06-20 22:00:00+00', 'success', 'USA', '', '2026-06-20 22:05:00+00')
ON CONFLICT (id) DO NOTHING;
INSERT INTO match_player_stats (id, match_id, arma_id, discord_id, role_played, kills, deaths, team_kills, longest_kill_m, vehicles_destroyed, is_command, command_win, source_event_id, created_at)
VALUES ('70000000-0000-0000-0000-000000000061', '70000000-0000-0000-0000-000000000060', '76561190000000002', '700000000000000002', 'SL', 12, 3, 0, 640, 2, true, true, 'evt-1', '2026-06-20 22:05:00+00')
ON CONFLICT (id) DO NOTHING;

REFRESH MATERIALIZED VIEW leaderboard_totals;
