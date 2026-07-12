// Faction Manager (T-152): author the reusable faction library the palette renders —
// side + name, ORBAT role templates (registry character + optional SlotLoadout v2 via the
// shared ArsenalPicksPanel in kind-only mode), and the vehicle pool. CRUD against
// /api/v1/factions (owner-scoped; docs schema-validated server-side).

import { useMemo, useState } from 'react'
import { Loader2, Plus, Trash2 } from 'lucide-react'
import { toast } from 'sonner'
import { Badge } from '@/components/ui/badge'
import { Dialog, DialogContent } from '@/components/ui/dialog'
import { useFactionLibrary, useRegistry } from '@/hooks/queries'
import { useDeleteFaction, useSaveFaction } from '@/hooks/mutations'
import type { RegistryItem } from '@/types/models/registry'
import {
  FACTION_SIDES,
  type FactionDoc,
  type FactionSide,
  type UserFaction,
} from '@/types/models/faction'
import {
  loadoutToPicks,
  picksToLoadout,
  type CompatSets,
  type LoadoutKey,
} from '../loadout/arsenalRules'
import { ArsenalPicksPanel } from '../loadout/ArsenalPicksPanel'
import { Field, SelectField, TextField } from '../layout/RightInspector/fields'

const EMPTY_DOC: FactionDoc = { side: 'BLUFOR', name: '', roles: [], vehicles: [] }
const NO_COMPAT: CompatSets = { edgeItems: {} }

/** Locale-sorted (value,label) options for one registry kind, abstracts/variants excluded. */
function kindOptions(catalog: readonly RegistryItem[], kind: RegistryItem['kind']) {
  return catalog
    .filter((i) => i.kind === kind && i.abstract !== true && !i.variant_of)
    .map((i) => ({ value: i.resource_name, label: i.display_name }))
    .sort((a, b) => a.label.localeCompare(b.label))
}

