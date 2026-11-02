import { Button } from '@/components/ui/button'
import { Slider } from '@/components/ui/slider'
import {
  defaultLyricsFontWeight,
  maxLyricsFontWeight,
  minLyricsFontWeight
} from '@/features/lyrics/use-lyrics-settings'

export function LyricFontWeightControl({
  value,
  onChange,
  label = '歌词字重',
  defaultWeight = defaultLyricsFontWeight
}: {
  value: number
  onChange: (value: number) => void
  label?: string
  defaultWeight?: number
}) {
  return (
    <div className="flex w-60 flex-col gap-2">
      <div className="flex items-center gap-3">
        <Slider
          className="min-w-0 flex-1"
          aria-label={label}
          min={minLyricsFontWeight}
          max={maxLyricsFontWeight}
          step={100}
          value={value}
          onValueChange={(next) => onChange(Number(next))}
        />
        <output className="w-12 text-right text-sm tabular-nums" aria-label={`当前${label}`}>
          {value}
        </output>
      </div>
      <div className="flex justify-between text-xs text-muted-foreground">
        <span>细</span>
        <span>粗</span>
      </div>
      <Button
        variant="ghost"
        size="sm"
        className="self-end"
        onClick={() => onChange(defaultWeight)}
      >
        恢复默认
      </Button>
    </div>
  )
}
