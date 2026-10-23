import { ListMusic, Volume1, Volume2 } from 'lucide-react'
import { isTauri } from '@tauri-apps/api/core'
import { useVolumeControl } from './use-volume-control'
import { ActionButton } from '@/components/music/action-button'
import { QualitySelect } from '@/features/playback/music-options'
import { PlaybackTimeline } from '@/features/playback/playback-timeline'
import { PlayerSlider } from '@/features/playback/player-slider'
import { PlaybackTransport } from '@/features/playback/playback-transport'
import { isAndroid } from '@/lib/platform'

export function NowPlayingControls({
  onQueue,
  onError,
  active = true
}: {
  onQueue: () => void
  onError: (error: unknown) => void
  active?: boolean
}) {
  const { volume, changeVolume, commitVolume } = useVolumeControl(onError)
  return (
    <div className="now-playing-controls flex shrink-0 flex-col" aria-label="正在播放控制">
      <PlaybackTimeline layout="below" onError={onError} active={active} />
      <PlaybackTransport withModes onError={onError} />
      {!isAndroid() && (
        <div className="flex items-center gap-3">
          <Volume1 className="size-4 shrink-0" aria-hidden="true" />
          <PlayerSlider
            className="min-w-0 flex-1"
            aria-label="音量"
            min={0}
            max={1}
            step={0.01}
            value={volume}
            disabled={!isTauri()}
            onValueChange={(next) => changeVolume(Number(next))}
            onValueCommitted={(next) => commitVolume(Number(next))}
          />
          <Volume2 className="size-5 shrink-0" aria-hidden="true" />
        </div>
      )}
      <div className="flex items-center justify-between gap-3">
        <QualitySelect />
        <ActionButton variant="ghost" size="icon" aria-label="显示播放队列" onClick={onQueue}>
          <ListMusic aria-hidden="true" />
        </ActionButton>
      </div>
    </div>
  )
}
