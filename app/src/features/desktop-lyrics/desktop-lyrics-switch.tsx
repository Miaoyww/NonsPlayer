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
    <div className="flex flex-col items-end gap-1">
      <Switch
        aria-label="启用桌面歌词"
        checked={state.enabled}
        disabled={busy}
        onCheckedChange={(checked) => void run(() => setDesktopLyricsEnabled(checked))}
      />
      {error && <p className="text-xs text-destructive">{error}</p>}
    </div>
  )
}