export function FactionManagerDialog({
  open,
  onOpenChange,
}: {
  open: boolean
  onOpenChange: (open: boolean) => void
}) {
  const { data: lib, isLoading } = useFactionLibrary()
  const { data: reg } = useRegistry()
  const save = useSaveFaction()
  const remove = useDeleteFaction()

  const catalog = useMemo(() => reg?.data ?? [], [reg])
  const catalogByName = useMemo(
    () => new Map<string, RegistryItem>(catalog.map((i) => [i.resource_name, i])),
    [catalog],
  )
  const characterOptions = useMemo(() => kindOptions(catalog, 'character'), [catalog])
  const vehicleOptions = useMemo(() => kindOptions(catalog, 'vehicle'), [catalog])

  const [editingId, setEditingId] = useState<string | null>(null)
  const [draft, setDraft] = useState<FactionDoc>(EMPTY_DOC)
  const [loadoutRoleIdx, setLoadoutRoleIdx] = useState<number | null>(null)

  const startNew = () => {
    setEditingId(null)
    setDraft(EMPTY_DOC)
    setLoadoutRoleIdx(null)
  }
  const startEdit = (f: UserFaction) => {
    setEditingId(f.id)
    setDraft(f.doc)
    setLoadoutRoleIdx(null)
  }

  const patchRole = (idx: number, patch: Partial<FactionDoc['roles'][number]>) => {
    setDraft((d) => ({
      ...d,
      roles: d.roles.map((r, i) => (i === idx ? { ...r, ...patch } : r)),
    }))
  }

  const onSave = async () => {
    if (!draft.name.trim()) {
      toast.error('Faction needs a name.')
      return
    }
    if (draft.roles.some((r) => !r.role.trim() || !r.character)) {
      toast.error('Every role needs a name and a character.')
      return
    }
    try {
      await save.mutateAsync({ id: editingId ?? undefined, doc: draft })
      toast.success(editingId ? 'Faction updated.' : 'Faction created.')
      if (!editingId) startNew()
    } catch (err) {
      const msg =
        (err as { response?: { data?: { error?: string } } }).response?.data?.error ??
        'Could not save the faction.'
      toast.error(msg)
    }
  }

  const onDelete = async (id: string) => {
    await remove.mutateAsync(id).catch(() => toast.error('Could not delete the faction.'))
    if (editingId === id) startNew()
  }

  const loadoutPicks =
    loadoutRoleIdx !== null && draft.roles[loadoutRoleIdx]
      ? loadoutToPicks(draft.roles[loadoutRoleIdx].loadout)
      : null

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        title="Faction Manager"
        description="Author reusable factions — the palette shows these instead of the raw registry."
        className="max-w-4xl"
      >
        <div className="grid max-h-[70vh] grid-cols-[220px_1fr] gap-4 overflow-hidden">
          {/* Library list */}
          <div className="flex min-h-0 flex-col gap-2 overflow-y-auto border-r border-outline-variant/20 pr-3">
            <button
              type="button"
              onClick={startNew}
              className="inline-flex items-center gap-1.5 rounded-md border border-primary/40 bg-primary/10 px-2 py-1.5 text-label-md text-primary hover:bg-primary/20"
            >
              <Plus className="size-3.5" /> New faction
            </button>
            {isLoading && (
              <div className="flex items-center gap-2 py-4 text-label-sm text-outline">
                <Loader2 className="size-3.5 animate-spin" /> Loading…
              </div>
            )}
            {(lib?.data ?? []).map((f) => (
              <div
                key={f.id}
                className={`group flex items-center justify-between rounded-md px-2 py-1.5 text-label-md ${
                  editingId === f.id
                    ? 'bg-primary/15 text-primary'
                    : 'text-on-surface hover:bg-white/5'
                }`}
              >
                <button type="button" onClick={() => startEdit(f)} className="min-w-0 flex-1 truncate text-left">
                  <Badge variant="neutral">{f.side}</Badge> <span className="ml-1">{f.name}</span>
                </button>
                <button
                  type="button"
                  aria-label={`Delete ${f.name}`}
                  onClick={() => onDelete(f.id)}
                  className="invisible text-outline hover:text-error group-hover:visible"
                >
                  <Trash2 className="size-3.5" />
                </button>
              </div>
            ))}
          </div>

          {/* Editor */}
          <div className="flex min-h-0 flex-col gap-3 overflow-y-auto pr-1">
            <div className="grid grid-cols-2 gap-3">
              <SelectField
                label="Side"
                value={draft.side}
                options={FACTION_SIDES.map((s) => ({ value: s, label: s }))}
                onChange={(v) => setDraft((d) => ({ ...d, side: v as FactionSide }))}
              />
              <TextField
                label="Name"
                value={draft.name}
                onChange={(v) => setDraft((d) => ({ ...d, name: v }))}
                placeholder="US Army 1980s"
              />
            </div>

            <Field label={`Roles (${draft.roles.length})`}>
              <div className="flex flex-col gap-2">
                {draft.roles.map((r, idx) => (
                  <div
                    key={idx}
                    className="flex flex-col gap-2 rounded-md border border-outline-variant/20 bg-surface-container-lowest/30 p-2"
                  >
                    <div className="grid grid-cols-[1fr_90px_auto] items-end gap-2">
                      <TextField
                        label="Role"
                        value={r.role}
                        onChange={(v) => patchRole(idx, { role: v })}
                        placeholder="Squad Leader"
                      />
                      <TextField
                        label="Tag"
                        value={r.tag ?? ''}
                        onChange={(v) => patchRole(idx, { tag: v || undefined })}
                        placeholder="MED"
                      />
                      <div className="flex gap-1 pb-1">
                        <button
                          type="button"
                          onClick={() => setLoadoutRoleIdx(loadoutRoleIdx === idx ? null : idx)}
                          className={`rounded-md border px-2 py-1 text-label-sm ${
                            loadoutRoleIdx === idx
                              ? 'border-primary/50 bg-primary/15 text-primary'
                              : 'border-outline-variant/30 text-on-surface-variant hover:bg-white/5'
                          }`}
                        >
                          Loadout{r.loadout ? ' ●' : ''}
                        </button>
                        <button
                          type="button"
                          aria-label={`Remove role ${r.role || idx + 1}`}
                          onClick={() => {
                            setLoadoutRoleIdx(null)
                            setDraft((d) => ({ ...d, roles: d.roles.filter((_, i) => i !== idx) }))
                          }}
                          className="rounded-md border border-outline-variant/30 px-2 py-1 text-outline hover:text-error"
                        >
                          <Trash2 className="size-3.5" />
                        </button>
                      </div>
                    </div>
                    <SelectField
                      label="Character"
                      value={r.character}
                      options={[{ value: '', label: '— pick a character —' }, ...characterOptions]}
                      onChange={(v) => patchRole(idx, { character: v })}
                    />
                  </div>
                ))}
                <button
                  type="button"
                  onClick={() =>
                    setDraft((d) => ({
                      ...d,
                      roles: [...d.roles, { role: '', character: '' }],
                    }))
                  }
                  className="inline-flex items-center gap-1.5 self-start rounded-md border border-outline-variant/30 px-2 py-1 text-label-md text-on-surface-variant hover:bg-white/5"
                >
                  <Plus className="size-3.5" /> Add role
                </button>
              </div>
            </Field>

            {loadoutPicks && loadoutRoleIdx !== null && (
              <Field label={`Loadout — ${draft.roles[loadoutRoleIdx].role || 'role'}`}>
                <div className="rounded-md border border-outline-variant/20 bg-surface-container-lowest/20 p-2">
                  <ArsenalPicksPanel
                    picks={loadoutPicks}
                    onPick={(key: LoadoutKey, value: string) => {
                      const next = picksToLoadout({ ...loadoutPicks, [key]: value }, catalogByName)
                      patchRole(loadoutRoleIdx, { loadout: next ?? undefined })
                    }}
                    catalog={catalog}
                    catalogByName={catalogByName}
                    sets={NO_COMPAT}
                    smart={false}
                  />
                </div>
              </Field>
            )}

            <Field label={`Vehicles (${draft.vehicles.length})`}>
              <div className="flex flex-col gap-2">
                {draft.vehicles.map((v, idx) => (
                  <div key={idx} className="grid grid-cols-[1fr_140px_auto] items-end gap-2">
                    <SelectField
                      label="Vehicle"
                      value={v.vehicle}
                      options={[{ value: '', label: '— pick a vehicle —' }, ...vehicleOptions]}
                      onChange={(val) =>
                        setDraft((d) => ({
                          ...d,
                          vehicles: d.vehicles.map((x, i) => (i === idx ? { ...x, vehicle: val } : x)),
                        }))
                      }
                    />
                    <TextField
                      label="Label"
                      value={v.label ?? ''}
                      onChange={(val) =>
                        setDraft((d) => ({
                          ...d,
                          vehicles: d.vehicles.map((x, i) =>
                            i === idx ? { ...x, label: val || undefined } : x,
                          ),
                        }))
                      }
                      placeholder="UAZ-469"
                    />
                    <button
                      type="button"
                      aria-label={`Remove vehicle ${idx + 1}`}
                      onClick={() =>
                        setDraft((d) => ({ ...d, vehicles: d.vehicles.filter((_, i) => i !== idx) }))
                      }
                      className="mb-1 rounded-md border border-outline-variant/30 px-2 py-1 text-outline hover:text-error"
                    >
                      <Trash2 className="size-3.5" />
                    </button>
                  </div>
                ))}
                <button
                  type="button"
                  onClick={() =>
                    setDraft((d) => ({ ...d, vehicles: [...d.vehicles, { vehicle: '' }] }))
                  }
                  className="inline-flex items-center gap-1.5 self-start rounded-md border border-outline-variant/30 px-2 py-1 text-label-md text-on-surface-variant hover:bg-white/5"
                >
                  <Plus className="size-3.5" /> Add vehicle
                </button>
              </div>
            </Field>

            <div className="flex justify-end gap-2 border-t border-outline-variant/20 pt-3">
              <button
                type="button"
                onClick={onSave}
                disabled={save.isPending}
                className="rounded-lg border border-primary/40 bg-primary/15 px-3 py-1.5 text-label-md text-primary hover:bg-primary/25 disabled:opacity-50"
              >
                {save.isPending ? 'Saving…' : editingId ? 'Save changes' : 'Create faction'}
              </button>
            </div>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  )
}
