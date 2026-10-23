import { CloseBehaviorSelect } from '@/features/settings/close-behavior-select'
import { AutostartSwitch } from '@/features/settings/autostart-switch'
import { isAndroid } from '@/lib/platform'
import { TelemetrySwitch } from '../telemetry-switch'

export function GeneralPage() {
  return (
    <div className="flex flex-col gap-8">
      <div>
        <h2 className="text-xl font-bold">常规</h2>
        <p className="mt-2 text-sm text-muted-foreground">调整应用行为与匿名使用统计。</p>
      </div>
      {!isAndroid() && <CloseBehaviorSelect />}
      {!isAndroid() && <AutostartSwitch />}
      <TelemetrySwitch />
    </div>
  )
}
