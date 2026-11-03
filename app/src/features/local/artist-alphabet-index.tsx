import { Button } from '@/components/ui/button'
import { ScrollArea } from '@/components/ui/scroll-area'
import { ARTIST_INDEX_LETTERS, type ArtistIndexLetter } from '@/features/local/artist-index'

export function ArtistAlphabetIndex({
  active,
  available,
  canLoadMore,
  disabled,
  pending,
  onSelect
}: {
  active?: ArtistIndexLetter
  available: Set<ArtistIndexLetter>
  canLoadMore: boolean
  disabled: boolean
  pending?: ArtistIndexLetter
  onSelect: (letter: ArtistIndexLetter) => void
}) {
  return (
    <nav className="local-artist-index" aria-label="艺术家首字母索引">
      <ScrollArea className="local-artist-index-scroll">
        <div className="local-artist-index-list">
          {ARTIST_INDEX_LETTERS.map((letter) => {
            const label = letter === '#' ? '其他' : letter
            return (
              <Button
                key={letter}
                type="button"
                variant={active === letter ? 'default' : 'ghost'}
                size="xs"
                className="local-artist-index-button"
                data-active={active === letter}
                aria-label={`跳转到 ${label} 开头的艺术家`}
                aria-current={active === letter ? 'location' : undefined}
                aria-busy={pending === letter}
                disabled={
                  disabled || pending !== undefined || (!available.has(letter) && !canLoadMore)
                }
                onClick={() => onSelect(letter)}
              >
                {letter}
              </Button>
            )
          })}
        </div>
      </ScrollArea>
    </nav>
  )
}
