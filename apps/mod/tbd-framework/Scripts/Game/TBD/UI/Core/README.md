# UI/Core

Foundational UI framework primitives, layout registry, Aegis styling system, and navigation stacks.

### Roles & Responsibilities
- `TBD_UILayouts.c`: Single source of truth registering all `.layout` resource GUIDs and relative fallback paths.
- `TBD_UITheme.c`: Styling layer porting Aegis design tokens (colors, typography, margins) from platform CSS to Enfusion ARGB integers.
- `TBD_MenuBase.c` & `TBD_MenuStack.c`: Chimera menu base class and stack manager governing window lifecycle, modal hierarchy, and Esc handling.
- `TBD_ShellScreen.c`: Reusable full-screen workstation shell providing uniform header, tabs, dock, and footer.
- `TBD_UIButton.c` & `TBD_UIInteractive.c`: Reusable interactive components normalizing mouse hover and gamepad focus behaviors.
- `TBD_ListBox.c` & `TBD_ListBoxRow.c`: High-performance pooled list box system avoiding widget churn during scrolling.

### Call Flow & Contracts
Pure client-side UI infrastructure. Consumed directly by all domain screens in `Session/*/UI/` and shared widgets in `UI/Common/`.
