import { reportCaughtError } from '@/lib/diagnostics'
import { ErrorNotice } from '@/components/error-notice'
import { Button } from '@/components/ui/button'
import { isTauri } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { Copy, Minus, Square, X } from 'lucide-react'
import { useEffect, useState, type MouseEvent } from 'react'
import { isAndroid } from '@/lib/platform'

export function WelcomeTitlebar() {
  const native = isTauri() && !isAndroid()
  const [maximized, setMaximized] = useState(false)
  const [focused, setFocused] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (!native) return
    const appWindow = getCurrentWindow()
    let disposed = false
    const listeners: UnlistenFn[] = []
    const retain = (unlisten: UnlistenFn) => {
      if (disposed) unlisten()
      else listeners.push(unlisten)
    }
    const updateMaximized = async () => {
      const value = await appWindow.isMaximized()
      if (!disposed) setMaximized(value)
    }
    void updateMaximized().catch((cause) =>
      reportCaughtError('app/src/features/onboarding/welcome-titlebar.tsx', cause)
    )
    void appWindow
      .isFocused()
      .then((value) => {
        if (!disposed) setFocused(value)
      })
      .catch((cause) =>
        reportCaughtError('app/src/features/onboarding/welcome-titlebar.tsx', cause)
      )
    void appWindow
      .onResized(() => {
        void updateMaximized().catch((cause) =>
          reportCaughtError('app/src/features/onboarding/welcome-titlebar.tsx', cause)
        )
      })
      .then(retain)
      .catch((cause) =>
        reportCaughtError('app/src/features/onboarding/welcome-titlebar.tsx', cause)
      )
    void appWindow
      .onFocusChanged(({ payload }) => {
        if (!disposed) setFocused(payload)
      })
      .then(retain)
      .catch((cause) =>
        reportCaughtError('app/src/features/onboarding/welcome-titlebar.tsx', cause)
      )
    return () => {
      disposed = true
      listeners.forEach((unlisten) => unlisten())
    }
  }, [native])

  async function windowAction(action: 'minimize' | 'toggleMaximize' | 'close' | 'startDragging') {
    if (!native) return
    try {
      setError(null)
      const appWindow = getCurrentWindow()
      await appWindow[action]()
      if (action === 'toggleMaximize') setMaximized(await appWindow.isMaximized())
    } catch (cause) {
      reportCaughtError(`welcome-window.${action}`, cause)
      setError('窗口操作失败，请重试。')
    }
  }

  function drag(event: MouseEvent<HTMLDivElement>) {
    if (event.button !== 0) return
    void windowAction(event.detail === 2 ? 'toggleMaximize' : 'startDragging')
  }

  return (
    <>
      <header className="welcome-titlebar" data-focused={focused}>
        <div className="welcome-titlebar-drag-region" aria-label="拖动窗口" onMouseDown={drag} />
        <div className="welcome-titlebar-controls">
          <Button
            variant="ghost"
            size="icon-sm"
            className="titlebar-control"
            aria-label="最小化"
            title="最小化"
            disabled={!native}
            onClick={() => void windowAction('minimize')}
          >
            <Minus aria-hidden="true" />
          </Button>
          <Button
            variant="ghost"
            size="icon-sm"
            className="titlebar-control"
            aria-label={maximized ? '还原窗口' : '最大化'}
            title={maximized ? '还原窗口' : '最大化'}
            disabled={!native}
            onClick={() => void windowAction('toggleMaximize')}
          >
            {maximized ? <Copy aria-hidden="true" /> : <Square aria-hidden="true" />}
          </Button>
          <Button
            variant="ghost"
            size="icon-sm"
            className="titlebar-control titlebar-close"
            aria-label="关闭窗口"
            title="关闭窗口"
            disabled={!native}
            onClick={() => void windowAction('close')}
          >
            <X aria-hidden="true" />
          </Button>
        </div>
      </header>
      {error && <ErrorNotice error={error} />}
    </>
  )
}
