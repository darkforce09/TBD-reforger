import { useState } from 'react'
import { toast } from 'sonner'
import { MaterialIcon } from '@/components/MaterialIcon'
import { OpsCard } from '@/components/OpsCard'
import { PageHeader } from '@/components/PageHeader'
import { AuthGate } from '@/components/AuthGate'
import { QueryState } from '@/components/QueryState'
import { useLinkStatus, useMe } from '@/hooks/queries'
import { useGenerateLinkCode, useUnlinkArma } from '@/hooks/mutations'
import { DEFAULT_AVATAR } from '@/lib/avatar'

export function SettingsPage() {
  const { data: me, isLoading, isError, error } = useMe()
  const { data: linkStatus } = useLinkStatus()
  const generateCode = useGenerateLinkCode()
  const unlink = useUnlinkArma()
  const [pendingCode, setPendingCode] = useState<string | null>(null)

  const user = me?.user

  const handleGenerate = () => {
    generateCode.mutate(undefined, {
      onSuccess: (data) => {
        setPendingCode(data.code)
        toast.success('Link code generated — enter it in-game')
      },
      onError: () => toast.error('Failed to generate link code'),
    })
  }

  const handleUnlink = () => {
    unlink.mutate(undefined, {
      onSuccess: () => {
        setPendingCode(null)
        toast.success('Arma identity unlinked')
      },
      onError: () => toast.error('Failed to unlink'),
    })
  }

  return (
    <AuthGate>
      <QueryState isLoading={isLoading} isError={isError} error={error as Error}>
        {user && (
          <div className="mx-auto w-full max-w-2xl">
            <PageHeader title="Settings" subtitle="Account profile, Arma identity, and service statistics." />

            <OpsCard className="mb-6 bg-surface-container-high">
              <h2 className="mb-4 text-lg font-semibold">Profile</h2>
              <div className="flex items-center gap-4">
                <img
                  src={user.avatar_url || DEFAULT_AVATAR}
                  alt=""
                  className="h-16 w-16 rounded-full border border-border-subtle object-cover"
                />
                <div>
                  <p className="text-lg font-semibold">{user.username}</p>
                  <p className="text-sm text-on-surface-variant">
                    {user.discord_handle ?? user.discord_id}
                  </p>
                  <span className="mt-2 inline-block rounded bg-primary/20 px-2 py-0.5 text-xs font-semibold text-primary uppercase">
                    {user.role}
                  </span>
                </div>
              </div>
            </OpsCard>

            <OpsCard className="mb-6 bg-surface-container-high">
              <h2 className="mb-4 text-lg font-semibold">Arma Identity</h2>
              <p className="mb-4 text-sm text-on-surface-variant">
                Status:{' '}
                <span className={linkStatus?.linked ? 'text-success' : 'text-on-surface-variant'}>
                  {linkStatus?.linked
                    ? `Linked (${linkStatus.arma_character || linkStatus.arma_id})`
                    : 'Unlinked'}
                </span>
              </p>
              {pendingCode ? (
                <p className="mb-4 rounded-lg border border-primary/30 bg-primary/10 p-3 font-mono text-sm">
                  Link code: {pendingCode}
                </p>
              ) : linkStatus?.pending_code ? (
                <p className="mb-4 rounded-lg border border-primary/30 bg-primary/10 p-3 text-sm">
                  A link code is already pending. Generate a new one to display it, then enter it
                  in-game.
                </p>
              ) : null}
              <div className="flex flex-wrap gap-2">
                <button
                  type="button"
                  onClick={handleGenerate}
                  disabled={generateCode.isPending}
                  className="rounded-lg bg-primary px-4 py-2 text-sm font-medium text-on-primary disabled:opacity-50"
                >
                  Generate Link Code
                </button>
                {linkStatus?.linked && (
                  <button
                    type="button"
                    onClick={handleUnlink}
                    disabled={unlink.isPending}
                    className="rounded-lg border border-border-subtle px-4 py-2 text-sm disabled:opacity-50"
                  >
                    Unlink Arma ID
                  </button>
                )}
              </div>
            </OpsCard>

            <OpsCard className="bg-surface-container-high">
              <h2 className="mb-4 flex items-center gap-2 text-lg font-semibold">
                <MaterialIcon name="military_tech" className="text-primary" />
                Service Stats
              </h2>
              <div className="grid grid-cols-2 gap-4 text-sm">
                <div>
                  <span className="text-on-surface-variant">Total Operations</span>
                  <p className="text-2xl font-bold text-primary">{user.total_deployments ?? 0}</p>
                </div>
                <div>
                  <span className="text-on-surface-variant">Attendance</span>
                  <p className="text-2xl font-bold text-success">{user.attendance_rate ?? 0}%</p>
                </div>
              </div>
            </OpsCard>
          </div>
        )}
      </QueryState>
    </AuthGate>
  )
}
