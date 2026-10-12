import type { ComponentProps } from 'react'
import { cn } from 'cn'
import { Loader2Icon } from 'lucide-react'

function Spinner({ className, ...props }: ComponentProps<'svg'>) {
  return (
    <Loader2Icon
      role="status"
      aria-label="正在加载"
      className={cn('size-4 animate-spin motion-reduce:animate-none', className)}
      {...props}
    />
  )
}

export { Spinner }
