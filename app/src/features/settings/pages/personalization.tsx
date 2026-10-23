import { Monitor, Moon, Sun } from 'lucide-react'
import type { Theme } from '@/features/settings/use-theme'
import { FontPicker } from '@/features/settings/font-picker'
import { useFontSettings } from '@/features/settings/use-font-settings'
import { SettingsCard } from '@/features/settings/settings-card'
import { isAndroid } from '@/lib/platform'
import { usePlaybackBarMode, type PlaybackBarMode } from '@/features/playback/use-playback-bar-mode'
import { ToggleGroup, ToggleGroupItem } from '@/components/ui/toggle-group'
import {
  useInterfaceDensity,
  type InterfaceDensity
} from '@/features/settings/use-interface-density'

const themes = [
  { value: 'light', label: '浅色', icon: Sun },
  { value: 'dark', label: '深色', icon: Moon },
  { value: 'system', label: '跟随系统', icon: Monitor }
] as const

export function PersonalizationPage({
  theme,
  onThemeChange
}: {
  theme: Theme
  onThemeChange: (theme: Theme) => void
}) {
  const { fonts, setFont } = useFontSettings()
  const [barMode, setBarMode] = usePlaybackBarMode()
  const [density, setDensity] = useInterfaceDensity()
  return (
    <div className="flex flex-col gap-8">
      <div>
        <h2 className="text-xl font-bold">个性化</h2>
        <p className="mt-2 text-sm text-muted-foreground">
          {isAndroid()
            ? '调整界面主题、应用字体与界面密度。'
            : '调整界面主题、应用字体、播放栏显示方式与界面密度。'}
        </p>
      </div>
      <SettingsCard title="界面主题" description="选择浅色、深色或跟随系统。">
        <div role="radiogroup" aria-label="界面主题" className="flex flex-wrap gap-1.5">
          {themes.map(({ value, label, icon: Icon }) => (
            <label key={value} className="relative cursor-pointer">
              <input
                className="peer sr-only"
                type="radio"
                name="theme"
                value={value}
                checked={theme === value}
                onChange={() => onThemeChange(value)}
              />
              <span className="flex h-8 items-center gap-1.5 rounded-lg px-3 text-xs text-muted-foreground transition-colors hover:bg-accent peer-checked:bg-secondary peer-checked:text-secondary-foreground peer-focus-visible:ring-2 peer-focus-visible:ring-ring">
                <Icon className="size-3.5" aria-hidden="true" />
                {label}
              </span>
            </label>
          ))}
        </div>
      </SettingsCard>
      <SettingsCard title="应用字体" description="选择已安装字体，缺失字符使用系统后备字体。">
        <FontPicker
          label="应用字体"
          value={fonts.app}
          onChange={(family) => setFont('app', family)}
          defaultLabel="应用默认字体"
        />
      </SettingsCard>
      {!isAndroid() && (
        <SettingsCard
          title="播放栏"
          description="选择底部播放栏的显示方式。关闭后仍可使用全屏播放器与快捷键。"
        >
          <ToggleGroup
            aria-label="播放栏显示方式"
            value={[barMode]}
            onValueChange={(values) => {
              if (values[0]) setBarMode(values[0] as PlaybackBarMode)
            }}
          >
            <ToggleGroupItem value="collapsible">折叠式</ToggleGroupItem>
            <ToggleGroupItem value="persistent">常驻式</ToggleGroupItem>
            <ToggleGroupItem value="off">关闭</ToggleGroupItem>
          </ToggleGroup>
        </SettingsCard>
      )}
      <SettingsCard
        title="界面密度"
        description="统一调整音乐页面的留白、详情封面、标题与歌曲行高。紧凑式显示更多内容，封面、歌名和艺术家仍一起放在左侧。"
      >
        <ToggleGroup
          aria-label="界面密度"
          value={[density]}
          onValueChange={(values) => {
            if (values[0]) setDensity(values[0] as InterfaceDensity)
          }}
        >
          <ToggleGroupItem value="standard">标准式</ToggleGroupItem>
          <ToggleGroupItem value="compact">紧凑式</ToggleGroupItem>
        </ToggleGroup>
      </SettingsCard>
    </div>
  )
}
