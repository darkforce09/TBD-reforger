# Systems/Markers

Mission map marker synchronization, tactical symbol translation, and map rendering.

### Roles & Responsibilities
- `TBD_MarkerData.c`: Wire models and serializable structures defining tactical map markers and drawings.
- `TBD_MarkerIcons.c`: Translates mission semantic icon identifiers into Reforger native map marker textures.
- `TBD_MarkerComponent.c`: Game mode authority component managing active mission map markers.
- `TBD_MarkerController.c`: Player controller RPC conduit synchronizing marker changes between server and clients.
- `TBD_MarkerClient.c`: Client-side cache rendering markers onto Reforger's native `SCR_MapEntity`.

### Call Flow & Contracts
Follows the Data/Service/Controller/Client pattern: `TBD_MarkerComponent` synchronizes marker lists via `TBD_MarkerController` RPCs to `TBD_MarkerClient`, which injects markers into the map UI.
