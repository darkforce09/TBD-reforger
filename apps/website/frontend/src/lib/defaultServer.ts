import type { ServerIntel } from '@/types/api'

export function pickDefaultServer(servers: ServerIntel[]): ServerIntel | undefined {
  const name = import.meta.env.VITE_DEFAULT_SERVER_NAME as string | undefined
  if (name) {
    const match = servers.find((s) => s.name === name)
    if (match) return match
  }
  return servers.find((s) => s.is_active) ?? servers[0]
}
