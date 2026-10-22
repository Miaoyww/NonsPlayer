import { useEffect, useRef, useState } from 'react'
import { nativeCall, errorText } from '@/lib/player'
import { useMusicSources } from '@/features/music/use-music-sources'
import { useAggregateLibrary, type SyncProgress } from './aggregate-library'

export function usePlaylistSync(id: string, automatic = false) {
  const [progress, setProgress] = useState<SyncProgress>()
  const [error, setError] = useState<string>()
  const [starting, setStarting] = useState(false)
  const mounted = useRef(true)
  const request = useRef(0)
  const { reload } = useAggregateLibrary()
  const { revision: sourceRevision } = useMusicSources()
  async function start(force: boolean) {
    const serial = ++request.current
    setStarting(true)
    setError(undefined)
    try {
      const value = await nativeCall<SyncProgress>('aggregate_sync', { id, force })
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
  const cancel = () =>
    nativeCall('aggregate_sync_cancel', { key: progress!.key }).catch((cause) =>
      setError(errorText(cause))
    )
  return { progress, error, starting, start, cancel }
}
