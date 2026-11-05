import { useEffect, useRef, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle
} from '@/components/ui/dialog'
import { ErrorNotice } from '@/components/error-notice'
import { isAndroid } from '@/lib/platform'
import { reportCaughtError } from '@/lib/diagnostics'
import { errorText, nativeCall } from '@/lib/player'

export function CloseBehaviorDialog() {
  const [open, setOpen] = useState(false)
  const [remember, setRemember] = useState(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string>()
  const visible = useRef(false)
  const submitting = useRef(false)

  useEffect(() => {
    if (!isTauri() || isAndroid()) return
    let disposed = false
    let stop: (() => void) | undefined
    async function request() {
      if (disposed) return
      try {
        const pending = await nativeCall<boolean>('take_main_close_request')
        if (disposed || !pending || visible.current || submitting.current) return
        visible.current = true
        setRemember(false)
        setError(undefined)
        setOpen(true)
      } catch (cause) {
        reportCaughtError('window.close_behavior.request', cause)
      }
    }
    void listen('main-close-requested', () => void request())
      .then((unlisten) => {
        if (disposed) {
          unlisten()
          return
        }
        stop = unlisten
        void request()
      })
      .catch((cause) => reportCaughtError('window.close_behavior.listen', cause))
    return () => {
      disposed = true
      stop?.()
    }
  }, [])

  function changeOpen(value: boolean) {
    if (submitting.current) return
    visible.current = value
    setOpen(value)
  }

  async function confirm(action: 'close' | 'minimize') {
    if (submitting.current) return
    submitting.current = true
    setBusy(true)
    setError(undefined)
    try {
      await nativeCall('confirm_main_close', { action, remember })
      visible.current = false
      setOpen(false)
    } catch (cause) {
      reportCaughtError('window.close_behavior.confirm', cause)
      setError(errorText(cause))
    } finally {
      submitting.current = false
      setBusy(false)
    }
  }

  return (
    <Dialog open={open} onOpenChange={changeOpen}>
      <DialogContent showCloseButton={!busy}>
        <DialogHeader>
          <DialogTitle>关闭主面板时</DialogTitle>
          <DialogDescription>
            最小化会收起到托盘并继续播放；关闭应用会停止播放并退出。
          </DialogDescription>
        </DialogHeader>
        <label className="flex items-center gap-2 text-sm">
          <input
            type="checkbox"
            className="size-4 accent-primary"
            checked={remember}
            disabled={busy}
            onChange={(event) => setRemember(event.target.checked)}
          />
          记住我的选择
        </label>
        {error && <ErrorNotice error={error} />}
        <DialogFooter>
          <Button variant="ghost" disabled={busy} onClick={() => changeOpen(false)}>
            取消
          </Button>
          <Button variant="outline" disabled={busy} onClick={() => void confirm('minimize')}>
            最小化
          </Button>
          <Button disabled={busy} onClick={() => void confirm('close')}>
            关闭应用
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
