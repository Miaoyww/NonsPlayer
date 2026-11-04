import { Check, Clapperboard, FolderOpen, Search, X } from 'lucide-react'
import { Cover } from '@/components/music/cover'
import { Button } from '@/components/ui/button'
import { Spinner } from '@/components/ui/spinner'
import { errorText, nativeCall } from '@/lib/player'
import { useCallback, useEffect, useRef, useState } from 'react'
import type { MvSelection } from './local-mv'

interface Candidate {
  reference: { source: string; kind: 'track'; id: string }
  title: string
  uploader: string
  durationMs: number
  cover: import('@/features/music/types').CoverImage
}
interface Group {
  source: string
  displayName: string
  items: Candidate[]
  nextCursor: string | null
  error: string | null
}

function duration(ms: number) {
  const total = Math.max(0, Math.round(ms / 1000))
  return `${Math.floor(total / 60)}:${String(total % 60).padStart(2, '0')}`
}

export function MvPanel({
  trackKey,
  defaultQuery,
  selection,
  linked,
  busy,
  onChoose,
  onApply,
  onBind,
  onCancel,
  onError
}: {
  trackKey: string
  defaultQuery: string
  selection?: MvSelection
  linked: boolean
  busy: boolean
  onChoose: () => void
  onApply: () => void
  onBind: (source: string, id: string, title: string) => Promise<boolean>
  onCancel: () => void
  onError: (message: string) => void
}) {
  const [query, setQuery] = useState(defaultQuery)
  const [groups, setGroups] = useState<Group[]>([])
  const [searching, setSearching] = useState(false)
  const [searched, setSearched] = useState(false)
  const serial = useRef(0)

  const search = useCallback(
    async (cursor?: string) => {
      const value = query.trim() || defaultQuery
      if (!value) return
      const request = ++serial.current
      setSearching(true)
      setSearched(true)
      try {
        const result = await nativeCall<Group[]>('search_track_mvs', {
          key: trackKey,
          input: { query: value, cursor: cursor ?? null }
        })
        if (request !== serial.current) return
        setGroups((current) =>
          cursor
            ? result.map((next) => {
                const previous = current.find((group) => group.source === next.source)
                return previous ? { ...next, items: [...previous.items, ...next.items] } : next
              })
            : result
        )
      } catch (cause) {
        if (request === serial.current) onError(errorText(cause))
      } finally {
        if (request === serial.current) setSearching(false)
      }
    },
    [defaultQuery, onError, query, trackKey]
  )

  useEffect(() => {
    serial.current++
    setQuery(defaultQuery)
    setGroups([])
    setSearched(false)
    setSearching(false)
  }, [defaultQuery, trackKey])

  useEffect(() => {
    if (!defaultQuery) return
    const timer = window.setTimeout(() => void search(), 250)
    return () => window.clearTimeout(timer)
  }, [defaultQuery, search])

  const remoteSelected = selection && !selection.local
  return (
    <section className="mv-panel" aria-label="MV">
      <header className="mv-panel-header">
        <h2>MV</h2>
        <p>搜索适配器提供的 MV，或选择本地视频作为兜底。确认后进入静音画面。</p>
      </header>

      <div className="mv-panel-search">
        <input
          value={query}
          aria-label="搜索 MV"
          placeholder="搜索视频"
          disabled={busy}
          onChange={(event) => setQuery(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === 'Enter') void search()
          }}
        />
        <Button
          type="button"
          size="icon"
          variant="secondary"
          aria-label="搜索 MV"
          disabled={busy || searching || !query.trim()}
          onClick={() => void search()}
        >
          {searching ? <Spinner /> : <Search aria-hidden="true" />}
        </Button>
      </div>

      <div className="mv-panel-results" aria-live="polite">
        {groups.map((group) => (
          <section key={group.source} aria-label={`${group.displayName} 的 MV`}>
            <h3 className="mv-panel-source">{group.displayName}</h3>
            {group.error && <p className="mv-panel-error">{group.error}</p>}
            {group.items.map((candidate) => {
              const active =
                remoteSelected &&
                selection.source === candidate.reference.source &&
                selection.id === candidate.reference.id
              return (
                <button
                  key={candidate.reference.id}
                  type="button"
                  className="mv-panel-result"
                  data-active={active || undefined}
                  disabled={busy}
                  onClick={() => {
                    void onBind(candidate.reference.source, candidate.reference.id, candidate.title)
                  }}
                >
                  <Cover cover={candidate.cover} size="small" className="mv-panel-cover" />
                  <span className="mv-panel-copy">
                    <strong>{candidate.title}</strong>
                    <small>
                      {candidate.uploader} · {duration(candidate.durationMs)}
                    </small>
                  </span>
                  {active ? <Check aria-hidden="true" /> : <Clapperboard aria-hidden="true" />}
                </button>
              )
            })}
            {group.nextCursor && (
              <Button
                type="button"
                size="sm"
                variant="ghost"
                disabled={busy || searching}
                onClick={() => void search(group.nextCursor ?? undefined)}
              >
                加载更多
              </Button>
            )}
          </section>
        ))}
        {searched && !searching && !groups.length && (
          <p className="mv-panel-empty">没有可用的适配器 MV 结果，可选择本地视频。</p>
        )}
      </div>

      <div className="mv-panel-card">
        <div className="mv-panel-selection">
          <span className="mv-panel-icon" aria-hidden="true">
            <FolderOpen />
          </span>
          <div className="mv-panel-file">
            {selection?.local ? (
              <>
                <strong>{selection.title}</strong>
                <span title={selection.localPath}>{selection.localPath}</span>
                <small data-state={linked ? 'linked' : 'ready'}>
                  {linked ? '已关联' : '尚未应用'}
                </small>
              </>
            ) : (
              <>
                <strong>本地视频</strong>
                <span>支持 MP4、M4V、WebM</span>
              </>
            )}
          </div>
        </div>
        <div className="mv-panel-actions">
          <Button type="button" variant="outline" disabled={busy} onClick={onChoose}>
            <FolderOpen aria-hidden="true" />
            {selection?.local ? '更换视频' : '选择本地'}
          </Button>
          {selection?.local && (
            <Button
              type="button"
              variant="secondary"
              disabled={busy || !selection.localPath}
              onClick={onApply}
            >
              <Check aria-hidden="true" />
              {linked ? '进入 MV' : '应用'}
            </Button>
          )}
          <Button type="button" variant="ghost" disabled={busy} onClick={onCancel}>
            <X aria-hidden="true" />
            返回
          </Button>
        </div>
      </div>
      <p className="mv-panel-hint">MV 静音播放，并跟随当前歌曲进度对齐。</p>
    </section>
  )
}
