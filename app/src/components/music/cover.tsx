import { Skeleton } from '@/components/ui/skeleton'
import { Music2 } from 'lucide-react'
import { useEffect, useRef, useState } from 'react'
import { useCoverImageSource } from '@/components/music/use-cover-source'

export function Cover({
  cover,
  className = '',
  loading = 'lazy',
  size = 'normal'
}: {
  cover?: import('@/features/music/types').CoverInput
  className?: string
  loading?: 'eager' | 'lazy'
  size?: import('@/features/music/types').CoverSize
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
  const [completeSource, setCompleteSource] = useState<string>()
  const [readySource, setReadySource] = useState<string>()
  const enabled = loading === 'eager' || visible
  const { source, onError, pending, skipAnimation } = useCoverImageSource(cover, enabled, size)
  const instant = skipAnimation || (!!source && completeSource === source)
  const ready = readySource === source || instant
  const pendingImage = pending || (!!source && readySource !== source) || (!!cover && !enabled)
  return (
    <div
      ref={host}
      className={`relative flex shrink-0 items-center justify-center overflow-hidden rounded-lg bg-muted ${className}`}
    >
      {!instant && (pendingImage || !!source) && (
        <Skeleton
          key={`skeleton:${source ?? cover}`}
          data-ready={!pendingImage}
          className={`music-cover-skeleton pointer-events-none absolute inset-0 rounded-[inherit] ${pendingImage ? '' : 'opacity-0'}`}
        />
      )}
      {source ? (
        <img
          key={source}
          ref={(image) => {
            if (image?.complete && image.naturalWidth > 0 && readySource !== source)
              setCompleteSource(source)
          }}
          src={source}
          alt=""
          loading={loading}
          data-ready={ready}
          data-animate={!instant}
          className={`music-cover-image h-full w-full object-cover ${ready ? 'opacity-100' : 'opacity-0'}`}
          onLoad={() => setReadySource(source)}
          onError={onError}
        />
      ) : !pendingImage ? (
        <Music2 className="size-5 text-muted-foreground" aria-hidden="true" />
      ) : null}
    </div>
  )
}
