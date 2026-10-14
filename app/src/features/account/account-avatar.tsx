import { UserRound } from 'lucide-react'
import { useCoverImageSource } from '@/components/music/use-cover-source'
import { cn } from '@/lib/utils'

export function AccountAvatar({
  avatar,
  className
}: {
  avatar?: import('@/features/music/types').CoverInput
  className?: string
}) {
  const { source, onError } = useCoverImageSource(avatar, true, 'small')
  return source ? (
    <img
      src={source}
      alt=""
      className={cn('size-6 shrink-0 rounded-full object-cover', className)}
      onError={onError}
    />
  ) : (
    <span
      className={cn(
        'flex size-6 shrink-0 items-center justify-center rounded-full bg-muted',
        className
      )}
    >
      <UserRound className="size-2/3" aria-hidden="true" />
    </span>
  )
}
