import './mobile-fullscreen-player.css'
import type { ReactNode } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { Info, Repeat, Repeat1 } from 'lucide-react'
import { ActionButton } from '@/components/music/action-button'
import { Cover } from '@/components/music/cover'
import { TrackArtists } from '@/components/music/music-links'
import { useSongActions } from '@/components/music/song-actions'
import { useMobileLyricsPage } from '@/features/lyrics/use-mobile-lyrics-page'
import { nativeCall, usePlayer, type Track } from '@/lib/player'
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
  onError: (cause: unknown) => void
  renderLyrics: () => ReactNode
  lyricControls: ReactNode
}) {
  const { lyricsPage, showLyricsPage, showOverview } = useMobileLyricsPage(present)
  const { repeatMode, shuffle } = usePlayer('repeatMode', 'shuffle')
  const actions = useSongActions()
  const repeatLabel =
    repeatMode === 'one' ? '单曲循环' : repeatMode === 'all' ? '列表循环' : '顺序播放'
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
      {lyricsPage && (
        <header className="mobile-lyrics-heading">
          <h1 className="truncate text-lg font-semibold">{track?.title}</h1>
          <p className="truncate text-sm text-muted-foreground">{track?.artist}</p>
        </header>
      )}
      <div className="mobile-fullscreen-content">
        {lyricsPage ? (
          <div
            className="mobile-fullscreen-lyrics now-playing-lyrics"
            role="button"
            tabIndex={0}
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
            {renderLyrics()}
          </div>
        ) : (
          <button
            type="button"
            className="mobile-fullscreen-open-lyrics"
            aria-label="查看歌词"
            disabled={!track}
            onClick={showLyricsPage}
          />
        )}
        {lyricsPage && <div className="mobile-lyric-controls">{lyricControls}</div>}
        <div className="mobile-fullscreen-bottom">
          <div className="mobile-track-row">
            <div className="mobile-track-identity">
              {!lyricsPage && (
                <>
                  <h1 className="truncate text-xl font-semibold tracking-tight">
                    {track?.title ?? '让音乐开始'}
                  </h1>
                  <p className="mt-1 truncate text-muted-foreground">
                    {track ? <TrackArtists track={track} /> : '选择一首喜欢的音乐'}
                  </p>
                </>
              )}
            </div>
            <div className="mobile-track-actions">
              <CurrentTrackLike track={track} onError={onError} />
              <NowPlayingMenu
                track={track}
                source={source}
                loading={loading}
                onRefresh={onRefresh}
                showLike={false}
              />
            </div>
          </div>
          <div className="mobile-fullscreen-controls now-playing-controls">
            <PlaybackTimeline layout="below" onError={onError} timeRowContent={<QualitySelect />} />
            <PlaybackTransport withModes onQueue={onQueue} onError={onError} />
            <div className="mobile-fullscreen-tools">
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
              <ActionButton
                variant="ghost"
                size="icon-lg"
                aria-label={`播放模式：${repeatLabel}，点击切换`}
                title={repeatLabel}
                aria-pressed={!shuffle && repeatMode !== 'off'}
                disabled={!isTauri()}
                onClick={() =>
                  void nativeCall('player_action', { action: 'repeat' }).catch(onError)
                }
              >
                {repeatMode === 'one' ? (
                  <Repeat1 aria-hidden="true" />
                ) : (
                  <Repeat aria-hidden="true" />
                )}
              </ActionButton>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}
