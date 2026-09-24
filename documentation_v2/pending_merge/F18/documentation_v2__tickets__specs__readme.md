# Ticket Specifications Repository (`tickets/specs/`)

This directory houses all authoritative technical specifications referenced by ticket records via the `spec = "..."` attribute in `.ai/tickets/*.toml`.

---

## 1. Architectural Invariant: Single Flat Repository (No Subfolders)

To eliminate the arbitrary and fragmented categorization of the legacy architecture (where specs were scattered across `existing/`, `ideas/`, `factory/`, `audit_2026_09/`, `website_reorg/`, and `Mission_Creator_Architecture/`), **all specifications reside directly in the root of `specs/` with zero subfolders**.

### Rationale:
1. **Deterministic Path Resolution**: Every ticket specification has an unambiguous, predictable path:
   `spec = "documentation/tickets/specs/<specification_name>.md"`
   Developers and AI agents never have to search across multiple nested subdirectories to locate or reference a spec.
2. **Symmetry with Implementation Plans**: Mirrors the flat structure of `tickets/plans/`, where all 206 implementation plans reside directly in the root without arbitrary grouping.
3. **Elimination of Arbitrary Buckets**: Subdirectories like `factory/` (which only contained 2 files) or `existing/` (a temporal artifact) created artificial boundaries. All specifications represent active technical requirements.
4. **Collision-Free Namespace**: An exhaustive audit confirmed zero filename collisions among all specification files across the monorepo.

---

## 2. Specification File Inventory & Naming Conventions

Specifications follow clear, self-describing naming patterns:
- **Slice & Ticket Specifications**: `t<ticket_id>_<descriptive_slug>.md` (e.g. `t052_undo_shortcuts.md`, `t165_node_eradication.md`, `t853_shell_to_xtask_waves.md`).
- **Feature & System Overviews**: Descriptive lowercase snake_case or canonical names (e.g. `ROADMAP.md`, `gap_analysis.md`, `engineering_plan.md`, `feature_inventory.md`).

---

## 3. Code & Tooling Linkages

- **Ticket Record Attribute**: `.ai/tickets/*.toml` declares `spec = "documentation/tickets/specs/<name>.md"`.
- **Integrity Validation**: `cargo xtask ticket check` verifies on-disk existence by joining the repository root with `spec`.
- **Desktop Ticketboard**: `apps/ticketboard/` watches `documentation/tickets/specs/ROADMAP.md` and displays in-app spec previews using `egui_commonmark`.
