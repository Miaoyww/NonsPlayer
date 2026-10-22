import { Menu } from '@base-ui/react/menu'
import {
  Ellipsis,
  Unlink,
  Trash2,
  Pencil,
  ExternalLink,
  RefreshCw,
  X,
  ListFilter,
  ListChecks
} from 'lucide-react'
import { ActionButton } from '@/components/music/action-button'
import { ErrorNotice } from '@/components/error-notice'
import { CollectionHeader } from '@/features/library/collection-header'
import { useMusicNavigation } from '@/features/workspace/music-navigation'
import { aggregateCollection, type AggregatePlaylist } from './aggregate-library'
import { AggregateCover } from './aggregate-cover'
import { usePlaylistSync } from './sync-control'

export function AggregateHeader({
  playlist,
  busy,
  onPlay,
  onEdit,
  onExclusions,
  onBatch,
  batch = false,
  excluded = false
}: {
  playlist: AggregatePlaylist
  busy: boolean
  onPlay: () => void
  onEdit: (operation: 'rename' | 'delete' | 'unlink') => void
  excluded?: boolean
  onBatch?: () => void
  batch?: boolean
  onExclusions: () => void
}) {
  const { navigate } = useMusicNavigation()
  const sync = usePlaylistSync(playlist.id, !!playlist.link)
  return (
    <>
      <CollectionHeader
        collection={{ ...aggregateCollection(playlist), subtitle: '' }}
        kindLabel="聚合歌单"
        total={playlist.trackCount}
        busy={busy}
        disabled={busy || !playlist.trackCount}
        onPlay={onPlay}
        cover={<AggregateCover playlist={playlist} className="library-detail-cover" />}
        actions={
          <>
            {playlist.link && (
              <ActionButton variant="outline" disabled={busy} onClick={() => onEdit('unlink')}>
                <Unlink aria-hidden="true" />
                解除关联
              </ActionButton>
            )}
            <Menu.Root>
              <Menu.Trigger
                disabled={busy}
                render={
                  <ActionButton
                    variant="ghost"
                    size="icon"
                    aria-label="歌单菜单"
                    title="歌单菜单"
                  />
                }
              >
                <Ellipsis aria-hidden="true" />
              </Menu.Trigger>
              <Menu.Portal>
                <Menu.Positioner className="z-[70]" side="bottom" align="end" sideOffset={6}>
                  <Menu.Popup className="song-context-menu" aria-label="歌单操作">
                    <Menu.Item onClick={onBatch}>
                      <ListChecks aria-hidden="true" />
                      {batch ? '完成批量操作' : '批量操作'}
                    </Menu.Item>
                    {!playlist.link && (
                      <Menu.Item onClick={() => onEdit('rename')}>
                        <Pencil aria-hidden="true" />
                        修改名称
                      </Menu.Item>
                    )}
                    {playlist.link && (
                      <>
                        <Menu.Item
                          onClick={() =>
                            navigate('collection', '', {
                              ...aggregateCollection(playlist),
                              id: playlist.link!.reference.id,
                              localId: undefined,
                              reference: playlist.link!.reference,
                              bypassAggregate: true
                            })
                          }
                        >
                          <ExternalLink aria-hidden="true" />
                          查看原歌单
                        </Menu.Item>
                        <Menu.Item
                          disabled={sync.starting || sync.progress?.running}
                          onClick={() => void sync.start(true)}
                        >
                          <RefreshCw aria-hidden="true" />
                          从原歌单更新
                        </Menu.Item>
                        {sync.progress?.running && (
                          <Menu.Item onClick={() => void sync.cancel()}>
                            <X aria-hidden="true" />
                            取消更新
                          </Menu.Item>
                        )}
                        <Menu.Item onClick={onExclusions}>
                          <ListFilter aria-hidden="true" />
                          {excluded ? '返回歌曲' : '管理同步排除'}
                        </Menu.Item>
                        <Menu.Separator />
                      </>
                    )}
                    <Menu.Item className="song-menu-remove" onClick={() => onEdit('delete')}>
                      <Trash2 aria-hidden="true" />
                      删除本机歌单
                    </Menu.Item>
                  </Menu.Popup>
                </Menu.Positioner>
              </Menu.Portal>
            </Menu.Root>
          </>
        }
      />
      {playlist.link && (
        <div className="mb-6 text-xs text-muted-foreground" role="status">
          {sync.progress?.running
            ? `正在从原歌单读取 ${sync.progress.count} 首…`
            : playlist.link.lastSuccess
              ? `上次从原歌单更新：${new Date(playlist.link.lastSuccess * 1000).toLocaleString()}`
              : '尚未更新'}
          <ErrorNotice error={sync.error ?? playlist.link.error ?? undefined} />
        </div>
      )}
    </>
  )
}
