import { reportCaughtError } from '@/lib/diagnostics'
import { useCallback, useState } from 'react'
import { MusicPage, MusicPageHeader } from '@/components/music/music-page'
import { ActionButton } from '@/components/music/action-button'
import { InfiniteLoad } from '@/components/music/infinite-load'
import { AlbumCard } from '@/features/library/album-card'
import { PlaylistCard } from '@/features/library/playlist-card'
import { useMusicNavigation } from '@/features/workspace/music-navigation'
import { usePagedList } from '@/lib/use-paged-list'
import { isTauri } from '@tauri-apps/api/core'
import { musicClient } from './client'
import { musicCollection } from './entities'
import type { MusicCollection } from '@/features/workspace/music-navigation'
import { playLibraryCollection } from '@/features/library/library-api'

// Platforms use their own library/recommendation operations. Optional unsupported
// operations remain visible as errors instead of falling back to another platform.
export default function SourceWorkspace({
  source,
  name,
  discovery,
  onError,
  onNotice
}: {
  source: string
  name: string
  discovery: boolean
  onError: (cause: unknown) => void
  onNotice: (message: string) => void
}) {
  const { navigate } = useMusicNavigation()
  const [kind, setKind] = useState<'playlist' | 'album' | 'artist'>('playlist')
  return (
    <MusicPage>
      <MusicPageHeader title={discovery ? '发现音乐' : '音乐库'}>
        <p>{name}</p>
      </MusicPageHeader>
      {!discovery && (
        <div className="mb-4 flex gap-2" role="group" aria-label="音乐库类型">
          {(['playlist', 'album', 'artist'] as const).map((value) => (
            <ActionButton
              key={value}
              size="sm"
              aria-pressed={kind === value}
              variant={kind === value ? 'secondary' : 'ghost'}
              onClick={() => setKind(value)}
            >
              {{ playlist: '歌单', album: '专辑', artist: '歌手' }[value]}
            </ActionButton>
          ))}
        </div>
      )}
      <SourceCollections
        key={kind}
        source={source}
        kind={kind}
        discovery={discovery}
        name={name}
        onError={onError}
        onNotice={onNotice}
      />
      <ActionButton variant="ghost" onClick={() => navigate('accounts', source)}>
        管理{name}账号
      </ActionButton>
    </MusicPage>
  )
}

function SourceCollections({
  source,
  kind,
  discovery,
  name,
  onError,
  onNotice
}: {
  source: string
  kind: 'playlist' | 'album' | 'artist'
  discovery: boolean
  name: string
  onError: (cause: unknown) => void
  onNotice: (message: string) => void
}) {
  const { navigate } = useMusicNavigation()
  const [playing, setPlaying] = useState(false)
  // usePagedList asks for offsets; this closure keeps the adapter's opaque cursor.
  const loader = useCallback(
    (() => {
      let cursor: string | null = null
      return async (offset: number) => {
        if (offset === 0) cursor = null
        const response = await musicClient.query(
          source,
          discovery
            ? {
                operation: 'recommendedPlaylists',
                section: 'recommended',
                category: '全部',
                order: 'hot',
                page: { cursor, limit: 30 }
              }
            : { operation: 'libraryCollections', kind, filter: 'all', page: { cursor, limit: 30 } }
        )
        if (response.type !== 'entities') throw new Error('音乐来源返回了错误的收藏列表')
        if (response.data.nextCursor !== null && response.data.nextCursor === cursor)
          throw new Error('音乐来源返回了重复游标')
        cursor = response.data.nextCursor
        return { items: response.data.items.map(musicCollection), more: cursor !== null }
      }
    })(),
    [source, discovery, kind]
  )
  const list = usePagedList(loader, 30, isTauri())
  async function play(item: MusicCollection) {
    if (playing) return
    setPlaying(true)
    try {
      if (await playLibraryCollection(item)) onNotice('已将前 1000 首歌曲加入播放队列。')
    } catch (cause) {
      reportCaughtError('app/src/features/music/source-workspace.tsx', cause)

      onError(cause)
    } finally {
      setPlaying(false)
    }
  }
  const open = (item: MusicCollection) => navigate('collection', '', item)
  return (
    <>
      <div className="discover-playlist-grid">
        {list.items.map((item) =>
          kind === 'album' ? (
            <AlbumCard
              key={JSON.stringify(item.reference)}
              item={item}
              sourceName={name}
              busy={playing}
              onOpen={open}
              onPlay={(item) => void play(item)}
            />
          ) : (
            <PlaylistCard
              key={JSON.stringify(item.reference)}
              item={item}
              sourceName={name}
              busy={playing}
              onOpen={open}
              onPlay={(item) => void play(item)}
            />
          )
        )}
      </div>
      {!list.busy && !list.error && !list.items.length && (
        <p className="py-6 text-sm text-muted-foreground">暂无内容</p>
      )}
      <InfiniteLoad more={list.more} busy={list.busy} error={list.error} onLoad={list.loadMore} />
    </>
  )
}
