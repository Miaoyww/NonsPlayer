import { CloseBehaviorSelect } from '@/features/settings/general/close-behavior-select'
import { BackgroundWebviewReleaseSwitch } from '@/features/settings/general/background-webview-release-switch'
import { AutostartSwitch } from '@/features/settings/general/autostart-switch'
import { isAndroid } from '@/lib/platform'
import { TelemetrySwitch } from '../telemetry/telemetry-switch'
import { SettingsCard } from '../shell/settings-card'
import { Button } from '@/components/ui/button'
import { RotateCcw } from 'lucide-react'
import { reopenWelcome } from '@/features/onboarding/welcome-state'

export function GeneralPage() {
  return (
    <div className="flex flex-col gap-8">
      <div>
        <h2 className="text-xl font-bold">常规</h2>
        <p className="mt-2 text-sm text-muted-foreground">
          {isAndroid() ? '调整引导与隐私统计。' : '调整窗口与启动、引导和隐私统计。'}
        </p>
      </div>
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
      <section aria-labelledby="general-welcome-heading" className="flex flex-col gap-8">
        <h3 id="general-welcome-heading" className="text-base font-semibold">
          引导
        </h3>
        <SettingsCard
          searchId="welcome-page"
          title="欢迎页"
          description="重新选择音乐适配器和插件。打开后会暂时离开当前工作区，完成或跳过后返回。"
        >
          <Button variant="outline" onClick={reopenWelcome}>
            <RotateCcw aria-hidden="true" />
            重新打开
          </Button>
        </SettingsCard>
      </section>
      <section aria-labelledby="general-privacy-heading" className="flex flex-col gap-8">
        <h3 id="general-privacy-heading" className="text-base font-semibold">
          隐私与统计
        </h3>
        <TelemetrySwitch />
      </section>
    </div>
  )
}
