# 3. After-Action Report (`src/v2/aar`)

The post-match forensics and telemetry replay suite.

---

## Purpose
Allows community members, commanders, and administrators to review completed matches with second-by-second telemetry playback (player positions, vehicle routes, shots, kills, and objective captures).

## Architecture
- **Consumes:** Mounts `<MapCanvas />` from `src/v2/map_engine`.
- **`ui/`**: Media player style playback deck, timeline scrubber bar, live killfeed ticker, match scoreboard drawer.
- **`features/`**: Telemetry log parser, casualty forensics, ballistic trajectory lines, player movement heatmaps.
- **`state/`**: Time-series telemetry buffer, playback clock (Play/Pause, 1x/2x/5x/10x speeds, jump-to-event).
