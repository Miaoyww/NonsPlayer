import { useLayoutEffect, useRef, useState } from 'react'
import { Popover } from '@base-ui/react/popover'
import { X } from 'lucide-react'

export function DetailDescription({ description }: { description?: string | null }) {
  const preview = useRef<HTMLParagraphElement>(null)
  const [overflow, setOverflow] = useState(false)
  useLayoutEffect(() => {
    const element = preview.current
    if (!element) return
    const measure = () => setOverflow(element.scrollWidth > element.clientWidth)
    measure()
    const observer = new ResizeObserver(measure)
    observer.observe(element)
    return () => observer.disconnect()
  }, [description])
  if (!description) return null
  return (
    <div className="library-detail-description">
      <p ref={preview} className="truncate">
        {description.replace(/\s+/g, ' ')}
      </p>
      {overflow && (
        <Popover.Root>
          <Popover.Trigger className="library-description-toggle">展开简介</Popover.Trigger>
          <Popover.Portal>
            <Popover.Positioner
              side="bottom"
              align="start"
              sideOffset={8}
              className="z-50"
              collisionPadding={16}
            >
              <Popover.Popup className="library-description-card">
                <Popover.Title className="sr-only">简介</Popover.Title>
                <Popover.Close className="library-description-close" aria-label="关闭简介">
                  <X aria-hidden="true" />
                </Popover.Close>
                <p>{description}</p>
              </Popover.Popup>
            </Popover.Positioner>
          </Popover.Portal>
        </Popover.Root>
      )}
    </div>
  )
}
