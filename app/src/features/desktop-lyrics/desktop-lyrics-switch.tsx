import { useState } from 'react'
import { Switch } from '@/components/ui/switch'
import { reportCaughtError } from '@/lib/diagnostics'
import { errorText } from '@/lib/player'
import { setDesktopLyricsEnabled, useDesktopLyricsState } from './state'

export function DesktopLyricsSwitch() {
  const state = useDesktopLyricsState()
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')

  const run = async (action: () => Promise<unknown>) => {
    if (busy) return
    setBusy(true)
    setError('')
    try {
      await action()
    } catch (cause) {
      reportCaughtError('app/src/features/desktop-lyrics/desktop-lyrics-switch.tsx', cause)
      setError(errorText(cause))
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="flex w-60 flex-col gap-3">
      <label className="flex items-center justify-between gap-3 text-sm">
        <span>启用桌面歌词</span>
        <Switch
          aria-label="启用桌面歌词"
          checked={state.enabled}
          disabled={busy}
          onCheckedChange={(checked) => void run(() => setDesktopLyricsEnabled(checked))}
        />
      </label>
      {state.enabled && state.locked && (
        <p className="text-xs leading-relaxed text-muted-foreground">
          解锁后可拖动窗口、切换翻译或发音、关闭或再次锁定。
        </p>
      )}
      {error && <p className="text-xs text-destructive">{error}</p>}
    </div>
  )
}
