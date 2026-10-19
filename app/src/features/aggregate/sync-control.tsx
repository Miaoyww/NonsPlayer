import { useEffect, useRef, useState } from 'react'
import { RefreshCw, X, GitFork } from 'lucide-react'
import { ActionButton } from '@/components/music/action-button'
import { ErrorNotice } from '@/components/error-notice'
import { nativeCall, errorText } from '@/lib/player'
import type { EntityRef } from '@/features/music/types'
import { useMusicSources } from '@/features/music/use-music-sources'
import { useMusicNavigation } from '@/features/workspace/music-navigation'
import {
  useAggregateLibrary,
  aggregateCollection,
  type AggregatePlaylist,
  type SyncProgress
} from './aggregate-library'

export function SyncControl({
  id,
  reference,
  automatic = false
}: {
  id?: string
  reference?: EntityRef
  automatic?: boolean
}) {
  const [progress, setProgress] = useState<SyncProgress>()
  const [error, setError] = useState<string>()
  const [starting, setStarting] = useState(false)
  const mounted = useRef(true)
  const request = useRef(0)
  const { reload } = useAggregateLibrary()
  const { revision: sourceRevision } = useMusicSources()
  const { navigate } = useMusicNavigation()
  async function start(force: boolean) {
    const serial = ++request.current
    setStarting(true)
    setError(undefined)
    try {
      const value = await nativeCall<SyncProgress>('aggregate_sync', { id, reference, force })
      if (mounted.current && request.current === serial) setProgress(value)
    } catch (cause) {
      if (mounted.current && request.current === serial) setError(errorText(cause))
    } finally {
      if (mounted.current && request.current === serial) setStarting(false)
    }
  }
  useEffect(() => {
    mounted.current = true
    if (automatic) void start(false)
    return () => {
      mounted.current = false
      request.current++
    }
  }, [id, automatic, sourceRevision])
  useEffect(() => {
    if (!progress?.running) return
    let disposed = false
    let timer: ReturnType<typeof setTimeout>
    async function poll() {
      try {
        const value = await nativeCall<SyncProgress | null>('aggregate_sync_status', {
          key: progress!.key
        })
        if (disposed) return
        if (!value) {
          setProgress(undefined)
          setError('同步任务已结束，请重新读取歌单。')
          return
        }
        setProgress(value)
        if (!value.running) {
          setError(value.error ?? undefined)
          await reload()
          if (!disposed && !id && value.playlistId && !value.error) {
            const items = await nativeCall<AggregatePlaylist[]>('aggregate_playlists', {
              refresh: true
            })
            const item = items.find((p) => p.id === value.playlistId)
            if (!disposed && item) navigate('local-playlist', '', aggregateCollection(item))
          }
        } else timer = setTimeout(poll, 750)
      } catch (cause) {
        if (!disposed) {
          setError(errorText(cause))
          timer = setTimeout(poll, 2000)
        }
      }
    }
    timer = setTimeout(poll, 300)
    return () => {
      disposed = true
      clearTimeout(timer)
    }
  }, [progress?.key, progress?.running])
  return (
    <div className="flex flex-wrap items-center gap-2">
      <ActionButton
        variant="outline"
        disabled={starting || progress?.running}
        onClick={() => void start(true)}
      >
        {id ? <RefreshCw aria-hidden="true" /> : <GitFork aria-hidden="true" />}
        {starting
          ? '正在准备…'
          : progress?.running
            ? `正在读取 ${progress.count} 首…`
            : id
              ? '立即同步'
              : '派生为聚合歌单'}
      </ActionButton>
      {progress?.running && (
        <ActionButton
          variant="ghost"
          onClick={() =>
            void nativeCall('aggregate_sync_cancel', { key: progress.key }).catch((cause) =>
              setError(errorText(cause))
            )
          }
        >
          <X aria-hidden="true" />
          取消同步
        </ActionButton>
      )}
      <ErrorNotice error={error} />
    </div>
  )
}
