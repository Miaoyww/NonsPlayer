import { useEffect, useRef, type ReactNode } from 'react'
import type { SettingsSearchItem } from './settings-search-index'

export type SettingsSearchTarget = SettingsSearchItem & { request: number }

export function SettingsSearchDestination({
  target,
  children
}: {
  target?: SettingsSearchTarget
  children: ReactNode
}) {
  const root = useRef<HTMLDivElement>(null)
  useEffect(() => {
    if (!target) return
    let timer: ReturnType<typeof setTimeout> | undefined
    let card: HTMLElement | undefined
    const frame = requestAnimationFrame(() => {
      const match = Array.from(
        root.current?.querySelectorAll<HTMLElement>('[data-setting-id]') ?? []
      ).find((element) => element.dataset.settingId === target.id)
      card = match?.closest<HTMLElement>('.settings-card') ?? match
      if (!card) return
      card.dataset.searchHighlighted = ''
      card.scrollIntoView({ block: 'center', behavior: 'instant' })
      card.focus({ preventScroll: true })
      timer = setTimeout(() => card?.removeAttribute('data-search-highlighted'), 2500)
    })
    return () => {
      cancelAnimationFrame(frame)
      clearTimeout(timer)
      card?.removeAttribute('data-search-highlighted')
    }
  }, [target])
  return <div ref={root}>{children}</div>
}
