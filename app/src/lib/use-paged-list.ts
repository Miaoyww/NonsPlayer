import { reportCaughtError } from '@/lib/diagnostics'
import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { errorText } from './player'
import { requestCache } from '@/lib/runtime-cache'

export interface ListPage<T, M = undefined> {
  items: T[]
  more: boolean
  metadata?: M
}

// Display state and requests belong to one query. A new query must never expose
// the previous query's rows, even during the render before effects run.
export function usePagedList<T, M = undefined>(
  loader: (offset: number) => Promise<ListPage<T, M>>,
  size: number,
  enabled: boolean,
  {
    refreshKey,
    cacheKey,
    reconcile
  }: {
    refreshKey?: unknown
    cacheKey?: string
    reconcile?: (previous: T[], next: T[]) => T[]
  } = {}
) {
  const query = useMemo(
    () => ({ loader, size, enabled, cacheKey }),
    [loader, size, enabled, cacheKey]
  )
  const cached = enabled && cacheKey ? requestCache.peekFresh<ListPage<T, M>>(cacheKey) : undefined
  const empty = {
    query,
    items: cached?.items ?? ([] as T[]),
    metadata: cached?.metadata,
    more: cached?.more ?? false,
    busy: enabled && !cached,
    error: undefined as string | undefined
  }
  const [view, setView] = useState(empty)
  const request = useRef({ query, offset: 0, busy: false, more: true, reloadPages: 1 })
  const load = useCallback(
    async (pages = request.current.offset === 0 ? request.current.reloadPages : 1) => {
      const state = request.current
      if (!enabled || state.query !== query || state.busy || !state.more) return
      state.busy = true
      const revision = requestCache.revision
      setView((previous) => ({
        ...previous,
        busy: !cached || previous.items.length > 0,
        error: undefined
      }))
      try {
        const firstPage = state.offset === 0
        let offset = state.offset
        let page: ListPage<T, M>
        const items: T[] = []
        for (let index = 0; index < pages; index++) {
          page = await loader(offset)
          if (request.current !== state) return
          if (offset === 0 && cacheKey && requestCache.revision === revision)
            requestCache.set(cacheKey, page)
          items.push(...page.items)
          offset += size
          if (!page.more) break
        }
        setView((previous) => ({
          query,
          items: firstPage
            ? reconcile && previous.query === query
              ? reconcile(previous.items, items)
              : items
            : [...previous.items, ...items],
          metadata: page!.metadata,
          more: page!.more,
          busy: true,
          error: undefined
        }))
        state.offset = offset
        state.more = page!.more
        state.reloadPages = Math.max(1, offset / size)
      } catch (cause) {
        reportCaughtError('app/src/lib/use-paged-list.ts', cause)

        if (request.current === state)
          setView((previous) => ({ ...previous, error: errorText(cause) }))
      } finally {
        if (request.current === state) {
          state.busy = false
          setView((previous) => ({ ...previous, busy: false }))
        }
      }
    },
    [query, reconcile]
  )
  const loadMore = useCallback(() => load(), [load])
  useEffect(() => {
    const pages = request.current.query === query ? request.current.reloadPages : 1
    request.current = { query, offset: 0, busy: false, more: true, reloadPages: pages }
    // Refresh this query in the background; only a different query clears its rows.
    setView((previous) =>
      previous.query === query
        ? { ...previous, busy: enabled && (!cached || previous.items.length > 0), error: undefined }
        : empty
    )
    void load(pages)
    return () => {
      request.current = { ...request.current }
    }
  }, [load, refreshKey])
  const current = view.query === query ? view : empty
  const { items, metadata, more, busy, error } = current
  return { items, metadata, more, busy, error, loadMore }
}
