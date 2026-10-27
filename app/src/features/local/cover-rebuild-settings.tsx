import { useEffect, useRef, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { RefreshCw } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { ErrorNotice } from '@/components/error-notice'
import { SettingsCard } from '@/features/settings/settings-card'
import { reportCaughtError } from '@/lib/diagnostics'
import { errorText, nativeCall } from '@/lib/player'

interface RebuildStatus {
  processed: number
  total: number
  failed: number
  running: boolean
  error?: string | null
}

export function CoverRebuildSettings() {
  const desktop = isTauri()
  const [status, setStatus] = useState<RebuildStatus>()
  const [ready, setReady] = useState(false)
  const [starting, setStarting] = useState(false)
  const [requested, setRequested] = useState(false)
  const [error, setError] = useState<string>()
  const generation = useRef(0)
  const events = useRef(0)

  useEffect(() => {
    if (!desktop) return
    const current = ++generation.current
    let stop: (() => void) | undefined
    void (async () => {
      try {
        const unlisten = await listen<RebuildStatus>(
          'local-cover-rebuild-progress',
          ({ payload }) => {
            if (current !== generation.current) return
            events.current++
            setStatus(payload)
          }
        )
        if (current !== generation.current) {
          unlisten()
          return
        }
        stop = unlisten
        setReady(true)
        const before = events.current
        const value = await nativeCall<RebuildStatus>('local_cover_rebuild_status')
        if (current === generation.current) {
          if (before === events.current) setStatus(value)
        }
      } catch (cause) {
        reportCaughtError('local.cover-rebuild.status', cause)
        if (current === generation.current) setError(errorText(cause))
      }
    })()
    return () => {
      generation.current++
      stop?.()
    }
  }, [desktop])

  async function rebuild() {
    if (!ready || starting || status?.running) return
    const current = generation.current
    const before = ++events.current
    setStarting(true)
    setError(undefined)
    try {
      const value = await nativeCall<RebuildStatus>('local_cover_rebuild')
      if (current === generation.current) {
        setRequested(true)
        if (before === events.current) setStatus(value)
      }
    } catch (cause) {
      reportCaughtError('local.cover-rebuild.start', cause)
      if (current === generation.current) setError(errorText(cause))
    } finally {
      if (current === generation.current) setStarting(false)
    }
  }

  const running = starting || !!status?.running
  const checked = status && (requested || status.running || status.processed > 0 || status.error)
  return (
    <div className="flex flex-col gap-3">
      <SettingsCard
        searchId="local-cover-rebuild"
        title="重建本地封面"
        description="重新提取已导入歌曲的封面，并更新专辑封面。"
      >
        <Button
          variant="outline"
          disabled={!desktop || !ready || running}
          onClick={() => void rebuild()}
        >
          <RefreshCw aria-hidden="true" />
          {running ? '正在重建…' : '重建本地封面'}
        </Button>
      </SettingsCard>
      {checked && (
        <p role="status" className="text-sm text-muted-foreground">
          {status.running ? '正在检查' : status.error ? '重建未完成，已检查' : '重建完成，已检查'}{' '}
          {status.processed} / {status.total} 首歌曲
          {status.failed > 0 ? `，${status.failed} 首处理失败。` : '。'}
        </p>
      )}
      {(error || status?.error) && <ErrorNotice error={error || status?.error} />}
    </div>
  )
}
