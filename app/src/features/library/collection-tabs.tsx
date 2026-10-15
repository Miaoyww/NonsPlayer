import { collectionReference } from '@/features/music/legacy-reference'
import { coverOriginal } from '@/features/music/cover-image'
import { useState, useSyncExternalStore, type ReactNode } from 'react'
import { Music2, Search } from 'lucide-react'
import { Tabs, TabsList, TabsTab, TabsPanel } from '@/components/animate-ui/components/base/tabs'
import { Input } from '@/components/ui/input'
import { isAndroid } from '@/lib/platform'
import { TrackSearchContext } from '@/components/music/track-search-context'
import type { MusicCollection } from '@/features/workspace/music-navigation'
import { PluginCollectionTab, usePlugins } from '@/plugins/host'
import {
  collectionTabSnapshot,
  subscribeCollectionTabs,
  type PluginCollection
} from '@/plugins/collection-tabs'

export function CollectionTabs({
  collection,
  children,
  hasMore = false
}: {
  collection: MusicCollection
  children: ReactNode
  hasMore?: boolean
}) {
  const { loaded } = usePlugins()
  const entries = useSyncExternalStore(subscribeCollectionTabs, collectionTabSnapshot)
  const [selected, setSelected] = useState('songs')
  const [query, setQuery] = useState('')
  const context: PluginCollection = {
    source: collection.localId === undefined ? collectionReference(collection).source : 'local',
    kind: collection.kind,
    id: collection.localId ?? collection.id,
    name: collection.name,
    subtitle: collection.subtitle,
    cover: /^https?:\/\//.test(coverOriginal(collection.cover))
      ? coverOriginal(collection.cover)
      : '',
    trackCount: collection.trackCount
  }
  const tabs = entries.filter(
    (entry) =>
      entry.scope.active &&
      loaded.get(entry.scope.descriptor.manifest.id)?.scope === entry.scope &&
      (!entry.kinds || entry.kinds.includes(context.kind)) &&
      (!entry.sources || entry.sources.includes(context.source))
  )
  const active = tabs.find((entry) => entry.key === selected)
  const value = active ? active.key : 'songs'
  // Removed/reloaded tabs must not reselect themselves if the same registration returns later.
  if (selected !== value) setSelected(value)
  if (isAndroid()) return <>{children}</>
  return (
    <Tabs
      value={value}
      onValueChange={(next) => setSelected(String(next))}
      className="collection-content-tabs gap-0"
    >
      <div className="collection-tabs-toolbar">
        <TabsList className="music-tabs" aria-label="收藏详情内容">
          <TabsTab value="songs">
            <Music2 aria-hidden="true" />
            歌曲
          </TabsTab>
          {tabs.map((entry) => (
            <TabsTab key={entry.key} value={entry.key}>
              {entry.label}
            </TabsTab>
          ))}
        </TabsList>
        {value === 'songs' && (
          <div className="collection-tabs-search">
            {hasMore && (
              <span className="text-xs text-muted-foreground">
                搜索已加载歌曲，滚动到底部继续加载
              </span>
            )}
            <label className="relative w-64 max-w-full">
              <span className="sr-only">搜索列表歌曲</span>
              <Search
                aria-hidden="true"
                className="pointer-events-none absolute top-2.5 left-3 size-4 text-muted-foreground"
              />
              <Input
                type="search"
                className="pl-9"
                placeholder="搜索歌曲、艺术家、专辑"
                value={query}
                onChange={(event) => setQuery(event.target.value)}
              />
            </label>
          </div>
        )}
      </div>
      <TabsPanel value="songs" transition={{ duration: 0.15 }}>
        <TrackSearchContext.Provider value={query}>
          {value === 'songs' && children}
        </TrackSearchContext.Provider>
      </TabsPanel>
      {active && (
        <TabsPanel key={active.key} value={active.key} transition={{ duration: 0.15 }}>
          <PluginCollectionTab entry={active} collection={context} />
        </TabsPanel>
      )}
    </Tabs>
  )
}
