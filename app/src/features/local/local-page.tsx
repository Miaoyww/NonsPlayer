import { presentationKey } from '@/lib/runtime-cache'
import { reportCaughtError } from '@/lib/diagnostics'
import { ErrorNotice } from '@/components/error-notice'
import { TrackListSkeleton, CollectionGridSkeleton } from '@/components/music/loading'
import { CollectionContent } from '@/features/library/collection-content'
import { CollectionHeader } from '@/features/library/collection-header'
import { ArtistCard } from '@/features/library/artist-card'
import { TrackArtists } from '@/components/music/music-links'
import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { Tabs } from '@base-ui/react/tabs'
import { ContextMenu } from '@base-ui/react/context-menu'
import {
  Tabs as MusicTabs,
  TabsList,
  TabsTab,
  TabsPanel
} from '@/components/animate-ui/components/base/tabs'
import {
  Disc3,
  FolderOpen,
  ListMusic,
  Mic2,
  Music2,
  Pencil,
  Play,
  Plus,
  Search,
  Trash2,
  X
} from 'lucide-react'
import { isAndroid } from '@/lib/platform'
import { nativeCall, errorText, usePlayer, type Track } from '@/lib/player'
import { localCollection, playLocalEntity, type LocalEntity } from '@/features/local/local-library'
import { usePagedList } from '@/lib/use-paged-list'
import { FolderManager } from '@/features/local/folder-manager'
import { Input } from '@/components/ui/input'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription
} from '@/components/ui/dialog'
import { MusicPage, MusicPageHeader } from '@/components/music/music-page'
import { useMusicNavigation } from '@/features/workspace/music-navigation'
import { ActionButton } from '@/components/music/action-button'
import { Cover } from '@/components/music/cover'
import { TrackList } from '@/components/music/track-list'
import { InfiniteLoad } from '@/components/music/infinite-load'
import { CreateAggregatePlaylist } from '@/features/aggregate/aggregate-page'
import {
  ArtistAvatarActionsProvider,
  ArtistAvatarContextItems,
  ArtistAvatarDetailMenu
} from '@/features/local/artist-avatar/actions'
import { ArtistAvatarCompletionButton } from '@/features/local/artist-avatar/completion-dialog'
import { ArtistAlphabetIndex } from '@/features/local/artist-alphabet-index'
import {
  artistIndexLetter,
  groupArtists,
  type ArtistIndexLetter
} from '@/features/local/artist-index'

const categories = [
  { value: 'song', label: '音乐', icon: Music2 },
  { value: 'artist', label: '艺术家', icon: Mic2 },
  { value: 'album', label: '专辑', icon: Disc3 },
  { value: 'playlist', label: '歌单', icon: ListMusic }
] as const
type Kind = (typeof categories)[number]['value']
interface Props {
  refresh: number
  importing: boolean
  onImport: () => void
  onError: (cause: unknown) => void
  onNotice: (message: string) => void
}
interface ResultProps {
  sectionTitle?: string
  kind: Kind
  keyword: string
  refresh: number
  onError: Props['onError']
  onNotice: Props['onNotice']
  list: ReturnType<typeof useLocalResults>
}
function useLocalResults(kind: Kind, keyword: string, refresh: number) {
  const loader = useCallback(
    async (offset: number) => {
      const page = await (kind === 'song'
        ? nativeCall<{ items: Track[]; more: boolean }>('local_entity_tracks', {
            kind: 'song',
            id: '',
            keyword,
            offset
          })
        : nativeCall<{ items: LocalEntity[]; more: boolean }>('local_entities', {
            kind,
            keyword,
            offset
          }))
      return { ...page, metadata: true as const }
    },
    [kind, keyword, refresh]
  )
  return usePagedList<Track | LocalEntity, true>(loader, kind === 'song' ? 100 : 30, isTauri(), {
    cacheKey: presentationKey('local', kind, keyword)
  })
}

