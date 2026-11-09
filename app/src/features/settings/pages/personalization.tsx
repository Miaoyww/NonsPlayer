import type { Theme } from '@/features/settings/appearance/use-theme'
import { PersonalizationControls } from '@/features/settings/appearance/personalization-controls'
import { TaskbarSettings } from '@/features/settings/appearance/taskbar-settings'
import { isWindows } from '@/lib/platform'

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
          调整界面主题、毛玻璃效果、应用字体、播放栏显示方式、界面密度与任务栏缩略图。
        </p>
      </div>
      <PersonalizationControls theme={theme} onThemeChange={onThemeChange} />
      {isWindows() && (
        <section aria-labelledby="personalization-taskbar-heading" className="flex flex-col gap-8">
          <h3 id="personalization-taskbar-heading" className="text-base font-semibold">
            任务栏
          </h3>
          <TaskbarSettings />
        </section>
      )}
    </div>
  )
}
