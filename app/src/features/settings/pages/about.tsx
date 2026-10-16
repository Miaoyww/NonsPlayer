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
      </div>
      <SettingsCard title="版本号" description="当前应用版本。">
        <span className="text-sm tabular-nums">v{version}</span>
      </SettingsCard>
      {import.meta.env.DEV && (
        <SettingsCard title="开发者工具" description="打开开发者工具进行调试。">
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
      <SettingsCard title="开源许可" description="本项目基于 GPL-3.0 协议开源。">
        {link(`${repository}/blob/main/LICENSE`, 'GPL-3.0', true)}
      </SettingsCard>
      <SettingsCard title="GitHub 仓库" description="查看源码或提交 Issue。">
        {link(repository, 'Miaoyww/Nons')}
      </SettingsCard>
      {error && <ErrorNotice error={error} />}
      <section aria-labelledby="thanks-heading" className="flex flex-col gap-4">
        <h2 id="thanks-heading" className="text-xl font-bold">
          特别鸣谢
        </h2>
        {thanks.map(([name, repo]) => (
          <SettingsCard key={repo} title={name} description="致谢与敬意。">
            {link(`https://github.com/${repo}`, repo)}
          </SettingsCard>
        ))}
      </section>
    </div>
  )
}
