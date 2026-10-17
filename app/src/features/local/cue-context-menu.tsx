import type { ReactElement, ReactNode } from 'react'
import { ContextMenu } from '@base-ui/react/context-menu'
import { Copy, FolderPlus, ListPlus, Play, Trash2 } from 'lucide-react'
import { isTauri } from '@tauri-apps/api/core'
import { nativeCall, type Track } from '@/lib/player'
import { useSongActions } from '@/components/music/song-actions'
import type { CueCardEntry } from './cue-card'

export function CueContextMenu({
  entries,
  busy,
  onPlay,
  onRemove,
  removeLabel = '从列表删除',
  render,
  children
}: {
  entries: CueCardEntry[]
  busy: boolean
  onPlay: () => void
  onRemove?: (track: Track, index: number) => void | Promise<unknown>
  removeLabel?: string
  render: ReactElement
  children: ReactNode
}) {
  const actions = useSongActions()
  const tracks = entries.map(({ track }) => track)
  const source = tracks[0].source
  const disabled = busy || !isTauri()
  return (
    <ContextMenu.Root>
      <ContextMenu.Trigger render={render} tabIndex={0}>
        {children}
      </ContextMenu.Trigger>
      <ContextMenu.Portal>
        <ContextMenu.Positioner className="z-[70]" sideOffset={4}>
          <ContextMenu.Popup
            className="song-context-menu"
            aria-label={`${tracks[0].album} 的 CUE 菜单`}
          >
            <ContextMenu.Item disabled={disabled} onClick={() => actions.run(onPlay)}>
              <Play aria-hidden="true" />
              播放全部分轨
            </ContextMenu.Item>
            <ContextMenu.Item
              disabled={disabled}
              onClick={() =>
                actions.run(
                  () => nativeCall('append_queue', { keys: tracks.map(({ key }) => key) }),
                  '已将 CUE 分轨设为下一首播放。'
                )
              }
            >
              <ListPlus aria-hidden="true" />
              下一首播放
            </ContextMenu.Item>
            <ContextMenu.Item disabled={disabled} onClick={() => actions.collectGroup(tracks)}>
              <FolderPlus aria-hidden="true" />
              添加到本地歌单
            </ContextMenu.Item>
            <ContextMenu.Separator />
            <ContextMenu.Item
              onClick={() =>
                actions.copy(source.kind === 'local' ? (source.cue?.sheetPath ?? source.path) : '')
              }
            >
              <Copy aria-hidden="true" />
              复制 CUE 文件路径
            </ContextMenu.Item>
            {onRemove && (
              <ContextMenu.Item
                className="song-menu-remove"
                disabled={disabled}
                onClick={() =>
                  actions.run(async () => {
                    for (const entry of entries) await onRemove(entry.track, entry.index)
                  }, '已移除 CUE 分轨。')
                }
              >
                <Trash2 aria-hidden="true" />
                {removeLabel}
              </ContextMenu.Item>
            )}
          </ContextMenu.Popup>
        </ContextMenu.Positioner>
      </ContextMenu.Portal>
    </ContextMenu.Root>
  )
}
