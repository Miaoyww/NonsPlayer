import { reportCaughtError } from '@/lib/diagnostics'
import { createContext, useContext, useEffect, useState, type ReactNode } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { Track } from '@/lib/player'
import { nativeCall } from '@/lib/player'
import { isWindows } from '@/lib/platform'
import { DownloadDialog } from './download-dialog'

export type DownloadState = 'waiting' | 'running' | 'completed' | 'failed' | 'cancelled'
export type DownloadSkipReason = 'alreadyQueued' | 'alreadyDownloaded'

export interface DownloadTask {
  id: number
  track: Track
  state: DownloadState
  quality: string
  actualQuality?: string
  bytes: number
  total?: number
  path?: string
  error?: string
  warning?: string
  completedAt?: number
}

export interface DownloadEnqueueResult {
  trackKey: string
  task?: DownloadTask
  reason?: DownloadSkipReason
  error?: string
}

interface DownloadContextValue {
  tasks: DownloadTask[]
  open: boolean
  setOpen: (open: boolean) => void
  start: (track: Track) => Promise<string>
  startMany: (tracks: Track[]) => Promise<DownloadEnqueueResult[]>
  cancel: (id: number) => Promise<void>
  retry: (id: number) => Promise<void>
  remove: (id: number) => Promise<void>
  reveal: (id: number) => Promise<void>
  clearFinished: () => Promise<void>
}

const DownloadContext = createContext<DownloadContextValue | null>(null)

export function DownloadProvider({ children }: { children: ReactNode }) {
  const [tasks, setTasks] = useState<DownloadTask[]>([])
  const [open, setOpen] = useState(false)

  useEffect(() => {
    if (!isTauri() || !isWindows()) return
    let disposed = false
    const changed = listen<DownloadTask[]>('downloads-changed', ({ payload }) => {
      if (!disposed) setTasks(payload)
    })
    const progress = listen<{ id: number; bytes: number; total?: number }>(
      'downloads-progress',
      ({ payload }) => {
        if (disposed) return
        setTasks((current) =>
          current.map((task) =>
            task.id === payload.id
              ? { ...task, bytes: payload.bytes, total: payload.total ?? task.total }
              : task
          )
        )
      }
    )
    void Promise.all([changed, progress])
      .then(async ([stopChanged, stopProgress]) => {
        if (disposed) {
          stopChanged()
          stopProgress()
          return
        }
        const list = await nativeCall<DownloadTask[]>('downloads_list')
        if (!disposed) setTasks(list)
      })
      .catch((cause) => {
        reportCaughtError('app/src/features/downloads/download-provider.tsx', cause)
      })
    return () => {
      disposed = true
      void changed.then((stop) => stop()).catch(() => {})
      void progress.then((stop) => stop()).catch(() => {})
    }
  }, [])

  async function refresh() {
    const list = await nativeCall<DownloadTask[]>('downloads_list')
    setTasks(list)
  }

  async function startMany(tracks: Track[]) {
    const results = await nativeCall<DownloadEnqueueResult[]>('downloads_add_many', { tracks })
    const returned = results.flatMap((result) => (result.task ? [result.task] : []))
    if (returned.length) {
      const ids = new Set(returned.map((task) => task.id))
      setTasks((current) => [...current.filter((task) => !ids.has(task.id)), ...returned])
      setOpen(true)
    }
    return results
  }

  async function start(track: Track) {
    const [result] = await startMany([track])
    if (result?.error) throw new Error(result.error)
    if (!result?.task) throw new Error('没有可加入的下载任务')
    return result.reason === 'alreadyDownloaded' ? '该歌曲已下载' : `已加入下载：${track.title}`
  }

  async function run(action: () => Promise<unknown>) {
    await action()
    await refresh()
  }

  return (
    <DownloadContext.Provider
      value={{
        tasks,
        open,
        setOpen,
        start,
        startMany,
        cancel: (id) => run(() => nativeCall('downloads_cancel', { id })),
        retry: (id) => run(() => nativeCall('downloads_retry', { id })),
        remove: (id) => run(() => nativeCall('downloads_remove', { id })),
        reveal: (id) => run(() => nativeCall('downloads_reveal', { id })),
        clearFinished: () => run(() => nativeCall('downloads_clear_finished'))
      }}
    >
      {children}
      <DownloadDialog />
    </DownloadContext.Provider>
  )
}

export function useDownloads() {
  const value = useContext(DownloadContext)
  if (!value) throw new Error('DownloadProvider is required')
  return value
}
