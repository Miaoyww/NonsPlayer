import type { Theme } from '@/features/settings/use-theme'
import { PersonalizationControls } from '@/features/settings/personalization-controls'

export function PersonalizationPage({
  theme,
  onThemeChange
}: {
  theme: Theme
  onThemeChange: (theme: Theme) => void
}) {
  return (
    <div className="flex flex-col gap-8">
      <div>
        <h2 className="text-xl font-bold">个性化</h2>
        <p className="mt-2 text-sm text-muted-foreground">
          调整界面主题、应用字体、播放栏显示方式与界面密度。
        </p>
      </div>
      <PersonalizationControls theme={theme} onThemeChange={onThemeChange} />
    </div>
  )
}
