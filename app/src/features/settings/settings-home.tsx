import { ChevronRight } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { AccountAvatar } from '@/features/account/account-avatar'
import { useAccountAdapters } from '@/features/account/use-account-adapters'
import { useMusicSources } from '@/features/music/use-music-sources'
import { settingsSections } from './settings-sections'
import { isAndroid } from '@/lib/platform'

export function SettingsHome({
  onSelect
}: {
  onSelect: (view: 'accounts' | 'settings', query?: string) => void
}) {
  const { preferences } = useMusicSources()
  const { adapters } = useAccountAdapters()
  const adapter = adapters.find((item) => item.descriptor.source === preferences?.current)
  const account = adapter?.current
  const card = ({ id, label, icon: Icon }: (typeof settingsSections)[number]) => (
    <Button
      key={id}
      variant="secondary"
      className={`mobile-settings-category ${id === 'about' || id === 'plugins' ? 'glass-surface' : ''}`}
      onClick={() => onSelect('settings', id)}
    >
      <Icon aria-hidden="true" />
      <span className="flex-1 text-left">{label}</span>
      <ChevronRight aria-hidden="true" />
    </Button>
  )
  return (
    <div className="settings-home-content">
      <div className="settings-home-scroll">
        <section className="settings-home-group" aria-label="账号">
          <h2 className="settings-home-heading">账号</h2>
          <Button
            variant="secondary"
            className="mobile-settings-account"
            onClick={() => onSelect('accounts')}
          >
            <AccountAvatar avatar={account?.avatar} className="size-12" />
            <span className="min-w-0 flex-1 text-left">
              <span className="block truncate font-semibold">
                {account?.displayName ?? '音乐账号'}
              </span>
              <span className="mt-2 block truncate text-xs font-normal text-muted-foreground">
                {adapter ? `适配器：${adapter.descriptor.displayName}` : '尚未选择适配器'}
              </span>
            </span>
            <ChevronRight aria-hidden="true" />
          </Button>
        </section>
        <section className="settings-home-group" aria-label="一般设置">
          <h2 className="settings-home-heading">一般设置</h2>
          <nav aria-label="设置分类" className="mobile-settings-categories">
            {settingsSections
              .filter(
                (item) =>
                  item.id !== 'plugins' &&
                  item.id !== 'about' &&
                  !(isAndroid() && item.id === 'shortcuts')
              )
              .map(card)}
          </nav>
        </section>
      </div>
      <nav aria-label="插件与关于" className="settings-home-footer">
        {['about', 'plugins'].map((id) => card(settingsSections.find((item) => item.id === id)!))}
      </nav>
    </div>
  )
}
