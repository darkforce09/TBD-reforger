import { describe, it, expect } from 'vitest'
import 'fake-indexeddb/auto'
import { saveState, loadState, clearState, saveStateDebounced, flushState } from './yrsPersist'

// T-145 Phase 3.2 flip F2 — the v3 persistence contract: blob round-trip + the idle-gated debounced
// writer (coalesce to latest, flush-on-demand, cancel-safe, never overwrite with an empty blob). Uses
// fake-indexeddb for a real IDB in the node test env. Distinct mission ids per test (module-singleton
// timers/chains are shared).

describe('yrsPersist (F2)', () => {
  it('round-trips a blob; clearState drops it', async () => {
    await saveState('m1', new Uint8Array([1, 2, 3, 4, 5]))
    const loaded = await loadState('m1')
    expect(loaded).not.toBeNull()
    expect(Array.from(loaded ?? [])).toEqual([1, 2, 3, 4, 5])

    await clearState('m1')
    expect(await loadState('m1')).toBeNull()
  })

  it('loadState returns null for an unknown mission', async () => {
    expect(await loadState('never-saved')).toBeNull()
  })

  it('debounced writer coalesces to the latest bytes at flush', async () => {
    let current = new Uint8Array([1])
    saveStateDebounced('m2', () => current)
    current = new Uint8Array([2])
    saveStateDebounced('m2', () => current) // resets the 5 s timer; nothing written yet
    expect(await loadState('m2')).toBeNull()

    current = new Uint8Array([9, 9])
    await flushState('m2') // writes the LATEST bytes at flush time
    expect(Array.from((await loadState('m2')) ?? [])).toEqual([9, 9])
  })

  it('isCancelled aborts a queued write', async () => {
    let cancelled = false
    saveStateDebounced(
      'm3',
      () => new Uint8Array([7]),
      () => cancelled,
    )
    cancelled = true
    await flushState('m3')
    expect(await loadState('m3')).toBeNull()
  })

  it('never overwrites a good record with an empty blob', async () => {
    await saveState('m4', new Uint8Array([5, 5]))
    saveStateDebounced('m4', () => new Uint8Array()) // detached shell → empty → skipped
    await flushState('m4')
    expect(Array.from((await loadState('m4')) ?? [])).toEqual([5, 5])
  })
})
