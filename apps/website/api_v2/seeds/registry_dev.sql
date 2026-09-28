-- registry_dev.sql
-- Dev seed for the Virtual Arsenal registry catalog. Mirrors the
-- Workbench export (contracts_v2/catalogs/registry-items.workbench.json):
-- 21 real rows across five gear/character kinds (8 character, 4 gear_primary,
-- 3 gear_uniform, 4 gear_vest, 2 gear_helmet), PLUS 4 vehicle rows so the
-- Vehicles tab and ORBAT > Add Vehicle have something to author in a dev DB (the
-- Workbench export carried no vehicle kind, which read as "No placeable vehicles"
-- even after a correct seed). Idempotent and self-contained so `cargo xtask db seed` works
-- WITHOUT mock_data.sql: it upserts the current modpack FK first.
--
-- This file owns the registry the recorded fixture GET__registry.json holds: every row carries a
-- pinned id, so a fresh database answers GET /api/v1/registry with the same ids on every capture.
-- An existing row keeps the id it already has (ON CONFLICT DO NOTHING), so a database seeded
-- before the ids were pinned keeps its generated ones.
--
-- modpack_id = the mock current modpack (mock_data.sql), the modpacks.is_current
-- row used by GET /api/v1/registry's default resolution.

-- Ensure the current modpack exists (FK target). No-op if mock_data.sql already ran. The creation
-- time is the one content_golden.sql pins for the same row.
INSERT INTO modpacks (id, name, version, total_size_bytes, workshop_url, is_current, created_at)
VALUES ('00000000-0000-4000-a000-000000000001', 'Core Modern Expansion', '2.1', 48532275200,
        'https://steamcommunity.com/sharedfiles/filedetails/?id=123456789', true,
        '2026-07-15 09:25:39.281298+00')
ON CONFLICT (id) DO NOTHING;

