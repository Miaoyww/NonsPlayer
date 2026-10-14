import { Skeleton } from '@/components/ui/skeleton'
import { Music2 } from 'lucide-react'
import { useEffect, useRef, useState } from 'react'
import { useCoverImageSource } from '@/components/music/use-cover-source'

export function Cover({
  cover,
  className = '',
  loading = 'lazy'
}: {
  cover?: import('@/features/music/types').CoverInput
  className?: string
  loading?: 'eager' | 'lazy'
}) {
  const host = useRef<HTMLDivElement>(null)
  const [visible, setVisible] = useState(false)
  useEffect(() => {
    if (loading === 'eager' || !host.current) return
    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) {
          setVisible(true)
          observer.disconnect()
        }
      },
      { rootMargin: '200px' }
    )
    observer.observe(host.current)
    return () => observer.disconnect()
  }, [loading])
  const [readySource, setReadySource] = useState<string>()
  const enabled = loading === 'eager' || visible
  const { source, onError, pending } = useCoverImageSource(cover, enabled)
  const pendingImage = pending || (!!source && readySource !== source) || (!!cover && !enabled)
  return (
    <div
      ref={host}
      className={`relative flex shrink-0 items-center justify-center overflow-hidden rounded-lg bg-muted ${className}`}
    >
      {(pendingImage || !!source) && (
        <Skeleton
          key={`skeleton:${source ?? cover}`}
          data-ready={!pendingImage}
          className={`music-cover-skeleton pointer-events-none absolute inset-0 rounded-[inherit] ${pendingImage ? '' : 'opacity-0'}`}
        />
      )}
      {source ? (
        <img
          key={source}
          src={source}
          alt=""
          loading={loading}
          data-ready={readySource === source}
          className={`music-cover-image h-full w-full object-cover ${readySource === source ? 'opacity-100' : 'opacity-0'}`}
          onLoad={() => setReadySource(source)}
          onError={onError}
        />
      ) : !pendingImage ? (
        <Music2 className="size-5 text-muted-foreground" aria-hidden="true" />
      ) : null}
    </div>
  )
}
