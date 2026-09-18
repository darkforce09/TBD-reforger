-- T-263 — Vehicle Database / IFF seed for `cargo xtask db seed`.
-- Rows match the content_golden §5 vehicle_databases block so a fresh DB has
-- the same IFF table the GET /vehicle-database golden was captured against.
-- Idempotent on primary key.

INSERT INTO vehicle_databases (id, name, faction, armor_type, amphibious, primary_threat,
                               profile_image_url)
VALUES
  ('00000000-0000-4000-3000-000000000001', 'BTR-70', 'USSR', 'Light Armour', 'Yes',
   'Autocannon — 14.5 mm KPVT', 'https://cdn.tbd-reforger.example/iff/btr70.png'),
  ('00000000-0000-4000-3000-000000000002', 'M113A3', 'US Army', 'Light Armour', 'No',
   'Heavy MG — M2 .50 cal', 'https://cdn.tbd-reforger.example/iff/m113a3.png'),
  ('00000000-0000-4000-3000-000000000003', 'UAZ-469', 'USSR', 'Unarmoured', 'No',
   'Small arms only', NULL),
  ('00000000-0000-4000-3000-000000000004', 'M998 Humvee', 'US Army', 'Unarmoured', 'No',
   'Small arms only', 'https://cdn.tbd-reforger.example/iff/m998.png'),
  ('00000000-0000-4000-3000-000000000005', 'Mi-8MT', 'USSR', 'Rotary — Transport', NULL,
   'Door guns — 7.62 mm', 'https://cdn.tbd-reforger.example/iff/mi8mt.png'),
  -- Every optional column empty: amphibious, threat and image all drop out.
  ('00000000-0000-4000-3000-000000000006', 'S105 Sedan', 'Civilian', 'Unarmoured', NULL, NULL, NULL)
ON CONFLICT (id) DO UPDATE SET
    name = EXCLUDED.name, faction = EXCLUDED.faction, armor_type = EXCLUDED.armor_type,
    amphibious = EXCLUDED.amphibious, primary_threat = EXCLUDED.primary_threat,
    profile_image_url = EXCLUDED.profile_image_url;
