import { ErrorNotice } from '@/components/error-notice'
import { Skeleton } from '@/components/ui/skeleton'
import { useEffect, useRef } from 'react'
import { ActionButton } from '@/components/music/action-button'

export function scrollParent(element: HTMLElement): HTMLElement | null {
  for (let parent = element.parentElement; parent; parent = parent.parentElement) {
    if (/auto|scroll/.test(getComputedStyle(parent).overflowY)) return parent
  }
  return null
}

export function InfiniteLoad({
  more,
  busy,
  error,
  onLoad
}: {
  more: boolean
  busy: boolean
  error?: string
  onLoad: () => void
}) {
  const marker = useRef<HTMLDivElement>(null)
  useEffect(() => {
    const element = marker.current
    if (!element || !more || busy || error) return
    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) onLoad()
      },
      { root: scrollParent(element), rootMargin: '0px 0px 120px 0px' }
    )
    observer.observe(element)
    return () => observer.disconnect()
  }, [more, busy, error, onLoad])
  return (
    <div
      ref={marker}
      className="flex min-h-12 items-center justify-center gap-3 py-3 text-sm text-muted-foreground"
      aria-live="polite"
    >
      {error ? (
        <>
          <ErrorNotice error={error} />
          <ActionButton size="sm" variant="ghost" onClick={onLoad}>
            重试
          </ActionButton>
        </>
      ) : busy ? (
        <div role="status" aria-label="正在加载更多" className="flex w-32 gap-2">
          <Skeleton className="h-2 flex-1" />
          <Skeleton className="h-2 flex-1" />
          <Skeleton className="h-2 flex-1" />
        </div>
      ) : more ? (
        <span className="sr-only">滚动到底部自动加载更多</span>
      ) : null}
    </div>
  )
}
