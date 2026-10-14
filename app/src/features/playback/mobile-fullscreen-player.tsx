import './mobile-fullscreen-player.css'
import { useState, type ReactNode } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { ChevronDown, Info } from 'lucide-react'
import { ActionButton } from '@/components/music/action-button'
import { Cover } from '@/components/music/cover'
import { useSongActions } from '@/components/music/song-actions'
import { useMobileLyricsPage } from '@/features/lyrics/use-mobile-lyrics-page'
import type { Track } from '@/lib/player'
import { CurrentTrackLike } from './current-track-like'
import { QualitySelect } from './music-options'
import { NowPlayingMenu } from './now-playing-menu'
import { PlaybackTimeline } from './playback-timeline'
import { PlaybackTransport } from './playback-transport'

export function MobileFullscreenPlayer({
  track,
  present,
  source,
  loading,
  onRefresh,
  onQueue,
  onClose,
  onError,
  renderLyrics,
  lyricControls
}: {
  track?: Track
  present: boolean
  source?: string
  loading: boolean
  onRefresh: () => void
  onQueue: () => void
  onClose: () => void
  onError: (cause: unknown) => void
  renderLyrics: (active: boolean) => ReactNode
  lyricControls: ReactNode
}) {
  const { lyricsPage, showLyricsPage, showOverview } = useMobileLyricsPage(present)
  const [lyricsVisited, setLyricsVisited] = useState(false)
  const actions = useSongActions()
  return (
    <div className="mobile-fullscreen-player" data-mobile-view={lyricsPage ? 'lyrics' : 'overview'}>
      <div className="mobile-fullscreen-artwork" aria-hidden="true">
        <Cover
          cover={track?.cover}
          size="original"
          loading="eager"
          className="mobile-fullscreen-cover"
        />
      </div>
      <header className="mobile-fullscreen-header">
        <ActionButton
          autoFocus
          variant="ghost"
          size="icon-lg"
          aria-label="返回音乐"
          onClick={() => (lyricsPage ? showOverview() : onClose())}
        >
          <ChevronDown aria-hidden="true" />
        </ActionButton>
        <Cover cover={track?.cover} size="small" loading="eager" className="mobile-header-cover" />
        <div className="mobile-header-identity">
          <h1 className="truncate font-semibold">{track?.title ?? '让音乐开始'}</h1>
          <p className="truncate text-sm text-muted-foreground">
            {track?.artist ?? '选择一首喜欢的音乐'}
          </p>
        </div>
        <CurrentTrackLike track={track} onError={onError} />
        <NowPlayingMenu
          track={track}
          source={source}
          loading={loading}
          onRefresh={onRefresh}
          showLike={false}
        />
      </header>
      <div className="mobile-fullscreen-content">
        <div className="mobile-fullscreen-stage">
          {lyricsVisited && (
            <div
              className="mobile-fullscreen-lyrics now-playing-lyrics"
              data-active={lyricsPage}
              aria-hidden={!lyricsPage}
              inert={!lyricsPage}
              role="button"
              tabIndex={lyricsPage ? 0 : -1}
              aria-label="返回封面与播放控制"
              onClickCapture={(event) => {
                event.preventDefault()
                event.stopPropagation()
                showOverview()
              }}
              onKeyDown={(event) => {
                if (event.key === 'Enter' || event.key === ' ') {
                  event.preventDefault()
                  showOverview()
                }
              }}
            >
              {renderLyrics(lyricsPage && present)}
            </div>
          )}
          {!lyricsPage && (
            <button
              type="button"
              className="mobile-fullscreen-open-lyrics"
              aria-label="查看歌词"
              disabled={!track}
              onClick={() => {
                setLyricsVisited(true)
                showLyricsPage()
              }}
            />
          )}
        </div>
        <div className="mobile-fullscreen-bottom">
          {lyricsPage && <div className="mobile-lyric-controls">{lyricControls}</div>}
          <div className="mobile-fullscreen-controls now-playing-controls">
            <PlaybackTimeline layout="below" onError={onError} timeRowContent={<QualitySelect />} />
            <PlaybackTransport
              withModes
              onQueue={onQueue}
              onError={onError}
              leadingControl={
                <ActionButton
                  variant="ghost"
                  size="icon-lg"
                  aria-label="查看歌曲信息"
                  disabled={!track || !isTauri()}
                  onClick={() => {
                    if (track) actions.details(track)
                  }}
                >
                  <Info aria-hidden="true" />
                </ActionButton>
              }
            />
          </div>
        </div>
      </div>
    </div>
  )
}
