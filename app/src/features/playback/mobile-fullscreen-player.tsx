import './mobile-fullscreen-player.css'
import { useState, type ReactNode } from 'react'
import { ChevronDown } from 'lucide-react'
import { ActionButton } from '@/components/music/action-button'
import { Cover } from '@/components/music/cover'
import { TrackArtists } from '@/components/music/music-links'
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
  onClose,
  onError,
  renderLyrics,
  lyricControls,
  background
}: {
  track?: Track
  present: boolean
  source?: string
  loading: boolean
  onRefresh: () => void
  onClose: () => void
  onError: (cause: unknown) => void
  renderLyrics: (active: boolean) => ReactNode
  lyricControls: ReactNode
  background: ReactNode
}) {
  const { lyricsPage, showLyricsPage, showOverview } = useMobileLyricsPage(present)
  const [lyricsVisited, setLyricsVisited] = useState(false)
  const trackActions = (
    <>
      <CurrentTrackLike track={track} onError={onError} />
      <NowPlayingMenu
        track={track}
        source={source}
        loading={loading}
        onRefresh={onRefresh}
        showLike={false}
      />
    </>
  )
  return (
    <div className="mobile-fullscreen-player" data-mobile-view={lyricsPage ? 'lyrics' : 'overview'}>
      {background}
      <div className="mobile-fullscreen-header-slot">
        {lyricsPage ? (
          <header className="mobile-fullscreen-header">
            <ActionButton variant="ghost" size="icon-lg" aria-label="返回音乐" onClick={onClose}>
              <ChevronDown aria-hidden="true" />
            </ActionButton>
            <Cover
              cover={track?.cover}
              size="small"
              loading="eager"
              className="mobile-header-cover"
            />
            <div className="mobile-header-identity">
              <h1 className="truncate font-semibold">{track?.title ?? '让音乐开始'}</h1>
              <p className="truncate text-sm text-muted-foreground">
                {track?.artist ?? '选择一首喜欢的音乐'}
              </p>
            </div>
            {trackActions}
          </header>
        ) : (
          <ActionButton
            variant="ghost"
            size="icon-lg"
            className="mobile-cover-dismiss"
            aria-label="返回音乐"
            onClick={onClose}
          >
            <ChevronDown aria-hidden="true" />
          </ActionButton>
        )}
      </div>
      <div className="mobile-fullscreen-content">
        <div className="mobile-fullscreen-stage">
          <div className="mobile-fullscreen-artwork" aria-hidden="true">
            <Cover
              cover={track?.cover}
              size="original"
              loading="eager"
              className="mobile-fullscreen-cover rounded-xl shadow-2xl"
            />
          </div>

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
          <div
            className="mobile-cover-identity"
            data-active={!lyricsPage}
            aria-hidden={lyricsPage}
            inert={lyricsPage}
          >
            <div className="min-w-0 flex-1">
              <h1 className="truncate text-xl font-semibold tracking-tight">
                {track?.title ?? '让音乐开始'}
              </h1>
              <p className="mt-1 truncate text-muted-foreground">
                {track ? <TrackArtists track={track} /> : '选择一首喜欢的音乐'}
              </p>
            </div>
            <div className="mobile-cover-actions">{trackActions}</div>
          </div>
          <div className="mobile-fullscreen-controls now-playing-controls">
            <div className="mobile-timeline-slot">
              {lyricsPage && <div className="mobile-lyric-controls">{lyricControls}</div>}
              <PlaybackTimeline
                layout="below"
                onError={onError}
                timeRowContent={<QualitySelect />}
              />
            </div>
            <PlaybackTransport withModes onError={onError} />
          </div>
        </div>
      </div>
    </div>
  )
}
