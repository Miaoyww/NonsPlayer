import { Play } from 'lucide-react'
import { Button } from '@/components/ui/button'

type PlayButtonProps = Omit<React.ComponentProps<typeof Button>, 'children'> & {
  children?: React.ReactNode
}

export function PlayButton({ className, children = '播放', ...props }: PlayButtonProps) {
  return (
    <Button
      className={`h-9 gap-2 rounded-full bg-[var(--library-primary)] px-4 text-white hover:brightness-95 ${className ?? ''}`}
      {...props}
    >
      <Play aria-hidden="true" className="fill-current" />
      {children}
    </Button>
  )
}
