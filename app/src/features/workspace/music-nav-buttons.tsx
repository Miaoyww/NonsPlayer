import { Button } from '@/components/ui/button'
import { cn } from 'cn'
import { supportsMusicPage } from '@/features/music/source-capabilities'
import { useMusicSources } from '@/features/music/use-music-sources'
import { useMusicNavigation, type MusicView } from './music-navigation'

const pages = [
  ['library', '音乐库'],
  ['discover', '发现'],
  ['local', '我的']
] as const

function isSelected(view: MusicView, current: MusicView) {
  return (
    current === view ||
    (view === 'library' &&
      (current === 'collection' || current === 'streaming-playlist' || current === 'accounts')) ||
    (view === 'discover' && current === 'search') ||
    (view === 'local' && (current.startsWith('local-') || current === 'aggregate-playlist'))
  )
}

export function MusicNavButtons({ mobile = false }: { mobile?: boolean } = {}) {
  const sources = useMusicSources()
  const current = sources.statuses.find(
    (item) => item.descriptor.source === sources.preferences?.current && item.enabled
  )
  const { page, navigate } = useMusicNavigation()
  return pages.map(([view, label]) => {
    const supported = view === 'local' || supportsMusicPage(current?.descriptor, view)
    const selected = isSelected(view, page.view)
    return (
      <Button
        key={view}
        disabled={!supported}
        title={supported ? undefined : `此适配器未提供${label}`}
        variant="ghost"
        className={cn(
          'cursor-pointer',
          mobile && 'mobile-navigation-item',
          selected && 'text-[var(--music-accent)] hover:text-[var(--music-accent)]'
        )}
        aria-current={selected ? 'page' : undefined}
        onClick={() => navigate(view)}
      >
        {label}
      </Button>
    )
  })
}
