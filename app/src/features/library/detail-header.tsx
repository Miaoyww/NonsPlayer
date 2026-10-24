import { useLayoutEffect, useRef, type ReactNode } from 'react'

export function DetailHeader({
  children,
  className = ''
}: {
  children: ReactNode
  className?: string
}) {
  const header = useRef<HTMLElement>(null)
  useLayoutEffect(() => {
    const element = header.current
    const info = element?.querySelector<HTMLElement>('.library-detail-info')
    if (!element || !info) return
    const measure = () =>
      element.style.setProperty('--detail-content-height', `${info.offsetHeight}px`)
    measure()
    let frame = 0
    const observer = new ResizeObserver(() => {
      cancelAnimationFrame(frame)
      frame = requestAnimationFrame(measure)
    })
    observer.observe(info)
    return () => {
      observer.disconnect()
      cancelAnimationFrame(frame)
    }
  }, [])
  return (
    <header ref={header} className={`library-detail-header ${className}`}>
      {children}
    </header>
  )
}
