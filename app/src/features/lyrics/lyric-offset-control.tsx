import { Minus, Plus } from 'lucide-react'
import { ActionButton } from '@/components/music/action-button'

export function LyricOffsetControl({
  offsetMs,
  disabled = false,
  onChange
}: {
  offsetMs: number
  disabled?: boolean
  onChange: (value: number) => void
}) {
  return (
    <div className="lyric-offset-control" role="group" aria-label="单曲额外歌词偏移">
      <ActionButton
        variant="ghost"
        size="icon"
        disabled={disabled || offsetMs >= 60000}
        aria-label="歌词延后 0.1 秒"
        title="歌词延后 0.1 秒"
        onClick={() => onChange(Math.min(60000, offsetMs + 100))}
      >
        <Plus aria-hidden="true" />
      </ActionButton>
      <output
        className="lyric-offset-value"
        aria-label="单曲额外偏移，单位秒"
        aria-live="polite"
        title="叠加在全局歌词偏移上"
      >
        {offsetMs > 0 ? '+' : ''}
        {(offsetMs / 1000).toFixed(1)} s
      </output>
      <ActionButton
        variant="ghost"
        size="icon"
        disabled={disabled || offsetMs <= -60000}
        aria-label="歌词提前 0.1 秒"
        title="歌词提前 0.1 秒"
        onClick={() => onChange(Math.max(-60000, offsetMs - 100))}
      >
        <Minus aria-hidden="true" />
      </ActionButton>
    </div>
  )
}
