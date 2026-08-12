-- faction_library.sql — T-256 starter faction library for a fresh install.
--
-- `cargo xtask db seed` previously only applied discord_roles + registry_dev, so
-- user_factions stayed empty and Load Predefined ORBAT had nothing to show
-- until an operator hand-built a row. This seed inserts BLUFOR + OPFOR docs
-- owned by the local Dev Operator (same discord_id as handlers/dev.rs
-- DEV_USER_ID / content_golden §11), so GET /api/v1/factions is non-empty
-- after the first `cargo xtask db seed` + `dev-login`.
--
-- Companion JSON (for humans / schema checks; SQL embeds the same bytes):
--   seeds/faction_library.blufor.json
--   packages/tbd-schema/registry/faction-library.sample.json  (OPFOR golden)
--
-- Idempotent on (owner_id, name). Does NOT require the users row to exist
-- first (user_factions.owner_id has no FK); rows become visible once
-- /auth/dev-login upserts that snowflake.

INSERT INTO user_factions (owner_id, side, name, doc)
VALUES
  (
    '000000000000000001',
    'BLUFOR',
    'US Army 1980s',
    '{
      "side": "BLUFOR",
      "name": "US Army 1980s",
      "roles": [
        {
          "role": "Squad Leader",
          "tag": "SL",
          "character": "{0B3167BB0FB68110}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_PL.et",
          "loadout": {
            "version": 2,
            "wear": {
              "headCover": "{FE5C49069C2499D9}Prefabs/Characters/HeadGear/Helmet_PASGT_01/Helmet_PASGT_01_cover.et",
              "jacket": "{C7861F11D5334C0E}Prefabs/Characters/Uniforms/Jacket_US_BDU.et",
              "pants": "{604BB72BE8E023C2}Prefabs/Characters/Uniforms/Pants_US_BDU.et",
              "boots": null,
              "vest": "{2835A0EA3B79E63E}Prefabs/Characters/Vests/Vest_ALICE/Variants/Vest_ALICE_rifleman.et",
              "armoredVest": "{4B57C11AA5161760}Prefabs/Characters/Vests/Vest_PASGT/Vest_PASGT.et",
              "backpack": null,
              "handwear": null
            },
            "weapons": [
              {
                "slotIndex": 0,
                "slotType": "primary",
                "weapon": "{3E413771E1834D2F}Prefabs/Weapons/Rifles/M16/Rifle_M16A2.et",
                "optic": null,
                "magazine": null,
                "attachments": []
              }
            ],
            "summary": "M16A2"
          }
        },
        {
          "role": "Grenadier",
          "tag": "GL",
          "character": "{84029128FA6F6BB9}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_GL.et"
        },
        {
          "role": "Combat Medic",
          "tag": "MED",
          "character": "{C9E4FEAF5AAC8D8C}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Medic.et"
        },
        {
          "role": "Automatic Rifleman",
          "tag": "AR",
          "character": "{5B1996C05B1E51A4}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_AR.et"
        },
        {
          "role": "Rifleman",
          "character": "{26A9756790131354}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Rifleman.et"
        }
      ],
      "vehicles": [
        {
          "vehicle": "{F649585ABB3706C4}Prefabs/Vehicles/Wheeled/M151A2/M151A2.et",
          "label": "M151A2"
        },
        {
          "vehicle": "{4A71F755A4513227}Prefabs/Vehicles/Wheeled/M998/M1025.et",
          "label": "M1025 Humvee"
        }
      ]
    }'::jsonb
  ),
  (
    '000000000000000001',
    'OPFOR',
    'Soviet Army 1980s',
    '{
      "side": "OPFOR",
      "name": "Soviet Army 1980s",
      "roles": [
        {
          "role": "Squad Leader",
          "tag": "SL",
          "character": "{5436629450D8387A}Prefabs/Characters/Factions/OPFOR/USSR_Army/Character_USSR_SL.et",
          "loadout": {
            "version": 2,
            "wear": {
              "headCover": null,
              "jacket": "{9F546CCA2582D16F}Prefabs/Characters/Uniforms/Jacket_M88.et",
              "pants": "{DCF980831E880F6A}Prefabs/Characters/Uniforms/Pants_M88.et",
              "boots": null,
              "vest": "{9713FE6DDCC9510D}Prefabs/Characters/Vests/Vest_Lifchik/Vest_Lifchik.et",
              "armoredVest": "{ADE19B33DCBB9005}Prefabs/Characters/Vests/Vest_6B2/Vest_6B2.et",
              "backpack": null,
              "handwear": null
            },
            "weapons": [
              {
                "slotIndex": 0,
                "slotType": "primary",
                "weapon": "{FA5C25BF66A53DCF}Prefabs/Weapons/Rifles/AK74/Rifle_AK74.et",
                "optic": null,
                "magazine": null,
                "attachments": []
              },
              {
                "slotIndex": 3,
                "slotType": "grenade",
                "weapon": "{645C73791ECA1698}Prefabs/Weapons/Grenades/Grenade_RGD5.et"
              }
            ],
            "summary": "AK-74 · RGD-5"
          }
        },
        {
          "role": "Rifleman",
          "tag": "AT",
          "character": "{1C78331E156A3D65}Prefabs/Characters/Factions/OPFOR/USSR_Army/Character_USSR_AT.et"
        }
      ],
      "vehicles": [
        {
          "vehicle": "{259EE7B78C51B624}Prefabs/Vehicles/Wheeled/UAZ469/UAZ469.et",
          "label": "UAZ-469"
        },
        {
          "vehicle": "{C012BB3488BEA0C2}Prefabs/Vehicles/Wheeled/BTR70/BTR70.et",
          "label": "BTR-70"
        }
      ]
    }'::jsonb
  )
ON CONFLICT (owner_id, name) DO UPDATE
  SET side       = EXCLUDED.side,
      doc        = EXCLUDED.doc,
      updated_at = now();
