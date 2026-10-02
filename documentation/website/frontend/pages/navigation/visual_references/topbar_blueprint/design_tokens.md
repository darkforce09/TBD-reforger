---
name: TBD Reforger TopBar
colors:
  primary: '#3b82f6'
  secondary: '#1f2937'
  surface: '#0b1120'
  on-surface: '#ffffff'
  error: '#f87171'
  success: '#22c55e'
  surface-dim: '#10131a'
  surface-bright: '#363941'
  surface-container-lowest: '#0b0e15'
  surface-container-low: '#191b23'
  surface-container: '#1d2027'
  surface-container-high: '#272a31'
  surface-container-highest: '#32353c'
  on-surface-variant: '#c2c6d6'
  inverse-surface: '#e1e2ec'
  inverse-on-surface: '#2e3038'
  outline: '#8c909f'
  outline-variant: '#424754'
  surface-tint: '#adc6ff'
  on-primary: '#002e6a'
  primary-container: '#4d8eff'
  on-primary-container: '#00285d'
  inverse-primary: '#005ac2'
  on-secondary: '#27313f'
  secondary-container: '#404a59'
  on-secondary-container: '#afb9cb'
  tertiary: '#ffb786'
  on-tertiary: '#502400'
  tertiary-container: '#df7412'
  on-tertiary-container: '#461f00'
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
  tertiary-fixed: '#ffdcc6'
  tertiary-fixed-dim: '#ffb786'
  on-tertiary-fixed: '#311400'
  on-tertiary-fixed-variant: '#723600'
  background: '#10131a'
  on-background: '#e1e2ec'
  surface-variant: '#32353c'
  border-subtle: '#1e3a5f'
  success-muted: rgba(34, 197, 94, 0.2)
typography:
  body-md:
    fontFamily: Inter
    fontSize: 16px
    fontWeight: 400
    lineHeight: 24px
  nav-breadcrumb-parent:
    fontFamily: Inter
    fontSize: 14px
    fontWeight: '400'
    lineHeight: 20px
    letterSpacing: 0.01em
  nav-breadcrumb-current:
    fontFamily: Inter
    fontSize: 14px
    fontWeight: '600'
    lineHeight: 20px
  label-sm:
    fontFamily: Inter
    fontSize: 12px
    fontWeight: '600'
    lineHeight: 16px
    letterSpacing: 0.05em
  menu-item:
    fontFamily: Inter
    fontSize: 14px
    fontWeight: '500'
    lineHeight: 20px
rounded:
  md: 6px
  sm: 0.125rem
  DEFAULT: 0.25rem
  lg: 0.5rem
  xl: 0.75rem
  full: 9999px
spacing:
  navbar-height: 64px
  container-padding: 1.5rem
  element-gap: 1rem
  pill-padding-x: 0.75rem
  pill-padding-y: 0.25rem
---

# Design System
A focused, premium dark interface for the TBD Reforger Event Platform. 
## Component to Generate: The Top Navigation Bar
Please generate a fixed Top Navigation Bar React component (`height: 64px`) that spans the full width of the main content area. Background should be `#0b1120` with a subtle bottom border (`#1e3a5f`).
### Left Side
- A Breadcrumb trail showing the current location (e.g., `Mission Hub / Operation Enduring Freedom`). Use muted text for the parent, and bright white text for the current page.
### Right Side (Profile Group)
Align these items horizontally with a clean gap:
1. **Identity Status Pill**: A small pill badge showing the "Linked" state: subtle green background (#22c55e at 20% opacity) with bright green text saying "Linked: 765611...".
2. **User Profile Dropdown Toggle**: A circular placeholder Discord avatar image next to the Discord username "Admin Dave".
3. **Dropdown Menu (Open State)**: Show the dropdown menu floating below the profile. It should be a `#1f2937` card with a subtle blue border. It should have three list items: "Settings", "Link Arma Identity", and "Sign Out" (in red text).