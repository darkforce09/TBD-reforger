import { useEffect, useRef, useState } from 'react'
import { useAuthStore } from '@/store/useAuthStore'
import type { ServerStatus } from '@/types/models/telemetry'

export function useServerTelemetry(serverId: string | undefined) {
  const accessToken = useAuthStore((s) => s.accessToken)
  const [status, setStatus] = useState<ServerStatus | null>(null)
  const [isConnected, setIsConnected] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const abortRef = useRef<AbortController | null>(null)

  useEffect(() => {
    if (!serverId || !accessToken) return

    let cancelled = false

    async function connect() {
      abortRef.current?.abort()
      const controller = new AbortController()
      abortRef.current = controller

      try {
        const base = import.meta.env.VITE_API_URL ?? '/api/v1'
        const res = await fetch(`${base}/servers/${serverId}/status/stream`, {
          headers: { Authorization: `Bearer ${accessToken}` },
          signal: controller.signal,
        })
        if (!res.ok || !res.body) throw new Error('SSE connection failed')
        setIsConnected(true)
        setError(null)
        const reader = res.body.getReader()
        const decoder = new TextDecoder()
        let buffer = ''

        while (!cancelled) {
          const { done, value } = await reader.read()
          if (done) break
          buffer += decoder.decode(value, { stream: true })
          const parts = buffer.split('\n\n')
          buffer = parts.pop() ?? ''
          for (const part of parts) {
            const line = part.trim()
            if (line.startsWith('data:')) {
              try {
                const json = JSON.parse(line.slice(5).trim()) as ServerStatus
                setStatus(json)
              } catch {
                /* ignore parse errors */
              }
            }
          }
        }
      } catch (e) {
        if (!controller.signal.aborted) {
          setError(e instanceof Error ? e.message : 'Stream error')
          setIsConnected(false)
        }
      }
    }

    connect()
    return () => {
      cancelled = true
      abortRef.current?.abort()
      setIsConnected(false)
    }
  }, [serverId, accessToken])

  return { status, isConnected, error }
}
