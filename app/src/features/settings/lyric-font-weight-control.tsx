import { useEffect, useState } from 'react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { ToggleGroup, ToggleGroupItem } from '@/components/ui/toggle-group'
import { useLyricsSettings } from '@/features/lyrics/use-lyrics-settings'

export function LyricFontWeightControl() {
  const { fontWeight, setFontWeight } = useLyricsSettings()
  const [draft, setDraft] = useState(String(fontWeight))
  useEffect(() => setDraft(String(fontWeight)), [fontWeight])
  return (
    <div className="flex flex-wrap items-center gap-3">
      <ToggleGroup
        aria-label="歌词字重预设"
        value={[String(fontWeight)]}
        onValueChange={(values) => {
          const value = Number(values[0])
          if (value === 400 || value === 500 || value === 700) setFontWeight(value)
        }}
      >
        <ToggleGroupItem value="400">常规</ToggleGroupItem>
        <ToggleGroupItem value="500">中等</ToggleGroupItem>
        <ToggleGroupItem value="700">粗体</ToggleGroupItem>
      </ToggleGroup>
      <form
        className="flex items-center gap-2"
        onSubmit={(event) => {
          event.preventDefault()
          setFontWeight(Number(draft))
        }}
      >
        <label htmlFor="lyrics-custom-font-weight" className="text-sm">
          自定义
        </label>
        <Input
          id="lyrics-custom-font-weight"
          type="number"
          min={100}
          max={900}
          step={1}
          required
          className="w-24"
          value={draft}
          onChange={(event) => setDraft(event.target.value)}
        />
        <Button variant="outline" size="sm" type="submit" disabled={Number(draft) === fontWeight}>
          应用
        </Button>
      </form>
    </div>
  )
}
