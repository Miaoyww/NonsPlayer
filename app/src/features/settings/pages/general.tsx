import { CloseBehaviorSelect } from '@/features/settings/close-behavior-select'
import { AutostartSwitch } from '@/features/settings/autostart-switch'
import { BackgroundWebviewReleaseSwitch } from '@/features/settings/background-webview-release-switch'
import { isAndroid } from '@/lib/platform'
import { TelemetrySwitch } from '../telemetry-switch'
import { SettingsCard } from '../settings-card'
import { ToggleGroup, ToggleGroupItem } from '@/components/ui/toggle-group'
import { useTrackDoubleClickSettings } from '@/features/playback/use-track-double-click'

export function GeneralPage() {
  const [doubleClick, updateDoubleClick] = useTrackDoubleClickSettings()
  return (
    <div className="flex flex-col gap-8">
      <div>
        <h2 className="text-xl font-bold">常规</h2>
        <p className="mt-2 text-sm text-muted-foreground">
          {isAndroid()
            ? '调整歌曲操作与隐私统计。'
            : '调整歌曲操作、窗口行为、开机启动与隐私统计。'}
        </p>
      </div>
      <section aria-labelledby="general-song-actions-heading" className="flex flex-col gap-8">
        <h3 id="general-song-actions-heading" className="text-base font-semibold">
          歌曲操作
        </h3>
        <SettingsCard
          searchId="track-double-click-action"
          title="双击播放行为"
          description="将双击歌曲加入下一曲，或插入下一曲并立即播放；保留已有播放队列。"
        >
          <ToggleGroup
            aria-label="双击播放行为"
            value={[doubleClick.action]}
            onValueChange={(values) => {
              const action = values[0]
              if (action === 'next' || action === 'play') updateDoubleClick({ action })
            }}
          >
            <ToggleGroupItem value="next">加入下一曲</ToggleGroupItem>
            <ToggleGroupItem value="play">立即播放</ToggleGroupItem>
          </ToggleGroup>
        </SettingsCard>
        <SettingsCard
          searchId="track-double-click-scope"
          title="双击播放范围"
          description="播放此队列优先：用歌曲所在列表替换当前队列，并从双击歌曲开始播放。仅播放这一首或没有所属列表时，按上方行为加入下一曲。列表使用当前筛选、排序后已加载的歌曲。"
        >
          <ToggleGroup
            aria-label="双击播放范围"
            value={[doubleClick.scope]}
            onValueChange={(values) => {
              const scope = values[0]
              if (scope === 'queue' || scope === 'single') updateDoubleClick({ scope })
            }}
          >
            <ToggleGroupItem value="queue">播放此队列</ToggleGroupItem>
            <ToggleGroupItem value="single">仅播放这一首</ToggleGroupItem>
          </ToggleGroup>
        </SettingsCard>
      </section>
      {!isAndroid() && (
        <section aria-labelledby="general-window-heading" className="flex flex-col gap-8">
          <h3 id="general-window-heading" className="text-base font-semibold">
            窗口与启动
          </h3>
          <CloseBehaviorSelect />
          <BackgroundWebviewReleaseSwitch />
          <AutostartSwitch />
        </section>
      )}
      <section aria-labelledby="general-privacy-heading" className="flex flex-col gap-8">
        <h3 id="general-privacy-heading" className="text-base font-semibold">
          隐私与统计
        </h3>
        <TelemetrySwitch />
      </section>
    </div>
  )
}
