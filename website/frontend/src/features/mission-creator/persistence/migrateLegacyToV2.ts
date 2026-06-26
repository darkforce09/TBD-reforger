// One-time v1→v2 migration (T-062.1). On the first return visit after the upgrade, the
// legacy y-indexeddb DB has already replayed the whole Y.Doc (the doc is populated). We
// snapshot meta + slots into the v2 chunk store so every subsequent load uses the chunked
// restore path. Idempotent (StrictMode-safe): skips if a v2 record already exists. The
// caller (useMissionDoc) owns destroying the legacy IndexeddbPersistence + deleting its DB
// once this resolves, so no further edits re-bloat y-indexeddb.

import type { MissionDoc } from '../../tactical-map/state/ydoc'
import type { LoadProgress } from '../hooks/useMissionDoc'
import { hasV2Persist } from './missionPersistSchema'
import { saveMissionMetaFromDoc } from './missionMetaStore'
import { saveSlotsFromDoc } from './slotChunkStore'

export async function migrateLegacyToV2(
  missionId: string,
  md: MissionDoc,
  onProgress?: (p: LoadProgress) => void,
  isCancelled?: () => boolean,
): Promise<void> {
  if (await hasV2Persist(missionId)) return // already migrated — don't double-write

  // Slot export progress rides the restoring band (0–0.15) with a distinct label so the
  // overlay reads "Migrating local save…" rather than "Reading local save…".
  const report = (done: number, total: number) =>
    onProgress?.({
      phase: 'restoring',
      value: total > 0 ? 0.15 * Math.min(done / total, 1) : 0,
      label: 'Migrating local save…',
      done,
      total,
    })

  await saveMissionMetaFromDoc(md, missionId)
  if (isCancelled?.()) return
  await saveSlotsFromDoc(md, missionId, report, isCancelled)
}
