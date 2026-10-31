import { AggregateLibraryProvider } from '@/features/aggregate/aggregate-library'
import { MotionConfig } from 'motion/react'
import { Titlebar } from '@/features/workspace/titlebar'
import { WelcomeTitlebar } from '@/features/onboarding/welcome-titlebar'
import { MusicWorkspace } from '@/features/workspace/music-workspace'
import { MusicOptionsProvider } from '@/features/playback/music-options'
import { MusicNavigationProvider } from '@/features/workspace/music-navigation'
import { AccountProvider } from '@/features/account/account'
import { useState } from 'react'
import { PluginProvider, PluginSlot } from '@/plugins/host'
import { useAndroidWindowLayout } from '@/features/workspace/use-android-window-layout'
import { AndroidMediaLyrics } from '@/features/lyrics/android-media-lyrics'
import { isAndroid } from '@/lib/platform'
import { UpdateProvider } from '@/features/updates/update-provider'
import { WelcomePage } from '@/features/onboarding/welcome-page'
import { hasCompletedWelcome, openWelcomeEvent } from '@/features/onboarding/welcome-state'
import { useEffect } from 'react'

function App() {
  const [welcomeOpen, setWelcomeOpen] = useState(() => !hasCompletedWelcome())
  const [playerPhase, setPlayerPhase] = useState<'closed' | 'open' | 'closing'>('closed')
  const nowPlaying = playerPhase === 'open'
  useAndroidWindowLayout(playerPhase !== 'closed')
  const setNowPlaying = (value: boolean) =>
    setPlayerPhase((phase) => (value ? 'open' : phase === 'closed' ? 'closed' : 'closing'))
  useEffect(() => {
    const open = () => {
      setWelcomeOpen(true)
      setPlayerPhase('closed')
    }
    window.addEventListener(openWelcomeEvent, open)
    return () => window.removeEventListener(openWelcomeEvent, open)
  }, [])

  const completeWelcome = () => {
    setWelcomeOpen(false)
    setPlayerPhase('closed')
  }

  return (
    <MotionConfig reducedMotion="user">
      <UpdateProvider>
        {isAndroid() && <AndroidMediaLyrics />}
        <AccountProvider>
          <MusicOptionsProvider>
            <AggregateLibraryProvider>
              <MusicNavigationProvider>
                <PluginProvider>
                  <div
                    className={`app-shell relative flex h-dvh flex-col overflow-hidden text-foreground ${welcomeOpen ? 'welcome-app-shell' : 'bg-background'} ${nowPlaying ? 'now-playing' : ''}`}
                  >
                    {welcomeOpen ? (
                      <>
                        <WelcomeTitlebar />
                        <WelcomePage onComplete={completeWelcome} />
                      </>
                    ) : (
                      <>
                        <Titlebar playerMode={nowPlaying} onBack={() => setNowPlaying(false)} />
                        <MusicWorkspace
                          nowPlaying={nowPlaying}
                          playerVisible={playerPhase !== 'closed'}
                          onNowPlayingChange={setNowPlaying}
                          onPlayerExitComplete={() =>
                            setPlayerPhase((phase) => (phase === 'closing' ? 'closed' : phase))
                          }
                        />
                        <div
                          className="pointer-events-none absolute inset-x-0 top-16 z-50 flex flex-col items-center gap-2"
                          aria-hidden={playerPhase !== 'closed'}
                          style={{ visibility: playerPhase !== 'closed' ? 'hidden' : undefined }}
                        >
                          <PluginSlot name="main.overlay" />
                        </div>
                      </>
                    )}
                  </div>
                </PluginProvider>
              </MusicNavigationProvider>
            </AggregateLibraryProvider>
          </MusicOptionsProvider>
        </AccountProvider>
      </UpdateProvider>
    </MotionConfig>
  )
}

export default App
