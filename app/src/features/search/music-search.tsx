import { useEffect, useRef, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { Combobox as Primitive } from '@base-ui/react/combobox'
import { History, Search } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { ComboboxContent, ComboboxItem, ComboboxList } from '@/components/ui/combobox'
import { useMusicNavigation } from '@/features/workspace/music-navigation'
import { useMusicSources } from '@/features/music/use-music-sources'
import { musicClient } from '@/features/music/client'

const historyKey = 'nons-search-history'
function readHistory(): string[] {
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
    ].slice(0, 10)
  } catch {
    return []
  }
}

export function MusicSearch() {
  const { page, navigate } = useMusicNavigation()
  const local = page.view === 'local'
  const { preferences, revision } = useMusicSources()
  const source = preferences?.current
  const [keyword, setKeyword] = useState(page.view === 'search' || local ? page.query : '')
  const [suggestions, setSuggestions] = useState<string[]>([])
  const [history, setHistory] = useState(readHistory)
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
            setSuggestions(items.slice(0, 5))
            setOpen(items.length > 0)
          }
        })
        .catch(() => {
          if (!disposed) setSuggestions([])
        })
    }, 250)
    return () => {
      disposed = true
      window.clearTimeout(timer)
    }
  }, [keyword, local, focused, composing, source, revision])
  function search(value: string) {
    if (!value.trim() || composing) return
    if (!local) {
      const next = [value.trim(), ...history.filter((item) => item !== value.trim())].slice(0, 10)
      setHistory(next)
      try {
        localStorage.setItem(historyKey, JSON.stringify(next))
      } catch {
        /* History remains available for this session if storage is unavailable. */
      }
    }
    setKeyword(value)
    setOpen(false)
    input.current?.blur()
    navigate(local ? 'local' : 'search', value.trim())
  }
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
