// Shared map-engine constants (T-065). Kept in their own module so the cluster index, the
// layer hooks, TacticalMap and useSelectTool can all import them without a circular dependency
// (slotIconCache → slotClusterIndex → layers → TacticalMap would otherwise loop through here).

/** Documents the default open zoom / detail floor (useOrthographicView opens at -2). No longer
 *  gates clustering — see ZOOM_CLUSTER_MAX. The OrthographicView band is [-6, 6]. */
export const ZOOM_DETAIL_MIN = -2

/** Cluster mode engages only at/below this zoom (T-065.2). @ ~367k, detail mode (all rings) already
 *  pans at ~160 fps on the GPU, so clustering is reserved for extreme zoom-out where the whole
 *  terrain is crammed into a few hundred px. Tunable to -5. Default open zoom (-2) stays detail. */
export const ZOOM_CLUSTER_MAX = -4

/** Cluster mode only engages when the placed-slot count exceeds this — small missions always
 *  show individual icons even when zoomed out. Matches BULK_SELECT_CAP (500) in
 *  MissionCreatorPage, NOT the outliner's VIRTUAL_SLOT_THRESHOLD (50, T-064). */
export const CLUSTER_SLOT_THRESHOLD = 500

/** Above this placed-slot count the detail IconLayer culls to the visible 512m chunks + halo +
 *  selection (T-067.0, slotIconCache.getBaseIconsForBbox) instead of feeding Deck every icon via
 *  getBaseIcons(). Below it the all-icons path is unchanged — small/medium missions never pay the
 *  bbox/chunk bookkeeping. Well above CLUSTER_SLOT_THRESHOLD: clustering only kicks in at extreme
 *  zoom-out (ZOOM_CLUSTER_MAX), so the cull is what keeps the 50k–1M detail band affordable. */
export const CHUNK_CULL_THRESHOLD = 50_000
