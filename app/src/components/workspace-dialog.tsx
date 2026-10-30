import type { ReactNode } from 'react'
import { X } from 'lucide-react'
import { useReducedMotion } from 'motion/react'
import { Button } from '@/components/ui/button'
import {
  DialogClose,
  DialogDescription,
  DialogPopup,
  DialogTitle
} from '@/components/animate-ui/components/base/dialog'
import { cn } from '@/lib/utils'
import { version } from '../../package.json'

export function WorkspaceDialogPopup({
  title,
  description,
  navigationHeader,
  navigation,
  contentLabel,
  contentKey,
  contentClassName,
  className,
  children
}: {
  title: string
  description: string
  navigationHeader?: ReactNode
  navigation: ReactNode
  contentLabel: string
  contentKey?: string
  contentClassName?: string
  className?: string
  children: ReactNode
}) {
  const reducedMotion = useReducedMotion()
  return (
    <DialogPopup
      showCloseButton={false}
      className={cn('overflow-hidden rounded-lg', className)}
      style={{ width: '85vw', maxWidth: 'calc(100vw - 40px)', height: '85dvh', padding: 0, gap: 0 }}
      initial={{ opacity: 0, scale: reducedMotion ? 1 : 0.97 }}
      animate={{ opacity: 1, scale: 1 }}
      exit={{ opacity: 0, scale: reducedMotion ? 1 : 0.98 }}
      transition={{ duration: reducedMotion ? 0 : 0.18 }}
    >
      <DialogClose
        render={
          <Button
            variant="ghost"
            size="icon-sm"
            className="absolute right-4 top-4 z-10 rounded-lg"
            aria-label={`关闭${title}`}
            title={`关闭${title}`}
          />
        }
      >
        <X aria-hidden="true" />
      </DialogClose>
      <div className="flex h-full min-h-0 w-full overflow-hidden">
        <aside className="flex w-44 shrink-0 flex-col bg-muted/50 sm:w-60">
          <div className="px-5 pb-4 pt-5">
            <DialogTitle className="text-[26px] font-bold leading-none tracking-tight">
              {title}
            </DialogTitle>
            <DialogDescription className="mt-1.5 text-sm text-muted-foreground">
              {description}
            </DialogDescription>
            {navigationHeader && <div className="mt-3">{navigationHeader}</div>}
          </div>
          <div className="mx-3 border-t border-border" role="separator" />
          <nav
            aria-label={`${title}导航`}
            className="flex min-h-0 flex-1 flex-col gap-0.5 overflow-auto px-3 py-3"
          >
            {navigation}
          </nav>
          <div className="flex flex-col gap-1 px-5 pb-5 pt-2">
            <span className="text-sm font-semibold">Nons</span>
            <span className="text-xs text-muted-foreground">Version {version}</span>
          </div>
        </aside>
        <section
          key={contentKey}
          aria-label={contentLabel}
          className={cn(
            'min-w-0 flex-1 overflow-auto bg-background p-5 pt-14 sm:p-10',
            contentClassName
          )}
        >
          {children}
        </section>
      </div>
    </DialogPopup>
  )
}
