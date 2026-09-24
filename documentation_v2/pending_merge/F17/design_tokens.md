# Aegis Tactical Design Tokens Specification

Authoritative specification for the platform visual design system, matching `apps/website/frontend/style/aegis.css`.

---

## 1. Core Visual Laws

- **Dark-Only Invariant**: The application enforces a single, authoritative tactical dark theme. Light mode is forbidden; `class="dark"` is permanently set on `<html>`.
- **T-794 Layout Shift Law**: Entrance animations (`dialog-in`, `menu-in`, popovers) must **never** animate `transform`, `translate`, or `scale`. Animations must strictly animate `opacity` to prevent sub-pixel layout shifts and visual jitter during canvas interactions.
- **Glassmorphism Discipline**: Tonal stacking uses semi-transparent backgrounds with `backdrop-filter: blur(16px)` and subtle borders (`rgba(255, 255, 255, 0.08)`).

---

## 2. Color Palette Tokens

### Surface Elevations (Midnight Navy)
| Token | Hex Value | Semantic Usage |
|---|---|---|
| `--surface-floor` | `#0d1322` | Viewport background floor |
| `--surface-canvas` | `#080e1d` | Deep 2D/3D CAD canvas backdrop |
| `--surface-base` | `#0b1120` | Form inputs, recessed containers |
| `--surface-card` | `#191f2f` | Standard cards, panels, list items |
| `--surface-raised` | `#242a3a` | Hover states, raised panels, popovers |
| `--surface-overlay`| `#2f3445` | Modal dialog sheets, floating toolbars |

### Brand & Interactive Accents
| Token | Hex Value | Semantic Usage |
|---|---|---|
| `--primary` | `#3b82f6` | Primary action buttons, active selections, focus rings |
| `--primary-hover` | `#2563eb` | Hover state for primary interactive elements |
| `--accent-nav` | `#adc6ff` | Active navigation item indicator and breadcrumb accent |
| `--on-primary` | `#ffffff` | Foreground text on primary action elements |

### Semantic State Tokens
| Token | Hex Value | Semantic Usage |
|---|---|---|
| `--success` | `#22c55e` | Confirmation, healthy server status, BLUFOR/Independent |
| `--warning` | `#eab308` | Warnings, countdown timers, pending reviews |
| `--destructive`| `#ef4444` | Errors, deletions, OPFOR hostile indicators |
| `--border-subtle`| `#374151` | Default container border |

---

## 3. Typography Scale

Fonts: **Inter** (Primary Sans) and **JetBrains Mono** (Tactical Monospace).

| Level | Size | Line Height | Weight | Usage |
|---|---|---|---|---|
| `headline-lg` | 30px | 38px | 700 | Main page hero titles |
| `headline-md` | 24px | 32px | 600 | Section headers, modal titles |
| `headline-sm` | 20px | 28px | 600 | Card titles, panel headers |
| `body-lg` | 18px | 28px | 400 | Introductory summaries |
| `body-md` | 16px | 24px | 400 | Standard body paragraphs |
| `label-md` | 14px | 20px | 500 | Form labels, button text |
| `label-sm` | 12px | 16px | 600 | Uppercase badges, status tags |
| `code-md` | 14px | 20px | 400 | MGRS coordinates, telemetry, IDs |

---

## 4. Spacing & Geometry
- **Base Unit**: 4px (`0.25rem`)
- **Container Radius**: `radius-md` = `6px` (`0.375rem`). Applied uniformly across buttons, inputs, and cards.
- **Top Navigation Height**: `64px` (`4rem`).
- **Sidebar Width**: `320px` (`20rem`).
