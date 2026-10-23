import { reportCaughtError } from '@/lib/diagnostics'
import { ErrorNotice } from '@/components/error-notice'
import { useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { openUrl } from '@tauri-apps/plugin-opener'
import { ExternalLink, FileText, Terminal } from 'lucide-react'
import { UpdateCard } from '@/features/updates/update-provider'
import appIcon from '../../../../app-icon.png'
import { Button } from '@/components/ui/button'
import { errorText, nativeCall } from '@/lib/player'
import { SettingsCard } from '@/features/settings/settings-card'

const repository = 'https://github.com/Miaoyww/Nons'
const statistics = [
  { label: '累计使用人次', value: import.meta.env.VITE_TELEMETRY_USAGE_COUNT, unit: '人次' },
  { label: '累计播放歌曲', value: import.meta.env.VITE_TELEMETRY_PLAY_COUNT, unit: '首' }
]
const generatedAt = import.meta.env.VITE_TELEMETRY_STATS_GENERATED_AT
const statisticsDate = generatedAt?.slice(0, 10)
const hasStatistics = statistics.every(({ value }) => /^\d+$/.test(value ?? ''))
const thanks = [
  ['YesPlayMusic', 'qier222/YesPlayMusic'],
  ['SPlayer', 'SPlayer-Dev/SPlayer'],
  ['coriander_player', 'Ferry-200/coriander_player'],
  ['Apple Music-like Lyrics', 'amll-dev/applemusic-like-lyrics'],
  ['ncm-api-rs', 'SPlayer-Dev/ncm-api-rs']
]

export function AboutPage({ version }: { version: string }) {
  const [error, setError] = useState<string>()
  async function run(operation: () => Promise<unknown>) {
    setError(undefined)
    try {
      await operation()
    } catch (cause) {
      reportCaughtError('app/src/features/settings/pages/about.tsx', cause)

      setError(errorText(cause))
    }
  }
  function link(url: string, label: string, license = false) {
    return (
      <Button
        variant="outline"
        size="sm"
        onClick={() =>
          void run(() =>
            isTauri()
              ? openUrl(url)
              : Promise.resolve(window.open(url, '_blank', 'noopener,noreferrer'))
          )
        }
      >
        {license ? <FileText aria-hidden="true" /> : <ExternalLink aria-hidden="true" />}
        {label}
      </Button>
    )
  }
  return (
    <div className="flex flex-col gap-8">
      <div className="flex flex-col items-center gap-3 py-6 text-center">
        <img src={appIcon} alt="" className="size-32 rounded-2xl" draggable={false} />
        <h2 className="text-3xl font-bold tracking-tight">Nons</h2>
        <section aria-label="所有用户累计统计" className="mt-4 w-full max-w-md">
          <dl className="grid grid-cols-2 divide-x divide-border/70 border-y border-border/60 py-5">
            {statistics.map(({ label, value, unit }) => (
              <div key={label} className="flex min-w-0 flex-col-reverse gap-2 px-3">
                <dt className="text-xs text-muted-foreground">{label}</dt>
                <dd className="flex items-baseline justify-center gap-1.5">
                  <span className="text-2xl font-semibold tracking-tight tabular-nums sm:text-3xl">
                    {hasStatistics ? Number(value).toLocaleString('en-US') : '暂无统计数据'}
                  </span>
                  {hasStatistics && <span className="text-xs text-muted-foreground">{unit}</span>}
                </dd>
              </div>
            ))}
          </dl>
          <p className="mt-3 text-xs text-muted-foreground">
            {hasStatistics && statisticsDate ? `统计截至 ${statisticsDate}` : '暂无统计数据'}
          </p>
        </section>
      </div>
      <SettingsCard searchId="app-version" title="版本号" description="当前应用版本。">
        <span className="text-sm tabular-nums">v{version}</span>
      </SettingsCard>
      {import.meta.env.DEV && (
        <SettingsCard searchId="devtools" title="开发者工具" description="打开开发者工具进行调试。">
          <Button
            variant="outline"
            size="sm"
            disabled={!isTauri()}
            onClick={() => void run(() => nativeCall('open_devtools'))}
          >
            <Terminal aria-hidden="true" />
            打开开发者工具
          </Button>
        </SettingsCard>
      )}
      <UpdateCard />
      <SettingsCard searchId="license" title="开源许可" description="本项目基于 GPL-3.0 协议开源。">
        {link(`${repository}/blob/main/LICENSE`, 'GPL-3.0', true)}
      </SettingsCard>
      <SettingsCard searchId="repository" title="GitHub 仓库" description="查看源码或提交 Issue。">
        {link(repository, 'Miaoyww/Nons')}
      </SettingsCard>
      {error && <ErrorNotice error={error} />}
      <section aria-labelledby="thanks-heading" className="flex flex-col gap-4">
        <h2 id="thanks-heading" className="text-xl font-bold">
          特别鸣谢
        </h2>
        {thanks.map(([name, repo]) => (
          <SettingsCard
            key={repo}
            searchId={`thanks-${repo}`}
            title={name}
            description="致谢与敬意。"
          >
            {link(`https://github.com/${repo}`, repo)}
          </SettingsCard>
        ))}
      </section>
    </div>
  )
}
