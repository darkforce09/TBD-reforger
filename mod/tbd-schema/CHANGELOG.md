# TBD Schema Changelog

## 1.1.0 — 2026-06-14

- Mission `schemaVersion` **1.1**: required `slots[]` array with per-player spawn positions (flattened ORBAT instances).
- ORBAT slot count validation against `slots[]` length.
- No faction spawn zones — spawn is driven entirely by `slots[]` coordinates.
- Helpers: `scripts/flatten-orbat-slots.mjs`, `scripts/validate-file.mjs`.

## 1.0.0 — 2026-06-13 (frozen)

Initial Mission JSON v1, registry format, VOIP bridge message schema, and two golden missions.

Breaking changes after 1.0.0 require an RFC and a new `schemaVersion`.
