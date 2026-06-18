import { format, formatDistanceToNowStrict } from 'date-fns'

export function formatLocalDateTime(iso: string): string {
  return format(new Date(iso), 'EEE MMM d, HH:mm zzz')
}

export function formatShortDate(iso: string): string {
  return format(new Date(iso), 'MMM d')
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
