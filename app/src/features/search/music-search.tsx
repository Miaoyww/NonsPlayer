import { reportCaughtError } from '@/lib/diagnostics'
import { useEffect, useRef, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { Combobox as Primitive } from '@base-ui/react/combobox'
import { History, Search, Trash2, ArrowUpLeft } from 'lucide-react'
import { Input } from '@/components/ui/input'
import { Button } from '@/components/ui/button'
import { ComboboxContent, ComboboxItem, ComboboxList } from '@/components/ui/combobox'
import { useMusicNavigation } from '@/features/workspace/music-navigation'
import { useMusicSources } from '@/features/music/use-music-sources'
import { musicClient } from '@/features/music/client'

const historyKey = 'nons-search-history'
function readHistory(limit = 10): string[] {
  try {
    const saved: unknown = JSON.parse(localStorage.getItem(historyKey) ?? '[]')
    if (!Array.isArray(saved)) return []
    return [
      ...new Set(
        saved
          .filter(
            (value): value is string =>
              typeof value === 'string' && value.trim().length > 0 && value.length <= 85
          )
          .map((value) => value.trim())
      )
    ].slice(0, limit)
  } catch (cause) {
    reportCaughtError('app/src/features/search/music-search.tsx', cause)

    return []
  }
}

export function MusicSearch({ pageMode = false }: { pageMode?: boolean } = {}) {
  const { page, navigate } = useMusicNavigation()
  const local = !pageMode && page.view === 'local'
  const historyLimit = pageMode ? 20 : 10
  const { preferences, revision } = useMusicSources()
  const source = preferences?.current
  const [keyword, setKeyword] = useState(page.view === 'search' || local ? page.query : '')
  const [suggestions, setSuggestions] = useState<string[]>([])
  const [history, setHistory] = useState(() => readHistory(historyLimit))
  const [focused, setFocused] = useState(false)
  const [open, setOpen] = useState(false)
  const [composing, setComposing] = useState(false)
  const input = useRef<HTMLInputElement>(null)
  const anchor = useRef<HTMLFormElement>(null)
  const showingHistory = !keyword.trim()
  const items = showingHistory ? history : suggestions
  useEffect(() => {
    if (page.view === 'search' || page.view === 'local') setKeyword(page.query)
    setOpen(false)
  }, [page])
  useEffect(() => {
    let disposed = false
    setSuggestions([])
    if (!keyword.trim()) {
      setOpen(focused && !local && !composing)
      return
    }
    if (local || !focused || composing || !keyword.trim() || !isTauri() || !source) return
    const timer = window.setTimeout(() => {
      void musicClient
        .query(source, { operation: 'suggestions', keyword: keyword.trim() })
        .then((response) => {
          const items = response.type === 'suggestions' ? response.data : []
          if (!disposed) {
            setSuggestions([...new Set(items)].slice(0, pageMode ? 10 : 5))
            setOpen(items.length > 0)
          }
        })
        .catch((cause) => {
          reportCaughtError('app/src/features/search/music-search.tsx', cause)

          if (!disposed) setSuggestions([])
        })
    }, 250)
    return () => {
      disposed = true
      window.clearTimeout(timer)
    }
  }, [keyword, local, focused, composing, source, revision, pageMode])
  function search(value: string) {
    if (!value.trim() || composing) return
    if (!local) {
      const next = [value.trim(), ...history.filter((item) => item !== value.trim())].slice(
        0,
        historyLimit
      )
      setHistory(next)
      try {
        localStorage.setItem(historyKey, JSON.stringify(next))
      } catch (cause) {
        reportCaughtError('app/src/features/search/music-search.tsx', cause)

        /* History remains available for this session if storage is unavailable. */
      }
    }
    setKeyword(value)
    setOpen(false)
    input.current?.blur()
    navigate(local ? 'local' : 'search', value.trim())
  }
  if (pageMode)
    return (
      <div className="mobile-search-entry">
        <form
          className="mobile-search-form"
          onSubmit={(event) => {
            event.preventDefault()
            search(keyword)
          }}
        >
          <Search className="size-5 shrink-0 text-muted-foreground" aria-hidden="true" />
          <Input
            ref={input}
            autoFocus
            data-global-music-search
            aria-label="搜索音乐"
            placeholder="歌曲、歌手、歌单或专辑"
            maxLength={85}
            value={keyword}
            onChange={(event) => setKeyword(event.target.value)}
            onFocus={() => setFocused(true)}
            onBlur={() => setFocused(false)}
            onCompositionStart={() => setComposing(true)}
            onCompositionEnd={() => setComposing(false)}
            className="min-w-0 border-0 bg-transparent shadow-none"
          />
          <Button type="submit" variant="secondary" disabled={!keyword.trim() || composing}>
            搜索
          </Button>
        </form>
        {showingHistory ? (
          <section className="mobile-search-history" aria-label="搜索记录">
            <div className="flex items-center justify-between">
              <h2 className="font-semibold">最近搜索</h2>
              <Button
                variant="ghost"
                size="icon-lg"
                aria-label="清空搜索记录"
                disabled={!history.length}
                onClick={() => {
                  setHistory([])
                  try {
                    localStorage.removeItem(historyKey)
                  } catch (cause) {
                    reportCaughtError('app/src/features/search/music-search.tsx', cause)

                    /* Storage is optional. */
                  }
                }}
              >
                <Trash2 aria-hidden="true" />
              </Button>
            </div>
            {history.length ? (
              <div className="flex flex-wrap gap-2">
                {history.map((value) => (
                  <Button
                    key={value}
                    variant="secondary"
                    className="max-w-full rounded-full"
                    onClick={() => search(value)}
                  >
                    <History aria-hidden="true" />
                    <span className="truncate">{value}</span>
                  </Button>
                ))}
              </div>
            ) : (
              <p className="py-6 text-sm text-muted-foreground">
                搜索喜欢的音乐，记录会显示在这里。
              </p>
            )}
          </section>
        ) : focused && !composing && suggestions.length > 0 ? (
          <section className="mobile-search-suggestions" aria-label="搜索补完">
            <h2 className="mb-2 text-sm font-semibold text-muted-foreground">搜索建议</h2>
            {suggestions.map((value) => (
              <Button
                key={value}
                variant="ghost"
                className="w-full justify-start"
                onPointerDown={(event) => event.preventDefault()}
                onClick={() => search(value)}
              >
                <Search aria-hidden="true" />
                <span className="min-w-0 flex-1 truncate text-left">{value}</span>
                <ArrowUpLeft aria-hidden="true" />
              </Button>
            ))}
          </section>
        ) : null}
      </div>
    )
  return (
    <div className="titlebar-search-slot ml-2">
      <Primitive.Root
        items={items}
        filter={null}
        value={null}
        inputValue={keyword}
        onInputValueChange={(value, details) => {
          // Closing the popup must not reset this search field to the null selection.
          if (details.reason === 'input-clear') {
            details.cancel()
            return
          }
          setKeyword(value)
        }}
        open={open && focused && !local && !composing && items.length > 0}
        onOpenChange={setOpen}
        onValueChange={(value) => {
          if (typeof value === 'string') search(value)
        }}
      >
        <form
          ref={anchor}
          className="titlebar-search"
          onSubmit={(event) => {
            event.preventDefault()
            search(keyword)
          }}
        >
          <Button
            type="button"
            variant="ghost"
            size="icon-sm"
            aria-label="展开音乐搜索"
            onClick={() => input.current?.focus()}
          >
            <Search aria-hidden="true" />
          </Button>
          <Primitive.Input
            data-global-music-search
            ref={input}
            aria-label={local ? '搜索本地曲库' : '搜索音乐'}
            placeholder={local ? '搜索本地曲库' : '搜索音乐'}
            maxLength={85}
            onFocus={() => {
              setFocused(true)
              setOpen(true)
            }}
            onBlur={() => setFocused(false)}
            onCompositionStart={() => setComposing(true)}
            onCompositionEnd={() => setComposing(false)}
          />
        </form>
        <ComboboxContent anchor={anchor} className="titlebar-search-suggestions">
          <p className="px-3 pb-1 pt-3 text-xs text-muted-foreground">
            {showingHistory ? '搜索历史' : '搜索建议'}
          </p>
          <ComboboxList>
            {(value: string) => (
              <ComboboxItem key={value} value={value}>
                {showingHistory ? <History aria-hidden="true" /> : <Search aria-hidden="true" />}
                <span className="truncate">{value}</span>
              </ComboboxItem>
            )}
          </ComboboxList>
        </ComboboxContent>
      </Primitive.Root>
    </div>
  )
}
