// Dev-only (T-062.2). Two jobs, both behind import.meta.env.DEV (this module is only
// imported from main.tsx in dev):
//
//  1. Block Vite's full page reload while the mission editor is mounted. When the tab is
//     backgrounded for a while, the HMR WebSocket disconnects; on reconnect @vite/client
//     triggers a FULL page reload, which cold-boots useMissionDoc (new Y.Doc → IndexedDB
//     replay → server GET → possible conflict prompt). On a 360k mission that means the
//     loading overlay reappears "on its own" every time the tab regains focus. We veto the
//     reload for the editor route so the live Y.Doc + in-memory state survive an alt-tab.
//
//  2. Diagnostics: log whether a `pageshow` is a bfcache restore and the navigation type,
//     so we can confirm empirically which path fired (true reload vs bfcache vs nav).

// Vite issue #5763: throwing inside the handler does NOT cancel the reload — reassigning
// `payload.path` to a non-matching path is what actually suppresses it.
const EDITOR_ROUTE = /\/missions\/[^/]+\/edit$/

if (import.meta.hot) {
  import.meta.hot.on('vite:beforeFullReload', (payload: { path?: string }) => {
    if (EDITOR_ROUTE.test(location.pathname)) {
      payload.path = '/__vite_editor_reload_blocked__'
      console.warn('[vite] Blocked full reload on mission editor (T-062.2)')
    }
  })
}

window.addEventListener('pageshow', (e) => {
  const nav = performance.getEntriesByType('navigation')[0] as
    | PerformanceNavigationTiming
    | undefined
  console.debug('[reload-guard] pageshow', {
    persisted: e.persisted, // true → restored from the bfcache (no cold boot)
    navType: nav?.type, // 'reload' | 'navigate' | 'back_forward' | 'prerender'
    path: location.pathname,
  })
})
