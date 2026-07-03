// Shared inspector field primitives — a label + control row in the Aegis glass
// style, reused by the Global Settings and Slot inspectors.

import { useState, type ReactNode } from 'react'
import { Switch } from '@/components/ui/switch'
import { cn } from '@/lib/utils'

const controlClass =
  'w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2.5 py-1.5 text-label-md text-on-surface outline-none transition-colors focus:border-primary/60'

export function Field({ label, children }: { label: string; children: ReactNode }) {
  return (
    <label className="flex flex-col gap-1">
      <span className="text-label-sm uppercase tracking-wider text-outline">{label}</span>
      {children}
    </label>
  )
}

export function TextField({
  label,
  value,
  onChange,
  placeholder,
  type = 'text',
}: {
  label: string
  value: string | number
  onChange: (v: string) => void
  placeholder?: string
  type?: 'text' | 'time' | 'number'
}) {
  return (
    <Field label={label}>
      <input
        type={type}
        value={value}
        placeholder={placeholder}
        onChange={(e) => onChange(e.target.value)}
        className={controlClass}
      />
    </Field>
  )
}

/**
 * Mono numeric field that commits on blur / Enter (not every keystroke) — so one numeric
 * edit is one Y.Doc transaction / undo step. While focused it shows the local draft; while
 * unfocused it always mirrors the external value (so a map-drag updates the readout live).
 * No effects (keeps clear of the set-state-in-effect lint rule).
 */
export function NumberField({
  label,
  value,
  onCommit,
  suffix,
}: {
  label: string
  value: number
  onCommit: (v: number) => void
  suffix?: string
}) {
  const [draft, setDraft] = useState('')
  const [focused, setFocused] = useState(false)
  const rounded = Math.round(value)

  const commit = () => {
    setFocused(false)
    const n = parseFloat(draft)
    if (Number.isFinite(n)) onCommit(n)
  }

  return (
    <Field label={label}>
      <div className="relative">
        <input
          type="number"
          value={focused ? draft : String(rounded)}
          onFocus={() => {
            setDraft(String(rounded))
            setFocused(true)
          }}
          onChange={(e) => setDraft(e.target.value)}
          onBlur={commit}
          onKeyDown={(e) => {
            if (e.key === 'Enter') e.currentTarget.blur()
          }}
          className={cn(controlClass, 'font-mono', suffix && 'pr-7')}
        />
        {suffix && (
          <span className="pointer-events-none absolute right-2.5 top-1/2 -translate-y-1/2 font-mono text-label-sm text-outline">
            {suffix}
          </span>
        )}
      </div>
    </Field>
  )
}

export function SelectField({
  label,
  value,
  options,
  onChange,
}: {
  label: string
  value: string
  options: { value: string; label: string }[]
  onChange: (v: string) => void
}) {
  return (
    <Field label={label}>
      <select value={value} onChange={(e) => onChange(e.target.value)} className={controlClass}>
        {options.map((o) => (
          <option key={o.value} value={o.value} className="bg-surface-container">
            {o.label}
          </option>
        ))}
      </select>
    </Field>
  )
}

export function ToggleField({
  label,
  checked,
  onChange,
}: {
  label: string
  checked: boolean
  onChange: (v: boolean) => void
}) {
  return (
    <div className="flex items-center justify-between py-0.5">
      <span className="text-label-md text-on-surface-variant">{label}</span>
      <Switch checked={checked} onCheckedChange={onChange} />
    </div>
  )
}

/**
 * Label + range slider + mono percent readout on one row (T-090.1.2.6 hillshade strength).
 * Emits on every drag tick for live preview — Y.UndoManager's capture window coalesces a
 * drag into one undo step. `disabled` dims the whole row (e.g. hillshade toggled off).
 */
export function SliderField({
  label,
  value,
  onChange,
  disabled = false,
  min = 0,
  max = 100,
  step = 5,
  decimals,
}: {
  label: string
  value: number
  onChange: (v: number) => void
  disabled?: boolean
  min?: number
  max?: number
  step?: number
  /** Readout decimal places; defaults to 1 when step < 1, else 0. */
  decimals?: number
}) {
  const clamped = Math.min(max, Math.max(min, value))
  const readoutDecimals = decimals ?? (step < 1 ? 1 : 0)
  const readout =
    readoutDecimals > 0 ? clamped.toFixed(readoutDecimals) : String(Math.round(clamped))
  return (
    <div
      className={cn('flex items-center justify-between gap-3 py-0.5', disabled && 'opacity-50')}
    >
      <span className="text-label-md text-on-surface-variant">{label}</span>
      <div className="flex items-center gap-2">
        <input
          type="range"
          min={min}
          max={max}
          step={step}
          value={clamped}
          disabled={disabled}
          onChange={(e) => onChange(Number(e.target.value))}
          className={cn('w-32 accent-primary', disabled && 'cursor-not-allowed')}
        />
        <span className="w-11 text-right font-mono text-label-md tabular-nums text-on-surface">
          {readout}%
        </span>
      </div>
    </div>
  )
}

export function ReadonlyField({ label, value }: { label: string; value: string }) {
  return (
    <Field label={label}>
      <div className="rounded-md border border-outline-variant/20 bg-surface-container-lowest/30 px-2.5 py-1.5 font-mono text-code-md text-on-surface-variant">
        {value}
      </div>
    </Field>
  )
}
