# T-068.9 — Claude Code handoff (Registry worker + compat ingest)

**Spec (wins):** [`t068_9_registry_worker_ingest.md`](../../docs/specs/Mission_Creator_Architecture/t068_9_registry_worker_ingest.md)
· **Upstream:** T-150 @ `9107bf4e` · verify [`t150_verify_log.md`](t150_verify_log.md)
· **CWD:** repo root `main`.

## Operator note

Ingest the committed Workbench envelopes (1,880 items / 4,012 edges). Do **not** re-export
unless the sample files are missing. `ammo_in_mag` stays empty (T-150 OPEN).

## What you build

Postgres ingest + API + Comlink worker (`canEquip` / `canAttach`) over T-150 graph.

## Do not

Docs/registry/CLAUDE · Forge UI · invent edges · recreate compat schema.

## Return

SHA + tag **T-068.9** · verify log · import counts.
