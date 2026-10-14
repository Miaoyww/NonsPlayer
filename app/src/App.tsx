import { MotionConfig } from 'motion/react'
import { Titlebar } from '@/features/workspace/titlebar'
import { MusicWorkspace } from '@/features/workspace/music-workspace'
import { MusicOptionsProvider } from '@/features/playback/music-options'
import { MusicNavigationProvider } from '@/features/workspace/music-navigation'
import { AccountProvider } from '@/features/account/account'
import { useEffect, useState } from 'react'
import { PluginProvider, PluginSlot } from '@/plugins/host'
import { useAndroidWindowLayout } from '@/features/workspace/use-android-window-layout'
import { AndroidMediaLyrics } from '@/features/lyrics/android-media-lyrics'
import { isAndroid } from '@/lib/platform'

function App() {
  const [playerPhase, setPlayerPhase] = useState<'closed' | 'open' | 'closing'>(() => {
    try {
      return localStorage.getItem('nons-now-playing-open') === 'true' ? 'open' : 'closed'
    } catch {
      return 'closed'
    }
  })
  useEffect(() => {
    try {
      localStorage.setItem('nons-now-playing-open', String(playerPhase === 'open'))
    } catch {
      /* Storage may be unavailable. */
    }
  }, [playerPhase])
  const nowPlaying = playerPhase === 'open'
  useAndroidWindowLayout(playerPhase !== 'closed')
  const setNowPlaying = (value: boolean) =>
    setPlayerPhase((phase) => (value ? 'open' : phase === 'closed' ? 'closed' : 'closing'))
  return (
    <MotionConfig reducedMotion="user">
      {isAndroid() && <AndroidMediaLyrics />}
      <AccountProvider>
        <MusicOptionsProvider>
          <MusicNavigationProvider>
            <PluginProvider>
              <div
                className={`app-shell relative flex h-dvh flex-col overflow-hidden bg-background text-foreground ${nowPlaying ? 'now-playing' : ''}`}
              >
                <Titlebar playerMode={nowPlaying} onBack={() => setNowPlaying(false)} />
                <MusicWorkspace
                  nowPlaying={nowPlaying}
                  playerVisible={playerPhase !== 'closed'}
                  onNowPlayingChange={setNowPlaying}
                  onPlayerExitComplete={() =>
                    setPlayerPhase((phase) => (phase === 'closing' ? 'closed' : phase))
                  }
                />
                <div className="pointer-events-none absolute inset-x-0 top-16 z-50 flex flex-col items-center gap-2">
                  <PluginSlot name="main.overlay" />
                </div>
              </div>
            </PluginProvider>
          </MusicNavigationProvider>
        </MusicOptionsProvider>
      </AccountProvider>
    </MotionConfig>
  )
}

export default App