function BrowseResults(props: Omit<ResultProps, 'list'>) {
  const list = useLocalResults(props.kind, props.keyword, props.refresh)
  return <Results {...props} list={list} />
}

function Results({ sectionTitle, kind, keyword, list, onError, onNotice }: ResultProps) {
  const { navigate } = useMusicNavigation()
  const player = usePlayer('index', 'queue')
  const [playing, setPlaying] = useState(false)
  const isArtistBrowse = kind === 'artist' && !keyword && !sectionTitle
  const artistGroups = useMemo(
    () => (isArtistBrowse ? groupArtists(list.items as LocalEntity[]) : []),
    [isArtistBrowse, list.items]
  )
  const availableLetters = useMemo(
    () => new Set(artistGroups.map((group) => group.letter)),
    [artistGroups]
  )
  const [activeLetter, setActiveLetter] = useState<ArtistIndexLetter>()
  const [pendingLetter, setPendingLetter] = useState<ArtistIndexLetter>()
  const groupRefs = useRef(new Map<ArtistIndexLetter, HTMLElement>())
  const indexContainer = useRef<HTMLDivElement>(null)
  async function play(item: LocalEntity) {
    if (playing) return
    setPlaying(true)
    try {
      if (await playLocalEntity(item.kind, item.id)) onNotice('已将前 1000 首歌曲加入播放队列。')
    } catch (cause) {
      reportCaughtError('app/src/features/local/local-page.tsx', cause)

      onError(cause)
    } finally {
      setPlaying(false)
    }
  }
  async function selectArtistLetter(letter: ArtistIndexLetter) {
    if (pendingLetter || list.busy) return
    setActiveLetter(letter)
    setPendingLetter(letter)
    if (availableLetters.has(letter) || !list.more) return
    let more: boolean = list.more
    while (more) {
      const page = await list.loadMore()
      if (!page?.items.length) break
      if (page.items.some((item) => artistIndexLetter((item as LocalEntity).name) === letter)) break
      more = page.more
    }
  }
  const tracks = list.items as Track[]
  useEffect(() => {
    if (!isArtistBrowse) return
    if (!artistGroups.length) {
      setActiveLetter(undefined)
      return
    }
    if (!activeLetter || !availableLetters.has(activeLetter)) {
      setActiveLetter(artistGroups[0].letter)
    }
  }, [activeLetter, artistGroups, availableLetters, isArtistBrowse])
  useEffect(() => {
    if (!isArtistBrowse) return
    const container = indexContainer.current
    const scroller = container?.closest('.music-library')
    if (!(scroller instanceof HTMLElement)) return
    let frame = 0
    const update = () => {
      frame = 0
      const threshold = scroller.getBoundingClientRect().top + 32
      let current = artistGroups[0]?.letter
      for (const group of artistGroups) {
        const element = groupRefs.current.get(group.letter)
        if (element && element.getBoundingClientRect().top <= threshold) current = group.letter
      }
      if (current && !pendingLetter) setActiveLetter(current)
    }
    const schedule = () => {
      if (!frame) frame = requestAnimationFrame(update)
    }
    scroller.addEventListener('scroll', schedule, { passive: true })
    update()
    return () => {
      scroller.removeEventListener('scroll', schedule)
      if (frame) cancelAnimationFrame(frame)
    }
  }, [artistGroups, isArtistBrowse, pendingLetter])
  useEffect(() => {
    if (!pendingLetter) return
    const element = groupRefs.current.get(pendingLetter)
    if (element) {
      element.scrollIntoView({ behavior: 'smooth', block: 'start' })
      setActiveLetter(pendingLetter)
      setPendingLetter(undefined)
    } else if (!list.more || list.error) {
      setPendingLetter(undefined)
    }
  }, [artistGroups, list.error, list.more, pendingLetter])
  const openEntity = (item: LocalEntity) =>
    navigate(
      item.kind === 'artist'
        ? 'local-artist'
        : item.kind === 'album'
          ? 'local-album'
          : 'local-playlist',
      '',
      localCollection(item)
    )
  const artistCard = (item: LocalEntity) => (
    <ArtistCard
      key={item.id}
      name={item.name}
      cover={item.cover}
      meta={`${item.trackCount} 首音乐${item.subtitle ? ` · ${item.subtitle}` : ''}`}
      onOpen={() => openEntity(item)}
      menu={<ArtistAvatarContextItems artistId={item.id} artistName={item.name} />}
    />
  )
  if (sectionTitle && !list.items.length && !list.busy && !list.error) return null
  const content = (
    <>
      {list.busy && !list.items.length ? (
        kind === 'song' ? (
          <TrackListSkeleton online={false} />
        ) : (
          <CollectionGridSkeleton artist={kind === 'artist'} />
        )
      ) : list.items.length > 0 ? (
        kind === 'song' ? (
          <TrackList
            tracks={tracks}
            hasMore={list.more}
            sortable
            busy={!isTauri()}
            currentKey={player.index === null ? undefined : player.queue[player.index]?.key}
            onPlay={(index, tracks) =>
              void nativeCall('play_queue', {
                keys: tracks
                  .slice(index >= 1000 ? index : 0, (index >= 1000 ? index : 0) + 1000)
                  .map((t) => t.key),
                index: index >= 1000 ? 0 : index
              }).catch(onError)
            }
            onAppend={(track) =>
              void nativeCall('append_queue', { keys: [track.key] })
                .then(() => onNotice('已设为下一首播放。'))
                .catch(onError)
            }
          />
        ) : isArtistBrowse ? (
          <div className="local-artist-browser" ref={indexContainer}>
            <div className="local-artist-sections">
              {artistGroups.map((group) => (
                <section
                  key={group.letter}
                  className="local-artist-section"
                  data-artist-letter={group.letter}
                  ref={(node) => {
                    if (node) groupRefs.current.set(group.letter, node)
                    else groupRefs.current.delete(group.letter)
                  }}
                >
                  <h2
                    className="local-artist-section-letter"
                    aria-label={group.letter === '#' ? '其他' : group.letter}
                  >
                    {group.letter}
                  </h2>
                  <div className="library-cover-grid">{group.items.map(artistCard)}</div>
                </section>
              ))}
            </div>
            <ArtistAlphabetIndex
              active={activeLetter}
              available={availableLetters}
              canLoadMore={list.more}
              disabled={list.busy}
              pending={pendingLetter}
              onSelect={(letter) => void selectArtistLetter(letter)}
            />
          </div>
        ) : (
          <div className="library-cover-grid">
            {(list.items as LocalEntity[]).map((item) => {
              const open = () => openEntity(item)
              if (kind === 'artist') {
                return artistCard(item)
              }
              const body = (
                <>
                  <div className="library-cover-art">
                    <button
                      type="button"
                      className="library-cover-open"
                      onClick={open}
                      aria-label={`打开${item.name}`}
                    >
                      <Cover cover={item.cover} className="aspect-square w-full" />
                    </button>
                    <ActionButton
                      size="icon-lg"
                      className="library-cover-play"
                      disabled={playing || !isTauri() || !item.trackCount}
                      aria-label={`播放${item.name}`}
                      onClick={() => void play(item)}
                    >
                      <Play aria-hidden="true" />
                    </ActionButton>
                  </div>
                  <button type="button" className="library-cover-title" onClick={open}>
                    {item.name}
                  </button>
                  <p className="library-cover-subtitle">
                    {item.trackCount} 首音乐{item.subtitle && ` · ${item.subtitle}`}
                  </p>
                </>
              )
              const openFolder =
                item.kind === 'album' && !isAndroid()
                  ? () =>
                      void nativeCall('open_local_entity_folder', {
                        kind: 'album',
                        id: item.id
                      }).catch(onError)
                  : undefined
              return openFolder ? (
                <ContextMenu.Root key={item.id}>
                  <ContextMenu.Trigger render={<article className="library-cover-card" />}>
                    {body}
                  </ContextMenu.Trigger>
                  <ContextMenu.Portal>
                    <ContextMenu.Positioner className="z-[70]" sideOffset={4}>
                      <ContextMenu.Popup
                        className="song-context-menu"
                        aria-label={`${item.name} 的菜单`}
                      >
                        <ContextMenu.Item
                          disabled={!isTauri() || !item.trackCount}
                          onClick={openFolder}
                        >
                          <FolderOpen aria-hidden="true" />
                          打开文件所在位置
                        </ContextMenu.Item>
                      </ContextMenu.Popup>
                    </ContextMenu.Positioner>
                  </ContextMenu.Portal>
                </ContextMenu.Root>
              ) : (
                <article key={item.id} className="library-cover-card">
                  {body}
                </article>
              )
            })}
          </div>
        )
      ) : (
        !list.error && (
          <div className="library-empty" role="status">
            <Music2 aria-hidden="true" />
            <p>
              {list.busy
                ? '正在读取…'
                : keyword
                  ? '没有找到匹配结果'
                  : kind === 'playlist'
                    ? '创建歌单，整理你喜欢的本地音乐'
                    : '打开音乐文件，或添加音乐文件夹'}
            </p>
          </div>
        )
      )}
      <InfiniteLoad more={list.more} busy={list.busy} error={list.error} onLoad={list.loadMore} />
    </>
  )
  return sectionTitle ? (
    <section className="local-result-section">
      <h2 className="mb-4 text-xl font-semibold">{sectionTitle}</h2>
      {content}
    </section>
  ) : (
    content
  )
}

