# Ticket Artifacts Hub (`tickets/`)

The `tickets/` directory contains living technical specifications and four-section implementation plans referenced directly by `.ai/tickets/*.toml` ticket records.

---

## 1. Directory Structure

Both subdirectories are organized as **single, flat directories with zero subfolders**, providing unambiguous, predictable paths for developers, agents, and tooling:

```text
documentation_v2/tickets/
├── README.md                                 <-- Ticket artifact architecture & authoring guide
├── specs/                                    <-- Single flat folder: all ticket specifications & RFCs
│   ├── README.md
│   ├── ROADMAP.md
│   ├── gap_analysis.md
│   └── t*.md (all 270+ specification files directly in root)
└── plans/                                    <-- Single flat folder: all ticket implementation plans
    ├── README.md
    ├── TEMPLATE.md                           <-- Canonical 4-section implementation plan template
    └── t-*_plan.md (all 206 implementation plans directly in root)
```

---

## 2. Path Symmetry & Determinism

| Artifact Type | Storage Directory | Ticket TOML Citation Pattern |
|---|---|---|
| **Technical Specifications** | `tickets/specs/` | `spec = "documentation_v2/tickets/specs/<name>.md"` |
| **Implementation Plans** | `tickets/plans/` | `plan = "documentation_v2/tickets/plans/t-<id>_plan.md"` |

This flat symmetry eliminates arbitrary subfolders (`existing/`, `ideas/`, `factory/`, `Mission_Creator_Architecture/`), ensuring that neither developers nor automated tooling need to navigate fragmented folder trees.

---

## 3. Code & Tooling Mapping
- **Crate**: `tools_v2/ticket-engine/` (Domain model, parser, `ops::default_plan_path`)
- **Desktop Viewer**: `apps/ticketboard/` (In-memory ticket index, reactive inotify file watching, in-app spec reader)
- **Validation**: `tools_v2/xtask/src/check.rs` (Ready-gate four-section check, on-disk existence assertions)
- **Ticket Registry**: `.ai/tickets/*.toml`