INSERT INTO registry_items (id, modpack_id, resource_name, display_name, category, kind, sort_order) VALUES
('74045b9b-43e4-4311-b095-d943e462c27b', '00000000-0000-4000-a000-000000000001', '{26A9756790131354}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Rifleman.et', 'US Rifleman', 'NATO/US_Army/Rifleman', 'character', 1),
('e16e4a80-c3d9-4a0b-bfd5-ed3b3697a580', '00000000-0000-4000-a000-000000000001', '{84029128FA6F6BB9}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_GL.et', 'US Grenadier', 'NATO/US_Army/Grenadier', 'character', 2),
('f1af553d-0a2e-407b-8a1b-2b0a4ac2693f', '00000000-0000-4000-a000-000000000001', '{C9E4FEAF5AAC8D8C}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Medic.et', 'US Medic', 'NATO/US_Army/Medic', 'character', 3),
('6ef6f09d-18a9-4542-b679-542d189796c9', '00000000-0000-4000-a000-000000000001', '{5B1996C05B1E51A4}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_AR.et', 'US Automatic Rifleman', 'NATO/US_Army/AutomaticRifleman', 'character', 4),
('eb3c43a7-43d4-47a6-bc38-073c8f38e220', '00000000-0000-4000-a000-000000000001', '{1623EA3AEFACA0E4}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_MG.et', 'US Machine Gunner', 'NATO/US_Army/MachineGunner', 'character', 5),
('1a5147aa-7fb8-485b-a25c-d364c11aa6a2', '00000000-0000-4000-a000-000000000001', '{0B3167BB0FB68110}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_PL.et', 'US Platoon Leader', 'NATO/US_Army/Leadership', 'character', 6),
('ca5c11b2-077c-4054-9673-ca8059282d32', '00000000-0000-4000-a000-000000000001', '{27BF1FF235DD6036}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_LAT.et', 'US Light Anti-Tank', 'NATO/US_Army/AntiTank', 'character', 7),
('3aa8441d-a332-4956-b248-85fde5ca7f10', '00000000-0000-4000-a000-000000000001', '{36CCDB4556ECDA06}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Engineer.et', 'US Engineer', 'NATO/US_Army/Engineer', 'character', 8),
('e7b438f9-b0a7-4897-a82a-84bd69ee1932', '00000000-0000-4000-a000-000000000001', '{3E413771E1834D2F}Prefabs/Weapons/Rifles/M16/Rifle_M16A2.et', 'M16A2', 'NATO/Weapons/Primary', 'gear_primary', 9),
('1af72399-25f4-4ff5-a576-7350f5ff8eb1', '00000000-0000-4000-a000-000000000001', '{5A987A8A13763769}Prefabs/Weapons/Rifles/M16/Rifle_M16A2_M203.et', 'M16A2 + M203', 'NATO/Weapons/Primary', 'gear_primary', 10),
('4c1be39d-3a87-409b-a105-b18172fdaf58', '00000000-0000-4000-a000-000000000001', '{D2B48DEBEF38D7D7}Prefabs/Weapons/MachineGuns/M249/MG_M249.et', 'M249 SAW', 'NATO/Weapons/Primary', 'gear_primary', 11),
('69039884-b4f0-4714-b046-37d2dae817ed', '00000000-0000-4000-a000-000000000001', '{D182DCDD72BF7E34}Prefabs/Weapons/MachineGuns/M60/MG_M60.et', 'M60', 'NATO/Weapons/Primary', 'gear_primary', 12),
('80fb8d34-3126-4136-b2d5-1679f6114e24', '00000000-0000-4000-a000-000000000001', '{C7861F11D5334C0E}Prefabs/Characters/Uniforms/Jacket_US_BDU.et', 'BDU Jacket (Woodland)', 'NATO/Uniform', 'gear_uniform', 13),
('677db8cc-5110-4dd4-8224-2ddb586910b1', '00000000-0000-4000-a000-000000000001', '{3CCA7A9BB4FD3197}Prefabs/Characters/Uniforms/Jacket_US_BDU_rolledup.et', 'BDU Jacket (Rolled)', 'NATO/Uniform', 'gear_uniform', 14),
('00b22323-5481-4c1a-a2ad-ed91ee74254d', '00000000-0000-4000-a000-000000000001', '{604BB72BE8E023C2}Prefabs/Characters/Uniforms/Pants_US_BDU.et', 'BDU Pants (Woodland)', 'NATO/Uniform', 'gear_uniform', 15),
('cd9668a5-60ab-4ef6-83a2-cca502947856', '00000000-0000-4000-a000-000000000001', '{4B57C11AA5161760}Prefabs/Characters/Vests/Vest_PASGT/Vest_PASGT.et', 'PASGT Vest', 'NATO/Vest', 'gear_vest', 16),
('be9bbec4-5f46-4643-b8ce-b8d356e1c8f8', '00000000-0000-4000-a000-000000000001', '{2835A0EA3B79E63E}Prefabs/Characters/Vests/Vest_ALICE/Variants/Vest_ALICE_rifleman.et', 'ALICE Vest (Rifleman)', 'NATO/Vest', 'gear_vest', 17),
('8becee43-ec6b-4e5d-8f0a-b094fe0a1544', '00000000-0000-4000-a000-000000000001', '{156DC7109CEE6F69}Prefabs/Characters/Vests/Vest_ALICE/Variants/Vest_ALICE_AR.et', 'ALICE Vest (Automatic Rifleman)', 'NATO/Vest', 'gear_vest', 18),
('472edbe8-bfe3-46db-bdee-2e209ff4c3c2', '00000000-0000-4000-a000-000000000001', '{725C5E1C75CADAF4}Prefabs/Characters/Vests/Vest_M69/Vest_M69_M81woodland.et', 'M69 Vest (M81 Woodland)', 'NATO/Vest', 'gear_vest', 19),
('cd4bfe05-7712-456a-a706-348b37118898', '00000000-0000-4000-a000-000000000001', '{FE5C49069C2499D9}Prefabs/Characters/HeadGear/Helmet_PASGT_01/Helmet_PASGT_01_cover.et', 'PASGT Helmet (Cover)', 'NATO/Helmet', 'gear_helmet', 20),
('fc92bda6-8f5f-44ad-83ed-119b01a20923', '00000000-0000-4000-a000-000000000001', '{E685A8D337D36204}Prefabs/Characters/HeadGear/Helmet_PASGT_01/Helmet_PASGT_01_cover_w_goggles.et', 'PASGT Helmet (Cover + Goggles)', 'NATO/Helmet', 'gear_helmet', 21),
-- Vehicle rows: without these the Vehicles palette and ORBAT > Add Vehicle
-- have nothing placeable in a dev DB (kind = 'vehicle', so `registry_vehicle_options`
-- and `build_vehicle_catalog_tree` both pick them up; category groups them under the
-- US Army folder like the character rows). Real BLUFOR/US_Army prefab GUIDs.
('00000000-0000-4000-a300-000000000001', '00000000-0000-4000-a000-000000000001', '{86B7B7522A75FF8B}Prefabs/Vehicles/Wheeled/M998/M1025_M2.et', 'M1025 Humvee (M2)', 'NATO/US_Army/Vehicles', 'vehicle', 22),
('00000000-0000-4000-a300-000000000002', '00000000-0000-4000-a000-000000000001', '{FA9635AC08876D3C}Prefabs/Vehicles/Wheeled/M998/M998.et', 'M998 Humvee (Transport)', 'NATO/US_Army/Vehicles', 'vehicle', 23),
('00000000-0000-4000-a300-000000000003', '00000000-0000-4000-a000-000000000001', '{15D63E9F5DE4A83D}Prefabs/Vehicles/Wheeled/M923A1/M923A1.et', 'M923A1 Cargo Truck', 'NATO/US_Army/Vehicles', 'vehicle', 24),
('00000000-0000-4000-a300-000000000004', '00000000-0000-4000-a000-000000000001', '{4D4D74A0BE9E8C1F}Prefabs/Vehicles/Tracked/M113/M113_M2.et', 'M113 APC (M2)', 'NATO/US_Army/Vehicles', 'vehicle', 25)
ON CONFLICT (modpack_id, resource_name) DO NOTHING;
