import { describe, expect, it } from 'vitest'
import 'fake-indexeddb/auto'

import {
  clearCached,
  getCached,
  getLastModpackId,
  putCached,
  type CachedCompat,
} from './registryCompatCache'

// T-068.9 proof-ledger gate G8 — IDB cache round-trip: edge-set identity,
// etag/'last' bookkeeping, side-by-side modpack isolation, miss -> null.
// fake-indexeddb provides a real IDB in the node test env (yrsPersist precedent).

const entry = (modpack: string, etag: string, edges: CachedCompat['edges']): CachedCompat => ({
  modpack_id: modpack,
  modpack_version: '1.0',
  etag,
  fetched_at: 1234567890,
  edges,
})

const EDGES_A: CachedCompat['edges'] = [
  {
    from_node: '{AB12CD34EF56AB01}Prefabs/Mags/Mag A.et',
    to_node: '{AB12CD34EF56AB02}Prefabs/Weapons/Rifle A.et',
    edge_type: 'mag_in_weapon',
    evidence: 'WellA',
  },
  {
    from_node: '{AB12CD34EF56AB03}Prefabs/Optics/Optic A.et',
    to_node: '{AB12CD34EF56AB02}Prefabs/Weapons/Rifle A.et',
    edge_type: 'optic_on_weapon',
    // evidence absent — NULL ≡ '' ≡ absent must survive the round-trip
  },
]

const EDGES_B: CachedCompat['edges'] = [
  {
    from_node: '{AB12CD34EF56AB04}Prefabs/Mags/Mag B.et',
    to_node: '{AB12CD34EF56AB05}Prefabs/Weapons/Rifle B.et',
    edge_type: 'mag_in_weapon',
    evidence: 'WellB',
  },
]

describe('registryCompatCache (G8)', () => {
  it('round-trips a graph byte-identically and tracks the last modpack', async () => {
    await putCached(entry('mp-a', 'W/"a-1"', EDGES_A))
    const hit = await getCached('mp-a')
    expect(hit).not.toBeNull()
    expect(hit?.etag).toBe('W/"a-1"')
    expect(hit?.modpack_version).toBe('1.0')
    expect(hit?.edges).toEqual(EDGES_A) // structural identity incl. absent evidence
    expect(await getLastModpackId()).toBe('mp-a')
  })

  it('misses return null', async () => {
    expect(await getCached('never-cached')).toBeNull()
  })

  it('caches modpacks side-by-side without cross-talk; last follows the newest put', async () => {
    await putCached(entry('mp-a', 'W/"a-1"', EDGES_A))
    await putCached(entry('mp-b', 'W/"b-1"', EDGES_B))
    expect((await getCached('mp-a'))?.edges).toEqual(EDGES_A)
    expect((await getCached('mp-b'))?.edges).toEqual(EDGES_B)
    expect(await getLastModpackId()).toBe('mp-b')
  })

  it('overwrites on a new etag (revalidation adopted a fresh graph)', async () => {
    await putCached(entry('mp-a', 'W/"a-1"', EDGES_A))
    await putCached(entry('mp-a', 'W/"a-2"', EDGES_B))
    const hit = await getCached('mp-a')
    expect(hit?.etag).toBe('W/"a-2"')
    expect(hit?.edges).toEqual(EDGES_B)
  })

  it('clearCached drops the record and the last pointer only if it matches', async () => {
    await putCached(entry('mp-a', 'W/"a-1"', EDGES_A))
    await putCached(entry('mp-b', 'W/"b-1"', EDGES_B))
    await clearCached('mp-a')
    expect(await getCached('mp-a')).toBeNull()
    expect(await getLastModpackId()).toBe('mp-b') // pointer belongs to mp-b, untouched
    await clearCached('mp-b')
    expect(await getLastModpackId()).toBeNull()
  })
})
