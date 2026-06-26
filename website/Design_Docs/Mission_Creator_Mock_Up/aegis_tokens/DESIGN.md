---
name: Aegis Tactical Command
colors:
  surface: '#0d1322'
  surface-dim: '#0d1322'
  surface-bright: '#333949'
  surface-container-lowest: '#080e1d'
  surface-container-low: '#151b2b'
  surface-container: '#191f2f'
  surface-container-high: '#242a3a'
  surface-container-highest: '#2f3445'
  on-surface: '#dde2f7'
  on-surface-variant: '#c4c6d0'
  inverse-surface: '#dde2f7'
  inverse-on-surface: '#2a3040'
  outline: '#8e909a'
  outline-variant: '#44474f'
  surface-tint: '#adc6ff'
  primary: '#d8e2ff'
  on-primary: '#122f5f'
  primary-container: '#adc6ff'
  on-primary-container: '#385283'
  inverse-primary: '#455e90'
  secondary: '#adc6ff'
  on-secondary: '#002e6a'
  secondary-container: '#0566d9'
  on-secondary-container: '#e6ecff'
  tertiary: '#c3e7ff'
  on-tertiary: '#00344a'
  tertiary-container: '#7bd0ff'
  on-tertiary-container: '#005979'
  error: '#ffb4ab'
  on-error: '#690005'
  error-container: '#93000a'
  on-error-container: '#ffdad6'
  primary-fixed: '#d8e2ff'
  primary-fixed-dim: '#adc6ff'
  on-primary-fixed: '#001a42'
  on-primary-fixed-variant: '#2c4677'
  secondary-fixed: '#d8e2ff'
  secondary-fixed-dim: '#adc6ff'
  on-secondary-fixed: '#001a42'
  on-secondary-fixed-variant: '#004395'
  tertiary-fixed: '#c4e7ff'
  tertiary-fixed-dim: '#7cd0ff'
  on-tertiary-fixed: '#001e2c'
  on-tertiary-fixed-variant: '#004c69'
  background: '#0d1322'
  on-background: '#dde2f7'
  surface-variant: '#2f3445'
  surface-glass: rgba(31, 41, 55, 0.7)
  border-subtle: '#374151'
  success-muted: '#064e3b'
  error-alert: '#f87171'
  tactical-yellow: '#facc15'
typography:
  headline-lg:
    fontFamily: Inter
    fontSize: 30px
    fontWeight: '700'
    lineHeight: 38px
    letterSpacing: -0.02em
  headline-md:
    fontFamily: Inter
    fontSize: 24px
    fontWeight: '600'
    lineHeight: 32px
    letterSpacing: -0.01em
  headline-sm:
    fontFamily: Inter
    fontSize: 20px
    fontWeight: '600'
    lineHeight: 28px
  body-lg:
    fontFamily: Inter
    fontSize: 18px
    fontWeight: '400'
    lineHeight: 28px
  body-md:
    fontFamily: Inter
    fontSize: 16px
    fontWeight: '400'
    lineHeight: 24px
  label-md:
    fontFamily: Inter
    fontSize: 14px
    fontWeight: '500'
    lineHeight: 20px
    letterSpacing: 0.01em
  label-sm:
    fontFamily: Inter
    fontSize: 12px
    fontWeight: '600'
    lineHeight: 16px
    letterSpacing: 0.05em
  code-md:
    fontFamily: JetBrains Mono
    fontSize: 14px
    fontWeight: '400'
    lineHeight: 20px
rounded:
  sm: 0.125rem
  DEFAULT: 0.25rem
  md: 0.375rem
  lg: 0.5rem
  xl: 0.75rem
  full: 9999px
spacing:
  gutter: 1.5rem
  stack-lg: 2rem
  stack-md: 1rem
  stack-sm: 0.5rem
  margin-mobile: 1rem
  container-max: 1440px
---

## Brand & Style

Aegis Tactical Command is a high-fidelity, mission-critical interface designed for complex simulation and strategic planning. The brand personality is **utilitarian, precise, and authoritative**, evoking the atmosphere of a modern military situational awareness display.

