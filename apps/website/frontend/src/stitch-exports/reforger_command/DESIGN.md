---
name: Reforger Command
colors:
  surface: '#0d1322'
  surface-dim: '#0d1322'
  surface-bright: '#33394a'
  surface-container-lowest: '#080e1d'
  surface-container-low: '#151b2b'
  surface-container: '#191f2f'
  surface-container-high: '#242a3a'
  surface-container-highest: '#2f3445'
  on-surface: '#ffffff'
  on-surface-variant: '#c2c6d6'
  inverse-surface: '#dde2f8'
  inverse-on-surface: '#2a3040'
  outline: '#8c909f'
  outline-variant: '#424754'
  surface-tint: '#adc6ff'
  primary: '#adc6ff'
  on-primary: '#002e6a'
  primary-container: '#4d8eff'
  on-primary-container: '#00285d'
  inverse-primary: '#005ac2'
  secondary: '#bdc7d9'
  on-secondary: '#27313f'
  secondary-container: '#404a59'
  on-secondary-container: '#afb9cb'
  tertiary: '#7bd0ff'
  on-tertiary: '#00354a'
  tertiary-container: '#009bd1'
  on-tertiary-container: '#002d40'
  error: '#f87171'
  on-error: '#690005'
  error-container: '#93000a'
  on-error-container: '#ffdad6'
  primary-fixed: '#d8e2ff'
  primary-fixed-dim: '#adc6ff'
  on-primary-fixed: '#001a42'
  on-primary-fixed-variant: '#004395'
  secondary-fixed: '#d9e3f6'
  secondary-fixed-dim: '#bdc7d9'
  on-secondary-fixed: '#121c2a'
  on-secondary-fixed-variant: '#3d4756'
  tertiary-fixed: '#c4e7ff'
  tertiary-fixed-dim: '#7bd0ff'
  on-tertiary-fixed: '#001e2c'
  on-tertiary-fixed-variant: '#004c69'
  background: '#0d1322'
  on-background: '#dde2f8'
  surface-variant: '#2f3445'
  border-subtle: '#374151'
  success-muted: '#064e3b'
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
  container-max: 1440px
  gutter: 1.5rem
  margin-mobile: 1rem
  stack-sm: 0.5rem
  stack-md: 1rem
  stack-lg: 2rem
---

## Brand & Style

The design system is engineered for high-stakes operational management within the PC gaming community. It adopts a **Corporate / Modern** aesthetic with a lean toward **Technical Minimalist** traits to serve power users who require data density without cognitive overload.

The interface evokes a "command center" atmosphere—serious, precise, and utilitarian. By leveraging a deep-space background and high-contrast interactive elements, the design ensures that critical administrative actions are unmistakable. The style prioritizes functional clarity over decorative flourish, using rigid structural alignment and a restricted color palette to maintain focus during complex event orchestration.

## Colors

The palette is anchored in a "Deep Night" surface to reduce eye strain during long administrative sessions. 

- **Primary (#3b82f6)**: Reserved strictly for high-priority global actions and active navigation states.
- **Secondary (#1f2937)**: Used for containers, panels, and component backgrounds to create subtle depth against the surface.
- **Tertiary (#38bdf8)**: Applied to borders and accents to define interactive zones without the weight of a solid fill.
- **Surface (#0b1120)**: The base canvas for the entire application.
- **Error (#f87171)**: Used for destructive actions (e.g., Delete, Kick) and validation states.

Avoid using pure white for anything other than text and small icons to maintain the dark-mode integrity.

## Typography

This design system utilizes **Inter** for all UI elements to ensure maximum legibility at small sizes. **JetBrains Mono** is introduced specifically for technical data, JSON inputs, and unique identifiers to differentiate "system data" from "interface labels."

- **Headlines**: Bold weights with tight letter-spacing for a strong, authoritative hierarchy.
- **Body**: Regular weight for readability.
- **Labels**: Medium weight, used for buttons and form headers.
- **Caps/Small Labels**: Used for table headers and metadata categories to create clear scan lines in dense views.

## Layout & Spacing

The system uses a **Fluid Grid** model optimized for desktop management but responsive enough for tablet monitoring. 

- **Desktop (1280px+)**: 12-column grid with 24px gutters. Use fixed sidebars for primary navigation and scrolling central areas for data tables.
- **Tablet (768px - 1279px)**: 8-column grid with 20px gutters. Sidebars collapse into icons or a hamburger menu.
- **Mobile (< 767px)**: 4-column grid with 16px margins. Tables must reflow into stacked card views.

Spacing follows a strict 4px/8px base-2 rhythm to maintain mathematical harmony in high-density layouts. Use `stack-lg` for separating major sections and `stack-sm` for grouping labels with their respective inputs.

## Elevation & Depth

Hierarchy is established through **Tonal Layers** rather than heavy shadows. In a data-dense dark UI, shadows can become muddy; therefore, depth is communicated via color luminance:

1.  **Floor (Level 0)**: `#0b1120` — The background.
2.  **Raised (Level 1)**: `#1f2937` — Cards, navigation bars, and secondary panels.
3.  **Overlay (Level 2)**: `#1f2937` with a 1px border of `#38bdf8` (at 30% opacity) — Modals, dropdown menus, and tooltips.

All interactive containers should use a subtle 1px border (`#374151`) to define edges against the floor. Focus states on inputs should use a primary blue glow (0px 0px 0px 2px) to ensure user orientation.

## Shapes

The shape language is **Soft** but disciplined. A 6px (`0.375rem`) border-radius is the standard for all interactive components (buttons, inputs, cards). This provides a professional, modern feel that isn't as aggressive as sharp corners nor as "consumer-focused" as pills.

- **Standard (6px)**: Buttons, Input fields, Cards.
- **Large (12px)**: Large modal containers.
- **Circular**: User avatars only.

## Components

### Buttons
- **Primary**: Solid `#3b82f6` with white text. Use for "Publish Mission" or "Submit Code."
- **Secondary**: Transparent background with a `#3b82f6` border. Use for "Assign" or "Edit."
- **Ghost/Tertiary**: No background or border. Use for "Cancel" or "Clear."

### Input Fields
- Background: `#0b1120` (darker than the card).
- Border: 1px solid `#374151`.
- Text: `#ffffff`.
- For the **Identity Linking Code**, use `font-family: 'JetBrains Mono'` and `letter-spacing: 0.25em` for the 6-digit input.

### Cards & Panels
- Background: `#1f2937`.
- Border: 1px solid `#38bdf8` (Primary-light) to indicate technical importance.
- Padding: 24px for standard sections; 16px for dense data.

### ORBAT Tables
- **Unassigned Slot**: Background `#1f2937` with dashed border. Text color muted.
- **Assigned Slot**: Solid background, high-contrast text, showing player name and a "Reassign" button.
- Headers should be `label-sm` (uppercase) with a bottom border of `#374151`.

### Status Alerts
- Success: Background `#064e3b` (muted green) with `#ffffff` text.
- Error: Border 1px solid `#f87171` with `#f87171` text.