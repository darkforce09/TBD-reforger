# UI/Common

Reusable widget components and sub-layout binding handlers shared across multiple screens.

### Roles & Responsibilities
- `ScriptedWidgetComponent` handlers for shared sub-layouts authored in `UI/layouts/Common/` (NavItem, FactionChip, PlayerRow, SquadCard, SearchBar, ModalChrome, StepperRow, MetricTile).
- Encapsulates isolated presentation widgets without holding screen knowledge or direct data network access.

### Call Flow & Contracts
Pure view-binding components. Instantiated by layout hierarchy or dynamically attached by `Session/*/UI` screens to bind model data to UI widgets.
