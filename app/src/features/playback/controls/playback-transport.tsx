import { SkipForward, Pause, Play, Repeat, Repeat1, SkipBack, Shuffle } from 'lucide-react'
import { isTauri } from '@tauri-apps/api/core'
import { adjacentIndex, nativeCall, usePlayer } from '@/lib/player'
import { ActionButton } from '@/components/music/action-button'

export function PlaybackTransport({
  onError,
  withModes = false
}: {
  onError: (error: unknown) => void
  withModes?: boolean
}) {
  const state = usePlayer('index', 'queue', 'repeatMode', 'shuffle', 'shuffleOrder', 'status')
  const playing = ['playing', 'buffering', 'loading'].includes(state.status)
  const disabled = !isTauri() || state.index === null
  const action = (action: string) => void nativeCall('player_action', { action }).catch(onError)
  const repeatLabel =
    state.repeatMode === 'one' ? '单曲循环' : state.repeatMode === 'all' ? '列表循环' : '顺序播放'
  return (
    <div
      className="now-playing-transport flex items-center justify-center"
      aria-label="播放控制按钮"
    >
      {withModes && (
        <ActionButton
          variant="ghost"
          size="icon-lg"
          aria-label="随机播放"
          className="playback-mode-button"
          title={state.shuffle ? '关闭随机播放' : '开启随机播放'}
          aria-pressed={state.shuffle}
          disabled={!isTauri()}
          onClick={() => action('shuffle')}
        >
          <Shuffle aria-hidden="true" />
        </ActionButton>
      )}
      <ActionButton
        variant="ghost"
        size="icon-lg"
        disabled={disabled || adjacentIndex(state, 'previous') === null}
        aria-label="上一首"
        onClick={() => action('previous')}
      >
        <SkipBack className="size-6 fill-current" aria-hidden="true" />
      </ActionButton>
      <ActionButton
        variant="ghost"
        size="icon-lg"
        className="now-playing-toggle"
        disabled={disabled}
        aria-label={playing ? '暂停' : '播放'}
        onClick={() => action('toggle')}
      >
        {playing ? <Pause aria-hidden="true" /> : <Play aria-hidden="true" />}
      </ActionButton>
      <ActionButton
        variant="ghost"
        size="icon-lg"
        disabled={disabled || adjacentIndex(state, 'next') === null}
        aria-label="下一首"
        onClick={() => action('next')}
      >
        <SkipForward className="size-6 fill-current" aria-hidden="true" />
      </ActionButton>
      {withModes && (
        <ActionButton
          variant="ghost"
          size="icon-lg"
          aria-label={`播放模式：${repeatLabel}，点击切换`}
          className="playback-mode-button"
          title={repeatLabel}
          aria-pressed={!state.shuffle && state.repeatMode !== 'off'}
          disabled={!isTauri()}
          onClick={() => action('repeat')}
        >
          {state.repeatMode === 'one' ? (
            <Repeat1 aria-hidden="true" />
          ) : (
            <Repeat aria-hidden="true" />
          )}
        </ActionButton>
      )}
    </div>
  )
}
