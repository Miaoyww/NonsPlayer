import { ChevronLeft, Search, X } from 'lucide-react'
import { AnimatePresence, motion, useReducedMotion } from 'motion/react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'

export function MobilePageHeader({
  title,
  description,
  onBack,
  search
}: {
  title: string
  description?: string
  onBack: () => void
  search?: {
    open: boolean
    query: string
    onOpen: () => void
    onClose: () => void
    onChange: (value: string) => void
  }
}) {
  const reduced = useReducedMotion()
  return (
    <header className="mobile-page-header">
      <AnimatePresence initial={false} mode="wait">
        {search?.open ? (
          <motion.div
            key="search"
            className="mobile-collection-search-row"
            initial={{ opacity: 0, clipPath: reduced ? 'inset(0)' : 'inset(0 0 0 100%)' }}
            animate={{ opacity: 1, clipPath: 'inset(0)' }}
            exit={{ opacity: 0 }}
            transition={{ duration: reduced ? 0 : 0.22, ease: 'easeOut' }}
          >
            <div className="mobile-collection-search-field">
              <Search className="size-5 shrink-0 text-muted-foreground" aria-hidden="true" />
              <Input
                autoFocus
                type="text"
                inputMode="search"
                enterKeyHint="search"
                aria-label="搜索列表歌曲"
                placeholder="搜索歌单内歌曲"
                maxLength={85}
                value={search.query}
                className="min-w-0 border-0 bg-transparent shadow-none focus-visible:border-transparent focus-visible:ring-0"
                onChange={(event) => search.onChange(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key === 'Escape') search.onClose()
                  if (event.key === 'Enter') event.currentTarget.blur()
                }}
              />
              {search.query && (
                <Button
                  variant="ghost"
                  size="icon-sm"
                  aria-label="清空列表搜索"
                  onClick={() => search.onChange('')}
                >
                  <X aria-hidden="true" />
                </Button>
              )}
            </div>
            <Button variant="ghost" onClick={search.onClose}>
              取消
            </Button>
          </motion.div>
        ) : (
          <motion.div
            key="title"
            className="mobile-page-title-row"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: reduced ? 0 : 0.12 }}
          >
            <Button variant="ghost" size="icon-lg" aria-label="返回上一页" onClick={onBack}>
              <ChevronLeft aria-hidden="true" />
            </Button>
            <div className="min-w-0 flex-1">
              <h1 className="truncate text-lg font-bold">{title}</h1>
              {description && <p className="text-sm text-muted-foreground">{description}</p>}
            </div>
            {search && (
              <Button
                variant="ghost"
                size="icon-lg"
                aria-label="搜索列表歌曲"
                onClick={search.onOpen}
              >
                <Search aria-hidden="true" />
              </Button>
            )}
          </motion.div>
        )}
      </AnimatePresence>
    </header>
  )
}
