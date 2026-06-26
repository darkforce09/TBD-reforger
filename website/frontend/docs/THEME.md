# TBD Reforger — Unified Design Tokens

Authoritative theme for all docs, Tailwind v4 `@theme`, and shadcn CSS variables.

**Aegis reference:** full token spec in [docs/specs/Mission_Creator_Mock_Up/aegis_tokens/DESIGN.md](../../docs/specs/Mission_Creator_Mock_Up/aegis_tokens/DESIGN.md). Runtime tokens live in `frontend/src/index.css` (`@theme` block).

## Principles

- **Single primary blue:** `#3b82f6` for buttons, nav accents, icons, links, progress bars, focus rings
- **No `#adc6ff`:** Stitch HTML used light blue for nav; map to `primary` or `primary/10`–`primary/30`
- **Dark only:** `class="dark"` on `<html>`; no light mode toggle
- **Times:** API UTC → display in user local timezone with abbreviated TZ label (`date-fns`)

## Stitch → Unified mapping

| Stitch (old) | Unified token |
|--------------|---------------|
| `#adc6ff` text-primary | `text-primary` (`#3b82f6`) |
| `primary-container` glow | `bg-primary/10` or `bg-primary/20` |
| `shadow-glow-*` rgba(173,198,255,…) | `rgba(59, 130, 246, …)` |
| OPS CENTER branding | **TBD Reforger** |
| Section emojis | Removed — Material Symbols only |

## Colors

| Token | Hex / value | Usage |
|-------|-------------|--------|
| `primary` | `#3b82f6` | CTAs, active nav, icons, links, progress fill |
| `on-primary` | `#ffffff` | Text on primary buttons |
| `secondary` | `#1f2937` | Card/panel backgrounds |
| `surface` | `#0b1120` | App canvas, input backgrounds |
| `background` | `#0d1322` | Page floor |
| `surface-dim` | `#0d1322` | Sidebar base |
| `surface-container-lowest` | `#080e1d` | Deepest layer |
| `surface-container-low` | `#151b2b` | |
| `surface-container` | `#191f2f` | Cards |
| `surface-container-high` | `#242a3a` | Raised cards |
| `surface-container-highest` | `#2f3445` | Pills, badges |
| `on-surface` | `#ffffff` | Primary text |
| `on-surface-variant` | `#c2c6d6` | Muted text, labels |
| `border-subtle` | `#374151` | 1px borders |
| `outline` | `#8c909f` | Secondary borders |
| `outline-variant` | `#424754` | |
| `success` | `#22c55e` | Online, linked state |
| `success-muted` | `#064e3b` | Success backgrounds |
| `warning` | `#eab308` | Caution |
| `error` | `#ef4444` | Destructive |
| `error-muted` | `#f87171` | Error text (Stitch) |

## Typography

| Token | Font | Size | Weight | Line height | Usage |
|-------|------|------|--------|-------------|--------|
| `headline-lg` | Inter | 30px | 700 | 38px | Page H1 |
| `headline-md` | Inter | 24px | 600 | 32px | Section H2 |
| `headline-sm` | Inter | 20px | 600 | 28px | Card H3 |
| `body-md` | Inter | 16px | 400 | 24px | Body |
| `label-md` | Inter | 14px | 500 | 20px | Buttons, form labels |
| `label-sm` | Inter | 12px | 600 | 16px | Uppercase metadata, table headers |
| `code-md` | JetBrains Mono | 14px | 400 | 20px | IP, uptime, IDs |
| `nav-breadcrumb-parent` | Inter | 14px | 400 | 20px | Breadcrumb parent |
| `nav-breadcrumb-current` | Inter | 14px | 600 | 20px | Breadcrumb current |
| `menu-item` | Inter | 14px | 500 | 20px | Nav links, dropdown |

## Spacing

| Token | Value |
|-------|-------|
| `navbar-height` | 64px |
| `container-padding` | 1.5rem (24px) |
| `element-gap` | 1rem |
| `gutter` | 1.5rem |
| `stack-sm` | 0.5rem |
| `stack-md` | 1rem |
| `stack-lg` | 2rem |
| `container-max` | 1440px |
| `sidebar-width` | 320px |

## Radii & shadows

| Token | Value |
|-------|-------|
| `radius-sm` | 0.125rem |
| `radius-md` | 0.375rem (6px) — standard for buttons, inputs, cards |
| `radius-lg` | 0.5rem |
| `radius-xl` | 0.75rem |
| `shadow-glow-primary` | `0 0 15px rgba(59, 130, 246, 0.5)` |
| `shadow-glow-active` | `inset 0 0 10px rgba(59, 130, 246, 0.3), 0 0 15px rgba(59, 130, 246, 0.2)` |

## Fonts

- **Sans:** Inter (`@fontsource/inter` weights 400, 500, 600, 700)
- **Mono:** JetBrains Mono (`@fontsource/jetbrains-mono` 400)
- **Icons:** Material Symbols Outlined (Google Fonts link in `index.html`)

## Shell utilities (index.css)

- `.sidebar-container` — gradient + grid texture
- `.nav-item-active` — primary gradient highlight + borders
- `.custom-scrollbar` — 4px sidebar scrollbar
- `.pulse-dot` — online indicator animation

## shadcn CSS variable mapping

```css
--background: #0d1322;
--foreground: #ffffff;
--primary: #3b82f6;
--primary-foreground: #ffffff;
--secondary: #1f2937;
--destructive: #ef4444;
--border: #374151;
--ring: #3b82f6;
```

## Time display

- Store and receive ISO 8601 UTC from API
- Format with `date-fns` `format()` in local timezone
- Append short TZ: `EEE HH:mm zzz` (e.g. `Tue 20:00 EDT`)
- Countdowns: `differenceInSeconds` from UTC `start_time` to now