function LocalSearchResults({
  keyword,
  refresh,
  onError,
  onNotice
}: Omit<ResultProps, 'kind' | 'list'>) {
  const [resultKind, setResultKind] = useState('all')
  const song = useLocalResults('song', keyword, refresh)
  const artist = useLocalResults('artist', keyword, refresh)
  const album = useLocalResults('album', keyword, refresh)
  const lists = { song, artist, album }
  const empty = Object.values(lists).every(
    (list) => list.metadata && !list.busy && !list.error && !list.more && !list.items.length
  )
  if (empty) {
    return (
      <div className="search-empty" role="status">
        <Search aria-hidden="true" />
        <p>没有找到与“{keyword}”匹配的本地音乐、艺术家或专辑</p>
        <span>试试其他关键词，或添加音乐文件和文件夹。</span>
      </div>
    )
  }
  return (
    <MusicTabs
      orientation="vertical"
      value={resultKind}
      onValueChange={setResultKind}
      className="local-search-results"
    >
      <TabsList className="music-tabs local-search-nav" aria-label="本地搜索结果类型">
        {[{ value: 'all', label: '所有', icon: Search }, ...categories.slice(0, 3)].map(
          ({ value, label, icon: Icon }) => (
            <TabsTab key={value} value={value}>
              <Icon aria-hidden="true" />
              {label}
            </TabsTab>
          )
        )}
      </TabsList>
      <div className="min-w-0 flex-1">
        <TabsPanel value="all" transition={{ duration: 0.15 }}>
          {categories.slice(0, 3).map(({ value, label }) => (
            <Results
              key={value}
              sectionTitle={label}
              kind={value}
              keyword={keyword}
              refresh={refresh}
              list={lists[value as keyof typeof lists]}
              onError={onError}
              onNotice={onNotice}
            />
          ))}
        </TabsPanel>
        {categories.slice(0, 3).map(({ value }) => (
          <TabsPanel key={value} value={value} transition={{ duration: 0.15 }}>
            <Results
              kind={value}
              keyword={keyword}
              refresh={refresh}
              list={lists[value as keyof typeof lists]}
              onError={onError}
              onNotice={onNotice}
            />
          </TabsPanel>
        ))}
      </div>
    </MusicTabs>
  )
}

