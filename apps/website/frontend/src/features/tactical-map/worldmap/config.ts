// T-090.5.1 — Map Engine v2 feature flag. Default OFF: the editor must render exactly as
// today (sat field + hillshade + grid, plan risk R3) until world layers prove out. Dev
// toggle: `VITE_WORLDMAP_ENABLED=1 make web`. Flips to a settings-driven control once the
// first real layers ship (T-090.5.2+).

export const WORLDMAP_ENABLED = import.meta.env.VITE_WORLDMAP_ENABLED === '1'
