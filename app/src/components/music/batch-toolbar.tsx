import type { ReactNode } from 'react'
import { Check, ListChecks } from 'lucide-react'
import { ActionButton } from '@/components/music/action-button'

export function BatchToolbar({
  selected,
  busy = false,
  onDone,
  children
}: {
  selected: number
  busy?: boolean
  onDone: () => void
  children: ReactNode
}) {
  return (
    <div className="mb-4 flex flex-wrap items-center gap-2" role="toolbar" aria-label="批量操作">
      <span className="mr-2 inline-flex items-center gap-2 text-sm text-muted-foreground">
        <ListChecks aria-hidden="true" className="size-4" />
        已选 {selected} 首
      </span>
      {children}
      <ActionButton variant="outline" disabled={busy} onClick={onDone}>
        <Check aria-hidden="true" />
        完成
      </ActionButton>
    </div>
  )
}
