import { ChevronLeft } from 'lucide-react'
import { Button } from '@/components/ui/button'

export function MobilePageHeader({
  title,
  description,
  onBack
}: {
  title: string
  description?: string
  onBack: () => void
}) {
  return (
    <header className="mobile-page-header">
      <Button variant="ghost" size="icon-lg" aria-label="返回上一页" onClick={onBack}>
        <ChevronLeft aria-hidden="true" />
      </Button>
      <div className="min-w-0">
        <h1 className="truncate text-lg font-bold">{title}</h1>
        {description && <p className="text-sm text-muted-foreground">{description}</p>}
      </div>
    </header>
  )
}