export default function LocalPage({ refresh, importing, onImport, onError, onNotice }: Props) {
  const { page, navigate } = useMusicNavigation()
  const [kind, setKind] = useState<Kind>('song')
  const [input, setInput] = useState(page.query)
  const [revision, setRevision] = useState(0)
  const [editing, setEditing] = useState<'create' | 'rename' | 'delete'>()
  const [name, setName] = useState('')
  const [saving, setSaving] = useState(false)
  const [dialogError, setDialogError] = useState<string>()
  const deleting = saving && editing === 'delete'
  useEffect(() => setDialogError(undefined), [editing])
  useEffect(() => {
    setInput(page.query)
  }, [page.query])
  const initialCollection = page.view !== 'local' ? page.collection : undefined
  const [detail, setDetail] = useState<{ identity: string; item: LocalEntity }>()
  const identity = `${initialCollection?.kind}:${initialCollection?.localId}`
  useEffect(() => {
    if (!initialCollection || !isTauri() || deleting) return
    let disposed = false
    void nativeCall<LocalEntity>('local_entity_detail', {
      kind: initialCollection.kind,
      id: initialCollection.localId
    })
      .then((item) => {
        if (!disposed) setDetail({ identity, item })
      })
      .catch(onError)
    return () => {
      disposed = true
    }
  }, [initialCollection, identity, refresh, revision, deleting, onError])
  const collection =
    detail?.identity === identity ? localCollection(detail.item) : initialCollection
  const loader = useCallback(
    (offset: number) =>
      nativeCall<{ items: Track[]; more: boolean }>('local_entity_tracks', {
        kind: collection?.kind ?? 'song',
        id: collection?.localId ?? '',
        keyword: '',
        offset
      }),
    [collection?.kind, collection?.localId, refresh, revision]
  )
  const list = usePagedList<Track>(loader, 100, isTauri() && !!collection && !deleting, {
    cacheKey: presentationKey('local-detail', collection?.kind, collection?.localId)
  })
  const player = usePlayer('index', 'queue')
  const [playing, setPlaying] = useState(false)
  async function play() {
    if (!collection || playing) return
    setPlaying(true)
    try {
      if (await playLocalEntity(collection.kind, collection.localId!))
        onNotice('已将前 1000 首歌曲加入播放队列。')
    } catch (cause) {
      reportCaughtError('app/src/features/local/local-page.tsx', cause)

      onError(cause)
    } finally {
      setPlaying(false)
    }
  }
  async function save() {
    if (saving) return
    setSaving(true)
    setDialogError(undefined)
    try {
      if (editing === 'create') {
        const id = await nativeCall<string>('create_local_playlist', { name })
        navigate('local-playlist', '', {
          id: 0,
          localId: id,
          reference: null,
          kind: 'playlist',
          playlistType: 'local',
          name: name.trim(),
          cover: '',
          subtitle: '',
          trackCount: 0
        })
      } else if (editing === 'rename') {
        await nativeCall('rename_local_playlist', { id: collection!.localId, name })
        navigate('local-playlist', '', { ...collection!, name: name.trim() })
      } else {
        await nativeCall('delete_local_playlist', { id: collection!.localId })
        navigate('local')
        setKind('playlist')
      }
      setRevision((v) => v + 1)
      setEditing(undefined)
    } catch (cause) {
      reportCaughtError('app/src/features/local/local-page.tsx', cause)

      setDialogError(errorText(cause))
    } finally {
      setSaving(false)
    }
  }
  const version = refresh + revision
  return (
    <ArtistAvatarActionsProvider onError={onError} onNotice={onNotice}>
      <MusicPage aria-label="本地音乐" className="local-page">
        {collection ? (
          <CollectionContent
            key={`${collection.kind}:${collection.localId ?? collection.id}`}
            hasMore={list.more}
          >
            <CollectionHeader
              collection={collection}
              total={collection.trackCount}
              busy={playing}
              disabled={playing || !isTauri() || !list.items.length}
              onPlay={() => void play()}
              cover={
                <Cover
                  cover={collection.cover || list.items[0]?.cover}
                  className={`library-detail-cover ${collection.kind === 'artist' ? 'rounded-full' : 'rounded-xl'}`}
                />
              }
              credits={
                collection.kind === 'album' && collection.subtitle && list.items[0] ? (
                  <TrackArtists track={{ ...list.items[0], artist: collection.subtitle }} />
                ) : undefined
              }
              actions={
                collection.kind === 'album' ? (
                  !isAndroid() && (
                    <ActionButton
                      variant="ghost"
                      disabled={playing || !isTauri() || !list.items.length}
                      onClick={() =>
                        void nativeCall('open_local_entity_folder', {
                          kind: 'album',
                          id: collection.localId
                        }).catch(onError)
                      }
                    >
                      <FolderOpen aria-hidden="true" />
                      打开文件夹
                    </ActionButton>
                  )
                ) : collection.kind === 'playlist' ? (
                  <>
                    <ActionButton
                      variant="ghost"
                      disabled={!isTauri()}
                      onClick={() => {
                        setName(collection.name)
                        setEditing('rename')
                      }}
                    >
                      <Pencil aria-hidden="true" />
                      改名
                    </ActionButton>
                    <ActionButton
                      variant="ghost"
                      disabled={!isTauri()}
                      onClick={() => setEditing('delete')}
                    >
                      <Trash2 aria-hidden="true" />
                      删除歌单
                    </ActionButton>
                  </>
                ) : collection.kind === 'artist' ? (
                  <ArtistAvatarDetailMenu
                    artistId={collection.localId!}
                    artistName={collection.name}
                  />
                ) : undefined
              }
            />
            {list.busy && !list.items.length ? (
              <TrackListSkeleton online={false} />
            ) : list.items.length > 0 ? (
              <TrackList
                tracks={list.items}
                key={`${collection.kind}:${collection.localId ?? collection.id}`}
                searchable
                sortable
                hasMore={list.more}
                busy={!isTauri()}
                currentKey={player.index === null ? undefined : player.queue[player.index]?.key}
                onPlay={(index, tracks) => {
                  const start = index >= 1000 ? index : 0
                  void nativeCall('play_queue', {
                    keys: tracks.slice(start, start + 1000).map((t) => t.key),
                    index: index - start
                  }).catch(onError)
                }}
                onAppend={(track) =>
                  void nativeCall('append_queue', { keys: [track.key] }).catch(onError)
                }
                onRemove={
                  collection.kind === 'playlist'
                    ? async (track) => {
                        await nativeCall('remove_local_playlist_track', {
                          id: collection.localId,
                          key: track.key
                        })
                        setRevision((v) => v + 1)
                      }
                    : undefined
                }
                removeLabel="从歌单移除"
              />
            ) : (
              !list.error && (
                <div className="library-empty">
                  {list.busy ? (
                    <TrackListSkeleton online={false} />
                  ) : (
                    '还没有歌曲，在本地歌曲的右键菜单中添加到此歌单。'
                  )}
                </div>
              )
            )}
            <InfiniteLoad
              more={list.more}
              busy={list.busy}
              error={list.error}
              onLoad={list.loadMore}
            />
          </CollectionContent>
        ) : (
          <>
            <MusicPageHeader title="本地音乐">
              <p>熟悉的收藏，随时聆听。</p>
            </MusicPageHeader>
            <div className="local-folder-actions">
              <ActionButton variant="ghost" disabled={importing || !isTauri()} onClick={onImport}>
                <Plus aria-hidden="true" />
                打开文件
              </ActionButton>
              <FolderManager />
            </div>
            <MusicTabs
              className="gap-0"
              value={kind}
              onValueChange={(value) => {
                setKind(value as Kind)
                if (page.query) navigate('local')
              }}
            >
              <div className="local-tabs-row">
                <TabsList className="music-tabs" aria-label="本地音乐分类">
                  {categories.map(({ value, label, icon: Icon }) => (
                    <TabsTab key={value} value={value}>
                      <Icon aria-hidden="true" />
                      {label}
                    </TabsTab>
                  ))}
                </TabsList>
                <div className="local-tabs-tools">
                  {!page.query && kind === 'artist' && (
                    <ArtistAvatarCompletionButton onError={onError} onNotice={onNotice} />
                  )}
                  {!page.query && kind === 'playlist' && (
                    <>
                      <ActionButton
                        variant="outline"
                        disabled={!isTauri()}
                        onClick={() => {
                          setName('')
                          setEditing('create')
                        }}
                      >
                        <Plus aria-hidden="true" />
                        创建歌单
                      </ActionButton>
                      <CreateAggregatePlaylist variant="outline" onError={onError} />
                    </>
                  )}
                  <form
                    className="local-search"
                    onSubmit={(event) => {
                      event.preventDefault()
                      navigate('local', input.trim())
                    }}
                  >
                    <Input
                      aria-label="搜索本地音乐、艺术家或专辑"
                      placeholder="搜索音乐、艺术家、专辑"
                      value={input}
                      maxLength={100}
                      onChange={(event) => setInput(event.target.value)}
                    />
                    {page.query && (
                      <ActionButton
                        variant="ghost"
                        size="icon-sm"
                        aria-label="清除本地搜索"
                        onClick={() => {
                          setInput('')
                          navigate('local')
                        }}
                      >
                        <X aria-hidden="true" />
                      </ActionButton>
                    )}
                    <Search aria-hidden="true" />
                  </form>
                </div>
              </div>
              {page.query ? (
                <LocalSearchResults
                  key={`${page.query}:${version}`}
                  keyword={page.query}
                  refresh={version}
                  onError={onError}
                  onNotice={onNotice}
                />
              ) : (
                categories.map(({ value }) => (
                  <Tabs.Panel key={value} value={value}>
                    <BrowseResults
                      kind={value}
                      keyword=""
                      refresh={version}
                      onError={onError}
                      onNotice={onNotice}
                    />
                  </Tabs.Panel>
                ))
              )}
            </MusicTabs>
          </>
        )}
        <Dialog
          open={!!editing}
          onOpenChange={(open) => {
            if (!open && !saving) setEditing(undefined)
          }}
        >
          <DialogContent>
            <DialogHeader>
              <DialogTitle>
                {editing === 'create'
                  ? '创建歌单'
                  : editing === 'rename'
                    ? '修改歌单名称'
                    : '删除歌单'}
              </DialogTitle>
              <DialogDescription>
                {editing === 'delete'
                  ? '删除歌单会保留曲库中的歌曲和原始文件。'
                  : editing === 'create'
                    ? '给喜欢的音乐一个名字。'
                    : '改变歌单的名称。'}
              </DialogDescription>
            </DialogHeader>
            <form
              onSubmit={(event) => {
                event.preventDefault()
                void save()
              }}
              className="flex flex-col gap-4"
            >
              {editing !== 'delete' && (
                <Input
                  autoFocus
                  aria-label="歌单名称"
                  value={name}
                  maxLength={100}
                  onChange={(event) => setName(event.target.value)}
                />
              )}
              {dialogError && <ErrorNotice error={dialogError} />}
              <ActionButton
                type="submit"
                disabled={saving || (editing !== 'delete' && !name.trim())}
              >
                {saving ? '正在保存…' : editing === 'delete' ? '删除歌单' : '保存'}
              </ActionButton>
            </form>
          </DialogContent>
        </Dialog>
      </MusicPage>
    </ArtistAvatarActionsProvider>
  )
}
