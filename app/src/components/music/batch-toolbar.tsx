import { Menu } from '@base-ui/react/menu'
import {
  ArrowLeftRight,
  Check,
  Download,
  Ellipsis,
  FolderPlus,
  ListChecks,
  ListEnd
} from 'lucide-react'
import { useMemo, useRef, useState, type ReactNode } from 'react'
import { isWindows } from '@/lib/platform'
import { nativeCall, type Track } from '@/lib/player'
import { reportCaughtError } from '@/lib/diagnostics'
import { useDownloads } from '@/features/downloads/download-provider'
import { ActionButton } from './action-button'
import { useSongActions } from './song-actions'

export function useTrackBatch(tracks: Track[]) {
  const [active, setActive] = useState(false)
  const [selected, setSelected] = useState<Set<string>>(new Set())
  const visible = useRef<string[]>([])
  const selectedTracks = useMemo(
    () => tracks.filter((track) => selected.has(track.key)),
    [tracks, selected]
  )
  const enter = () => {
    visible.current = []
    setSelected(new Set())
    setActive(true)
  }
  const exit = () => {
    visible.current = []
    setSelected(new Set())
    setActive(false)
  }
  const onToggle = (key: string, checked: boolean) =>
    setSelected((old) => {
      const next = new Set(old)
      if (checked) next.add(key)
      else next.delete(key)
      return next
    })
  const onToggleAll = (keys: string[], checked: boolean) =>
    setSelected((old) => {
      const next = new Set(old)
      for (const key of keys) {
        if (checked) next.add(key)
        else next.delete(key)
      }
      return next
    })
  const invert = () =>
    setSelected((old) => {
      const next = new Set(old)
      for (const key of visible.current) {
        if (next.has(key)) next.delete(key)
        else next.add(key)
      }
      return next
    })
  const selection = useMemo(
    () => ({
      keys: selected,
      onToggle,
      onToggleAll,
      onVisibleChange: (keys: string[]) => {
        visible.current = keys
      }
    }),
    [selected]
  )
  return {
    active,
    selected,
    selectedTracks,
    selectedCount: selectedTracks.length,
    selection,
    enter,
    exit,
    invert
  }
}

export function BatchMenuItem({
  batch,
  onClick
}: {
  batch: boolean
  onClick: () => void
}) {
  return (
    <Menu.Item onClick={onClick}>
      <ListChecks aria-hidden="true" />
      {batch ? '完成批量操作' : '批量操作'}
    </Menu.Item>
  )
}

export function BatchMenuButton({
  batch,
  onBatch
}: {
  batch: boolean
  onBatch: () => void
}) {
  return (
    <Menu.Root>
      <Menu.Trigger
        render={
          <ActionButton
            variant="ghost"
            size="icon"
            aria-label="批量操作"
            title={batch ? '完成批量操作' : '批量操作'}
          />
        }
      >
        <Ellipsis aria-hidden="true" />
      </Menu.Trigger>
      <Menu.Portal>
        <Menu.Positioner className="z-[70]" side="bottom" align="end" sideOffset={6}>
          <Menu.Popup className="song-context-menu" aria-label="批量操作">
            <BatchMenuItem batch={batch} onClick={onBatch} />
          </Menu.Popup>
        </Menu.Positioner>
      </Menu.Portal>
    </Menu.Root>
  )
}

export function BatchToolbar({
  tracks,
  busy = false,
  onInvert,
  onDone,
  children
}: {
  tracks: Track[]
  busy?: boolean
  onInvert: () => void
  onDone: () => void
  children?: ReactNode
}) {
  const actions = useSongActions()
  const [queueing, setQueueing] = useState(false)
  const disabled = busy || !tracks.length
  const addToQueue = () => {
    if (disabled || queueing) return
    setQueueing(true)
    actions.run(async () => {
      try {
        await nativeCall('append_queue', { keys: tracks.map((track) => track.key) })
      } finally {
        setQueueing(false)
      }
      onDone()
    }, `已将 ${tracks.length} 首歌曲添加到播放列表。`)
  }
  return (
    <div className="mb-4 flex flex-wrap items-center gap-2" role="toolbar" aria-label="批量操作">
      <span className="mr-2 inline-flex items-center gap-2 text-sm text-muted-foreground">
        <ListChecks aria-hidden="true" className="size-4" />
        已选 {tracks.length} 首
      </span>
      <ActionButton variant="outline" disabled={disabled} onClick={onInvert}>
        <ArrowLeftRight aria-hidden="true" />
        反选
      </ActionButton>
      <ActionButton
        variant="outline"
        disabled={disabled}
        onClick={() => actions.collectGroup(tracks)}
      >
        <FolderPlus aria-hidden="true" />
        添加到歌单
      </ActionButton>
      <ActionButton variant="outline" disabled={disabled || queueing} onClick={addToQueue}>
        <ListEnd aria-hidden="true" />
        添加到播放列表
      </ActionButton>
      {children}
      <ActionButton variant="outline" disabled={busy} onClick={onDone}>
        <Check aria-hidden="true" />
        完成
      </ActionButton>
    </div>
  )
}

export function BatchDownloadButton({
  tracks,
  onComplete,
  onError,
  onNotice
}: {
  tracks: Track[]
  onComplete: () => void
  onError: (cause: unknown) => void
  onNotice?: (message: string) => void
}) {
  const downloads = useDownloads()
  const [busy, setBusy] = useState(false)
  const items = tracks.filter((track) => track.source.kind === 'online')
  if (!isWindows()) return null
  const run = async () => {
    if (!items.length || busy) return
    setBusy(true)
    try {
      const results = await downloads.startMany(items)
      let queued = 0
      let skipped = 0
      let failed = 0
      for (const result of results) {
        if (result.error) {
          failed++
          reportCaughtError(
            'app/src/components/music/batch-toolbar.tsx:downloads_add_many',
            new Error(result.error)
          )
        } else if (result.reason) {
          skipped++
        } else {
          queued++
        }
      }
      const skippedText = skipped ? `，跳过 ${skipped} 首重复` : ''
      const failedText = failed ? `，${failed} 首失败` : ''
      onNotice?.(`已加入下载 ${queued} 首${skippedText}${failedText}。`)
      onComplete()
    } catch (cause) {
      reportCaughtError('app/src/components/music/batch-toolbar.tsx', cause)
      onError(cause)
    } finally {
      setBusy(false)
    }
  }
  return (
    <ActionButton variant="outline" disabled={!items.length || busy} onClick={() => void run()}>
      <Download aria-hidden="true" />
      下载
    </ActionButton>
  )
}
