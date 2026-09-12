# Systems/Mission/Ingestion

Translates abstract mission definitions into live Enfusion world states, environment settings, and spatial offsets.

### Roles & Responsibilities
- `TBD_EnvironmentReader.c`: Applies static authored mission environment settings (weather preset, fog density, wind vectors, view distance) to engine managers.
- `TBD_WeatherRuntime.c`: Evaluates authored dynamic weather timelines, transitioning keyframe presets as the match round elapses.
- `TBD_PlacementScatter.c`: Parses placement dispersion radiuses and shapes, computing randomized spawn offsets around authored coordinates.

### Call Flow & Contracts
Authority-only execution. Configures engine environment managers (`BaseWeatherManagerEntity`, `TimeAndWeatherManagerEntity`) and feeds calculated positions into `Systems/Spawning/`.
