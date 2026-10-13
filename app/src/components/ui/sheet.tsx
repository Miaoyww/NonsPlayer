import { Dialog } from '@base-ui/react/dialog'
import type { ComponentProps } from 'react'
import { cn } from '@/lib/utils'

export const Sheet = Dialog.Root
export const SheetTrigger = Dialog.Trigger
export const SheetClose = Dialog.Close
export const SheetTitle = Dialog.Title
export const SheetDescription = Dialog.Description

export function SheetContent({
  className,
  children,
  ...props
}: ComponentProps<typeof Dialog.Popup>) {
  return (
    <Dialog.Portal>
      <Dialog.Backdrop className="fixed inset-0 z-50 bg-black/20 transition-opacity duration-200 data-[starting-style]:opacity-0 data-[ending-style]:opacity-0 motion-reduce:transition-none" />
      <Dialog.Popup
        className={cn(
          'settings-sheet glass-surface fixed inset-y-0 left-0 z-50 flex w-[min(360px,90vw)] flex-col overflow-hidden border-r transition-transform duration-200 data-[starting-style]:-translate-x-full data-[ending-style]:-translate-x-full motion-reduce:transition-none',
          className
        )}
        {...props}
      >
        {children}
      </Dialog.Popup>
    </Dialog.Portal>
  )
}
