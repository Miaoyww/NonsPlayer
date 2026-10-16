import { useLayoutEffect, useRef, useState, useSyncExternalStore } from 'react'
import { errorNotices } from '@/features/playback/error-notices'

function hasVisiblePlayback() {
  return [...document.querySelectorAll('[data-playback-notices]')].some((slot) => {
    for (let element: Element | null = slot; element; element = element.parentElement) {
      const style = window.getComputedStyle(element)
      if (
        element.hasAttribute('hidden') ||
        element.getAttribute('aria-hidden') === 'true' ||
        style.display === 'none' ||
        style.visibility === 'hidden'
      )
        return false
    }
    return true
  })
}

/** Dialogs own their errors. Other pages use a mounted playback notice slot. */
export function ErrorNotice({ error: message }: { error?: string | null }) {
  const error = message === '账号或音乐来源状态已变化，请重试' ? undefined : message
  const anchor = useRef<HTMLSpanElement>(null)
  const [inline, setInline] = useState(false)
  useLayoutEffect(() => {
    if (!error) return
    const local = !!anchor.current?.closest('[role="dialog"], [role="alertdialog"]')
    const playback = !local && hasVisiblePlayback()
    setInline(local || !playback)
    if (!local && playback) errorNotices.push(error)
  }, [error])
  return (
    <>
      <span ref={anchor} hidden />
      {error && inline && (
        <p role="alert" className="whitespace-pre-line break-words text-sm text-destructive">
          {error}
        </p>
      )}
    </>
  )
}

/** Render in the existing bar or full-screen slot, inheriting its placement. */
export function ErrorNoticeViewport() {
  const messages = useSyncExternalStore(errorNotices.subscribe, errorNotices.getSnapshot)
  return messages.map((item) => (
    <p key={item.id} role="alert" className="playback-notice glass-surface">
      {item.message}
    </p>
  ))
}
