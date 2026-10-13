import { SettingsCard } from '@/features/settings/settings-card'
import { isAndroid } from '@/lib/platform'
import { usePlaybackBarMode, type PlaybackBarMode } from '@/features/playback/use-playback-bar-mode'
import { ToggleGroup, ToggleGroupItem } from '@/components/ui/toggle-group'
import {
  useInterfaceDensity,
  type InterfaceDensity
} from '@/features/settings/use-interface-density'

export function PersonalizationPage() {
  const [barMode, setBarMode] = usePlaybackBarMode()
  const [density, setDensity] = useInterfaceDensity()
  return (
    <div className="flex flex-col gap-8">
      <div>
        <h2 className="text-xl font-bold">个性化</h2>
        <p className="mt-2 text-sm text-muted-foreground">
          {isAndroid() ? '调整界面密度。' : '调整播放栏显示方式与界面密度。'}
        </p>
      </div>
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
