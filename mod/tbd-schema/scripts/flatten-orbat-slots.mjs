#!/usr/bin/env node
/**
 * Flatten mission ORBAT roles (with count) into slots[] with stable ids and placeholder coords.
 * Usage: node flatten-orbat-slots.mjs path/to/mission.json [--in-place]
 */
import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

const args = process.argv.slice(2);
const inPlace = args.includes("--in-place");
const pathArg = args.find((a) => !a.startsWith("--"));
if (!pathArg) {
  console.error("usage: flatten-orbat-slots.mjs <mission.json> [--in-place]");
  process.exit(2);
}

const file = resolve(pathArg);
const mission = JSON.parse(readFileSync(file, "utf8"));

/** @type {Record<string, {x:number,z:number}>} */
const anchors = {};
for (const zone of mission.zones || []) {
  if (zone.type === "spawn" && zone.faction && zone.shape?.circle) {
    anchors[zone.faction] = { x: zone.shape.circle.x, z: zone.shape.circle.z };
  }
}
if (!anchors.blufor) anchors.blufor = { x: 4831.2, z: 6620.8 };
if (!anchors.opfor) anchors.opfor = { x: 6010.0, z: 7211.5 };

/** @type {Array<object>} */
const slots = [];
let slotIndex = 0;

for (const [factionKey, factionOrbat] of Object.entries(mission.orbat || {})) {
  const anchor = anchors[factionKey] || { x: 6400, z: 6400 };
  for (const group of factionOrbat.groups || []) {
    for (const role of group.roles || []) {
      for (let i = 0; i < role.count; i++) {
        const ring = Math.floor(slotIndex / 8);
        const posInRing = slotIndex % 8;
        const angle = (posInRing / 8) * Math.PI * 2;
        const radius = 8 + ring * 6;
        const x = anchor.x + Math.cos(angle) * radius;
        const z = anchor.z + Math.sin(angle) * radius;
        const headingDeg = ((Math.atan2(anchor.x - x, anchor.z - z) * 180) / Math.PI + 360) % 360;
        slots.push({
          id: `${factionKey}:${group.callsign}:${role.slot}:${i}`,
          faction: factionKey,
          groupCallsign: group.callsign,
          role: role.slot,
          kit: role.kit,
          x: Math.round(x * 10) / 10,
          z: Math.round(z * 10) / 10,
          headingDeg: Math.round(headingDeg),
        });
        slotIndex++;
      }
    }
  }
}

mission.schemaVersion = "1.1";
mission.slots = slots;

const out = JSON.stringify(mission, null, 2) + "\n";
if (inPlace) {
  writeFileSync(file, out);
  console.log(`Wrote ${slots.length} slots to ${file}`);
} else {
  process.stdout.write(out);
}
