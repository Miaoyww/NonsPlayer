import { MicVocal } from 'lucide-react'
import { isTauri } from '@tauri-apps/api/core'
import { useState } from 'react'
import { ActionButton } from '@/components/music/action-button'
import { reportCaughtError } from '@/lib/diagnostics'
import { setDesktopLyricsEnabled, useDesktopLyricsState } from './state'

export function DesktopLyricsButton({
  onError,
  size = 'icon-sm'
}: {
  onError: (error: unknown) => void
  size?: 'icon-sm' | 'icon-lg'
}) {
  const { enabled } = useDesktopLyricsState()
  const [busy, setBusy] = useState(false)
  const label = enabled ? '关闭桌面歌词' : '开启桌面歌词'

  return (
    <ActionButton
      variant="ghost"
      size={size}
      className="desktop-lyrics-button playback-mode-button"
      aria-label={label}
      title={label}
      aria-pressed={enabled}
      aria-busy={busy}
      disabled={!isTauri() || busy}
      onClick={() => {
        if (busy) return
        setBusy(true)
        void setDesktopLyricsEnabled(!enabled)
          .catch((cause) => {
            reportCaughtError('app/src/features/desktop-lyrics/desktop-lyrics-button.tsx', cause)
            onError(cause)
          })
          .finally(() => setBusy(false))
      }}
    >
      <MicVocal aria-hidden="true" />
    </ActionButton>
  )
}
