import { format, formatDistanceToNowStrict } from 'date-fns'

function isValidDate(d: Date): boolean {
  return !Number.isNaN(d.getTime())
}

export function formatLocalDateTime(iso: string): string {
  const d = new Date(iso)
  if (!isValidDate(d)) return '—'
  return format(d, 'EEE MMM d, HH:mm zzz')
}

export function formatShortDate(iso: string): string {
  const d = new Date(iso)
  if (!isValidDate(d)) return '—'
  return format(d, 'MMM d')
}

/** UTC time-of-day for tactical log lines, e.g. `14:02:00Z`. */
export function formatZuluTime(iso: string): string {
  const d = new Date(iso)
  if (!isValidDate(d)) return '--:--:--Z'
  return `${d.toISOString().slice(11, 19)}Z`
}

/** Strip HTML tags (bodies are stored as HTML) and collapse whitespace for plain previews. */
export function stripHtml(html: string): string {
  return html
    .replace(/<[^>]*>/g, ' ')
    .replace(/&nbsp;/g, ' ')
    .replace(/\s+/g, ' ')
    .trim()
}

export function formatUptime(seconds: number): string {
  const h = Math.floor(seconds / 3600)
  const m = Math.floor((seconds % 3600) / 60)
  const s = seconds % 60
  return [h, m, s].map((n) => String(n).padStart(2, '0')).join(':')
}

export function formatBytes(bytes: number): string {
  if (bytes < 1) return '0 B'
  const gb = bytes / 1024 ** 3
  if (gb >= 1) return `${gb.toFixed(1)} GB`
  const mb = bytes / 1024 ** 2
  return `${mb.toFixed(0)} MB`
}

export function countdownLabel(iso: string): string {
  const target = new Date(iso)
  if (!isValidDate(target)) return '—'
  if (target.getTime() <= Date.now()) return 'LIVE NOW'
  return formatDistanceToNowStrict(target, { addSuffix: false }).toUpperCase()
}

export function gameModeLabel(mode: string): string {
  switch (mode) {
    case 'pve_coop':
      return 'COOP'
    case 'pvp':
      return 'PvP'
    case 'zeus':
      return 'Zeus'
    default:
      return mode
  }
}

export function terrainLabel(t: string): string {
  if (!t) return '—'
  return t.charAt(0).toUpperCase() + t.slice(1)
}

export function tagLabel(tag: string): string {
  return tag.replace(/_/g, ' ').toUpperCase()
}
