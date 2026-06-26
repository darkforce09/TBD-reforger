-- registry_dev.sql
-- Dev seed for the T-068 Virtual Arsenal registry catalog. Mirrors the T-068.1
-- Workbench export (packages/tbd-schema/registry/registry-items.workbench.json):
-- 21 real rows across all 5 kinds (8 character, 4 gear_primary, 3 gear_uniform,
-- 4 gear_vest, 2 gear_helmet). Idempotent and self-contained so `make seed` works
-- WITHOUT cmd/seed's mock_data.sql: it upserts the current modpack FK first.
--
-- modpack_id = the mock current modpack (mock_data.sql), the modpacks.is_current
-- row used by GET /api/v1/registry's default resolution.

-- Ensure the current modpack exists (FK target). No-op if mock_data.sql already ran.
INSERT INTO modpacks (id, name, version, total_size_bytes, workshop_url, is_current, created_at)
VALUES ('00000000-0000-4000-a000-000000000001', 'Core Modern Expansion', '2.1', 48532275200,
        'https://steamcommunity.com/sharedfiles/filedetails/?id=123456789', true, NOW())
ON CONFLICT (id) DO NOTHING;

INSERT INTO registry_items (modpack_id, resource_name, display_name, category, kind, sort_order) VALUES
('00000000-0000-4000-a000-000000000001', '{26A9756790131354}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Rifleman.et', 'US Rifleman', 'NATO/US_Army/Rifleman', 'character', 1),
('00000000-0000-4000-a000-000000000001', '{84029128FA6F6BB9}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_GL.et', 'US Grenadier', 'NATO/US_Army/Grenadier', 'character', 2),
('00000000-0000-4000-a000-000000000001', '{C9E4FEAF5AAC8D8C}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Medic.et', 'US Medic', 'NATO/US_Army/Medic', 'character', 3),
('00000000-0000-4000-a000-000000000001', '{5B1996C05B1E51A4}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_AR.et', 'US Automatic Rifleman', 'NATO/US_Army/AutomaticRifleman', 'character', 4),
('00000000-0000-4000-a000-000000000001', '{1623EA3AEFACA0E4}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_MG.et', 'US Machine Gunner', 'NATO/US_Army/MachineGunner', 'character', 5),
('00000000-0000-4000-a000-000000000001', '{0B3167BB0FB68110}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_PL.et', 'US Platoon Leader', 'NATO/US_Army/Leadership', 'character', 6),
('00000000-0000-4000-a000-000000000001', '{27BF1FF235DD6036}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_LAT.et', 'US Light Anti-Tank', 'NATO/US_Army/AntiTank', 'character', 7),
('00000000-0000-4000-a000-000000000001', '{36CCDB4556ECDA06}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Engineer.et', 'US Engineer', 'NATO/US_Army/Engineer', 'character', 8),
('00000000-0000-4000-a000-000000000001', '{3E413771E1834D2F}Prefabs/Weapons/Rifles/M16/Rifle_M16A2.et', 'M16A2', 'NATO/Weapons/Primary', 'gear_primary', 9),
('00000000-0000-4000-a000-000000000001', '{5A987A8A13763769}Prefabs/Weapons/Rifles/M16/Rifle_M16A2_M203.et', 'M16A2 + M203', 'NATO/Weapons/Primary', 'gear_primary', 10),
('00000000-0000-4000-a000-000000000001', '{D2B48DEBEF38D7D7}Prefabs/Weapons/MachineGuns/M249/MG_M249.et', 'M249 SAW', 'NATO/Weapons/Primary', 'gear_primary', 11),
('00000000-0000-4000-a000-000000000001', '{D182DCDD72BF7E34}Prefabs/Weapons/MachineGuns/M60/MG_M60.et', 'M60', 'NATO/Weapons/Primary', 'gear_primary', 12),
('00000000-0000-4000-a000-000000000001', '{C7861F11D5334C0E}Prefabs/Characters/Uniforms/Jacket_US_BDU.et', 'BDU Jacket (Woodland)', 'NATO/Uniform', 'gear_uniform', 13),
('00000000-0000-4000-a000-000000000001', '{3CCA7A9BB4FD3197}Prefabs/Characters/Uniforms/Jacket_US_BDU_rolledup.et', 'BDU Jacket (Rolled)', 'NATO/Uniform', 'gear_uniform', 14),
('00000000-0000-4000-a000-000000000001', '{604BB72BE8E023C2}Prefabs/Characters/Uniforms/Pants_US_BDU.et', 'BDU Pants (Woodland)', 'NATO/Uniform', 'gear_uniform', 15),
('00000000-0000-4000-a000-000000000001', '{4B57C11AA5161760}Prefabs/Characters/Vests/Vest_PASGT/Vest_PASGT.et', 'PASGT Vest', 'NATO/Vest', 'gear_vest', 16),
('00000000-0000-4000-a000-000000000001', '{2835A0EA3B79E63E}Prefabs/Characters/Vests/Vest_ALICE/Variants/Vest_ALICE_rifleman.et', 'ALICE Vest (Rifleman)', 'NATO/Vest', 'gear_vest', 17),
('00000000-0000-4000-a000-000000000001', '{156DC7109CEE6F69}Prefabs/Characters/Vests/Vest_ALICE/Variants/Vest_ALICE_AR.et', 'ALICE Vest (Automatic Rifleman)', 'NATO/Vest', 'gear_vest', 18),
('00000000-0000-4000-a000-000000000001', '{725C5E1C75CADAF4}Prefabs/Characters/Vests/Vest_M69/Vest_M69_M81woodland.et', 'M69 Vest (M81 Woodland)', 'NATO/Vest', 'gear_vest', 19),
('00000000-0000-4000-a000-000000000001', '{FE5C49069C2499D9}Prefabs/Characters/HeadGear/Helmet_PASGT_01/Helmet_PASGT_01_cover.et', 'PASGT Helmet (Cover)', 'NATO/Helmet', 'gear_helmet', 20),
('00000000-0000-4000-a000-000000000001', '{E685A8D337D36204}Prefabs/Characters/HeadGear/Helmet_PASGT_01/Helmet_PASGT_01_cover_w_goggles.et', 'PASGT Helmet (Cover + Goggles)', 'NATO/Helmet', 'gear_helmet', 21)
ON CONFLICT (modpack_id, resource_name) DO NOTHING;