The visual style is a sophisticated blend of **Glassmorphism** and **Technical Minimalism**. It prioritizes information density and clarity through:
- **Translucent Layering:** Semi-transparent "glass" panels with high-intensity backdrop blurs to maintain context over spatial maps.
- **Micro-interactions:** Fine-lined borders and sharp, intentional spacing.
- **Tactical Aesthetic:** A palette of deep nocturnal blues and vibrant cybernetic accents that suggest low-light operational environments.

## Colors

The color system is built on a "Midnight Navy" foundation to minimize eye strain during long-form planning sessions.

- **Primary (#adc6ff):** A high-visibility, desaturated blue used for active states, primary iconography, and critical text headings.
- **Secondary (#3b82f6):** A vibrant "Action Blue" reserved for high-priority global triggers like "Deploy" or "Save."
- **Tertiary (#7bd0ff):** Used for informational accents and secondary system icons to differentiate from primary navigation.
- **Neutral (#0d1322):** Deep space backgrounds that provide maximum contrast for technical overlays.

Semantic colors are strictly applied: **Tactical Yellow** for group identities, **Error Alert** for destructive actions, and **Success Muted** for confirmed system states.

## Typography

The system utilizes **Inter** for all UI and administrative text to ensure maximum legibility at small sizes. **JetBrains Mono** is introduced for telemetry, coordinates, and property values to provide a distinctive technical feel and prevent character confusion in numerical data.

- **Headlines:** Use tight tracking and heavy weights for a disciplined, structural look.
- **Labels:** Small labels (`label-sm`) always use uppercase with increased letter spacing for a "data-tag" aesthetic.
- **Functional Text:** Code-style mono fonts are used for dynamic values that update in real-time (e.g., timers, rotation degrees).

## Layout & Spacing

The layout follows a **Fixed-Sidebar Fluid-Center** model designed for widescreen mission-editing desktops.

- **Sidebars:** Fixed-width anchors (Left: 256px, Right: 320px) that house navigation and asset libraries.
- **Workspace:** A central fluid area that displays the topographical map or primary visual canvas.
- **Grid:** A systematic 8px linear scale (4px increments for micro-spacing).
- **Floating Overlays:** The bottom property panel is centered horizontally and detached from the screen edges, creating a "HUD" (Heads Up Display) feel.

## Elevation & Depth

Hierarchy is established through **Backdrop Blur** and **Tonal Stacking** rather than traditional drop shadows.

- **Base Layer:** The tactical map (dark, complex).
- **Surface Layer:** Glass panels with `16px` blur and `rgba(255, 255, 255, 0.1)` borders. These represent the primary interaction containers.
- **Raised Layer:** Floating panels (like the bottom properties bar) use a high-contrast border-top in `tertiary` color to simulate a glowing edge or light source from above.
- **Shadows:** Use `shadow-xl` or `shadow-2xl` with low opacity (10-20%) to subtly lift panels without muddying the dark background.

## Shapes

The shape language is **Technical and Compact**.
- **Base Rounding:** 0.125rem (2px) for buttons and inputs, providing a sharp, precise edge.
- **Container Rounding:** 0.5rem (8px) for major panels (sidebars, HUDs) to soften the overall silhouette of the complex interface.
- **Active Indicators:** Vertical bar indicators (2px width) are used on the left edge of selected tree items to show focus without occupying significant horizontal space.

## Components

### Buttons
- **Primary:** Solid `#3b82f6` with white text. High contrast for final actions.
- **Ghost/Outline:** Thin `primary` borders with transparent backgrounds for navigation and utility.
- **Icon Buttons:** Fixed size (32px or 40px) with `on-surface-variant` colors that transition to `primary` on hover.

### Inputs & Controls
- **Tactical Select:** Dark background (`surface-container`) with a 1px `border-subtle`. Focus states use a thin `primary` ring.
- **Range Sliders:** Thin tracks with high-contrast `primary` accents for the "filled" portion.
- **Tree View:** Nodes use `material-symbols` for expand/collapse. Hover states use `surface-variant/50`.

### Asset Cards
- **Square Grid:** Fixed-aspect ratio cards for 3D assets. Feature a centered icon and a bottom-aligned label in `label-sm`. Border highlights on hover simulate "selection lock."

### Panels
- **HUD Header:** Every panel must have a header with an icon, title in `label-md` (uppercase), and a close or filter utility button.